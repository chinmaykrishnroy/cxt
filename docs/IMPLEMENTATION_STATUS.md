# Task-aware implementation status

The 0.5.2-dev.1 prerelease previews the [evidence-guided query architecture](QUERY_ARCHITECTURE.md): shared binding decisions, original reference sites and bounded typed traversal. This is an incremental milestone, not the complete architecture or stable 0.5.2. Assets are available only after its tagged release workflow publishes them. The [local verification report](QUERY_ARCHITECTURE_VERIFICATION.md) identifies tested binaries, regression results and retained failures; historical measurements below do not validate a different build.

For 0.5.1 query changes and final-binary regression results, see [the release verification report](QUERY_0_5_1.md). The [first query reliability pass](QUERY_RETRIEVAL_CYCLE.md) and release measurements below remain historical evidence for their identified binaries.

Release preparation: **0.5.0**, 2026-10-14. The measurements below were captured from the preceding **0.5.0-dev** snapshot unless a section explicitly identifies final-release validation. They are not measurements of the published GitHub binaries. The installed user executable has not been replaced. See the live CI/release badges in the README for hosted status.

## Implemented

| Analysis area | Implementation and boundary |
| --- | --- |
| Fidelity/export correctness | Literal/token-aware conservative compaction, dynamic Markdown fences, JSON recovery and XML forbidden-character rejection. |
| Symbol-level context | Names/kinds/qualified identities, original byte/line ranges, named implementations, complete/map/focused/adaptive modes. Grammar coverage fixtures include Rust, Python, JS, TS, Go, C, C++, Arduino, Java, C#, Ruby and PHP. |
| Explainable retrieval | BM25/path/symbol signals and bounded lexical-reference/import expansion; lexical/symbol/legacy and expansion-off ablations. Not semantic search or a resolved call graph. |
| Context budgets | Offline cl100k/o200k exact final-packet accounting, heuristic fallback profiles, bounded unit selection, explicit exclusions. Pins cannot silently become adaptive maps. |
| Index reuse | SHA-validated parser metadata and persistent static edges; config hashes, negative probes, canonical targets and package-directory observations invalidate reuse. Edit/delete/rename/branch/corruption regressions and cache-free config-impact parity. Source is still reread; not a compiler graph. |
| Secrets | Structured/provider patterns, explicit example allowlists, original-source fail checks, query-metadata checks and an optional installed Gitleaks backend with failure/timeout blocking. Best effort, not a universal security guarantee. |
| Agent integration | Bounded, scoped, read-only stdio MCP context/search/trace/symbol/diff/test/map tools; fixed policy, no cxt configuration overrides or output/cache writes. |
| Ecosystem reliability | JSONC inheritance, alias alternatives, selected Node package exports, Python parent imports, Cargo local/workspace/target path dependencies, Go modules/workspaces/local replacements, Java source roots, Arduino static C++ parsing. Not compiler-complete. |
| TUI/onboarding | Seven views including Symbols; representation, ranker, symbol, cache, scanner and scope controls; updated help, README and guides. |
| Evaluation/release reliability | Versioned synthetic corpus, real baseline executable, ranker/expansion ablations, independent final token counts, retained failures, latency/memory reporting, locked CI builds and a pre-publication validation/version gate. |

The module boundaries follow the shared-core design: CLI/TUI/MCP do not implement separate packing/security pipelines. Agent instructions record those constraints and the required public export/API test boundaries.

## Verification

### Final 0.5.0 local release gate

Windows x64, Rust 1.97.1, 2026-10-14:

- Format check, **150 passing tests**, strict all-target/all-feature Clippy and optimized binary/example builds passed. The ordinary suite ignores one optional Gitleaks case; that case was executed separately and passed with Gitleaks 8.30.1.
- All seven integration suites passed against the final optimized CLI: **113 passed**, plus the separate installed-Gitleaks case. These include fixed-policy stdio MCP, workspace containment, cache invalidation, Git selection, fidelity, safety caps and skill export.
- Skill export tested for generic/custom/Unicode paths, all five named agents, conflicting modes and non-overwrite behavior. The portable guide passed the skill-authoring validator and includes every public long option, checked automatically.
- Real terminal acceptance: Claude default path, custom skill export, overwrite refusal, retry to a new path, Settings change to Kiro, updated Kiro default, cancellation and clean quit. The exported guides matched the bundled guide exactly. No actual personal/global agent configuration was changed; activation in each third-party client remains unverified.
- Optimized scan-cap smoke refused export as intended. Named `run_snapshots` Markdown fit at 5,694 exact cl100k tokens under 8K. JSON plus manifest required 8,061 tokens: the 8K attempt correctly refused; a separate 16K attempt passed. Format/manifest overhead is not waived to manufacture acceptance.
- Final-binary synthetic comparison completed **720 attempts**, including the retained published 0.4.2 baseline. Focused/adaptive retained 100% labeled-span recall on these synthetic tasks at each budget. Complete mode still failed 56/60 attempts at 8K. The final ranker/expansion ablation completed **96 attempts** (no legacy binary added to that run): hybrid/symbols 100% span recall, lexical/legacy 0%; all emitted accepted packets. These are synthetic regressions, not downstream coding success.
- The 500-module / 100,000-function graph workload passed reuse/mutation assertions: 4,506 ms cold, 1,177 repeat, 1,248 after one edit, 1,213 after rename/import edit, 2,659 without cache. Other validation ran concurrently; these timings are not isolated speed comparisons.
- Workflow YAML, four Bash bodies, Markdown local links and SVG XML passed validation. `.gitattributes` keeps checked-in text/fixtures LF across platforms. Hosted platform results must be checked on the actual commit; local Windows success is not a claim of Linux/macOS runtime success.

Final local optimized binary SHA-256: `e5f0b6f8ac0e3b3bb954ad8879f93d5d112ce2098b9cf5ce02db168dffe6eace`. Preserved test binary: `target/evaluation/cxt-0.5.0-e5f0b6f8ac0e.exe`. Reports: `retrieval-0.5.0.json`, `ablation-0.5.0.json`, `graph-0.5.0.json` under `target/evaluation/` (local, not committed). Published cross-platform assets are different builds and have their own checksum manifest.

### Earlier development snapshot

- `cargo fmt -- --check`: passed.
- `cargo test --all-targets`: **145 passed**, one external-tool test ignored in the ordinary suite. This includes the final nested-project, export/decorator-wrapper, qualified-label/hash and config-impact regressions.
- That installed-Gitleaks test: executed separately and passed, using Gitleaks 8.30.1.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo build --release --bins --examples`: final build passed. An earlier attempt was blocked by the benchmark's Windows executable lock and a temporary approval-service limit; both were resolved. Future benchmarks use a hashed executable copy instead of locking the rebuild target.
- Final optimized CLI: help/version, fail-closed scan cap and named `run_snapshots` focused JSON delivery passed; the named packet was 7,978 exact tokens within its 8,000-token budget. Broader tracing/Git/serialization/scanner smokes are exercised by the integration suites below.
- All six integration suites rerun against the final optimized CLI: **108 passed**, one optional scanner case ignored in the ordinary run. The installed-Gitleaks case also passed separately against that binary. StdIO MCP, scope/policy checks, scanner errors, source freshness, Git modes, cache invalidation and export fidelity are included. Outside-target Windows junction rejection was exercised previously and the temporary junction removed.
- Final real terminal: startup animation, Symbols, Checks (including dependency-index statistics), Output and clean quit exercised successfully. Earlier checks also exercised named-symbol editing, Settings and reviewed JSON export; current render/worker tests cover all views/dialogs and shared reviewed delivery.
- Both workflow YAML files parsed, and release Bash bodies passed `bash -n`. GitHub-hosted execution has **not** run for this unpushed tree.
- Linux local validation was unavailable because Docker Desktop's WSL engine failed to start. No Docker reset, container deletion or data change was attempted. macOS execution is unverified locally.

Measured development executable: SHA-256 `3ca528b312ea5e09b2e15c24b2d1d2dd5822fe041fbd1245753feef4e78f4b30`. Preserved benchmark copy: `target/evaluation/cxt-final-3ca528b312ea.exe`; rebuilding `target/release/cxt.exe` replaces that working artifact. The development benchmark run did not publish or install that executable.

## Synthetic measurements

Commands and dataset semantics are in [EVALUATION.md](EVALUATION.md). Final local reports (not committed): `target/evaluation/retrieval-final.json` (720 attempts), `target/evaluation/ablation-final.json` (108 attempts), run with the hashed final executable copy identified above. Original-span scoring, declaration-wrapper preservation and the boundary-indentation correction are included. Intermediate `*-current.json` reports and older name-presence-only reports are retained for debugging, not final evidence. Baseline: actual published cxt 0.4.2 Windows executable, SHA-256 `27463be0f635c7cca6c5042d203c37ceec32269aac83f7e191ab44418fda4a3e`.

Each table row includes 30 tasks × cold/repeat, so **60 attempts**, not 60 independent tasks. Recall is averaged across all attempts, with failures scored zero. Accepted means the invocation returned a packet within the independently measured exact budget. Mean packet cost is over accepted outputs only.

| Mode | Budget | Accepted / 60 | Required-symbol recall | Mean accepted packet tokens |
| --- | ---: | ---: | ---: | ---: |
| Published 0.4.2 complete | 8K | 24 | 0% | 1,925 |
| Published 0.4.2 complete | 16K | 60 | 93.3% | 9,078 |
| Published 0.4.2 complete | 32K | 60 | 100% | 9,861 |
| Development complete | 8K | 4 | 0% | 2,056 |
| Development complete | 16K | 60 | 100% | 9,979 |
| Development complete | 32K | 60 | 100% | 9,979 |
| Development focused | 8K / 16K / 32K | 60 at each budget | 100% at each budget | 1,046 |
| Development adaptive | 8K / 16K / 32K | 60 at each budget | 100% at each budget | 1,046 |

Whole-file failures are retained, not hidden. Exact accounting, richer metadata and final rendering can make a complete implementation exceed a tight budget even if its raw content nearly fits; the whole-file planner can fail rather than export an empty packet. Use focused/adaptive context or increase the budget. The old estimated profile does not provide equal-cost assurances; the runner independently checks its actual output.

In the final one-file routing ablation, hybrid and symbols-only achieved 100% required-span recall; lexical and legacy achieved 0%, selecting repeated keyword logs. The intermediate stricter scorer dropped to 66.7% before the missing export wrappers and boundary-indentation mismatch were fixed; final coverage now requires actual original source spans, not name mentions. Expansion on/off made no difference in these simple fixtures. That demonstrates this specific distractor regression, **not** graph quality or general symbol-search superiority.

Final accepted focused/adaptive CLI runs averaged roughly 194–207 ms versus 67–106 ms for baseline budget groups. These include process/tokenizer startup on this Windows machine and overlapping validation load; they are not an isolated performance comparison. The current engine is not proven faster than 0.4.2. Per-child memory, phase times and failures are retained in the reports; failed invocations do not contribute accepted latency/token means. These results do not establish public-monorepo performance or general speed gains.

## Remaining evidence and scope

The analysis's engineering priorities now have concrete first-pass implementations: source/export correctness, symbols, explainable ranking, focused/adaptive planning, exact final budgets, incremental static edges, scoped MCP, TUI controls, onboarding and reproducible evaluation. Its suggested milestone numbers are a roadmap, not released versions or a stable 1.0 compatibility promise. Optional embeddings remain deferred until static retrieval has defensible evidence.

The evidence gates are different work: independently reviewed public labels, controlled downstream AI task success, genuine relevant-token precision, equivalent-mode competitor comparisons and large-monorepo measurements. The current public research has 30 pinned cases across six repositories, two quarantines and 28 byte-verified input trees, not 30 reviewed gold tasks. Native Repomix packing was actually run; other pinned recipes are not execution results. Broader compiler/framework resolution, stable API/schema/config guarantees and signed/notarized distribution remain separate work.

## Incremental graph measurement

The generated Python workload has 500 modules and 100,000 functions. The final example's full workspace load, including enumeration/source reads/hash validation/graph assembly, took 6,801 ms cold, 1,484 ms repeat, 1,724 ms after a source edit, 1,545 ms after a rename/import edit and 4,197 ms with caches disabled. Repeat reused all 500 nodes; the single edit rebuilt one and the rename rebuilt two. Raw final report: `target/evaluation/graph-final.json`; earlier run: `graph.json` (4,304/1,166 ms cold/repeat). Other validation workloads were active, and these are one-machine timings, not isolated replicated distributions, memory measurements, public-monorepo results or zero-read querying. The mutation/reuse assertions are the regression gate, not a universal speedup claim.

## Evaluation integrity corrections

The first public run beneath `target/public-evaluation` was rejected: the scanner applied junk-directory names to absolute ancestors and pruned source subdirectories. A public CLI regression now exercises explicitly chosen projects beneath `target` while still excluding their own generated subtree. Comparison trees were moved to a dedicated temporary directory so the unmodified 0.4.2 baseline is not handicapped by this historical bug.

Requiring actual original source spans rather than symbol-name substrings exposed lost TypeScript export/declaration wrappers and boundary-indentation scoring mismatches. The declaration-wrapper fix invalidates parser cache schema; corrected final synthetic measurements now supersede the earlier name-only claim. Intermediate failures and raw comparator stdout/stderr are retained locally, not discarded or used to claim category leadership.

The **final-binary** native Repomix comparison completed 72 attempts across six provisional cases, three budgets and two repetitions. Its full dumps were approximately 134K–813K tokens, exceeding every tested 8K–32K budget. Final cxt focused accepted 18/36 attempts: 6/12 at each budget. Mean required-span recall across all attempts was 0% at 8K and 16.7% at 16K/32K; safety refusals and Java timeouts remain failures. This small native whole-dump versus task-retrieval experiment is not evidence against equivalent compressed/selected competitor modes. Raw final report and artifacts: `target/evaluation/repomix-final.json` and its sibling `.artifacts` directory; the earlier run is retained as `repomix-public.json`. The final report records executable/runtime/lock/config/vocabulary hashes and source fingerprints.

The external-directory public baseline comparison completed **672 attempts**, 28 provisional tasks × three budgets × four modes × two repetitions. At 16K, intermediate focused output was accepted in 31/56 attempts and averaged approximately 25% required-span recall (failures scored zero), versus 28/56 accepted and approximately 9% recall for the published baseline. These are exploratory labels and a pre-wrapper-fix build, **not a release target met or proof of superiority**. All modes together retained 275 exit-code-4 refusals, five exit-code-2 failures and 40 independently measured budget overflows. Selected test-fixture secret findings and Java scan timeouts are observed causes, but the legacy runner intentionally does not retain stderr, so exit-code totals alone are not a complete cause breakdown. Raw report: `target/evaluation/public-retrieval-external.json`.

The practical conclusion is mixed: graph reuse and the safety/fidelity regressions have concrete positive evidence, but natural-language retrieval still misses important code on several real candidates. More features are not a substitute for held-out ranking improvements, reviewed labels and downstream evaluation. Keep these limitations visible in any launch/demo.

Optional embeddings, hosted accounts, cloud synchronization and built-in chat remain deliberately deferred, as the analysis recommends. Public benchmarks and a public release must not be presented as complete until their separate evidence/authorization gates are satisfied.
