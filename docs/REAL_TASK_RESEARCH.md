# Public real-task retrieval corpus research

Research date: 2026-10-14. Research-only sidecar; no agents spawned. Ownership is limited to this new file and `eval/public-tasks.json`. Existing Rust, tests, documentation and Cargo files were not edited.

## First integration handoff — partial validation, available now

Use `integration_handoff` in `eval/public-tasks.json`, batch **public-bootstrap-6-v1**, immediately; do not wait for all 30 cases to become gold. It selects these six source-checked cases with exact existing task metadata and runner-compatible labels:

| Task ID | Language | Validation delivered | Runner projection |
| --- | --- | --- | --- |
| `fd-format-hyphen` | Rust | Parent, pre-fix source/span, license; runtime not run | File recall only; field-attribute region is not a function label |
| `click-black-color-zero` | Python | Above plus two isolated before/after behavioral checks | `_interpret_color` 572–580; `style` 583–709 |
| `ky-json-error-utf8` | TS | Source checked; retrospective query; runtime not run | `#readResponseText` 860–922 |
| `afero-tar-independent-handles` | Go | Parent, source/spans, earlier issue, license; runtime not run | `New` 31–81; `Open` 83–97 |
| `jsoup-malformed-end-tag` | Java | Parent, source/span, earlier issue, license; runtime not run | `EndTagOpen` block 128–147; source-span, not callable-function recall |
| `arduinojson-long-numeric-string` | C++ | Parent, source/span, earlier issue, license; no sanitizer/device run | `make_float` 195–210 |

All exact repository/base/fix/source/license links remain in the full task records below. All six still have `admitted_for_scoring: false`: usable for exploratory integration is not independently reviewed gold. The remaining 24 candidates are not prerequisites for integrating this batch. Quarantined fixes are excluded. The reported main-agent resolver/index/impact improvements are not validated by these cases; dedicated configuration/missing-path/local-replacement/Cargo-target regressions are a separate gate, and no Rust or existing test file was changed here.

### First behavioral validation: Click, narrowly scoped

Executed on Windows with **CPython 3.14.6**, `-I -B`, using only AST-extracted `_interpret_color` and `style` definitions from the pinned official pre-fix and fix `termui.py`. An identity gettext function and typing namespace were supplied; `reset=False` avoids reset constants. No whole-module imports, dependency installation or upstream test suite ran.

| Call | Pre-fix result | Fix result |
| --- | --- | --- |
| `style("x", fg=0, reset=False)` | `"x"` (incorrect) | `"\\x1b[38;5;0mx"` (expected color prefix) |
| `style("x", bg=0, reset=False)` | `"x"` (incorrect) | `"\\x1b[48;5;0mx"` (expected color prefix) |

The combined reproducer asserted that both pre-fix results fail the expected-output check and both fix results pass; process exit code 0. This validates the isolated palette-zero symptom only, not complete repair correctness or full-checkout tests. `bug_reproduced`/`fix_tested` remain false for full-checkout validation; the new `behavioral_validation` field records the narrower positive evidence.

Sources and excerpt fingerprints:

- [Base termui.py](https://github.com/pallets/click/blob/7925a3410d7098c28cfca3b2baa6c852666bbd14/src/click/termui.py#L572-L710): excerpt lines 572–710, SHA-256 `10f75748283bfe0080a9f3cf0c30bec7ac94f04f23e6d251064aa5fcd7e3f1af`.
- [Fix termui.py](https://github.com/pallets/click/blob/07c909f23f0f83b5ca137c167b9a134d66201f67/src/click/termui.py#L572-L722): excerpt lines 572–722, SHA-256 `240ca3e45388ff335eb34c181f5353eced98b4f10615285623f084ac47034b8f`.

Excerpt extraction is `source[source.index("\ndef _interpret_color") + 1 : source.index("\ndef unstyle", start)]`, decoded UTF-8 without newline conversion. Hashes describe that excerpt, not the whole file. The successful launcher also emitted `Failed to find real location of C:\Python314\python.exe`; it is retained as a diagnostic, not suppressed. An initial attempt embedding both whole source files exceeded Windows command length (launcher error 206) before Python executed; the smaller exact-function excerpts resolved that harness limit.

### Concrete materialization and runner format

Integration update: use the checked-in [materializer](../eval/materialize_public.py) and [evaluation guide](EVALUATION.md), not the historical reference script below. The implemented runner now accepts qualified display names and checks original source spans plus optional source hashes. All 28 non-quarantined candidates have been materialized with verified parents, licenses and original bytes. On Windows the materializer overrides Git text/filter attributes for byte-preserving checkout. The initial run under a `target` ancestor exposed a cxt scanner bug and is invalid as retrieval evidence; comparative inputs were moved to a dedicated temporary directory. The sidecar's original research-only observations and reference recipe are retained below for provenance.

`integration_handoff.fetch_contract` records the original base-only depth-1 fetch proposal, detached checkout, overwrite refusal and source checks. Its projection supplied literal names for the older name-presence scorer. The implemented evaluator now accepts qualified identities such as `Fs.Open` and checks actual original spans instead; use the checked-in materializer for all 28 non-quarantined cases. The historical projection remains provisional; jsoup's enum block is a span rather than a callable symbol, while fd's flag-attribute region is not a function label.

Proposed output layout (not created by this sidecar):

```text
DEST/
  corpus.json                         # cxt.eval.v1, exploratory, successful rows only
  materialization-ledger.json         # attempts, failures, pinned source SHA-256
  inputs/
    click-black-color-zero/           # only the pre-fix shallow Git checkout
    ...
```

Copy the following **reference materializer** to a scratch script outside the repository, then run it from a fresh parent directory. This code was syntax-checked, but its Git/network/materialization path has NOT been executed here. It never fetches fix objects into the retriever input, executes upstream code, deletes/reuses a destination, or silently promotes labels to gold. A failed fetch produces a ledger entry and nonzero exit; successful earlier tasks still produce an exploratory corpus. An empty corpus is not runnable. Use an isolated Git/user environment as required below.

```bash
python materialize_public.py /absolute/path/to/eval/public-tasks.json /absolute/fresh/parent/DEST
# Or start with the one behavior-checked case:
python materialize_public.py /absolute/path/to/eval/public-tasks.json /absolute/fresh/parent/CLICK_DEST click-black-color-zero
```

```python
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath

metadata = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
handoff = metadata["integration_handoff"]
projection = handoff["projection"]
selected = sys.argv[3:] or handoff["task_ids"]
if len(set(selected)) != len(selected) or not set(selected) <= set(handoff["task_ids"]):
    raise SystemExit("Select unique IDs from integration_handoff.task_ids only")
tasks = {t["id"]: t for t in metadata["tasks"]}
destination = Path(sys.argv[2]).absolute()
destination.mkdir(parents=False, exist_ok=False)  # fresh destination only
inputs = destination / "inputs"
inputs.mkdir()
corpus_tasks, ledger = [], []

def git(root, *args):
    return subprocess.run(
        ["git", "-C", str(root), *args],
        check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=180,
    ).stdout

def checked_relative(path):
    p = PurePosixPath(path)
    if p.is_absolute() or ".." in p.parts or "\\" in path or ":" in path:
        raise ValueError("Unsafe repository-relative path")
    return p

for task_id in selected:
    row = {"id": task_id, "status": "failed"}
    try:
        task = tasks[task_id]
        if task["label_status"].startswith("quarantined"):
            raise ValueError("Quarantined task")
        base = task["base_commit"]
        repo = task["repository"]
        if not re.fullmatch(r"[a-z0-9][a-z0-9-]*", task_id):
            raise ValueError("Unsafe task ID")
        if not re.fullmatch(r"[0-9a-f]{40}", base):
            raise ValueError("Expected full commit SHA")
        if not re.fullmatch(r"https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo):
            raise ValueError("Expected public GitHub origin without credentials")
        root = inputs / task_id
        root.mkdir()
        git(root, "init", "--template=")
        git(root, "config", "core.autocrlf", "false")
        git(root, "remote", "add", "origin", repo + ".git")
        git(root, "fetch", "--depth=1", "--no-tags", "origin", base)
        if git(root, "rev-parse", "FETCH_HEAD").decode().strip() != base:
            raise ValueError("Fetched revision differs")
        entries = git(root, "ls-tree", "-rz", "--full-tree", base).split(b"\0")
        if any(e.startswith((b"120000 ", b"160000 ")) for e in entries if e):
            raise ValueError("Symlink/submodule requires separate reviewed handling")
        git(root, "checkout", "--detach", "FETCH_HEAD")
        if git(root, "rev-parse", "HEAD").decode().strip() != base:
            raise ValueError("Checkout revision differs")
        git(root, "diff", "--exit-code")

        paths = set(task["required_files"])
        paths.update(s["path"] for s in task["supporting_files"])
        paths.add(task["license"]["path"])
        blobs, sources = {}, {}
        for path in sorted(paths):
            rel = checked_relative(path)
            file = root.joinpath(*rel.parts)
            file.resolve().relative_to(root.resolve())
            blob = git(root, "show", base + ":" + path)
            if not file.is_file() or file.read_bytes() != blob:
                raise ValueError("Checkout bytes differ from pinned blob: " + path)
            sources[path] = blob
            blobs[path] = hashlib.sha256(blob).hexdigest()

        for span in task["required_source_spans"]:
            raw = sources[span["path"]]
            lines = raw.split(b"\n")
            start, end = span["start_line"], span["end_line"]
            if not 1 <= start <= end <= len(lines):
                raise ValueError("Invalid original line range")
            start_byte = sum(len(line) + 1 for line in lines[:start-1])
            end_byte = sum(len(line) + 1 for line in lines[:end])
            if end == len(lines):
                end_byte -= 1
            if (start_byte, end_byte) != (span["start_byte"], span["end_byte"]):
                raise ValueError("Original byte bounds differ")
        labels = projection["required_symbols_by_task"][task_id]
        for label in labels:
            text = sources[label["path"]].decode("utf-8")
            window = "\n".join(text.split("\n")[label["start_line"]-1:label["end_line"]])
            if label["name"] not in window:
                raise ValueError("Runner label is not literal source in its span")
        corpus_tasks.append({
            "id": task_id,
            "root": "inputs/" + task_id,
            "query": task["query"],
            "required_files": task["required_files"],
            "required_symbols": labels,
        })
        row.update(status="materialized_source_checked", base_commit=base,
                   source_sha256=blobs, gold_reviewed=False,
                   upstream_tests_run=False, fix_fetched_into_input=False)
    except Exception as exc:
        row["error"] = str(exc)
        if isinstance(exc, subprocess.CalledProcessError):
            row["stderr"] = exc.stderr.decode("utf-8", errors="replace")[-2000:]
    ledger.append(row)

corpus = {k: projection[k] for k in
          ("schema", "dataset_kind", "budgets", "contexts", "rankers", "query_expansion")}
corpus["tasks"] = corpus_tasks
(destination / "corpus.json").write_text(json.dumps(corpus, indent=2) + "\n", encoding="utf-8")
(destination / "materialization-ledger.json").write_text(
    json.dumps({"batch_id": handoff["batch_id"], "attempts": ledger,
                "retrieval_run": False, "confirmatory_gold": False}, indent=2) + "\n",
    encoding="utf-8",
)
print(json.dumps({"materialized": len(corpus_tasks), "attempted": len(selected),
                  "corpus": str(destination / "corpus.json")}))
raise SystemExit(0 if len(corpus_tasks) == len(selected) else 1)
```

After successful materialization, the integrator may point its separately verified cxt executable at `DEST/corpus.json` with `--evaluate` and save results outside `inputs/`. This sidecar does not build/run cxt, run the materializer, or assert runner acceptance. Preserve `materialization-ledger.json` alongside results and label them `public_real_partial_source_checked_exploratory`. Treat any `success` in the existing report as packet acceptance, not bug-fix success. Separate fix-parent verification can use the evaluator-only evidence recipe later in this document.

## Result and evidence boundary

30 genuine upstream bug-fix candidates were located across six public codebases, five per language. Every record has a full 40-character fix SHA and its single-parent pre-fix SHA, official commit/report/source URLs, a symptom-only query, proposed pre-fix required paths and spans, a base-revision license, supporting context, and task-specific uncertainty. These are **30 source-checked candidates, not 30 reviewed gold tasks**. There are 55 proposed original-source spans and 70 base-read supporting-file entries. Twenty queries paraphrase earlier official issue/PR reports; ten are explicitly retrospective reconstructions from official commits/tests. No paid services or downstream LLMs were used.

Checks actually performed through the GitHub connector:

- Read all 30 official commit objects; each fix has exactly one parent and the recorded base is that parent.
- Read 49 changed pre-fix source files. Compared every old-side context/deletion line in their official diff hunks with the pinned raw base file: **zero mismatches**.
- Inspected and checked bounds for all 55 proposed line spans. Names and boundaries are source-reviewed, not AST-certified.
- Read 32 existing pre-fix test/test-build files, plus each task's pinned build/dependency context; all 70 supporting entries in the manifest have a successful base read.
- Read the license at each of the 30 base revisions. Recorded six representative, non-truncated Git tree inventories.
- Inspected pinned competitor CLI definitions, build metadata and relevant budget/output code. None was installed, built, invoked or timed.

Not done: local task checkouts, full-checkout bug reproduction/fix verification, independent label review, exact output-token measurement, retrieval runs, cold/warm performance runs, memory measurements, device testing or downstream task-success trials. The narrow exception is the two isolated Click palette-zero checks in the first integration handoff above. All records have `admitted_for_scoring: false`. Two historical fixes are additionally quarantined for fix-regression concerns. Do not report this research as a completed runtime benchmark.

`eval/public-tasks.json` deliberately uses `cxt.public-task-research.v1`, NOT the executable `cxt.eval.v1` schema. It contains no materialized roots and must not be passed directly to `cxt --evaluate`. Existing evaluation semantics are described in [EVALUATION.md](EVALUATION.md); no changes to that file or the runner were made.

## Codebases and redistribution

Sizes below are summed Git blob sizes, not clone size, working-set RAM, parsed-source size or measured runtime. The exact representative base and official tree endpoint are in the manifest. All are focused libraries/CLIs rather than giant application monorepos; no performance inference follows.

| Codebase | Main language | Tasks | Repository license at inspected bases | Representative tracked blobs | Tracked bytes |
| --- | --- | ---: | --- | ---: | ---: |
| [sharkdp/fd ](https://github.com/sharkdp/fd) | Rust | 5 | MIT | 60 | 576,464 |
| [pallets/click ](https://github.com/pallets/click) | Python | 5 | BSD-3-Clause | 178 | 1,639,013 |
| [sindresorhus/ky ](https://github.com/sindresorhus/ky) | TypeScript | 5 | MIT | 70 | 914,927 |
| [spf13/afero ](https://github.com/spf13/afero) | Go | 5 | Apache-2.0 | 87 | 437,646 |
| [jhy/jsoup ](https://github.com/jhy/jsoup) | Java | 5 | MIT | 319 | 3,219,653 |
| [bblanchon/ArduinoJson ](https://github.com/bblanchon/ArduinoJson) | C++ (Arduino-compatible) | 5 | MIT | 517 | 1,725,157 |

Every task below links its own immutable base license. fd's Cargo.toml declares MIT OR Apache-2.0 at all five inspected bases; this plan selects its MIT option and cites LICENSE-MIT, rather than claiming fd is exclusively MIT licensed. Preserve the upstream copyright/license text with any redistributed source; for Apache-2.0 preserve applicable notices and mark modifications. These are repository-level license observations, not a legal clearance of every vendored fixture/dependency. Prefer fetching pinned checkouts over copying source bodies into cxt. No upstream source bodies, issue bodies, binaries or third-party fixtures are vendored by this sidecar. Task descriptions are newly written symptom paraphrases; report bodies are mutable and were not independently archived as they existed before the fix.

## Admission, leakage and scoring protocol

1. Freeze the candidate manifest before looking at any retriever output. Have an independent reviewer reproduce the symptom at base, inspect the fix and unchanged callers/config/tests, and mark required versus supporting labels with a written rationale. A touched file is evidence, not automatically necessary ground truth. Record rejected candidates, too.
2. Verify the full fix/base relationship again during materialization. Keep fix objects, diffs, new assertions, task labels, the research document and this manifest **outside** every retriever's accessible input. Give the tool only the detached pre-fix tree plus `query`. Never give required paths, fix messages, fix SHA, related solution discussion or post-fix tests as query hints.
3. Split by repository (or at least deduplicated bug family), not random commits. Click help tasks, Ky header/init tasks, fd sanitizer tasks and jsoup tokenizer tasks are correlated. A later base may include an earlier selected fix. Do not tune on one and claim the other is an independent holdout; temporal novelty is not established.
4. For ten retrospective queries, require a separate prospective-style human query review, or report them only as a separate retrospective stratum. For earlier issues, redact diagnoses/patch suggestions from the query; an issue's creation date does not prove its currently fetched body is historically unchanged. `fd-and-path-diagnostic` has a related primary-pattern report, not a direct pre-fix report of the additional-pattern bug.
5. Use original base files. Line bounds are one-based inclusive. Manifest bytes are half-open UTF-8 offsets, including the last labeled line terminator if present; compute from original bytes without CRLF, Unicode, minification or redaction normalization. Re-verify bytes with `git show BASE:path`, not a checkout that silently converts newlines. Source-body hashing must be added to actual run artifacts; it was not fabricated here.
6. Proposed function/class spans include complete existing bodies where identified. Three records use a field-attribute region, header translation unit or adapter-specialization region instead; do not miscount them as function implementations. The added `cloneDeep`, sentinel special methods, new C++ adapter and new test files cannot be pre-fix symbols. Maps/signatures alone earn no implementation-span coverage. Do not infer complete causal coverage from these 55 spans.
7. For each of 8,000 / 16,000 / 32,000 tokens, independently count the complete final model-facing payload with one frozen offline `cl100k_base` tokenizer: task text, common instructions, rendered source, paths, summaries, maps and wrappers all consume budget. Retain raw stdout, stderr, exit code, SHA-256, tokenizer implementation/version/vocabulary hash, source blob hashes, executable hash, arguments, launch directory, environment, tool config, platform and attempt number. Never substitute a tool's estimate for final rendered cost.
8. Publish every failure and oversize result, per-task recall and language/repository-macro averages, plus uncertainty/omissions. Source-span recall is not downstream repair success or relevant-token precision. Do not run downstream LLMs as part of this sidecar. Hardware cases remain hardware-unverified even if future host tests pass.

### Equal-budget comparison tracks

- Native source-packet track: run cxt, Repomix, code2prompt, Gitingest and repomap `task` in frozen documented modes against exactly the same base input. Dumping tools need not have a task-query ranker; say so. Preserve native overflow as a failed budget acceptance, not a discarded row.
- Native orientation track: evaluate repomap's enriched map separately. Count discovery of a file/symbol separately from source-body coverage; a declaration is not an implementation. Repomix compression or code2prompt entity maps, if tested, are separate frozen variants rather than substitutions made after seeing failures.
- Optional packer-only track: use the SAME label-blind deterministic selector for all packers (for example, a frozen lexical ranking of the query over all eligible base files). Materialize its selected whole files into an identical projection with the same paths and required ignore/license files. Fit by rerendering whole-file prefixes and recounting exact final tokens; report the wrapper and its time separately. Do not use gold paths, raw byte/token truncation, code-fragment corruption or an oracle-selected projection. This tests packaging, not native retrieval quality. Implementing this selector/harness is future work.

Run from an empty control directory in an isolated disposable user/container with no personal/global tool configs, no API keys and no network during retrieval. Explicitly audit repository-specific config/ignore rules and record exclusions. Use identical file eligibility where possible; otherwise report differences instead of implying parity. Source configuration must not execute arbitrary transforms. Keep dependency/tool installation and tokenizer cache population outside timed offline runs.

## Reproducible materialization plan (not executed)

Use a fresh benchmark directory, not cxt's active worktree. For each row, an evaluator reads the JSON metadata and uses these POSIX-shell steps with `REPO`, `BASE`, `FIX`, `ID` set from that record, and absolute `INPUTS`/`EVIDENCE` paths. Stop if a destination exists; do not reset/reuse a potentially dirty checkout.

```bash
test ! -e "$EVIDENCE/$ID"
git init "$EVIDENCE/$ID"
git -C "$EVIDENCE/$ID" remote add origin "$REPO.git"
git -C "$EVIDENCE/$ID" fetch --depth=2 --no-tags origin "$FIX"
test "$(git -C "$EVIDENCE/$ID" rev-parse FETCH_HEAD)" = "$FIX"
test "$(git -C "$EVIDENCE/$ID" rev-parse "$FIX^")" = "$BASE"
test "$(git -C "$EVIDENCE/$ID" rev-list --parents -n 1 "$FIX" | wc -w)" -eq 2

# Separate, base-only repository: the retriever must not access EVIDENCE.
test ! -e "$INPUTS/$ID"
git init "$INPUTS/$ID"
git -C "$INPUTS/$ID" remote add origin "$REPO.git"
git -C "$INPUTS/$ID" -c core.autocrlf=false fetch --depth=1 --no-tags origin "$BASE"
git -C "$INPUTS/$ID" -c core.autocrlf=false checkout --detach FETCH_HEAD
test "$(git -C "$INPUTS/$ID" rev-parse HEAD)" = "$BASE"
git -C "$INPUTS/$ID" diff --exit-code
```

Pre-create only the parent directories. Retain the fetched repository license and any NOTICE files; inventory symlinks/submodules and archive hashes. Never follow links outside the task workspace. No submodules are assumed pinned by this plan. For a complete reproduction, freeze language/runtime/compiler/container digests and dependencies per task, then validate base failure/fix success in separate sandboxes. Do not install/execute upstream project hooks or test code merely to perform static retrieval. For future schema conversion, materialize roots beneath the corpus directory, copy only approved labels into `cxt.eval.v1`, and generate byte hashes; do not silently drop label uncertainty or quarantine state.

## Pinned competitor recipes

These are source-checked recipes, **not executed commands or installation success claims**. Full commits below were resolved from official repository history with an observation cutoff of 2026-10-14T00:00:00Z. A package version declared in a source tree does not assert an identically named release artifact exists. Pin compiler/interpreter/OS, locked dependencies and artifact SHA-256 separately before benchmarking.

All invocation examples are POSIX-shell recipes for an isolated Linux/WSL/container harness, not commands run on this Windows sidecar. `TOOLS`, `CONTROL`, `ROOT`, `OUT` are absolute disposable benchmark paths; `B` is the tool's requested budget, `QUERY` is the symptom-only text. No generic command below points at a gold required file.

### Repomix — yamadashy/repomix

Pin: `55c6de6aebde4c38877fbcaf005c4cc396bf6e2d` ([commit](https://github.com/yamadashy/repomix/commit/55c6de6aebde4c38877fbcaf005c4cc396bf6e2d)); source declares 1.18.1, Node >=22, and includes package-lock.json. [Build metadata](https://github.com/yamadashy/repomix/blob/55c6de6aebde4c38877fbcaf005c4cc396bf6e2d/package.json).

```bash
git clone https://github.com/yamadashy/repomix.git "$TOOLS/repomix"
git -C "$TOOLS/repomix" checkout --detach 55c6de6aebde4c38877fbcaf005c4cc396bf6e2d
(cd "$TOOLS/repomix" && npm ci && npm run build)

node "$TOOLS/repomix/bin/repomix.cjs" "$ROOT" \
  --config "$CONTROL/repomix.empty.json" --style xml --stdout \
  --token-count-encoding cl100k_base --token-budget "$B" \
  > "$OUT/repomix.xml" 2> "$OUT/repomix.stderr"
```

Have the future harness create `repomix.empty.json` in CONTROL with exactly:

```json
{"input":{"processors":[]},"output":{"git":{"includeDiffs":false,"includeLogs":false}}}
```

Explicit --config wins over repository/global configuration; this avoids source-supplied executable processors/instruction paths. Leave security checking on. Do not enable --include-logs, --include-diffs, --remote, --copy or --watch. The CLI defines --stdin/--include for controlled selection and --stdout for payload-only output. [CLI](https://github.com/yamadashy/repomix/blob/55c6de6aebde4c38877fbcaf005c4cc396bf6e2d/src/cli/cliRun.ts), [config precedence](https://github.com/yamadashy/repomix/blob/55c6de6aebde4c38877fbcaf005c4cc396bf6e2d/src/config/configLoad.ts), [config schema](https://github.com/yamadashy/repomix/blob/55c6de6aebde4c38877fbcaf005c4cc396bf6e2d/src/config/configSchema.ts).

Important: --token-budget is a **post-output failure guard**, not a relevance ranker or truncating selector. An oversize output may already exist; keep it and the nonzero status. The independent exact final-payload check remains authoritative. [Budget implementation](https://github.com/yamadashy/repomix/blob/55c6de6aebde4c38877fbcaf005c4cc396bf6e2d/src/cli/cliTokenBudget.ts). Native compression can be tested as a separately named variant, not as an undisclosed budget repair.

### code2prompt — mufeedvh/code2prompt

Pin: `66585136062c04275ddd5a01c5518a4d78c4aa76` ([commit](https://github.com/mufeedvh/code2prompt/commit/66585136062c04275ddd5a01c5518a4d78c4aa76)); the CLI source package declares 4.3.0, edition 2024. Install the package from this exact source revision with its Cargo.lock; this is not a claim that a 4.3.0 release binary is published. [Package](https://github.com/mufeedvh/code2prompt/blob/66585136062c04275ddd5a01c5518a4d78c4aa76/crates/code2prompt/Cargo.toml), [lock](https://github.com/mufeedvh/code2prompt/blob/66585136062c04275ddd5a01c5518a4d78c4aa76/Cargo.lock).

```bash
cargo install --locked \
  --git https://github.com/mufeedvh/code2prompt \
  --rev 66585136062c04275ddd5a01c5518a4d78c4aa76 \
  --root "$TOOLS/code2prompt-install" code2prompt

cd "$CONTROL/empty"
"$TOOLS/code2prompt-install/bin/code2prompt" "$ROOT" \
  --encoding cl100k --sort name_asc --output-format markdown \
  --output-file - --quiet \
  > "$OUT/code2prompt.md" 2> "$OUT/code2prompt.stderr"
```

The pinned parser defines --include/--exclude (repeat per pattern), --output-file -, --quiet, --line-numbers, --sort and --encoding; it has **no final-output token-cap or query-ranking flag**. Do not invent --budget, use --token-map as a cap, or treat --token-format raw as an independently exact final-packet count. --entity-map needs the optional entity-map build feature and is excluded from the source-packet default. Do not enable --diff, --git-diff-branch, --git-log-branch or --tui. [Actual arguments](https://github.com/mufeedvh/code2prompt/blob/66585136062c04275ddd5a01c5518a4d78c4aa76/crates/code2prompt/src/args.rs), [encoding aliases](https://github.com/mufeedvh/code2prompt/blob/66585136062c04275ddd5a01c5518a4d78c4aa76/crates/code2prompt-core/src/tokenizer.rs).

Run under a fresh isolated user as well as empty CONTROL: the CLI loads .c2pconfig from its launch directory, then the user's platform config directory; configuration can override the target path. No personal config may enter a benchmark. [Loader](https://github.com/mufeedvh/code2prompt/blob/66585136062c04275ddd5a01c5518a4d78c4aa76/crates/code2prompt/src/config_loader.rs), [session/config precedence](https://github.com/mufeedvh/code2prompt/blob/66585136062c04275ddd5a01c5518a4d78c4aa76/crates/code2prompt/src/config.rs). Oversized native dumps fail external admission; a common selected-file projection belongs to the separate packer-only track.

### repomap — explicitly dotcommander/repomap

The name is ambiguous. The primary comparison here chooses the Go CLI **dotcommander/repomap**, not Aider's interactive chat and not an arbitrary package named repomap. Pin `a61d6afce25a7623e221166555680bc7b7b2895d` ([commit](https://github.com/dotcommander/repomap/commit/a61d6afce25a7623e221166555680bc7b7b2895d)). Its go.mod specifies Go 1.26 / toolchain go1.26.2 and pins tree-sitter/Go dependencies; prepare that toolchain, a suitable C/CGO environment and dependency cache before offline runs. [go.mod](https://github.com/dotcommander/repomap/blob/a61d6afce25a7623e221166555680bc7b7b2895d/go.mod), [go.sum](https://github.com/dotcommander/repomap/blob/a61d6afce25a7623e221166555680bc7b7b2895d/go.sum).

```bash
git clone https://github.com/dotcommander/repomap.git "$TOOLS/repomap"
git -C "$TOOLS/repomap" checkout --detach a61d6afce25a7623e221166555680bc7b7b2895d
(cd "$TOOLS/repomap" && go build -mod=readonly -o "$TOOLS/repomap-bin" ./cmd/repomap)

# Orientation-only track:
"$TOOLS/repomap-bin" --intent "$QUERY" --tokens "$B" --format enriched "$ROOT" \
  > "$OUT/repomap-map.md" 2> "$OUT/repomap-map.stderr"

# Separate native source/task-packet track:
"$TOOLS/repomap-bin" task "$QUERY" "$ROOT" --tokens "$B" \
  > "$OUT/repomap-task.md" 2> "$OUT/repomap-task.stderr"
```

The actual CLI has both --intent maps and a task goal/directory/--tokens command. Do not seed --consumed, --ident or gold symbol names. Leave optional semantic --calls off in the baseline, freeze whether --include-tests/--symbol-refs variants are enabled, and inventory fallback/parser availability per language. [Root parser](https://github.com/dotcommander/repomap/blob/a61d6afce25a7623e221166555680bc7b7b2895d/internal/cli/root.go), [task parser](https://github.com/dotcommander/repomap/blob/a61d6afce25a7623e221166555680bc7b7b2895d/internal/cli/task.go). Task and map modes are different outputs; never aggregate them as one run.

Important: the inspected output-budget code bounds serialized bytes by tokens * 4 and computes reported encoded tokens as ceil(bytes/4). This is **not cl100k exact counting**, even when a README says token budget. Independently count the emitted complete packet, and retain overflows. [Complete-output budget](https://github.com/dotcommander/repomap/blob/a61d6afce25a7623e221166555680bc7b7b2895d/internal/cli/output_budget.go), [selection cost estimates](https://github.com/dotcommander/repomap/blob/a61d6afce25a7623e221166555680bc7b7b2895d/budget.go).

#### Aider-style alternative, not silently substituted

If the intended competitor was **unixsysdev/repomap (repomap-tool)**, the separately inspected pin is `a12e4cb137199a535387455c46a4124cb10bfbc9`. Its CLI recipe is `repomap-tool --root "$ROOT" --context "$QUERY" --map-tokens "$B" --refresh files --force-refresh`, and --chat-file/--mention/--ident must not be given oracle labels. [Pinned CLI](https://github.com/unixsysdev/repomap/blob/a12e4cb137199a535387455c46a4124cb10bfbc9/repomap_tool/cli.py). Its model shim defaults to len(text)*0.25 estimates and does not call an LLM. [Shim](https://github.com/unixsysdev/repomap/blob/a12e4cb137199a535387455c46a4124cb10bfbc9/repomap_tool/models.py).

Concrete unresolved installation concern: the root pyproject names license.file as ../LICENSE.txt, while the inspected non-truncated Git tree has no LICENSE file; README also describes installation from a subdirectory although pyproject is at the root. Do not claim `pip install` succeeds, patch packaging silently, or conflate this code with dotcommander. Archive actual install failure/dependency resolution and obtain a correctly licensed/pinned distribution before admitting this optional alternative. [pyproject](https://github.com/unixsysdev/repomap/blob/a12e4cb137199a535387455c46a4124cb10bfbc9/pyproject.toml), [README](https://github.com/unixsysdev/repomap/blob/a12e4cb137199a535387455c46a4124cb10bfbc9/README.md), [exact tree](https://api.github.com/repos/unixsysdev/repomap/git/trees/a12e4cb137199a535387455c46a4124cb10bfbc9?recursive=1).

### Gitingest — coderamp-labs/gitingest

Pin: `4e259a02fe72115bee538271622f1234a81c8e1a` ([commit](https://github.com/coderamp-labs/gitingest/commit/4e259a02fe72115bee538271622f1234a81c8e1a)); source package declares 0.3.1. Use the official current-owner repository identity rather than an unrelated fork. The source uses version ranges and has no dependency lock in the inspected tree; source pin alone is not a reproducible Python environment. Resolve and freeze a constraints/wheel-hash set first, then retain it outside the task input. [pyproject](https://github.com/coderamp-labs/gitingest/blob/4e259a02fe72115bee538271622f1234a81c8e1a/pyproject.toml), [tree](https://api.github.com/repos/coderamp-labs/gitingest/git/trees/4e259a02fe72115bee538271622f1234a81c8e1a?recursive=1).

```bash
python -m venv "$TOOLS/gitingest-venv"
"$TOOLS/gitingest-venv/bin/python" -m pip install \
  -c "$CONTROL/gitingest.constraints.txt" \
  "git+https://github.com/coderamp-labs/gitingest.git@4e259a02fe72115bee538271622f1234a81c8e1a"

"$TOOLS/gitingest-venv/bin/gitingest" "$ROOT" \
  --max-size 10485760 --output - \
  > "$OUT/gitingest.txt" 2> "$OUT/gitingest.stderr"
```

The constraints file is a required future environment artifact, not a file created by this sidecar. Source defines repeated --include-pattern/-i and --exclude-pattern/-e, --output -, local directory input, and --max-size in **bytes per file**. There is no query ranker or final-output token cap. Crucially, -t/--token is a GitHub credential, **not a token budget**; never pass the budget there. Do not use remote URLs, --branch, private tokens or --include-submodules in timed retrieval runs. Retain separate stdout/stderr; summary/progress diagnostics use stderr in stdout mode. [Pinned CLI implementation](https://github.com/coderamp-labs/gitingest/blob/4e259a02fe72115bee538271622f1234a81c8e1a/src/gitingest/__main__.py).

## Concrete blockers and next decisions

- Shell Git network access failed here with `getaddrinfo() thread failed to start`. The GitHub connector enabled source/commit inspection, but no local clone/materialization or runtime test was substituted.
- Zero independently reviewed/full-checkout-reproduced gold tasks: the 30 records are a concrete review queue. One has narrow isolated-function before/after evidence; none is blocked from exploratory integration merely by the lack of a complete 30-case gold set. Retrospective queries, correlated fixes, broad spans and unchanged dependencies need adjudication before confirmatory recall claims.
- Two candidate fixes are quarantined below. An upstream fix commit is not proof of a correct or safe repair; preserve these exclusions in conversion to runnable evaluation data.
- Compiler/interpreter/container pins, per-task dependencies, exact source fingerprints, competitor executable hashes and all actual --help/install/build/output captures remain to be generated. Gitingest's Python dependency closure is not locked yet. Repomap's selected Go toolchain/CGO path and parser coverage are not locally verified.
- Arduino Due/ESP32 cases require their actual core/toolchain/macro environment. No hardware acceptance follows from this plan; compile-only and host-regression cases must be labeled separately from device tests.
- Fair whole-file projection fitting, independently counted final payloads and native map-vs-source scoring are planned, not implemented. No claim of cxt superiority, real-task success, latency, memory savings or equal-budget benchmark completion is supported.

## Candidate task records (evaluator-only)

Only the **Problem/query** paragraph is eligible as retrieval task text. Everything else below, including source labels, report links and fix identities, is evaluator metadata. A cited official issue can contain solution discussion; do not feed its full body automatically. The JSON is the structured counterpart of these records.

### 01. fd-format-hyphen — Hyphen-leading format templates

Repository: [sharkdp/fd](https://github.com/sharkdp/fd) · Rust.

Base: [`bb80489efb9fe2bdeabe1844feb5ee0af6fc9b1c`](https://github.com/sharkdp/fd/commit/bb80489efb9fe2bdeabe1844feb5ee0af6fc9b1c).
Fix: [`b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5`](https://github.com/sharkdp/fd/commit/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5) (2026-09-09T01:47:37Z). [Exact comparison](https://github.com/sharkdp/fd/compare/bb80489efb9fe2bdeabe1844feb5ee0af6fc9b1c...b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5). Single-parent relationship verified.

License: [MIT at base](https://github.com/sharkdp/fd/blob/bb80489efb9fe2bdeabe1844feb5ee0af6fc9b1c/LICENSE-MIT) (`LICENSE-MIT`, read).

Problem/query: Passing a Markdown-list format as the separate argument --format "- {/.}" is rejected as an option, although the equals form works. Accept the documented separate-value invocation.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/sharkdp/fd/issues/2126) (created 2026-09-07T22:22:49Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/cli.rs:481-487`](https://github.com/sharkdp/fd/blob/bb80489efb9fe2bdeabe1844feb5ee0af6fc9b1c/src/cli.rs#L481-L487) — `Opts.format` (field attributes).

Supporting files, not required labels:

- [`tests/tests.rs`](https://github.com/sharkdp/fd/blob/bb80489efb9fe2bdeabe1844feb5ee0af6fc9b1c/tests/tests.rs) — existing test or test build context; read at base.
- [`Cargo.toml`](https://github.com/sharkdp/fd/blob/bb80489efb9fe2bdeabe1844feb5ee0af6fc9b1c/Cargo.toml) — build or dependency context; read at base.

Held-out fix/test evidence: `tests/tests.rs` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Opts.format is a field with parser attributes, not a function; score source-span recall separately from full-symbol recall.

### 02. fd-error-newlines — Multiline diagnostic rendering

Repository: [sharkdp/fd](https://github.com/sharkdp/fd) · Rust.

Base: [`fe0e455feea831a055599accb5ac4eea2e3d6ba1`](https://github.com/sharkdp/fd/commit/fe0e455feea831a055599accb5ac4eea2e3d6ba1).
Fix: [`a3942dbb2507505d9fd6f899d5506c0682222acf`](https://github.com/sharkdp/fd/commit/a3942dbb2507505d9fd6f899d5506c0682222acf) (2026-08-28T04:37:42Z). [Exact comparison](https://github.com/sharkdp/fd/compare/fe0e455feea831a055599accb5ac4eea2e3d6ba1...a3942dbb2507505d9fd6f899d5506c0682222acf). Single-parent relationship verified.

License: [MIT at base](https://github.com/sharkdp/fd/blob/fe0e455feea831a055599accb5ac4eea2e3d6ba1/LICENSE-MIT) (`LICENSE-MIT`, read).

Problem/query: Running fd with a slash-containing search pattern displays literal escape text instead of line breaks in its multiline error. Preserve readable multiline diagnostics.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/sharkdp/fd/issues/2104) (created 2026-08-27T15:05:23Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/error.rs:5-9`](https://github.com/sharkdp/fd/blob/fe0e455feea831a055599accb5ac4eea2e3d6ba1/src/error.rs#L5-L9) — `print_error`.

Supporting files, not required labels:

- [`Cargo.toml`](https://github.com/sharkdp/fd/blob/fe0e455feea831a055599accb5ac4eea2e3d6ba1/Cargo.toml) — build or dependency context; read at base.

Held-out fix/test evidence: No added/test file in this patch; reproduction support is unresolved.

Status: `quarantined_fix_regression_risk`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Quarantine for repair-success experiments: this historical fix is subsequently identified by issue #2119 as reintroducing unsanitized error-path output. It remains a genuine retrieval problem, not an acceptable standalone safety fix.

### 03. fd-broken-symlink-depth — Broken symlinks and minimum depth

Repository: [sharkdp/fd](https://github.com/sharkdp/fd) · Rust.

Base: [`ee20f426ddf338ac7ead5c5f00ea49258005caaf`](https://github.com/sharkdp/fd/commit/ee20f426ddf338ac7ead5c5f00ea49258005caaf).
Fix: [`350b33acda4cbcdec7245d3c4794bc7ba6ad55db`](https://github.com/sharkdp/fd/commit/350b33acda4cbcdec7245d3c4794bc7ba6ad55db) (2026-08-10T09:30:46Z). [Exact comparison](https://github.com/sharkdp/fd/compare/ee20f426ddf338ac7ead5c5f00ea49258005caaf...350b33acda4cbcdec7245d3c4794bc7ba6ad55db). Single-parent relationship verified.

License: [MIT at base](https://github.com/sharkdp/fd/blob/ee20f426ddf338ac7ead5c5f00ea49258005caaf/LICENSE-MIT) (`LICENSE-MIT`, read).

Problem/query: A broken symlink is listed while following links, but disappears when --min-depth=0 or --min-depth=1 is added even though its depth should qualify.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/sharkdp/fd/issues/1017) (created 2022-05-12T14:54:07Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/dir_entry.rs:34-40`](https://github.com/sharkdp/fd/blob/ee20f426ddf338ac7ead5c5f00ea49258005caaf/src/dir_entry.rs#L34-L40) — `DirEntry::broken_symlink`.
- [`src/dir_entry.rs:98-103`](https://github.com/sharkdp/fd/blob/ee20f426ddf338ac7ead5c5f00ea49258005caaf/src/dir_entry.rs#L98-L103) — `DirEntry::depth`.
- [`src/walk.rs:443-614`](https://github.com/sharkdp/fd/blob/ee20f426ddf338ac7ead5c5f00ea49258005caaf/src/walk.rs#L443-L614) — `WorkerState::spawn_senders`.

Supporting files, not required labels:

- [`tests/tests.rs`](https://github.com/sharkdp/fd/blob/ee20f426ddf338ac7ead5c5f00ea49258005caaf/tests/tests.rs) — existing test or test build context; read at base.
- [`Cargo.toml`](https://github.com/sharkdp/fd/blob/ee20f426ddf338ac7ead5c5f00ea49258005caaf/Cargo.toml) — build or dependency context; read at base.

Held-out fix/test evidence: `tests/tests.rs` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Symlink setup and behavior are platform dependent; no Windows/Unix reproduction performed. The large spawn_senders span contains unrelated routing; minimal causal labeling needs review.

### 04. fd-and-path-diagnostic — Path-like additional search patterns

Repository: [sharkdp/fd](https://github.com/sharkdp/fd) · Rust.

Base: [`a8a77d21621a11dbe499f1732331448c48ce9b76`](https://github.com/sharkdp/fd/commit/a8a77d21621a11dbe499f1732331448c48ce9b76).
Fix: [`6aac84e4ae30c502d22a76c2e80183ac7306d544`](https://github.com/sharkdp/fd/commit/6aac84e4ae30c502d22a76c2e80183ac7306d544) (2026-06-26T08:02:47Z). [Exact comparison](https://github.com/sharkdp/fd/compare/a8a77d21621a11dbe499f1732331448c48ce9b76...6aac84e4ae30c502d22a76c2e80183ac7306d544). Single-parent relationship verified.

License: [MIT at base](https://github.com/sharkdp/fd/blob/a8a77d21621a11dbe499f1732331448c48ce9b76/LICENSE-MIT) (`LICENSE-MIT`, read).

Problem/query: An additional --and pattern containing a path separator silently yields no results when matching file names. The same invalid primary pattern gets an explanatory diagnostic.

Problem provenance: **retrospective official commit or fix test symptom paraphrase**. [official issue](https://github.com/sharkdp/fd/issues/1873) (created 2026-01-15T00:32:10Z; related context not direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/main.rs:166-202`](https://github.com/sharkdp/fd/blob/a8a77d21621a11dbe499f1732331448c48ce9b76/src/main.rs#L166-L202) — `ensure_search_pattern_is_not_a_path`.

Supporting files, not required labels:

- [`tests/tests.rs`](https://github.com/sharkdp/fd/blob/a8a77d21621a11dbe499f1732331448c48ce9b76/tests/tests.rs) — existing test or test build context; read at base.
- [`Cargo.toml`](https://github.com/sharkdp/fd/blob/a8a77d21621a11dbe499f1732331448c48ce9b76/Cargo.toml) — build or dependency context; read at base.

Held-out fix/test evidence: `tests/tests.rs` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Issue #1873 is related context about the primary-pattern diagnostic, not a direct report of the --and extension. Query is retrospectively reconstructed from the fix commit; do not count it as a verified historical issue query.

### 05. fd-error-path-controls — Control characters in diagnostics

Repository: [sharkdp/fd](https://github.com/sharkdp/fd) · Rust.

Base: [`b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5`](https://github.com/sharkdp/fd/commit/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5).
Fix: [`24198d904a074f2943b2a3be925e53134c994734`](https://github.com/sharkdp/fd/commit/24198d904a074f2943b2a3be925e53134c994734) (2026-09-15T08:00:58Z). [Exact comparison](https://github.com/sharkdp/fd/compare/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5...24198d904a074f2943b2a3be925e53134c994734). Single-parent relationship verified.

License: [MIT at base](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/LICENSE-MIT) (`LICENSE-MIT`, read).

Problem/query: File-system errors and command-not-found diagnostics containing unusual file names can send control sequences directly to the terminal. Keep diagnostics safe without destroying their intended multiline layout.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/sharkdp/fd/issues/2119) (created 2026-09-04T14:06:04Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/error.rs:1-3`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/src/error.rs#L1-L3) — `print_error`.
- [`src/sanitize.rs:28-46`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/src/sanitize.rs#L28-L46) — `sanitize_for_terminal`.
- [`src/sanitize.rs:49-55`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/src/sanitize.rs#L49-L55) — `maybe_sanitize`.
- [`src/exec/command.rs:101-115`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/src/exec/command.rs#L101-L115) — `handle_cmd_error`.
- [`src/exec/job.rs:11-44`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/src/exec/job.rs#L11-L44) — `job`.
- [`src/exec/job.rs:46-64`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/src/exec/job.rs#L46-L64) — `batch`.

Supporting files, not required labels:

- [`src/cli.rs`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/src/cli.rs) — changed runtime or contract context; read at base.
- [`src/main.rs`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/src/main.rs) — changed runtime or contract context; read at base.
- [`src/output.rs`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/src/output.rs) — changed runtime or contract context; read at base.
- [`src/walk.rs`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/src/walk.rs) — changed runtime or contract context; read at base.
- [`tests/testenv/mod.rs`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/tests/testenv/mod.rs) — existing test or test build context; read at base.
- [`tests/tests.rs`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/tests/tests.rs) — existing test or test build context; read at base.
- [`Cargo.toml`](https://github.com/sharkdp/fd/blob/b422e5d8c9cffaa1ae43ba68e7b97a60fb3e8ae5/Cargo.toml) — build or dependency context; read at base.

Held-out fix/test evidence: `tests/testenv/mod.rs` (file exists at base; changed assertions/content held out); `tests/tests.rs` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Broad refactor: proposed required labels cover error output, escaping and two execution paths, but cli.rs/main.rs/walk.rs/output.rs changed too. Their necessity is unresolved; no terminal escape payload was executed.

### 06. click-short-help-abbreviation — Short help with abbreviations

Repository: [pallets/click](https://github.com/pallets/click) · Python.

Base: [`6d5bac5c712195cbfa9666ba0ecbf0573cdea356`](https://github.com/pallets/click/commit/6d5bac5c712195cbfa9666ba0ecbf0573cdea356).
Fix: [`f67c2bb6c2f055c1c7a57cd46c3697fc26a7742e`](https://github.com/pallets/click/commit/f67c2bb6c2f055c1c7a57cd46c3697fc26a7742e) (2026-09-11T07:01:09Z). [Exact comparison](https://github.com/pallets/click/compare/6d5bac5c712195cbfa9666ba0ecbf0573cdea356...f67c2bb6c2f055c1c7a57cd46c3697fc26a7742e). Single-parent relationship verified.

License: [BSD-3-Clause at base](https://github.com/pallets/click/blob/6d5bac5c712195cbfa9666ba0ecbf0573cdea356/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: Command short help stops prematurely at abbreviations such as vs. or e.g. within the first sentence. Keep a useful first-sentence summary while respecting the short-help length limit.

Problem provenance: **earlier official report symptom paraphrase**. [official pull_request](https://github.com/pallets/click/pull/3865) (created 2026-09-11T06:25:50Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/click/utils.py:62-112`](https://github.com/pallets/click/blob/6d5bac5c712195cbfa9666ba0ecbf0573cdea356/src/click/utils.py#L62-L112) — `_make_default_short_help`.

Supporting files, not required labels:

- [`tests/test_utils/test_make_default_short_help.py`](https://github.com/pallets/click/blob/6d5bac5c712195cbfa9666ba0ecbf0573cdea356/tests/test_utils/test_make_default_short_help.py) — existing test or test build context; read at base.
- [`pyproject.toml`](https://github.com/pallets/click/blob/6d5bac5c712195cbfa9666ba0ecbf0573cdea356/pyproject.toml) — build or dependency context; read at base.

Held-out fix/test evidence: `tests/test_utils/test_make_default_short_help.py` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Earlier PR contains both symptom and solution discussion. Query uses only the symptom; exact historical issue-body version was not archived.

### 07. click-interrupt-race — Late interrupt during prompt abort

Repository: [pallets/click](https://github.com/pallets/click) · Python.

Base: [`08a7bc6d2a2af02268b09b3b5fd8079546c81c03`](https://github.com/pallets/click/commit/08a7bc6d2a2af02268b09b3b5fd8079546c81c03).
Fix: [`fc518e41218b447e875fe27244ae692c269bcb77`](https://github.com/pallets/click/commit/fc518e41218b447e875fe27244ae692c269bcb77) (2026-08-31T20:34:35Z). [Exact comparison](https://github.com/pallets/click/compare/08a7bc6d2a2af02268b09b3b5fd8079546c81c03...fc518e41218b447e875fe27244ae692c269bcb77). Single-parent relationship verified.

License: [BSD-3-Clause at base](https://github.com/pallets/click/blob/08a7bc6d2a2af02268b09b3b5fd8079546c81c03/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: Ctrl-C during a command prompt can produce an unhandled traceback instead of a graceful abort, depending on interrupt timing. Handle the late interrupt without changing non-standalone behavior.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/pallets/click/issues/3802) (created 2026-08-28T16:16:08Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/click/core.py:1484-1595`](https://github.com/pallets/click/blob/08a7bc6d2a2af02268b09b3b5fd8079546c81c03/src/click/core.py#L1484-L1595) — `Command.main`.

Supporting files, not required labels:

- [`pyproject.toml`](https://github.com/pallets/click/blob/08a7bc6d2a2af02268b09b3b5fd8079546c81c03/pyproject.toml) — build or dependency context; read at base.

Held-out fix/test evidence: `tests/test_abort_interrupt.py` (added by fix; absent at base)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Race reproduced only by upstream report, not here. Newly added regression file is held out and absent from the base; existing prompt/exception/stream helpers may be required supporting context.

### 08. click-sentinel-copy — Sentinel identity under serialization

Repository: [pallets/click](https://github.com/pallets/click) · Python.

Base: [`420c8fb44eeadb537cae69d2fee3796e808558dd`](https://github.com/pallets/click/commit/420c8fb44eeadb537cae69d2fee3796e808558dd).
Fix: [`f58ca3e81424a35626c8a475eb59ab95589008ce`](https://github.com/pallets/click/commit/f58ca3e81424a35626c8a475eb59ab95589008ce) (2026-08-29T06:53:55Z). [Exact comparison](https://github.com/pallets/click/compare/420c8fb44eeadb537cae69d2fee3796e808558dd...f58ca3e81424a35626c8a475eb59ab95589008ce). Single-parent relationship verified.

License: [BSD-3-Clause at base](https://github.com/pallets/click/blob/420c8fb44eeadb537cae69d2fee3796e808558dd/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: Pickling and restoring Click sentinel members fails to recover the original sentinel identity. Serialization round trips should retain the same sentinel member, without changing copy or deepcopy identity behavior.

Problem provenance: **retrospective official commit or fix test symptom paraphrase**. No earlier issue/PR located; evidence is the official commit and changed tests, if any.

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/click/_utils.py:7-19`](https://github.com/pallets/click/blob/420c8fb44eeadb537cae69d2fee3796e808558dd/src/click/_utils.py#L7-L19) — `Sentinel`.

Supporting files, not required labels:

- [`tests/test_utils/test_sentinel.py`](https://github.com/pallets/click/blob/420c8fb44eeadb537cae69d2fee3796e808558dd/tests/test_utils/test_sentinel.py) — existing test or test build context; read at base.
- [`pyproject.toml`](https://github.com/pallets/click/blob/420c8fb44eeadb537cae69d2fee3796e808558dd/pyproject.toml) — build or dependency context; read at base.

Held-out fix/test evidence: `tests/test_utils/test_sentinel.py` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Retrospective commit-and-test-derived query; no earlier issue/PR report was identified. New special methods do not exist at base, so label the existing Sentinel class, not the added methods. The fix adds explicit copy/deepcopy methods too; this research does not establish that ordinary copying itself failed at base.

### 09. click-help-parameter-collision — User parameter named help

Repository: [pallets/click](https://github.com/pallets/click) · Python.

Base: [`8a4ce842564ae94ab050062db8525196ad476c19`](https://github.com/pallets/click/commit/8a4ce842564ae94ab050062db8525196ad476c19).
Fix: [`3495fba163dd61dea837cd557095df6c21515ac8`](https://github.com/pallets/click/commit/3495fba163dd61dea837cd557095df6c21515ac8) (2026-07-09T12:04:00Z). [Exact comparison](https://github.com/pallets/click/compare/8a4ce842564ae94ab050062db8525196ad476c19...3495fba163dd61dea837cd557095df6c21515ac8). Single-parent relationship verified.

License: [BSD-3-Clause at base](https://github.com/pallets/click/blob/8a4ce842564ae94ab050062db8525196ad476c19/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: A positional parameter named help can be parsed as the automatic boolean help option, causing invalid-boolean or missing-argument errors. Such user parameters should not corrupt ordinary parsing.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/pallets/click/issues/2819) (created 2024-12-15T02:14:28Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/click/core.py:1099-1122`](https://github.com/pallets/click/blob/8a4ce842564ae94ab050062db8525196ad476c19/src/click/core.py#L1099-L1122) — `Command.get_params`.
- [`src/click/core.py:1151-1176`](https://github.com/pallets/click/blob/8a4ce842564ae94ab050062db8525196ad476c19/src/click/core.py#L1151-L1176) — `Command.get_help_option`.

Supporting files, not required labels:

- [`tests/test_basic.py`](https://github.com/pallets/click/blob/8a4ce842564ae94ab050062db8525196ad476c19/tests/test_basic.py) — existing test or test build context; read at base.
- [`tests/test_info_dict.py`](https://github.com/pallets/click/blob/8a4ce842564ae94ab050062db8525196ad476c19/tests/test_info_dict.py) — existing test or test build context; read at base.
- [`tests/test_options.py`](https://github.com/pallets/click/blob/8a4ce842564ae94ab050062db8525196ad476c19/tests/test_options.py) — existing test or test build context; read at base.
- [`pyproject.toml`](https://github.com/pallets/click/blob/8a4ce842564ae94ab050062db8525196ad476c19/pyproject.toml) — build or dependency context; read at base.

Held-out fix/test evidence: `tests/test_basic.py` (file exists at base; changed assertions/content held out); `tests/test_info_dict.py` (file exists at base; changed assertions/content held out); `tests/test_options.py` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Fix also adds warnings for other parameter-name collisions. Those are secondary scope, not independent tasks; unchanged parameter parsing/decorator helpers are not exhaustively labeled.

### 10. click-black-color-zero — Explicit palette index zero

Repository: [pallets/click](https://github.com/pallets/click) · Python.

Base: [`7925a3410d7098c28cfca3b2baa6c852666bbd14`](https://github.com/pallets/click/commit/7925a3410d7098c28cfca3b2baa6c852666bbd14).
Fix: [`07c909f23f0f83b5ca137c167b9a134d66201f67`](https://github.com/pallets/click/commit/07c909f23f0f83b5ca137c167b9a134d66201f67) (2026-07-08T14:46:39Z). [Exact comparison](https://github.com/pallets/click/compare/7925a3410d7098c28cfca3b2baa6c852666bbd14...07c909f23f0f83b5ca137c167b9a134d66201f67). Single-parent relationship verified.

License: [BSD-3-Clause at base](https://github.com/pallets/click/blob/7925a3410d7098c28cfca3b2baa6c852666bbd14/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: Calling style or secho with foreground or background palette index 0 silently omits the requested black color. Preserve explicitly requested palette colors.

Problem provenance: **earlier official report symptom paraphrase**. [official pull_request](https://github.com/pallets/click/pull/3677) (created 2026-07-08T11:21:22Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/click/termui.py:572-580`](https://github.com/pallets/click/blob/7925a3410d7098c28cfca3b2baa6c852666bbd14/src/click/termui.py#L572-L580) — `_interpret_color`.
- [`src/click/termui.py:583-709`](https://github.com/pallets/click/blob/7925a3410d7098c28cfca3b2baa6c852666bbd14/src/click/termui.py#L583-L709) — `style`.

Supporting files, not required labels:

- [`tests/test_utils/test_style.py`](https://github.com/pallets/click/blob/7925a3410d7098c28cfca3b2baa6c852666bbd14/tests/test_utils/test_style.py) — existing test or test build context; read at base.
- [`pyproject.toml`](https://github.com/pallets/click/blob/7925a3410d7098c28cfca3b2baa6c852666bbd14/pyproject.toml) — build or dependency context; read at base.

Held-out fix/test evidence: `tests/test_utils/test_style.py` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Fix additionally changes invalid-color validation. This task scores the index-zero bug, not every new validation rule; labels include the adjacent interpreter as proposed support/implementation context. Two palette-zero calls have now been reproduced in isolated extracted functions; full-checkout reproduction and independent causal review remain pending. See the first integration handoff and the manifest's behavioral_validation for the exact narrow scope.

### 11. ky-init-nested-leak — Nested request defaults leak

Repository: [sindresorhus/ky](https://github.com/sindresorhus/ky) · TypeScript.

Base: [`830ab472fa18e1ba48861b2f36214d4c20a10f5d`](https://github.com/sindresorhus/ky/commit/830ab472fa18e1ba48861b2f36214d4c20a10f5d).
Fix: [`88e24329ed447b46f73183352e6fbce800f585d1`](https://github.com/sindresorhus/ky/commit/88e24329ed447b46f73183352e6fbce800f585d1) (2026-09-12T00:12:24Z). [Exact comparison](https://github.com/sindresorhus/ky/compare/830ab472fa18e1ba48861b2f36214d4c20a10f5d...88e24329ed447b46f73183352e6fbce800f585d1). Single-parent relationship verified.

License: [MIT at base](https://github.com/sindresorhus/ky/blob/830ab472fa18e1ba48861b2f36214d4c20a10f5d/license) (`license`, read).

Problem/query: An init hook that mutates a nested JSON or context object changes the defaults observed by later requests from the same Ky instance. Requests should not unexpectedly inherit earlier hook mutations.

Problem provenance: **retrospective official commit or fix test symptom paraphrase**. No earlier issue/PR located; evidence is the official commit and changed tests, if any.

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`source/core/Ky.ts:125-141`](https://github.com/sindresorhus/ky/blob/830ab472fa18e1ba48861b2f36214d4c20a10f5d/source/core/Ky.ts#L125-L141) — `cloneInitHookOptions`.
- [`source/utils/merge.ts:91-113`](https://github.com/sindresorhus/ky/blob/830ab472fa18e1ba48861b2f36214d4c20a10f5d/source/utils/merge.ts#L91-L113) — `cloneShallow`.

Supporting files, not required labels:

- [`test/hooks.ts`](https://github.com/sindresorhus/ky/blob/830ab472fa18e1ba48861b2f36214d4c20a10f5d/test/hooks.ts) — existing test or test build context; read at base.
- [`package.json`](https://github.com/sindresorhus/ky/blob/830ab472fa18e1ba48861b2f36214d4c20a10f5d/package.json) — build or dependency context; read at base.

Held-out fix/test evidence: `test/hooks.ts` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Retrospective commit-and-test-derived query. cloneDeep is newly added and cannot be a base label; existing cloneShallow is proposed context, not proof it is uniquely required. Plain/non-plain, cyclic and shared-reference semantics need independent review.

### 12. ky-json-array-object — Stale entries after JSON overrides

Repository: [sindresorhus/ky](https://github.com/sindresorhus/ky) · TypeScript.

Base: [`854608078545442344a29a0714a50b978e6d77e9`](https://github.com/sindresorhus/ky/commit/854608078545442344a29a0714a50b978e6d77e9).
Fix: [`8cf26e0e78d7a734f5e2846bdd65b66a936c13b3`](https://github.com/sindresorhus/ky/commit/8cf26e0e78d7a734f5e2846bdd65b66a936c13b3) (2026-09-12T00:51:40Z). [Exact comparison](https://github.com/sindresorhus/ky/compare/854608078545442344a29a0714a50b978e6d77e9...8cf26e0e78d7a734f5e2846bdd65b66a936c13b3). Single-parent relationship verified.

License: [MIT at base](https://github.com/sindresorhus/ky/blob/854608078545442344a29a0714a50b978e6d77e9/license) (`license`, read).

Problem/query: Overriding nested JSON values between array and object shapes can retain stale entries from instance defaults. The outgoing JSON should reflect the override rather than combine incompatible shapes.

Problem provenance: **retrospective official commit or fix test symptom paraphrase**. No earlier issue/PR located; evidence is the official commit and changed tests, if any.

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`source/utils/merge.ts:266-397`](https://github.com/sindresorhus/ky/blob/854608078545442344a29a0714a50b978e6d77e9/source/utils/merge.ts#L266-L397) — `deepMergeInternal`.

Supporting files, not required labels:

- [`test/main.ts`](https://github.com/sindresorhus/ky/blob/854608078545442344a29a0714a50b978e6d77e9/test/main.ts) — existing test or test build context; read at base.
- [`package.json`](https://github.com/sindresorhus/ky/blob/854608078545442344a29a0714a50b978e6d77e9/package.json) — build or dependency context; read at base.

Held-out fix/test evidence: `test/main.ts` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Retrospective commit-and-test-derived query, with no earlier issue report identified. The entire deepMergeInternal is labeled; this is broad compared with the changed condition.

### 13. ky-json-error-utf8 — JSON error response decoding

Repository: [sindresorhus/ky](https://github.com/sindresorhus/ky) · TypeScript.

Base: [`3e2d81ab7c4d80f0f22741c64bdc3923bcba4427`](https://github.com/sindresorhus/ky/commit/3e2d81ab7c4d80f0f22741c64bdc3923bcba4427).
Fix: [`854608078545442344a29a0714a50b978e6d77e9`](https://github.com/sindresorhus/ky/commit/854608078545442344a29a0714a50b978e6d77e9) (2026-09-12T00:47:42Z). [Exact comparison](https://github.com/sindresorhus/ky/compare/3e2d81ab7c4d80f0f22741c64bdc3923bcba4427...854608078545442344a29a0714a50b978e6d77e9). Single-parent relationship verified.

License: [MIT at base](https://github.com/sindresorhus/ky/blob/3e2d81ab7c4d80f0f22741c64bdc3923bcba4427/license) (`license`, read).

Problem/query: A JSON error response containing non-ASCII text is decoded incorrectly when its Content-Type declares a conflicting charset. JSON error data should retain the original UTF-8 text.

Problem provenance: **retrospective official commit or fix test symptom paraphrase**. No earlier issue/PR located; evidence is the official commit and changed tests, if any.

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`source/core/Ky.ts:860-922`](https://github.com/sindresorhus/ky/blob/3e2d81ab7c4d80f0f22741c64bdc3923bcba4427/source/core/Ky.ts#L860-L922) — `#readResponseText`.

Supporting files, not required labels:

- [`test/http-error.ts`](https://github.com/sindresorhus/ky/blob/3e2d81ab7c4d80f0f22741c64bdc3923bcba4427/test/http-error.ts) — existing test or test build context; read at base.
- [`package.json`](https://github.com/sindresorhus/ky/blob/3e2d81ab7c4d80f0f22741c64bdc3923bcba4427/package.json) — build or dependency context; read at base.

Held-out fix/test evidence: `test/http-error.ts` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Retrospective commit-and-test-derived query. Other text response charset behavior must stay distinct; unchanged decoder and content-type helpers may be necessary support.

### 14. ky-init-missing-headers — Header mutation without initial headers

Repository: [sindresorhus/ky](https://github.com/sindresorhus/ky) · TypeScript.

Base: [`d5bacefa0bf46315aa9964f7df02726bee3c3469`](https://github.com/sindresorhus/ky/commit/d5bacefa0bf46315aa9964f7df02726bee3c3469).
Fix: [`0e2501bc9dbb7da2ed46d51504873a5a7fda1c40`](https://github.com/sindresorhus/ky/commit/0e2501bc9dbb7da2ed46d51504873a5a7fda1c40) (2026-09-12T00:12:24Z). [Exact comparison](https://github.com/sindresorhus/ky/compare/d5bacefa0bf46315aa9964f7df02726bee3c3469...0e2501bc9dbb7da2ed46d51504873a5a7fda1c40). Single-parent relationship verified.

License: [MIT at base](https://github.com/sindresorhus/ky/blob/d5bacefa0bf46315aa9964f7df02726bee3c3469/license) (`license`, read).

Problem/query: An init hook that adds a header in place throws when the request and defaults did not supply headers. The hook should receive the documented usable headers object.

Problem provenance: **retrospective official commit or fix test symptom paraphrase**. No earlier issue/PR located; evidence is the official commit and changed tests, if any.

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`source/core/Ky.ts:125-140`](https://github.com/sindresorhus/ky/blob/d5bacefa0bf46315aa9964f7df02726bee3c3469/source/core/Ky.ts#L125-L140) — `cloneInitHookOptions`.

Supporting files, not required labels:

- [`test/hooks.ts`](https://github.com/sindresorhus/ky/blob/d5bacefa0bf46315aa9964f7df02726bee3c3469/test/hooks.ts) — existing test or test build context; read at base.
- [`package.json`](https://github.com/sindresorhus/ky/blob/d5bacefa0bf46315aa9964f7df02726bee3c3469/package.json) — build or dependency context; read at base.

Held-out fix/test evidence: `test/hooks.ts` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Retrospective commit-and-test-derived query. Hook contracts and default option merging are supporting context; no runtime request was made.

### 15. ky-request-header-removal — Removing inherited request headers

Repository: [sindresorhus/ky](https://github.com/sindresorhus/ky) · TypeScript.

Base: [`91ea8a947747b406273df45777a62f2d91fef5c3`](https://github.com/sindresorhus/ky/commit/91ea8a947747b406273df45777a62f2d91fef5c3).
Fix: [`d5bacefa0bf46315aa9964f7df02726bee3c3469`](https://github.com/sindresorhus/ky/commit/d5bacefa0bf46315aa9964f7df02726bee3c3469) (2026-09-12T00:12:24Z). [Exact comparison](https://github.com/sindresorhus/ky/compare/91ea8a947747b406273df45777a62f2d91fef5c3...d5bacefa0bf46315aa9964f7df02726bee3c3469). Single-parent relationship verified.

License: [MIT at base](https://github.com/sindresorhus/ky/blob/91ea8a947747b406273df45777a62f2d91fef5c3/license) (`license`, read).

Problem/query: Setting a header to undefined in Ky options does not reliably remove a header inherited from a Request input. Header overrides should work across the documented header container forms.

Problem provenance: **retrospective official commit or fix test symptom paraphrase**. No earlier issue/PR located; evidence is the official commit and changed tests, if any.

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`source/utils/merge.ts:64-92`](https://github.com/sindresorhus/ky/blob/91ea8a947747b406273df45777a62f2d91fef5c3/source/utils/merge.ts#L64-L92) — `mergeHeaders`.
- [`source/utils/merge.ts:135-147`](https://github.com/sindresorhus/ky/blob/91ea8a947747b406273df45777a62f2d91fef5c3/source/utils/merge.ts#L135-L147) — `mergeHeaderObjects`.
- [`source/utils/merge.ts:149-155`](https://github.com/sindresorhus/ky/blob/91ea8a947747b406273df45777a62f2d91fef5c3/source/utils/merge.ts#L149-L155) — `mergeHeaderContainers`.

Supporting files, not required labels:

- [`source/core/Ky.ts`](https://github.com/sindresorhus/ky/blob/91ea8a947747b406273df45777a62f2d91fef5c3/source/core/Ky.ts) — changed runtime or contract context; read at base.
- [`source/types/hooks.ts`](https://github.com/sindresorhus/ky/blob/91ea8a947747b406273df45777a62f2d91fef5c3/source/types/hooks.ts) — changed runtime or contract context; read at base.
- [`source/types/ky.ts`](https://github.com/sindresorhus/ky/blob/91ea8a947747b406273df45777a62f2d91fef5c3/source/types/ky.ts) — changed runtime or contract context; read at base.
- [`source/types/options.ts`](https://github.com/sindresorhus/ky/blob/91ea8a947747b406273df45777a62f2d91fef5c3/source/types/options.ts) — changed runtime or contract context; read at base.
- [`test/headers.ts`](https://github.com/sindresorhus/ky/blob/91ea8a947747b406273df45777a62f2d91fef5c3/test/headers.ts) — existing test or test build context; read at base.
- [`test/hooks.ts`](https://github.com/sindresorhus/ky/blob/91ea8a947747b406273df45777a62f2d91fef5c3/test/hooks.ts) — existing test or test build context; read at base.
- [`package.json`](https://github.com/sindresorhus/ky/blob/91ea8a947747b406273df45777a62f2d91fef5c3/package.json) — build or dependency context; read at base.

Held-out fix/test evidence: `test/headers.ts` (file exists at base; changed assertions/content held out); `test/hooks.ts` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Retrospective commit-and-test-derived query. Patch changes docs/types and a nonfunctional comment in Ky.ts; those are supporting rather than required runtime labels. Header casing and duplicate-header behavior enlarge the unresolved label scope.

### 16. afero-regexp-create — Creating a matching filtered file

Repository: [spf13/afero](https://github.com/spf13/afero) · Go.

Base: [`768f1fb0e5535b77d90e44c531aacd652aabd96a`](https://github.com/spf13/afero/commit/768f1fb0e5535b77d90e44c531aacd652aabd96a).
Fix: [`36101d9aec5096ec864594105467cc6823b54981`](https://github.com/spf13/afero/commit/36101d9aec5096ec864594105467cc6823b54981) (2026-09-22T22:04:18Z). [Exact comparison](https://github.com/spf13/afero/compare/768f1fb0e5535b77d90e44c531aacd652aabd96a...36101d9aec5096ec864594105467cc6823b54981). Single-parent relationship verified.

License: [Apache-2.0 at base](https://github.com/spf13/afero/blob/768f1fb0e5535b77d90e44c531aacd652aabd96a/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: Opening a missing file through RegexpFs with O_CREATE fails even when its name matches the filter. Matching-file creation should work while disallowed names and real file-system errors remain rejected.

Problem provenance: **retrospective official commit or fix test symptom paraphrase**. No earlier issue/PR located; evidence is the official commit and changed tests, if any.

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`regexpfs.go:118-123`](https://github.com/spf13/afero/blob/768f1fb0e5535b77d90e44c531aacd652aabd96a/regexpfs.go#L118-L123) — `RegexpFs.OpenFile`.

Supporting files, not required labels:

- [`go.mod`](https://github.com/spf13/afero/blob/768f1fb0e5535b77d90e44c531aacd652aabd96a/go.mod) — build or dependency context; read at base.

Held-out fix/test evidence: `regexpfs_test.go` (added by fix; absent at base)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Retrospective commit-and-test-derived query. regexpfs_test.go is newly added and held out, not present in the retrieval checkout. No independent create/exclusive/truncate regression execution.

### 17. afero-regexp-basename — Name filter under a directory

Repository: [spf13/afero](https://github.com/spf13/afero) · Go.

Base: [`defe436bfc75a5d3a118bc66032192a0e55e09bd`](https://github.com/spf13/afero/commit/defe436bfc75a5d3a118bc66032192a0e55e09bd).
Fix: [`37820ac908734fda1cb8b8d52097035befc858af`](https://github.com/spf13/afero/commit/37820ac908734fda1cb8b8d52097035befc858af) (2026-05-07T18:15:41Z). [Exact comparison](https://github.com/spf13/afero/compare/defe436bfc75a5d3a118bc66032192a0e55e09bd...37820ac908734fda1cb8b8d52097035befc858af). Single-parent relationship verified.

License: [Apache-2.0 at base](https://github.com/spf13/afero/blob/defe436bfc75a5d3a118bc66032192a0e55e09bd/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: A RegexpFs filter intended to match a file name, such as ^a, rejects a matching file when it is opened under a directory path. Apply the documented file-name filtering semantics consistently.

Problem provenance: **retrospective official commit or fix test symptom paraphrase**. No earlier issue/PR located; evidence is the official commit and changed tests, if any.

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`regexpfs.go:27-35`](https://github.com/spf13/afero/blob/defe436bfc75a5d3a118bc66032192a0e55e09bd/regexpfs.go#L27-L35) — `RegexpFs.matchesName`.

Supporting files, not required labels:

- [`go.mod`](https://github.com/spf13/afero/blob/defe436bfc75a5d3a118bc66032192a0e55e09bd/go.mod) — build or dependency context; read at base.

Held-out fix/test evidence: No added/test file in this patch; reproduction support is unresolved.

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Retrospective commit-derived query; the patch contains no regression tests. README contract and separator/platform cases require follow-up validation.

### 18. afero-close-errors — Writable file close errors

Repository: [spf13/afero](https://github.com/spf13/afero) · Go.

Base: [`26af649ce6bdf695e9050385c64c554a311599b0`](https://github.com/spf13/afero/commit/26af649ce6bdf695e9050385c64c554a311599b0).
Fix: [`defe436bfc75a5d3a118bc66032192a0e55e09bd`](https://github.com/spf13/afero/commit/defe436bfc75a5d3a118bc66032192a0e55e09bd) (2026-05-07T18:14:10Z). [Exact comparison](https://github.com/spf13/afero/compare/26af649ce6bdf695e9050385c64c554a311599b0...defe436bfc75a5d3a118bc66032192a0e55e09bd). Single-parent relationship verified.

License: [Apache-2.0 at base](https://github.com/spf13/afero/blob/26af649ce6bdf695e9050385c64c554a311599b0/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: Writable file close failures may be silently discarded even though closing can flush buffered writes. Callers of union files and reader-to-file helpers need to observe failed completion.

Problem provenance: **earlier official report symptom paraphrase**. [official pull_request](https://github.com/spf13/afero/pull/339) (created 2022-02-06T10:25:57Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`unionFile.go:31-42`](https://github.com/spf13/afero/blob/26af649ce6bdf695e9050385c64c554a311599b0/unionFile.go#L31-L42) — `UnionFile.Close`.
- [`util.go:41-62`](https://github.com/spf13/afero/blob/26af649ce6bdf695e9050385c64c554a311599b0/util.go#L41-L62) — `WriteReader`.
- [`util.go:69-96`](https://github.com/spf13/afero/blob/26af649ce6bdf695e9050385c64c554a311599b0/util.go#L69-L96) — `SafeWriteReader`.

Supporting files, not required labels:

- [`go.mod`](https://github.com/spf13/afero/blob/26af649ce6bdf695e9050385c64c554a311599b0/go.mod) — build or dependency context; read at base.

Held-out fix/test evidence: No added/test file in this patch; reproduction support is unresolved.

Status: `quarantined_fix_regression_risk`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Quarantine: static inspection of this fix's UnionFile.Close shows BADFD is returned when both closes succeed; util.go also changes named error-return plumbing. Fix completeness and compilation were not tested. Do not treat this historical fix as a passing repair oracle.

### 19. afero-readdir-mutation — Chunked reads while a directory changes

Repository: [spf13/afero](https://github.com/spf13/afero) · Go.

Base: [`5c5e03d31a90a3b98af0b953daaa93ef06fff6fa`](https://github.com/spf13/afero/commit/5c5e03d31a90a3b98af0b953daaa93ef06fff6fa).
Fix: [`e8875b6404002be03a1701f0cfa150e4593aea3d`](https://github.com/spf13/afero/commit/e8875b6404002be03a1701f0cfa150e4593aea3d) (2026-05-07T18:12:22Z). [Exact comparison](https://github.com/spf13/afero/compare/5c5e03d31a90a3b98af0b953daaa93ef06fff6fa...e8875b6404002be03a1701f0cfa150e4593aea3d). Single-parent relationship verified.

License: [Apache-2.0 at base](https://github.com/spf13/afero/blob/5c5e03d31a90a3b98af0b953daaa93ef06fff6fa/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: Removing files between chunked MemMapFs directory reads can skip entries or violate slice bounds. A directory traversal should not fail merely because entries change between reads.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/spf13/afero/issues/261) (created 2020-08-20T00:16:46Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`mem/file.go:154-187`](https://github.com/spf13/afero/blob/5c5e03d31a90a3b98af0b953daaa93ef06fff6fa/mem/file.go#L154-L187) — `File.Readdir`.

Supporting files, not required labels:

- [`go.mod`](https://github.com/spf13/afero/blob/5c5e03d31a90a3b98af0b953daaa93ef06fff6fa/go.mod) — build or dependency context; read at base.

Held-out fix/test evidence: No added/test file in this patch; reproduction support is unresolved.

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Upstream issue is older than the fix; the fix adds no tests. Required span is Readdir, but File.Open/reset state and memDir.Files behavior remain possible additional labels.

### 20. afero-tar-independent-handles — Independent tar file handles

Repository: [spf13/afero](https://github.com/spf13/afero) · Go.

Base: [`7511d5e706648e0fd974ba6305ecd0aa30ede1c5`](https://github.com/spf13/afero/commit/7511d5e706648e0fd974ba6305ecd0aa30ede1c5).
Fix: [`22177ff6c4f5183fd7050c2d0d4c9dfd053afb67`](https://github.com/spf13/afero/commit/22177ff6c4f5183fd7050c2d0d4c9dfd053afb67) (2026-05-07T18:07:49Z). [Exact comparison](https://github.com/spf13/afero/compare/7511d5e706648e0fd974ba6305ecd0aa30ede1c5...22177ff6c4f5183fd7050c2d0d4c9dfd053afb67). Single-parent relationship verified.

License: [Apache-2.0 at base](https://github.com/spf13/afero/blob/7511d5e706648e0fd974ba6305ecd0aa30ede1c5/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: Reading or closing one handle for a tarfs file affects later or simultaneous handles to the same path. Independent opens should have independent read positions and remain readable.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/spf13/afero/issues/486) (created 2025-04-02T20:30:37Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`tarfs/fs.go:31-81`](https://github.com/spf13/afero/blob/7511d5e706648e0fd974ba6305ecd0aa30ede1c5/tarfs/fs.go#L31-L81) — `New`.
- [`tarfs/fs.go:83-97`](https://github.com/spf13/afero/blob/7511d5e706648e0fd974ba6305ecd0aa30ede1c5/tarfs/fs.go#L83-L97) — `Fs.Open`.

Supporting files, not required labels:

- [`go.mod`](https://github.com/spf13/afero/blob/7511d5e706648e0fd974ba6305ecd0aa30ede1c5/go.mod) — build or dependency context; read at base.

Held-out fix/test evidence: No added/test file in this patch; reproduction support is unresolved.

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Existing tarfs/tarfs_test.go is available at base; its new assertions are held out. Unchanged tarfs/file.go is important potential supporting context and needs explicit review before full causal-span scoring.

### 21. jsoup-tracking-formatting — Source tracking changes parsed structure

Repository: [jhy/jsoup](https://github.com/jhy/jsoup) · Java.

Base: [`ea6d5bae43ada574e650814690ffa3622416daa6`](https://github.com/jhy/jsoup/commit/ea6d5bae43ada574e650814690ffa3622416daa6).
Fix: [`118acb619ea8ad6a8dbcd829edda9d4e1c0f3dcf`](https://github.com/jhy/jsoup/commit/118acb619ea8ad6a8dbcd829edda9d4e1c0f3dcf) (2026-09-28T07:05:52Z). [Exact comparison](https://github.com/jhy/jsoup/compare/ea6d5bae43ada574e650814690ffa3622416daa6...118acb619ea8ad6a8dbcd829edda9d4e1c0f3dcf). Single-parent relationship verified.

License: [MIT at base](https://github.com/jhy/jsoup/blob/ea6d5bae43ada574e650814690ffa3622416daa6/LICENSE) (`LICENSE`, read).

Problem/query: Parsing repeated formatting tags with source-position tracking enabled reconstructs more active formatting elements than the same parse without tracking. Tracking should not change the resulting DOM structure.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/jhy/jsoup/issues/2633) (created 2026-09-28T07:03:38Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/main/java/org/jsoup/nodes/Attributes.java:605-619`](https://github.com/jhy/jsoup/blob/ea6d5bae43ada574e650814690ffa3622416daa6/src/main/java/org/jsoup/nodes/Attributes.java#L605-L619) — `Attributes.equals`.
- [`src/main/java/org/jsoup/nodes/Attributes.java:626-631`](https://github.com/jhy/jsoup/blob/ea6d5bae43ada574e650814690ffa3622416daa6/src/main/java/org/jsoup/nodes/Attributes.java#L626-L631) — `Attributes.hashCode`.

Supporting files, not required labels:

- [`src/test/java/org/jsoup/nodes/AttributesTest.java`](https://github.com/jhy/jsoup/blob/ea6d5bae43ada574e650814690ffa3622416daa6/src/test/java/org/jsoup/nodes/AttributesTest.java) — existing test or test build context; read at base.
- [`src/test/java/org/jsoup/parser/HtmlParserTest.java`](https://github.com/jhy/jsoup/blob/ea6d5bae43ada574e650814690ffa3622416daa6/src/test/java/org/jsoup/parser/HtmlParserTest.java) — existing test or test build context; read at base.
- [`pom.xml`](https://github.com/jhy/jsoup/blob/ea6d5bae43ada574e650814690ffa3622416daa6/pom.xml) — build or dependency context; read at base.

Held-out fix/test evidence: `src/test/java/org/jsoup/nodes/AttributesTest.java` (file exists at base; changed assertions/content held out); `src/test/java/org/jsoup/parser/HtmlParserTest.java` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Query deliberately omits issue-body diagnosis and fix symbols. Labels infer Attributes equality/hash behavior from the patch; unchanged adoption-algorithm callers may be necessary supporting code.

### 22. jsoup-malformed-end-tag — Malformed end-tag comment content

Repository: [jhy/jsoup](https://github.com/jhy/jsoup) · Java.

Base: [`37aea49902972cec9a53dc2c65023729f1c3715b`](https://github.com/jhy/jsoup/commit/37aea49902972cec9a53dc2c65023729f1c3715b).
Fix: [`81718491972c21a911c5964b2cae06fdc201dfb3`](https://github.com/jhy/jsoup/commit/81718491972c21a911c5964b2cae06fdc201dfb3) (2026-09-27T04:44:59Z). [Exact comparison](https://github.com/jhy/jsoup/compare/37aea49902972cec9a53dc2c65023729f1c3715b...81718491972c21a911c5964b2cae06fdc201dfb3). Single-parent relationship verified.

License: [MIT at base](https://github.com/jhy/jsoup/blob/37aea49902972cec9a53dc2c65023729f1c3715b/LICENSE) (`LICENSE`, read).

Problem/query: The malformed HTML end tag </ COMMENT > creates a comment containing an extra slash. Its recovered comment content should agree with the documented parsing expectation.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/jhy/jsoup/issues/2631) (created 2026-09-27T04:42:42Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/main/java/org/jsoup/parser/TokeniserState.java:128-147`](https://github.com/jhy/jsoup/blob/37aea49902972cec9a53dc2c65023729f1c3715b/src/main/java/org/jsoup/parser/TokeniserState.java#L128-L147) — `EndTagOpen` (enum state).

Supporting files, not required labels:

- [`src/test/java/org/jsoup/parser/HtmlParserTest.java`](https://github.com/jhy/jsoup/blob/37aea49902972cec9a53dc2c65023729f1c3715b/src/test/java/org/jsoup/parser/HtmlParserTest.java) — existing test or test build context; read at base.
- [`src/test/java/org/jsoup/parser/TokeniserStateTest.java`](https://github.com/jhy/jsoup/blob/37aea49902972cec9a53dc2c65023729f1c3715b/src/test/java/org/jsoup/parser/TokeniserStateTest.java) — existing test or test build context; read at base.
- [`pom.xml`](https://github.com/jhy/jsoup/blob/37aea49902972cec9a53dc2c65023729f1c3715b/pom.xml) — build or dependency context; read at base.

Held-out fix/test evidence: `src/test/java/org/jsoup/parser/HtmlParserTest.java` (file exists at base; changed assertions/content held out); `src/test/java/org/jsoup/parser/TokeniserStateTest.java` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Label is the EndTagOpen enum-state block including read, not every read overload in TokeniserState. HTML and XML behavior should not be conflated.

### 23. jsoup-node-self-insert — Inserting a node next to itself

Repository: [jhy/jsoup](https://github.com/jhy/jsoup) · Java.

Base: [`5fe93bffc080529027d051679cb7f5cc3a24cf0d`](https://github.com/jhy/jsoup/commit/5fe93bffc080529027d051679cb7f5cc3a24cf0d).
Fix: [`12390349fab508968ca1a7309f3149d7e9f09029`](https://github.com/jhy/jsoup/commit/12390349fab508968ca1a7309f3149d7e9f09029) (2026-09-24T02:33:26Z). [Exact comparison](https://github.com/jhy/jsoup/compare/5fe93bffc080529027d051679cb7f5cc3a24cf0d...12390349fab508968ca1a7309f3149d7e9f09029). Single-parent relationship verified.

License: [MIT at base](https://github.com/jhy/jsoup/blob/5fe93bffc080529027d051679cb7f5cc3a24cf0d/LICENSE) (`LICENSE`, read).

Problem/query: Calling before or after with the receiver node itself throws a NullPointerException and can detach the node from its document. The operation must not unexpectedly mutate the document or lose the node.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/jhy/jsoup/issues/2628) (created 2026-09-24T01:54:23Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/main/java/org/jsoup/nodes/Node.java:403-412`](https://github.com/jhy/jsoup/blob/5fe93bffc080529027d051679cb7f5cc3a24cf0d/src/main/java/org/jsoup/nodes/Node.java#L403-L412) — `Node.before(Node)`.
- [`src/main/java/org/jsoup/nodes/Node.java:431-440`](https://github.com/jhy/jsoup/blob/5fe93bffc080529027d051679cb7f5cc3a24cf0d/src/main/java/org/jsoup/nodes/Node.java#L431-L440) — `Node.after(Node)`.

Supporting files, not required labels:

- [`src/test/java/org/jsoup/nodes/NodeTest.java`](https://github.com/jhy/jsoup/blob/5fe93bffc080529027d051679cb7f5cc3a24cf0d/src/test/java/org/jsoup/nodes/NodeTest.java) — existing test or test build context; read at base.
- [`pom.xml`](https://github.com/jhy/jsoup/blob/5fe93bffc080529027d051679cb7f5cc3a24cf0d/pom.xml) — build or dependency context; read at base.

Held-out fix/test evidence: `src/test/java/org/jsoup/nodes/NodeTest.java` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Existing NodeTest is available; self-insert assertions are held out. Root/detached-node behavior and validation order are not fully specified by this task.

### 24. jsoup-script-double-escape — Premature script text termination

Repository: [jhy/jsoup](https://github.com/jhy/jsoup) · Java.

Base: [`49a15317317970a7ea3f0a5ded303ef319860f4a`](https://github.com/jhy/jsoup/commit/49a15317317970a7ea3f0a5ded303ef319860f4a).
Fix: [`f79f21d251453fcf9efc1fe98ca6878e25df5bfc`](https://github.com/jhy/jsoup/commit/f79f21d251453fcf9efc1fe98ca6878e25df5bfc) (2026-09-24T01:37:34Z). [Exact comparison](https://github.com/jhy/jsoup/compare/49a15317317970a7ea3f0a5ded303ef319860f4a...f79f21d251453fcf9efc1fe98ca6878e25df5bfc). Single-parent relationship verified.

License: [MIT at base](https://github.com/jhy/jsoup/blob/49a15317317970a7ea3f0a5ded303ef319860f4a/LICENSE) (`LICENSE`, read).

Problem/query: A script in the body containing a comment opener and nested script marker closes at an inner end marker, leaving trailing script text outside. Preserve the intended script text, including mixed-case marker variants.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/jhy/jsoup/issues/2627) (created 2026-09-24T01:34:58Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/main/java/org/jsoup/parser/HtmlTreeBuilderState.java:289-647`](https://github.com/jhy/jsoup/blob/49a15317317970a7ea3f0a5ded303ef319860f4a/src/main/java/org/jsoup/parser/HtmlTreeBuilderState.java#L289-L647) — `inBodyStartTag`.
- [`src/main/java/org/jsoup/parser/TokeniserState.java:361-376`](https://github.com/jhy/jsoup/blob/49a15317317970a7ea3f0a5ded303ef319860f4a/src/main/java/org/jsoup/parser/TokeniserState.java#L361-L376) — `ScriptDataEscapedLessthanSign` (enum state).
- [`src/main/java/org/jsoup/parser/TokeniserState.java:1838-1865`](https://github.com/jhy/jsoup/blob/49a15317317970a7ea3f0a5ded303ef319860f4a/src/main/java/org/jsoup/parser/TokeniserState.java#L1838-L1865) — `handleDataDoubleEscapeTag`.

Supporting files, not required labels:

- [`src/test/java/org/jsoup/parser/HtmlTreeBuilderTest.java`](https://github.com/jhy/jsoup/blob/49a15317317970a7ea3f0a5ded303ef319860f4a/src/test/java/org/jsoup/parser/HtmlTreeBuilderTest.java) — existing test or test build context; read at base.
- [`src/test/java/org/jsoup/parser/TokeniserStateTest.java`](https://github.com/jhy/jsoup/blob/49a15317317970a7ea3f0a5ded303ef319860f4a/src/test/java/org/jsoup/parser/TokeniserStateTest.java) — existing test or test build context; read at base.
- [`pom.xml`](https://github.com/jhy/jsoup/blob/49a15317317970a7ea3f0a5ded303ef319860f4a/pom.xml) — build or dependency context; read at base.

Held-out fix/test evidence: `src/test/java/org/jsoup/parser/HtmlTreeBuilderTest.java` (file exists at base; changed assertions/content held out); `src/test/java/org/jsoup/parser/TokeniserStateTest.java` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Two source files and one large dispatch method are required candidates. Query draws body case from earlier issue and mixed-case symptom from the fix's tests; the mixed-case extension is retrospective.

### 25. jsoup-void-children — Serializing manipulated void elements

Repository: [jhy/jsoup](https://github.com/jhy/jsoup) · Java.

Base: [`387c6e1234bbcd175e371774b2f4d5962c07f1c0`](https://github.com/jhy/jsoup/commit/387c6e1234bbcd175e371774b2f4d5962c07f1c0).
Fix: [`b523b7aa713c70dcbbde1ce58ac2a60966a4d892`](https://github.com/jhy/jsoup/commit/b523b7aa713c70dcbbde1ce58ac2a60966a4d892) (2026-09-23T06:53:05Z). [Exact comparison](https://github.com/jhy/jsoup/compare/387c6e1234bbcd175e371774b2f4d5962c07f1c0...b523b7aa713c70dcbbde1ce58ac2a60966a4d892). Single-parent relationship verified.

License: [MIT at base](https://github.com/jhy/jsoup/blob/387c6e1234bbcd175e371774b2f4d5962c07f1c0/LICENSE) (`LICENSE`, read).

Problem/query: After DOM manipulation adds children to an HTML void element, serializing HTML emits those children and inconsistent markup. Serialization should respect void-element behavior without losing children from similarly named foreign elements.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/jhy/jsoup/issues/2619) (created 2026-09-23T06:51:00Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/main/java/org/jsoup/nodes/Element.java:1999-2016`](https://github.com/jhy/jsoup/blob/387c6e1234bbcd175e371774b2f4d5962c07f1c0/src/main/java/org/jsoup/nodes/Element.java#L1999-L2016) — `outerHtmlHead`.
- [`src/main/java/org/jsoup/nodes/Element.java:2062-2071`](https://github.com/jhy/jsoup/blob/387c6e1234bbcd175e371774b2f4d5962c07f1c0/src/main/java/org/jsoup/nodes/Element.java#L2062-L2071) — `html(QuietAppendable)`.
- [`src/main/java/org/jsoup/nodes/Printer.java:56-60`](https://github.com/jhy/jsoup/blob/387c6e1234bbcd175e371774b2f4d5962c07f1c0/src/main/java/org/jsoup/nodes/Printer.java#L56-L60) — `Printer.head`.
- [`src/main/java/org/jsoup/nodes/Printer.java:63-67`](https://github.com/jhy/jsoup/blob/387c6e1234bbcd175e371774b2f4d5962c07f1c0/src/main/java/org/jsoup/nodes/Printer.java#L63-L67) — `Printer.tail`.

Supporting files, not required labels:

- [`src/test/java/org/jsoup/nodes/ElementTest.java`](https://github.com/jhy/jsoup/blob/387c6e1234bbcd175e371774b2f4d5962c07f1c0/src/test/java/org/jsoup/nodes/ElementTest.java) — existing test or test build context; read at base.
- [`pom.xml`](https://github.com/jhy/jsoup/blob/387c6e1234bbcd175e371774b2f4d5962c07f1c0/pom.xml) — build or dependency context; read at base.

Held-out fix/test evidence: `src/test/java/org/jsoup/nodes/ElementTest.java` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Foreign-namespace distinction is reconstructed from fix tests. Existing visitor/filter infrastructure is unchanged supporting context; implementation coverage cannot be credited from a class signature map.

### 26. arduinojson-long-numeric-string — Long numeric strings during conversion

Repository: [bblanchon/ArduinoJson](https://github.com/bblanchon/ArduinoJson) · C++ (Arduino-compatible).

Base: [`ab8c9be6099af1915bdfec0ea482002bdc0e90de`](https://github.com/bblanchon/ArduinoJson/commit/ab8c9be6099af1915bdfec0ea482002bdc0e90de).
Fix: [`5ccf645dc1be17a6d7223287c46d4eea649661dc`](https://github.com/bblanchon/ArduinoJson/commit/5ccf645dc1be17a6d7223287c46d4eea649661dc) (2026-03-03T16:53:57Z). [Exact comparison](https://github.com/bblanchon/ArduinoJson/compare/ab8c9be6099af1915bdfec0ea482002bdc0e90de...5ccf645dc1be17a6d7223287c46d4eea649661dc). Single-parent relationship verified.

License: [MIT at base](https://github.com/bblanchon/ArduinoJson/blob/ab8c9be6099af1915bdfec0ea482002bdc0e90de/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: Converting a JSON string containing a very long decimal number to a numeric type can crash with a buffer overrun. Such input should not access memory outside valid bounds.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/bblanchon/ArduinoJson/issues/2220) (created 2026-03-01T02:46:49Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/ArduinoJson/Numbers/FloatTraits.hpp:195-210`](https://github.com/bblanchon/ArduinoJson/blob/ab8c9be6099af1915bdfec0ea482002bdc0e90de/src/ArduinoJson/Numbers/FloatTraits.hpp#L195-L210) — `make_float`.

Supporting files, not required labels:

- [`extras/tests/Numbers/parseDouble.cpp`](https://github.com/bblanchon/ArduinoJson/blob/ab8c9be6099af1915bdfec0ea482002bdc0e90de/extras/tests/Numbers/parseDouble.cpp) — existing test or test build context; read at base.
- [`extras/tests/CMakeLists.txt`](https://github.com/bblanchon/ArduinoJson/blob/ab8c9be6099af1915bdfec0ea482002bdc0e90de/extras/tests/CMakeLists.txt) — build or dependency context; read at base.

Held-out fix/test evidence: `extras/tests/Numbers/parseDouble.cpp` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Security-sensitive upstream report; only a symptom paraphrase is supplied to retrieval, not its root-cause analysis or reproducer. No sanitizer or device run was performed. parseNumber and conversion callers remain important supporting context.

### 27. arduinojson-due-truncated-strings — Truncated strings on Arduino Due

Repository: [bblanchon/ArduinoJson](https://github.com/bblanchon/ArduinoJson) · C++ (Arduino-compatible).

Base: [`411424b74eae641b1f2f248c33e85f1ab3f1f694`](https://github.com/bblanchon/ArduinoJson/commit/411424b74eae641b1f2f248c33e85f1ab3f1f694).
Fix: [`f051f8328d3acce6c674dbeebee9f32b9665c7e6`](https://github.com/bblanchon/ArduinoJson/commit/f051f8328d3acce6c674dbeebee9f32b9665c7e6) (2025-06-19T11:59:15Z). [Exact comparison](https://github.com/bblanchon/ArduinoJson/compare/411424b74eae641b1f2f248c33e85f1ab3f1f694...f051f8328d3acce6c674dbeebee9f32b9665c7e6). Single-parent relationship verified.

License: [MIT at base](https://github.com/bblanchon/ArduinoJson/blob/411424b74eae641b1f2f248c33e85f1ab3f1f694/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: On Arduino Due, serializing an object can emit only the first character of each key and string value after upgrading the library. Preserve complete strings on the affected Arduino core.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/bblanchon/ArduinoJson/issues/2181) (created 2025-05-22T20:02:55Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/ArduinoJson.hpp:1-56`](https://github.com/bblanchon/ArduinoJson/blob/411424b74eae641b1f2f248c33e85f1ab3f1f694/src/ArduinoJson.hpp#L1-L56) — `header macro/include ordering` (translation unit).

Supporting files, not required labels:

- [`extras/tests/Misc/CMakeLists.txt`](https://github.com/bblanchon/ArduinoJson/blob/411424b74eae641b1f2f248c33e85f1ab3f1f694/extras/tests/Misc/CMakeLists.txt) — existing test or test build context; read at base.
- [`extras/tests/CMakeLists.txt`](https://github.com/bblanchon/ArduinoJson/blob/411424b74eae641b1f2f248c33e85f1ab3f1f694/extras/tests/CMakeLists.txt) — build or dependency context; read at base.

Held-out fix/test evidence: `extras/tests/Misc/CMakeLists.txt` (file exists at base; changed assertions/content held out); `extras/tests/Misc/issue2181.cpp` (added by fix; absent at base)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Required label is a translation-unit include/preprocessor region, not a symbol. Arduino core dependency and macro environment are not vendored or pinned here. No firmware compilation, flashing or device validation.

### 28. arduinojson-tiny-flash-string — Short Flash-string keys crash

Repository: [bblanchon/ArduinoJson](https://github.com/bblanchon/ArduinoJson) · C++ (Arduino-compatible).

Base: [`96281de68284f5675dbda29af5613359697139c6`](https://github.com/bblanchon/ArduinoJson/commit/96281de68284f5675dbda29af5613359697139c6).
Fix: [`deab127a2fc846c80fe4691489e54fcf9fe2f95f`](https://github.com/bblanchon/ArduinoJson/commit/deab127a2fc846c80fe4691489e54fcf9fe2f95f) (2025-04-11T08:23:43Z). [Exact comparison](https://github.com/bblanchon/ArduinoJson/compare/96281de68284f5675dbda29af5613359697139c6...deab127a2fc846c80fe4691489e54fcf9fe2f95f). Single-parent relationship verified.

License: [MIT at base](https://github.com/bblanchon/ArduinoJson/blob/96281de68284f5675dbda29af5613359697139c6/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: After upgrading ArduinoJson, using a short F()-wrapped key such as uid causes an assertion or crash, while the equivalent ordinary string key works.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/bblanchon/ArduinoJson/issues/2170) (created 2025-04-10T20:51:03Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/ArduinoJson/Memory/StringBuffer.hpp:40-47`](https://github.com/bblanchon/ArduinoJson/blob/96281de68284f5675dbda29af5613359697139c6/src/ArduinoJson/Memory/StringBuffer.hpp#L40-L47) — `StringBuffer::save`.
- [`src/ArduinoJson/Memory/StringBuilder.hpp:28-49`](https://github.com/bblanchon/ArduinoJson/blob/96281de68284f5675dbda29af5613359697139c6/src/ArduinoJson/Memory/StringBuilder.hpp#L28-L49) — `StringBuilder::save`.
- [`src/ArduinoJson/Variant/VariantData.hpp:529-536`](https://github.com/bblanchon/ArduinoJson/blob/96281de68284f5675dbda29af5613359697139c6/src/ArduinoJson/Variant/VariantData.hpp#L529-L536) — `VariantData::setTinyString`.
- [`src/ArduinoJson/Variant/VariantImpl.hpp:21-46`](https://github.com/bblanchon/ArduinoJson/blob/96281de68284f5675dbda29af5613359697139c6/src/ArduinoJson/Variant/VariantImpl.hpp#L21-L46) — `VariantData::setString`.

Supporting files, not required labels:

- [`extras/tests/JsonDocument/set.cpp`](https://github.com/bblanchon/ArduinoJson/blob/96281de68284f5675dbda29af5613359697139c6/extras/tests/JsonDocument/set.cpp) — existing test or test build context; read at base.
- [`extras/tests/CMakeLists.txt`](https://github.com/bblanchon/ArduinoJson/blob/96281de68284f5675dbda29af5613359697139c6/extras/tests/CMakeLists.txt) — build or dependency context; read at base.

Held-out fix/test evidence: `extras/tests/JsonDocument/set.cpp` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Four changed existing string-storage functions are labeled, but target Flash-string adapters and architecture-specific reads are unresolved support. Upstream report concerns ESP32; host tests alone would not establish hardware acceptance.

### 29. arduinojson-const-char-array — Static const character-array keys

Repository: [bblanchon/ArduinoJson](https://github.com/bblanchon/ArduinoJson) · C++ (Arduino-compatible).

Base: [`91397f9f06f719ba185cc7b48aff4763d6529eea`](https://github.com/bblanchon/ArduinoJson/commit/91397f9f06f719ba185cc7b48aff4763d6529eea).
Fix: [`f0e84e4933b0d4120cd30ac1e0054ae67319f773`](https://github.com/bblanchon/ArduinoJson/commit/f0e84e4933b0d4120cd30ac1e0054ae67319f773) (2025-04-09T06:55:09Z). [Exact comparison](https://github.com/bblanchon/ArduinoJson/compare/91397f9f06f719ba185cc7b48aff4763d6529eea...f0e84e4933b0d4120cd30ac1e0054ae67319f773). Single-parent relationship verified.

License: [MIT at base](https://github.com/bblanchon/ArduinoJson/blob/91397f9f06f719ba185cc7b48aff4763d6529eea/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: A static const char array key declared in a class and defined separately no longer works as a JSON object key after an upgrade, producing a compile-time assignment error. Preserve support for this key form.

Problem provenance: **earlier official report symptom paraphrase**. [official issue](https://github.com/bblanchon/ArduinoJson/issues/2166) (created 2025-04-03T19:35:11Z; direct report; fetched body may have later edits)

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/ArduinoJson/Strings/Adapters/RamString.hpp:69-97`](https://github.com/bblanchon/ArduinoJson/blob/91397f9f06f719ba185cc7b48aff4763d6529eea/src/ArduinoJson/Strings/Adapters/RamString.hpp#L69-L97) — `StringAdapter specializations` (specializations).

Supporting files, not required labels:

- [`extras/tests/Misc/CMakeLists.txt`](https://github.com/bblanchon/ArduinoJson/blob/91397f9f06f719ba185cc7b48aff4763d6529eea/extras/tests/Misc/CMakeLists.txt) — existing test or test build context; read at base.
- [`extras/tests/Misc/StringAdapters.cpp`](https://github.com/bblanchon/ArduinoJson/blob/91397f9f06f719ba185cc7b48aff4763d6529eea/extras/tests/Misc/StringAdapters.cpp) — existing test or test build context; read at base.
- [`extras/tests/CMakeLists.txt`](https://github.com/bblanchon/ArduinoJson/blob/91397f9f06f719ba185cc7b48aff4763d6529eea/extras/tests/CMakeLists.txt) — build or dependency context; read at base.

Held-out fix/test evidence: `extras/tests/Misc/CMakeLists.txt` (file exists at base; changed assertions/content held out); `extras/tests/Misc/StringAdapters.cpp` (file exists at base; changed assertions/content held out); `extras/tests/Misc/issue2166.cpp` (added by fix; absent at base)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: New unsized-array specialization and issue2166.cpp do not exist at base. Labels cover the existing alternative adapter specializations, not invented pre-fix symbols. Requires compile-only regression/toolchain review, not a device claim.

### 30. arduinojson-nul-object-key — NUL-containing object keys

Repository: [bblanchon/ArduinoJson](https://github.com/bblanchon/ArduinoJson) · C++ (Arduino-compatible).

Base: [`7946ebe1a3ad8e568148631f27cadd488fbf9198`](https://github.com/bblanchon/ArduinoJson/commit/7946ebe1a3ad8e568148631f27cadd488fbf9198).
Fix: [`811005872916f04562dfea7144be4a3241731557`](https://github.com/bblanchon/ArduinoJson/commit/811005872916f04562dfea7144be4a3241731557) (2024-11-17T14:40:21Z). [Exact comparison](https://github.com/bblanchon/ArduinoJson/compare/7946ebe1a3ad8e568148631f27cadd488fbf9198...811005872916f04562dfea7144be4a3241731557). Single-parent relationship verified.

License: [MIT at base](https://github.com/bblanchon/ArduinoJson/blob/7946ebe1a3ad8e568148631f27cadd488fbf9198/LICENSE.txt) (`LICENSE.txt`, read).

Problem/query: Deserializing objects whose keys contain embedded NUL characters fails to preserve distinct complete key names, including when selecting object members with a filter.

Problem provenance: **retrospective official commit or fix test symptom paraphrase**. No earlier issue/PR located; evidence is the official commit and changed tests, if any.

Required candidates at the PRE-FIX revision (not gold; original one-based inclusive lines):

- [`src/ArduinoJson/Json/JsonDeserializer.hpp:233-315`](https://github.com/bblanchon/ArduinoJson/blob/7946ebe1a3ad8e568148631f27cadd488fbf9198/src/ArduinoJson/Json/JsonDeserializer.hpp#L233-L315) — `JsonDeserializer::parseObject`.

Supporting files, not required labels:

- [`extras/tests/JsonDeserializer/filter.cpp`](https://github.com/bblanchon/ArduinoJson/blob/7946ebe1a3ad8e568148631f27cadd488fbf9198/extras/tests/JsonDeserializer/filter.cpp) — existing test or test build context; read at base.
- [`extras/tests/JsonDeserializer/object.cpp`](https://github.com/bblanchon/ArduinoJson/blob/7946ebe1a3ad8e568148631f27cadd488fbf9198/extras/tests/JsonDeserializer/object.cpp) — existing test or test build context; read at base.
- [`extras/tests/CMakeLists.txt`](https://github.com/bblanchon/ArduinoJson/blob/7946ebe1a3ad8e568148631f27cadd488fbf9198/extras/tests/CMakeLists.txt) — build or dependency context; read at base.

Held-out fix/test evidence: `extras/tests/JsonDeserializer/filter.cpp` (file exists at base; changed assertions/content held out); `extras/tests/JsonDeserializer/object.cpp` (file exists at base; changed assertions/content held out)

Status: `provisional_static_source_checked`; admitted for scoring: **no**. Source/parent/diff checks passed; no independent review, bug reproduction or fix execution.

Uncertainty: Retrospective commit-and-test-derived query. Existing object/filter tests are supporting context; unchanged JsonString, adaptString and object lookup implementations may also be required.

## Handoff and transferable changes

Written directly in the existing `main` workspace, not a fork: `docs/REAL_TASK_RESEARCH.md` and `eval/public-tasks.json` only. No commit/push/tag was made. To transfer these two new files from this checkout, the maintainer can copy them verbatim, or generate a two-file patch without staging unrelated work:

```bash
git diff --no-index -- /dev/null docs/REAL_TASK_RESEARCH.md
git diff --no-index -- /dev/null eval/public-tasks.json
```

`git diff --no-index` exits 1 when differences exist; this is expected. On native Windows use `NUL` instead of `/dev/null`. No extra patch file was created, because file ownership is limited to the two artifacts above.
