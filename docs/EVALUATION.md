# Retrieval evaluation protocol

The development tree includes a bounded runner and checked-in synthetic corpus. This protocol also describes the larger real-task acceptance gate, which is not completed or a claim of superiority. The hardening tests in `tests/fidelity.rs` cover source/export correctness, not task relevance.

Local verification, synthetic results, the exploratory public comparison and retained failures are recorded in [implementation status](IMPLEMENTATION_STATUS.md). CI exercises the synthetic ablation runner on the build matrix; consult the live workflow status for the released revision rather than treating local Windows results as cross-platform evidence.

## Run it locally

```bash
cargo build --release
target/release/cxt --evaluate eval/corpus.json --output target/retrieval-report.json
target/release/cxt --evaluate eval/ablation.json --output target/ablation-report.json
target/release/cxt --evaluate eval/relationships.json --output target/relationships-report.json
target/release/cxt --evaluate eval/bindings.json --output target/bindings-report.json
target/release/cxt --evaluate eval/corpus.json --eval-baseline /trusted/path/to/cxt-0.4.2 --output target/comparison.json
```

On Windows use `.\target\release\cxt.exe`. The optional baseline must be a trusted executable, not an arbitrary download. Keep its version, release URL and SHA-256 with your results. The corpus cannot choose executables. Evaluation reports can contain task text, paths and arguments; they intentionally do not contain source bodies or subprocess stderr.

`eval/corpus.json` labels 30 synthetic tasks across Rust, Python, TypeScript, Go, Java and Arduino-style C++, at 8K/16K/32K budgets. `eval/ablation.json` compares four rankers with expansion on/off on six routing tasks with a one-file cap and repetitive logging distractors. These simple fixtures are regression demonstrations, **not** 30 real bug fixes. Do not infer broad relevance, graph quality, AI success or competitor superiority from them.

Corpus schema `cxt.eval.v1` contains `dataset_kind`, `budgets`, optional `contexts`/`rankers`/`query_expansion` (array of booleans)/`max_files` (1–1000), and `tasks` with `id`, `root` (relative to corpus directory), `query`, `required_files` and `required_symbols` (`path`, `name`, `start_line`, `end_line`, optional `source_sha256`). Labels are original one-based inclusive line ranges; names may be qualified display identities, not literal substrings. Coverage requires the actual original source span in the export, plus span coordinates when present; maps do not count. Optional source hashes reject stale labels. Roots and label sources must remain under the corpus directory. The runner limits corpus size, task/mode counts, total invocations (5000), child execution duration and captured output.

Report schema `cxt.eval.report.v1` retains every cold/repeat attempt, query/arguments/root, source fingerprints, exact independently counted **actual stdout** cost, budget acceptance, file/span recall, wall time and per-child peak memory where supported. `success` means invocation and packet-budget acceptance—not development-task success. A map never counts as implementation coverage. `required_source_token_share_proxy` measures covered labeled-source cost / packet cost, not true relevant-token precision; labels may omit supporting code or overlap. Failed/oversized baseline output stays recorded. Legacy estimates are not silently treated as equal-cost exact budgets.

Since 0.5.1, reports classify failed children with `failure_kind`, `exit_code`, elapsed time and peak memory when available. Categories distinguish secret fail-policy, security scanner errors, scan timeout/limits, no query match, token budget, invocation timeout and otherwise unclassified CLI/invocation errors. Classification uses bounded in-memory stderr; raw stderr is drained but never stored in the report. Categories are diagnostic hints, not a replacement for inspecting a failing invocation locally. Original-source candidate-wide fail-policy is unchanged, including findings outside focused slices and in candidates later omitted by budgeting. Older reports lack these fields and cannot be retrospectively diagnosed from exit code alone.

`eval/relationships.json` is a separate Python/TypeScript synthetic shared-state regression. It labels the query entry, state helper and a second handler without query terms. It controls expansion on/off at 2K/4K final packet budgets in focused/adaptive modes. These deliberately small cases measure a known failure, not independent real-task generalization.

`eval/bindings.json` adds development-influencing Python module-alias and TypeScript named-alias cases with duplicate names and shadowed-callback distractors at the same 2K/4K budgets. For a like-for-like 0.5.1 comparison, run that frozen executable's own `--evaluate` on the same corpus into a separate report and compare identical context/ranker/expansion/budget/phase records. The runner's `--eval-baseline` track deliberately uses legacy complete-mode arguments for older tools; it is not an equivalent-mode comparison with a modern cxt release. Required-span recall measures the labeled implementations; negative relationship assertions live in `tests/query_resolution.rs`. This corpus is synthetic regression evidence, not unseen public-task accuracy, independently reviewed labels, aggregate relationship precision or downstream coding success.

Repeat uses the current tool's metadata cache but still launches a new process and rereads sources; tokenizer startup is included. Baseline may rescan. This does not demonstrate incremental-query latency. Linux memory is sampled VmHWM; Windows uses process peak working set; unsupported platforms report null. Do not compare those methods as interchangeable measurements.

## Baseline and reproducibility

Retain the immutable `v0.4.2` source/binary as the baseline. Store repository URL, license, exact commit, tool version, launch directory, query, arguments, model/tokenizer, config, platform, wall time, exit code, warnings and output hashes with each run. Use public redistributable or synthetic fixtures; never incorporate customer/private repositories without permission. Pin tool modes and versions rather than comparing whichever defaults happen to run.

Run unmodified checkouts at 8K, 16K and 32K final-output token budgets. Until an exact tokenizer is available, report estimated-budget runs separately and measure outputs with one named offline tokenizer before making equal-cost claims. Include rendering/manifest overhead. Verify no secret-containing fixture values escape into benchmark logs or artifacts.

## Task corpus

Start with a small reproducible synthetic corpus, then expand to about 30 reviewed real tasks across 5–6 public codebases. Cover Rust, TypeScript/JavaScript, Python and Go, with additional embedded/language fixtures as supported. Include terminology-mismatch queries, routing/state bugs, dependency expansion, Git impact, config changes, failing tests and oversized primary implementation files.

For each task record:

- Repository revision and task description available before the fix.
- Reviewed required files and required symbols/spans, plus optional supporting context.
- Exact original byte/line ranges and why each label matters.
- Distractors and known unresolved relationships.
- Provenance for the labels and any uncertainty.

Historical patches help identify labels but are not complete ground truth: unchanged dependencies/configuration may be necessary, while incidental changes may not be relevant. Do not include the solution patch in the retrieval input. Label review should be independent of the selected retrieval output.

## Materialize public exploratory tasks

[Public research](REAL_TASK_RESEARCH.md) supplies 30 pinned bug-fix candidates across six languages, with 55 proposed original-source spans. Two fixes are quarantined for regression concerns; none is independently reviewed gold. One Click case has narrow isolated-function before/after evidence, not full-checkout repair validation.

```powershell
# Inspect the plan first; no network request without --download.
python eval/materialize_public.py --output "$env:TEMP/cxt-public-evaluation" --include-provisional
python eval/materialize_public.py --output "$env:TEMP/cxt-public-evaluation" --include-provisional --download
.\target\release\cxt.exe --evaluate "$env:TEMP/cxt-public-evaluation/corpus.json" --eval-baseline target/cxt-published-0.4.2.exe --output target/evaluation/public-retrieval.json
```

Use a dedicated directory without a build/junk-named ancestor when testing older cxt binaries: 0.4.2 prunes source subdirectories beneath such ancestors. The development scanner has a public regression for this bug. An initial `target/public-evaluation` experiment exposed it and was rejected as retrieval evidence, not reported as a ranking failure.

The checked-in materializer fetches immutable fix/base identities into separate evidence/input repositories, verifies the single-parent relationship, raw source bytes/ranges and base license, and produces a ledger plus executable corpus. Only base history is fetched into retriever inputs. Git hooks, text/identity filters and working-tree encoding are disabled; upstream build/test code is not executed. Existing dirty or mismatched checkouts are refused. Quarantines stay excluded, failures are retained and exit nonzero, and provisional labels require explicit opt-in. Source bodies remain in ignored/local checkouts, not versioned cxt fixtures. Preserve upstream licenses for any redistribution.

## Native competitor track

[Pinned source/build recipes](REAL_TASK_RESEARCH.md#pinned-competitor-recipes) cover Repomix, code2prompt, dotcommander/repomap and Gitingest. Only measured tools count as executed comparisons; installing a recipe is not a result.

`eval/compare_repomix.mjs` runs a trusted pinned Repomix build and the explicit cxt executable on six predeclared public tasks, one per language. It verifies the source pin, uses explicit processor-free configuration with security checking on, and keeps Git diffs/logs out. Build the exact documented source with locked dependencies; the local run used `npm ci --ignore-scripts --no-audit --no-fund` followed by `node node_modules/typescript/bin/tsc -p tsconfig.build.json`. No dependency lifecycle scripts or upstream task tests were run.

```powershell
node eval/compare_repomix.mjs --corpus "$env:TEMP/cxt-public-evaluation/corpus.json" --repomix target/benchmark-tools/repomix --cxt target/release/cxt.exe --output target/evaluation/repomix-public.json
```

The harness measures the complete native JSON plus an identical task envelope, independently using pinned `gpt-tokenizer` cl100k ordinary-text counting (package/vocabulary hashes recorded). It retains stdout, bounded stderr, exit codes, arguments, hashes, latency and overflows in a separate artifact directory; it refuses to overwrite that directory. cxt uses focused selection with no persistent cache; first/repeat here means separate-process repetition, not a controlled cache-cold experiment. cxt's fail-on-secret policy and Repomix's native scanner are **not equivalent** policies; safety refusals remain failures. Per-child execution is capped at 60 seconds and stdout at 32 MiB. Peak memory is not measured in this track.

This compares native task retrieval against native whole-repository packing, **not two equal-capability rankers**. Repomix's token-budget flag rejects oversized output after rendering; the harness never trims it to manufacture acceptance. Default whole dumps predictably overflow small budgets; this does not establish cxt superiority over compressed, manually selected or map modes. A fair packer-only comparison needs an independently frozen common selection; map orientation must be scored separately from source-body recall. Downstream repair success remains unmeasured.

## Incremental graph workload

```bash
cargo run --release --example graph_benchmark -- 500 200
```

This generates 500 Python modules with 100,000 functions, times full workspace loads (cold/repeat/edit/rename/no-cache), and asserts all unchanged nodes are reused and one source edit rebuilds one node. Source enumeration/read/hash and metadata validation are included. It is a synthetic mutation/performance regression, not a public monorepo, compiler graph, memory result or zero-read query benchmark. The local report is `target/evaluation/graph.json`.

## Measurement definitions

Line labels discard only first-line indentation and terminal CR/LF when testing copied-span presence, because an AST node begins after indentation; interior bytes and declaration modifiers still have to match. Source fingerprints continue to cover the complete original bytes. This prevents a boundary-whitespace false negative without accepting a map, a symbol-name mention, a missing `export` or a changed implementation.

| Metric | Definition and limitation |
| --- | --- |
| Required-file recall | Required files actually represented / labeled required files; distinguish empty/signature-only from implementation coverage. |
| Required-symbol recall | Required spans represented in full / labeled required spans; a filename alone does not count. |
| Relevant-token precision | Offline-tokenized required/supporting source represented / all packet tokens. Headers and maps consume budget; document how overlapping spans are deduplicated. |
| Task success | Identical model, instructions, execution environment and test criteria; report every attempted task and failure. Separate this experiment from static recall. |
| Cold/warm latency | Separate uncached first use, repeat use and edits. A repeat full scan is not evidence of an incremental index. |
| Peak memory | Measure process memory consistently and identify platform/method. Input-byte caps are not memory caps. |
| Fidelity and safety | Literal/token and serialization regressions, forbidden-character handling, source freshness and known synthetic secret-policy checks. |

Report per-task results, aggregate distributions and omissions, not only averages or successful examples. Use file/symbol recall first while whole-file output exists; do not imply semantic completeness from static import expansion.

## Experiments and decision gate

Compare filename/content search, the frozen 0.4.2 ranker and each new ranking strategy. Run lexical-only, symbol-only and graph-expansion ablations before tuning weights. Evaluate all tools in genuinely comparable modes; a complete dump and a signature map answer different questions.

Treat improved recall at fixed final cost and reduced irrelevant content without lower task success as hypotheses. The analysis's +15 percentage-point symbol-recall and 25% irrelevant-token reduction targets are aspirational, not current results or mandatory thresholds proven suitable for this corpus. Publish reproducible commands/data before placing benchmark claims in the README.
