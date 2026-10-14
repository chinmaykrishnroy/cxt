// Explicit trusted-tool paths only; never execute commands from corpus metadata.
import { createHash } from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { readFile, writeFile, mkdir, realpath } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const PIN = '55c6de6aebde4c38877fbcaf005c4cc396bf6e2d';
const options = Object.fromEntries(process.argv.slice(2).reduce((pairs, item, index, args) => {
  if (index % 2 === 0) pairs.push([item.replace(/^--/, ''), args[index + 1]]);
  return pairs;
}, []));
for (const key of ['corpus', 'repomix', 'cxt', 'output']) {
  if (!options[key]) throw new Error(`Required: --${key} PATH`);
}
const hash = value => createHash('sha256').update(value).digest('hex');
const readJson = async filename => JSON.parse(await readFile(filename, 'utf8'));
const tools = await realpath(options.repomix);
const cxt = await realpath(options.cxt);
if (execFileSync('git', ['-C', tools, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim() !== PIN) {
  throw new Error('Repomix source pin mismatch');
}
execFileSync('git', ['-C', tools, 'diff', '--quiet', 'HEAD']);
const tokenizerFile = path.join(tools, 'node_modules/gpt-tokenizer/esm/encoding/cl100k_base.js');
const { encode } = await import(pathToFileURL(tokenizerFile));
const count = text => encode(text, { allowedSpecial: new Set(), disallowedSpecial: new Set() }).length;
if (count('hello world') !== 2 || count('<|endoftext|>') !== 7) throw new Error('Tokenizer smoke failed');
const corpusPath = await realpath(options.corpus);
const corpus = await readJson(corpusPath);
if (corpus.schema !== 'cxt.eval.v1') throw new Error('Unsupported corpus');
const defaultIds = ['fd-format-hyphen', 'click-black-color-zero', 'ky-json-error-utf8',
  'afero-tar-independent-handles', 'jsoup-malformed-end-tag', 'arduinojson-long-numeric-string'];
const ids = options.tasks ? options.tasks.split(',') : defaultIds;
if (ids.length > 30 || new Set(ids).size !== ids.length) throw new Error('Invalid task list');
const tasks = ids.map(id => {
  const task = corpus.tasks.find(task => task.id === id);
  if (!task || !/^[a-z0-9-]+$/.test(id)) throw new Error(`Unknown or unsafe task: ${id}`);
  return task;
});
const output = path.resolve(options.output);
await mkdir(path.dirname(output), { recursive: true });
const artifacts = output + '.artifacts';
await mkdir(artifacts); // Refuse to overwrite a previous experiment.
const config = fileURLToPath(new URL('repomix.json', import.meta.url));
const report = {
  schema: 'cxt.native-competitor-report.v1', dataset_kind: corpus.dataset_kind,
  track: 'native focused query retrieval versus native whole-repository packing; NOT equivalent rankers',
  scoring: 'Provisional exact source-span recall. No oracle paths supplied. Overflow and nonzero exits score zero.',
  payload: 'identical Task prefix + full native JSON stdout; no clipping or budget repair',
  downstream_task_success: 'not_measured', platform: process.platform, node: process.version,
  cxt_sha256: hash(await readFile(cxt)), repomix_commit: PIN,
  node_sha256: hash(await readFile(process.execPath)),
  repomix_launcher_sha256: hash(await readFile(path.join(tools, 'bin/repomix.cjs'))),
  repomix_cli_sha256: hash(await readFile(path.join(tools, 'lib/cli/cliRun.js'))),
  repomix_lock_sha256: hash(await readFile(path.join(tools, 'package-lock.json'))),
  config_sha256: hash(await readFile(config)),
  tokenizer: { name: 'cl100k_base ordinary text', package: await readJson(path.join(tools, 'node_modules/gpt-tokenizer/package.json')),
    vocabulary_sha256: hash(await readFile(path.join(tools, 'node_modules/gpt-tokenizer/esm/bpeRanks/cl100k_base.js'))) },
  memory: 'not measured in this harness', runs: []
};
// Preserve version, but avoid embedding the complete dependency manifest in results.
report.tokenizer.package = report.tokenizer.package.version;
async function execute(command, args, cwd) {
  const started = performance.now();
  return await new Promise(resolve => {
    const child = spawn(command, args, { cwd, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'], env: { ...process.env, NO_COLOR: '1' } });
    const stdout = [], stderr = [];
    let bytes = 0, errors = 0, problem;
    const timer = setTimeout(() => { problem = '60 second timeout'; child.kill(); }, 60000);
    child.stdout.on('data', data => {
      bytes += data.length;
      if (bytes <= 32 * 1024 * 1024) stdout.push(data);
      else { problem = '32 MiB output limit'; child.kill(); }
    });
    child.stderr.on('data', data => { errors += data.length; if (errors <= 65536) stderr.push(data); });
    child.on('error', error => { problem = error.message; });
    child.on('close', code => {
      clearTimeout(timer);
      resolve({ code, problem, elapsed_ms: performance.now() - started,
        stdout: Buffer.concat(stdout).toString('utf8'), stderr: Buffer.concat(stderr).toString('utf8') });
    });
  });
}
for (const task of tasks) {
  const root = await realpath(path.resolve(path.dirname(corpusPath), task.root));
  const relativeRoot = path.relative(path.dirname(corpusPath), root);
  if (relativeRoot.startsWith('..') || path.isAbsolute(relativeRoot)) throw new Error('Input root escapes corpus');
  const labels = await Promise.all(task.required_symbols.map(async label => {
    const sourcePath = await realpath(path.join(root, label.path));
    const relative = path.relative(root, sourcePath);
    if (relative.startsWith('..') || path.isAbsolute(relative)) throw new Error('Label escapes input');
    const bytes = await readFile(sourcePath);
    if (!label.source_sha256 || hash(bytes) !== label.source_sha256) throw new Error('Source revision mismatch');
    const lines = bytes.toString('utf8').split(/(?<=\n)/);
    return { ...label, text: lines.slice(label.start_line - 1, label.end_line).join('').replace(/^[ \t]+/, '').replace(/[\r\n]+$/, '') };
  }));
  for (const budget of corpus.budgets) {
    if (![8000, 16000, 32000].includes(budget)) throw new Error('Unexpected budget');
    const prefix = `Task: ${task.query}\n\nContext:\n`;
    for (const tool of ['cxt-focused', 'repomix-native']) {
      for (const phase of ['first', 'repeat']) {
        const args = tool === 'cxt-focused'
          ? ['.', '--query', task.query, '--context', 'focused', '--model', 'cl100k', '--budget', String(budget - count(prefix)),
            '--format', 'json', '--stdout', '--no-minify', '--no-cache', '--secrets', 'fail', '--workspace-root', root]
          : [path.join(tools, 'bin/repomix.cjs'), root, '--config', config, '--style', 'json', '--parsable-style',
            '--stdout', '--token-count-encoding', 'cl100k_base', '--token-budget', String(budget)];
        const result = await execute(tool === 'cxt-focused' ? cxt : process.execPath, args, root);
        const stem = `${task.id}-${budget}-${tool}-${phase}`;
        await writeFile(path.join(artifacts, stem + '.stdout'), result.stdout);
        await writeFile(path.join(artifacts, stem + '.stderr'), result.stderr);
        const tokens = count(prefix + result.stdout);
        let files = [], parseError;
        try {
          const packet = JSON.parse(result.stdout);
          files = Array.isArray(packet.files) ? packet.files : Object.entries(packet.files).map(([path, content]) => ({ path, content }));
        } catch { parseError = 'native output was not parseable JSON'; }
        const accepted = result.code === 0 && !result.problem && !parseError && tokens <= budget;
        const find = name => files.find(file => file.path.replaceAll('\\', '/') === name.replaceAll('\\', '/'));
        const fileRecall = task.required_files.filter(name => find(name)).length / task.required_files.length;
        const spanRecall = labels.length ? labels.filter(label => {
          const file = find(label.path);
          return file && file.representation !== 'map' && typeof file.content === 'string' && file.content.includes(label.text);
        }).length / labels.length : null;
        report.runs.push({ task: task.id, label_status: task.label_status, admitted_for_scoring: false,
          tool, phase, budget, arguments: args, exit_code: result.code,
          error: result.problem || parseError || (tokens > budget ? 'complete payload exceeds exact budget' : result.code !== 0 ? 'native nonzero exit; inspect retained stderr' : null),
          accepted, final_tokens: tokens, file_recall: accepted ? fileRecall : 0,
          span_recall: accepted ? spanRecall : 0, elapsed_ms: result.elapsed_ms,
          stdout_sha256: hash(result.stdout), source_fingerprints: labels.map(({ path, source_sha256 }) => ({ path, source_sha256 })),
          artifacts: stem });
        await writeFile(output, JSON.stringify(report, null, 2) + '\n');
        console.log(`${stem}: ${accepted ? 'accepted' : 'failed'}; ${tokens} final tokens`);
      }
    }
  }
}
