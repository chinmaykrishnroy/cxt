# 0.5.2 candidate verification — 2026-10-16

This is an **unpublished candidate**, not stable-release or package-manager
acceptance. The user authorized a DEV commit/push and an empty-tag, six-target
build-only workflow after local validation. No main merge, tag, public release,
package update or installed-binary replacement is included.

## Identity and boundaries

- Working branch: DEV, based on `ad92d801759c498c5bc145271fc108b3863120ea`.
- Version: `cxt 0.5.2`; frozen Windows executable:
  `target/evaluation/cxt-0.5.2-candidate.exe`, 31,729,152 bytes.
- Executable SHA-256:
  `f766e285a1e8ea853672f86e57368533780f863b541e04db24e9e49d97533b0e`.
- Implementation/build/test input fingerprint (49 files):
  `445c7636b94871a12039db28586942f84a7e7748101417105d8d366b985022a5`.
  Method: SHA-256 of compact JSON `[git path, SHA-256 of working-tree bytes]`
  entries, sorted by path, from tracked Cargo.toml/Cargo.lock, src, tests,
  examples, skills/cxt/SKILL.md, docs/CLI_REFERENCE.md, .cargo and
  .github/workflows. This is a working-input fingerprint, not a Git tree hash.
- Frozen pre-fix stdio baseline: `cxt-dev-mcp-tui.exe`, version 0.5.2-dev.1,
  SHA-256 `736fc814ffb58523afab1c95e53f6bf8f5b060e5d59b465645d5637d22284cdf`.
- Installed user executable remains unchanged, SHA-256
  `cd54d91589fcfd3a0f138d55b9bbac17fead7c0b0043c30a5953a3be2fe24a54`.

No private repositories from the external review were accessed. Held-out task
details were not opened or included in retrieval, fingerprints or staging.
An untracked document inside the workspace is still not an access-controlled
holdout; independent evaluation requires actual separation.

## Local gates

Windows MSVC; Rust 1.97.1. One Cargo build job, incremental compilation disabled;
native linker/SDK library paths were supplied for this machine.

| Gate | Result |
| --- | --- |
| `cargo fmt -- --check`, `git diff --check` | Passed |
| `cargo test --locked --all-targets -j 1 -- --test-threads 8` | 194 passed, one optional Gitleaks test ignored |
| `cargo clippy --locked --all-targets --all-features -j 1 -- -D warnings` | Passed |
| `cargo build --locked --release --bins --examples -j 1` | Passed |
| All ten integration executables with `CARGO_BIN_EXE_cxt` set to the frozen optimized binary, one test thread | 153 passed, one optional ignored |
| Optional installed-Gitleaks core test, run separately | Passed |
| Three additional 39-test library/worker runs, eight threads | All passed; 12.90 / 12.56 / 13.06 seconds for the whole suite |
| Isolated optimized TUI terminal smoke | Ready, Output view, clean quit with exit 0; no export or clipboard mutation |
| Workflow source validation | Both YAML files parse; six release targets; Windows Rust smoke does not run through Git Bash; empty-tag DEV dispatch skips publication |

Logs are local ignored artifacts under `target/evaluation/stable052-candidate-*`.
Earlier validation sequences were intentionally superseded when new reproductions
required changes; their logs remain and are not final-source gate evidence.

## External-feedback regressions and actual stdio replay

Use the [dogfooding log](MCP_DOGFOODING.md) for red/green details. Public MCP tests
exercise the real Server interface; the frozen executable was also replayed over
initialize/initialized/tools-call stdio, with no registered connector available.

| Reproduction | Frozen candidate outcome |
| --- | --- |
| One-function repo map with `max_tokens: 100` | Error names `max_tokens=100`, explains metadata overhead, suggests bounded increase/narrowing, exports no packet |
| Test importing `app/leaves.py`, different test filename | Retained `test_leave_requests.py`, 393 packet tokens |
| Bare JS call inside a declared class with a same-named member | Correct global helper retained; member helper excluded; 387 tokens |
| Java implicit field supporting a method | Field evidence retained, 380 tokens |

Regressions also verify a successful larger-budget retry, matching and different
test filenames, an unrelated test excluded, all eight JS/TS extensions, and MCP
tool-discovery budget/map-to-source guidance. The changed error wording initially
broke failure categorization; a complete-context public CLI regression caught it.
Budget refusals again retain `failure_kind: token_budget`.

The older source-only discovery replay remains mixed:

- At 2,500 tokens, search retains one map, 2,269 packet tokens, 28 omissions.
- At 8,000, it retains all three requested maps plus bindings.rs: 5,271 tokens,
  25 omissions. These are maps, **not implementation coverage**.
- Qualified `IndexStore::load` succeeds at 1,653 tokens. Focused `rank` succeeds
  at 559 tokens without unrelated test imports.
- Four requests in one local server process took 3.182 seconds in this replay.
  This is not a p95, independent task benchmark or client token-billing result.

## Synthetic retrieval results

All attempts, exact accepted packet counts, fingerprints and failures remain in
`stable052-candidate-{ablation,bindings,relationships,corpus}.json`.

| Corpus | Accepted / attempts | Required-symbol recall interpretation |
| --- | --- | --- |
| Ablation | 96 / 96 | Hybrid/symbol rankers 100%; lexical/legacy 0% on deliberately symbol-led fixtures |
| Import bindings | 32 / 32 | Expansion on 100%, off 33.3% |
| Shared relationships | 32 / 32 | Expansion on 100%, off 33.3% |
| Main synthetic corpus | 484 / 540 | Focused/adaptive all three budgets and complete 16K/32K: 100%; complete 8K: four accepted with zero required-symbol recall, 56 token-budget refusals |

The 56 refusals are retained, not silently discarded. Successful packet acceptance
does not imply useful implementation coverage: the four accepted complete-8K
packets demonstrate that distinction. These results match the known development
pattern; they are not new public-task accuracy, coding success or competitor proof.

## Generated scale/mutation check

The optimized `graph_benchmark 500 200` passed: 500 generated Python modules,
100,000 functions, not a public monorepo. Cold/repeat/source-edit/rename/cache-off
elapsed milliseconds were 5,971 / 1,529 / 1,403 / 1,483 / 2,916. Repeat reused 500
nodes; one edit rebuilt one; rename/import edit rebuilt two.

This measures full Workspace loading, source reads/hashes, metadata validation and
graph assembly with the benchmark's 180-second guard. It is not a full MCP query
under its fixed ten-second limits, zero-source-read indexing or a controlled
speedup comparison. It does not reproduce or resolve the external whole-repo stall.

## Remaining release/quality gates

- Obtain hosted results for the exact pushed DEV source. Local Windows success
  is not macOS/Linux/ARM runtime acceptance. Build-only artifacts are not stable
  release downloads; cross-compilation is distinct from native execution.
- Resolve or explicitly disposition the historical load-sensitive scan/TUI
  failures. The helper now polls a queued result before deadline evaluation, but
  its ten-second deadline was not increased. Repeated green runs do not prove the
  historical cause or absence under every load.
- Obtain the external stall's exact sanitized version/request/error. Idle MCP
  process lifetime and PowerShell's native-stderr display alone do not prove a hang.
- Retain the documented Java bare method-helper omission and small-budget search
  coverage limits. DeepQuery's lexical/session index, behavioral edges, terminology
  grounding, global allocation and compiler integration remain incomplete.
- Freeze before independently reviewed real-task evaluation. Measure useful
  implementation/test coverage, first-call acceptance, retries, full tool traffic,
  latency and memory. No 9/10 agent rating or best-in-class claim is established.
- Obtain separate authorization before merging main, tagging or publishing.

Candidate-wide secret fail-policy, fixed MCP redaction/no persistent writes,
workspace boundaries, scan caps, freshness checks and omitted-pin blockers remain.
