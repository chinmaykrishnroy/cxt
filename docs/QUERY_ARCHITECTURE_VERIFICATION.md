# Query architecture foundation: local verification

For the versioned prerelease executable and its fresh checks, see
[0.5.2-dev.1 validation](releases/v0.5.2-dev.1-validation.md).
The foundation snapshot below predates that version and identifies a different
binary; its measurements are retained as historical evidence, not substituted
for prerelease acceptance.

Date: 2026-10-15. Version: **0.5.2-dev, unreleased**. This is the first bounded
implementation of the [query architecture](QUERY_ARCHITECTURE.md), not the entire
design or a claim of best-in-class retrieval.

## Verified implementation

- Original AST reference sites, declaration/binding scopes and supported import
  aliases; one shared binding module for graph expansion and focused helpers.
- Python from-import/simple module aliases and JavaScript/TypeScript
  named/default/namespace imports with direct declaration exports.
- Regressions for parameter shadowing, Python local rebinding and class
  namespaces, TypeScript block scopes, unknown receivers, duplicate definitions,
  unresolved imports and configuration changes with cache reuse enabled.
- Bounded weighted best-first traversal over syntactic `CALLS`/`REFERENCES`,
  including inverse traversal, source locations and binding/heuristic labels.
- Limited unresolved-import and suppressed-hub diagnostics. No compiler-resolved
  labels, new model calls, weakened secret policy or workspace-policy override.

## Binary and source identity

Measurements use an optimized Windows x86-64 build with Rust 1.97.1. The working
tree is uncommitted over `f4f4a591785d0db29fa92a46fae898584a5b2c7a`;
that base commit alone does **not** identify the development sources.

| Executable | SHA-256 |
| --- | --- |
| Development `target/evaluation/cxt-0.5.2-dev-f3d1c499b486.exe` | `f3d1c499b486d63a97408b626c72de2a5e1d58559eb8657803456859740ad74f` |
| Preserved published 0.5.1 Windows executable | `20dbc4c8dc77925b8f36f27a399bce5055717e8a321053e3f636bf85e1ced44b` |

The baseline reports its version as 0.5.1. Each executable ran its **own**
`--evaluate` against the identical corpus, context/ranker/expansion modes, token
limits and original labels. Source fingerprints match for both compared corpora.
The legacy `--eval-baseline` track was not used: it selects complete/legacy mode
and would not be an equivalent-mode modern-release comparison.

## Validation gates

- `cargo fmt -- --check`: pass.
- `cargo test --all-targets -j 1`: **181 passed**, one optional test ignored.
- Installed-Gitleaks regression, explicitly enabled: pass.
- `cargo clippy --all-targets --all-features -j 1 -- -D warnings`: pass.
- `cargo build --release --bins --examples -j 1`: pass.
- All **144 integration cases**, plus the optional Gitleaks test, replayed with
  `CARGO_BIN_EXE_cxt` pointing to the hash-identified optimized executable: pass.
  Public core calls in those test programs still exercise their linked test build;
  spawned CLI exports use the optimized executable.
- Optimized runtime smokes pass: help, directory dry-run, source tracing, Git
  working/impact, named-symbol output and query output. A one-entry scan cap
  rejects export with exit 2; no partial pack is delivered.

An initial parallel full run hit the existing TUI test's ten-second per-worker
deadline. The same test passed in isolation without changing its timeout, and
both the unchanged full rerun and the final post-fix full run passed. The initial
failure is retained, not called fixed or discarded. Its root cause is unconfirmed.
Logs: `target/evaluation/query-architecture-tests{,-repeat,-final}.txt`.

These are local Windows results. No new hosted CI, macOS/Linux/ARM runtime result,
physical-terminal TUI acceptance, signed artifact, package-manager publication,
PATH replacement, commit, push or release tag is claimed.

## Retrieval results

Counts are actual rendered JSON stdout, independently recounted by the evaluator
with ordinary-text cl100k. Maps do not count as implementation coverage. Repeated
phases and mode/budget combinations are **not independent tasks**.

### New import-binding regressions

`eval/bindings.json`: two synthetic, development-influencing Python/TypeScript
tasks, focused/adaptive at 2K/4K, cold/repeat, expansion on/off. Each binary has
32 accepted invocations, with no refused or oversized attempts.

| Expansion | Attempts per binary | 0.5.1 span recall | Development span recall | Mean packet tokens, baseline / development |
| --- | ---: | ---: | ---: | ---: |
| On | 16 | 66.7% | 100% | 420.5 / 640.5 |
| Off | 16 | 33.3% | 33.3% | 312 / 312 |

This demonstrates recovery of the second imported-helper caller despite aliasing
and duplicate names in these fixtures. Better coverage costs more tokens, while
remaining below the same limits. Negative relationship assertions are in the
14 public CLI/core resolver regressions, not an aggregate graph-precision metric.

Expansion-on mean process latency was 170.3 ms for baseline versus 189.5 ms for
development; mean peak working set was 34,613,760 versus 34,753,536 bytes. These
small sequential local runs are not controlled latency/memory distributions, and
do **not** establish a performance improvement. Persistent search remains work
to do, not an already-proven optimization.

### Existing shared-state and synthetic controls

- `eval/relationships.json`: both executables retain 100% mean labeled-span recall
  with expansion on, and 33.3% with it off. All 32 attempts per binary accept;
  mean packet sizes remain 648 and 317.5 tokens respectively.
- `eval/ablation.json`: all 96 attempts accept. Hybrid/symbol modes retain 100%
  labeled-span recall; lexical/legacy retain 0% on these deliberately symbol-led
  fixtures. This is not balanced ranker-quality evidence.
- `eval/corpus.json`: focused/adaptive retain 100% labeled-span recall at all
  8K/16K/32K budgets, 60 accepted attempts per context/budget. Mean packets are
  1,108.9 tokens at 8K/16K and 2,854.4 at 32K. Complete mode has **56 retained
  token-budget failures** at 8K; its four accepted attempts have zero mean span
  recall. Complete 16K/32K accepts all attempts with 100% recall. Overall,
  484/540 attempts accept.

Reports under `target/evaluation/`:

- `bindings-architecture-f3d1c499b486.json`
- `bindings-published-0.5.1-architecture.json`
- `relationships-architecture-f3d1c499b486.json`
- `relationships-published-0.5.1-architecture.json`
- `ablation-architecture-f3d1c499b486.json`
- `retrieval-architecture-f3d1c499b486.json`

### Generated graph workload

`cargo run --release --example graph_benchmark -- 500 200` passes mutation/reuse
assertions across 500 Python modules and 100,000 functions. Cold/repeat/edit/
rename/cache-disabled workspace times were 5,652 / 1,392 / 1,321 / 1,315 / 3,153 ms.
Repeat reuses all 500 source nodes, one edit rebuilds one, and rename/import edit
rebuilds two. Report: `graph-architecture-f3d1c499b486.json` in the same directory.
This is a generated workspace/dependency-cache workload, **not** a 100,000-function
query benchmark, a public monorepo, a controlled speedup or a zero-read index.

## Runtime example

```powershell
.\target\release\cxt.exe eval/fixtures/bindings-python `
  --query "changing one room's volume affects every room" `
  --context adaptive --model cl100k --budget 8000 --manifest `
  --format json --output target/evaluation/query-architecture-smoke.json
```

This fixture exports the original `change_volume`, `commit` and `dispatch`
implementations in a 1,005-token packet, excluding the two distractor files.
It demonstrates static binding traversal, not general understanding of arbitrary
room/audio terminology or a diagnosis of an actual application's state bug.

## Remaining gates and work

The [architecture status table](QUERY_ARCHITECTURE.md#current-development-status)
separates implemented work from planned compiler/type resolution, richer graph
relations, task/vocabulary grounding, searchable postings and coverage-aware span
optimization. Current retrieval still has heuristic fallbacks and rigid span
allocation. It is not a complete program slice.

No new independently reviewed held-out public-task result, competitor-superiority
result or downstream coding-success experiment was run. Provisional tasks are not
gold labels. Coding experiments remain offline-harness-only in scope; this pass
does not implement or execute a model/agent harness. Candidate-wide secret
fail-policy remains unchanged, including original findings outside focused spans
and candidates later omitted by budgeting.
