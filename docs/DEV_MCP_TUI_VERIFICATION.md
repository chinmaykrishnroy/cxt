# DEV: MCP retrieval and TUI worker verification

Local Windows verification on 2026-10-15. This is an unreleased working-tree
slice, not stable 0.5.2 acceptance or cross-platform CI evidence.

## Identity and scope

`DEV` was created and pushed **before implementation**, at
`ef8ddb223fe82bd53667436cc24142648a752a3d`; the remote identity was checked.
The fixes were verified as local working-tree changes before the maintainer
authorized committing them on DEV and merging into main, while retaining DEV.
This report does not assert remote publication. No public tag, release,
package-manager submission or installed executable was changed by this slice.
The existing untracked held-out protocol was not opened or staged;
leaving it unread does not make it an access-controlled holdout.

The optimized working executable is frozen at
`target/evaluation/cxt-dev-mcp-tui.exe`, SHA-256
`736fc814ffb58523afab1c95e53f6bf8f5b060e5d59b465645d5637d22284cdf`.
It reports `cxt 0.5.2-dev.1`, so use the hash, not just that version string, to
distinguish it from the published prerelease. The published baseline and installed
executable hashes were checked and remain unchanged.

## Reproduced regressions

Six new tests went red before their corresponding fixes:

- MCP qualified Rust method lookup lost the impl owner. Same-named methods are
  now distinguished for the tested syntax, including a negative case ensuring a
  bare function call does not bind an unrelated impl member.
- Focused output copied every import, including imports in unrelated nested test
  modules. Recognized enclosing scopes now filter supporting imports; original
  symbol bodies and original source spans are preserved. This is not complete
  import-usage analysis or compiler resolution.
- Import-only query expansion inherited almost the anchor's relevance score. A
  three-file Python fixture reproduced a utility displacing a genuine query
  match at 1,200 tokens. Supporting imports now receive score 1; independent
  lexical/symbol/graph evidence still survives candidate merging.
- Output-only TUI settings reloaded files changed after review. Format, path
  style, manifest, compression, dry-run and CI settings now replan ready snapshots.
  The regression checks unchanged reviewed content, selection/pins, current JSON
  rendering and refusal of a stale export. Output-path, source-affecting and
  pending-refresh changes still reload.
- A disconnected TUI response channel left `busy` true indefinitely. It now
  clears the exportable workspace/plan and reports that cxt must be restarted.

Parser metadata was invalidated with schema
`cxt.symbols.v16-scoped-imports-tree-sitter-0.20`. Cache entries validate import
scope spans and ownership. MCP still writes no persistent cache. Candidate-wide
secret fail-policy, fixed MCP redaction, workspace/scan limits, source/HEAD
freshness and omitted-pin checks remain unchanged.

## Main validation gates

- `cargo fmt -- --check`: pass.
- `cargo test --locked --all-targets -j 1`: **187 passed**, one optional ignored.
- `cargo clippy --locked --all-targets --all-features -j 1 -- -D warnings`: pass.
- `cargo build --locked --release --bins --examples -j 1`: pass.
- Installed-Gitleaks regression, explicitly enabled against spawned optimized
  CLI calls: pass (5.41 seconds). Its public core calls use the linked test build.
- Optimized integration replay, serial test execution: **148 passed**, one
  optional ignored. `CARGO_BIN_EXE_cxt` pointed spawned CLI exports to the frozen
  executable; public core calls still use linked test code. Local logs are
  `target/evaluation/dev-optimized-serial-*.txt`. The earlier parallel failure
  remains retained and described below, so this is not unconditional acceptance
  under arbitrary machine load.
- Real stdio MCP replay on the frozen published and DEV executables: completed;
  see [request-level results](MCP_DOGFOODING.md#2026-10-15-dev-regression-fixes-and-replay).
- Isolated Windows terminal smoke: `--tui --no-cache --model cl100k --budget 2000`
  reached Ready with one file and 31 packet tokens, displayed Output, then quit
  with `q` and exit zero. No context export or clipboard write was requested.
- `git diff --check`: pass, including this verification report.

Main logs are local artifacts:
`target/evaluation/dev-mcp-tui-{tests,clippy,build,gitleaks}.txt`.
The stdio replay stores source fingerprints and metadata rather than source bodies
in `target/evaluation/dev-mcp-replay.jsonl`. No exposed cxt connector was available;
actual stdio requests were used instead of labeling CLI output as MCP.

## Synthetic retrieval checks

Each corpus ran through the frozen optimized executable's own evaluator. Reports
retain every attempt, source fingerprint and independently counted rendered
stdout cost in `target/evaluation/dev-mcp-tui-{ablation,bindings,relationships,corpus}.json`.

| Corpus | Attempts | Accepted | Failures |
| --- | ---: | ---: | --- |
| Ranking ablation | 96 | 96 | None |
| Import bindings | 32 | 32 | None |
| Shared relationships | 32 | 32 | None |
| Full synthetic corpus | 540 | 484 | 56 token-budget failures |

Acceptance means invocation and packet-budget acceptance, not successful code
retrieval or a solved coding task. Maps do not count as implementation coverage.
These are development-influencing synthetic fixtures, not independently reviewed
held-out tasks, measured competitor superiority or downstream AI task success.

Required-symbol recall is 100% for all focused/adaptive full-corpus attempts at
8K/16K/32K, and complete mode at 16K/32K. Complete mode at 8K accepts only 4 of 60
attempts, with zero required-symbol recall across that group; its 56 failures are
all classified `token_budget`. Binding/relationship expansion-on recall is 100%
versus 33.3% with expansion off. In the deliberately symbol-led ablations,
hybrid/symbol rankers have 100% recall and lexical/legacy have zero. These match
the previously recorded synthetic pattern; they do not establish real-task gains.

The generated graph workload (500 modules, 200 functions each) passed reuse and
mutation assertions. Cold/repeat/edit/rename/cache-disabled elapsed milliseconds
were 6,262 / 2,838 / 1,829 / 3,823 / 6,282. Repeat reused all 500 nodes; one source
edit rebuilt one node, rename/import edit rebuilt two. The local report is
`target/evaluation/dev-mcp-tui-graph.json`. This includes source reads/hashing and
is not a public-monorepo result or a controlled speedup comparison.

## Historical timeout: still open

The retained original failure is the ten-second `ready` deadline in
`all_settings_edit_and_feed_the_shared_final_plan`, with status
`Workspace refreshed; pins remain until explicitly cleared`. Four unchanged
baseline runs passed before these fixes (35 library tests each, eight test
threads). Logs: `target/evaluation/dev-tui-baseline-{1,2,3,4}.txt`.

The new full validation run passed all 37 library tests in 10.29 seconds, including
that settings test, without changing its ten-second **per-wait** deadline. The
whole suite's elapsed time is not a worker deadline. Diagnostics now also record
revision/workspace/plan state if it fails again. The unnecessary reload and
disconnected-channel bugs were independently reproduced, but neither is proven
to explain the historical failure. A green rerun is not closure of R2.

After an approval-service usage-limit error blocked an optional replay command
before execution, a user-requested retry ran all 37 library tests successfully
with eight test threads (39.67 seconds for the whole suite). The per-wait deadline
was still unchanged. This is another passing run, not proof of the old failure's
cause or absence under every load.

A later parallel optimized integration replay passed CLI (45), evaluation
failure-classification (2) and fidelity (11) tests, then stopped in intelligence
with 26 passed, three failed and one optional ignored. All three failures were
`Scan stopped: time limit reached`, with empty stdout and no partial export:
`exact_budget_is_checked_on_actual_stdout_for_every_output_format`,
`go_local_replacements_trace_the_replacement_package`, and
`cargo_target_specific_local_dependencies_are_discovered_conservatively`.
The failed run remains in `target/evaluation/dev-optimized-intelligence.txt`.
These are scan-budget refusals, not the TUI helper's ten-second assertion, and
must not be erased by a successful serial retry. No timeout was increased.
All 148 integration tests subsequently passed with `--test-threads 1`, including
all three previously failing intelligence tests (the intelligence suite took
27.72 seconds). This is consistent with load sensitivity, but is not a proven
root cause or a claim that all timeout behavior has been repaired.

## Remaining work

- M1: the original 2,500-token search still retains only `query.rs` as a map.
  At 8,000, all three explicit seeds now survive, but still as maps. Global
  coverage/representation selection and packet-metadata overhead remain open.
- M3: persistent lexical postings and bounded MCP session reuse are not added.
- R2: reproduce or explicitly disposition the historical load-sensitive deadline
  failure; do not increase the timeout and call it fixed.
- Independent held-out labeling, downstream model execution, new public release
  tags and package-manager updates are not part of this slice.
