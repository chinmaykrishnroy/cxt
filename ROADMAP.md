# cxt roadmap

`cxt 0.5.0` adds experimental task-aware retrieval over the reusable review engine. The long-term identity remains: **do not dump a repository into an LLM; determine a reviewable useful slice for the task.**

## Implemented in 0.5.0

Source/export hardening, AST symbol indexing, hybrid BM25/symbol/reference ranking, complete/map/focused/adaptive representations, exact offline named tokenizers, parser-metadata reuse, scoped read-only MCP, optional Gitleaks, improved ecosystem resolution, Symbols TUI controls and a synthetic evaluation/ablation runner are implemented. See [engine contracts](docs/CONTEXT_ENGINE.md) and [evaluation](docs/EVALUATION.md).

This combines the initial implementation work across the milestones below; it does **not** complete every milestone's evidence gate. Stable 1.0 APIs, compiler-complete graphs, zero-read persistent querying, downstream task-success gains and competitor superiority are not established. Release assets are available only after the tagged build workflow succeeds.

## Next: retrieval quality, not feature count

The updated external assessment identifies the main remaining gap: finding the correct implementation in real public tasks. The next quality milestone should categorize retained refusal/timeout/budget failures by tool build and stage, improve definition/reference relationships and low-confidence fallback without weakening secret policy, and measure held-out tasks with independently reviewed labels. A persistent searchable index is a candidate latency improvement, not an existing feature. Controlled downstream coding-task success and cost comparisons remain a separate future experiment; proposed recall/acceptance targets are not measured results.

## Implemented in 0.2

- Engine/library split (`cxt_core` + `cxt` binary)
- Multiple paths and stdin path lists
- Include/ignore/extension/language filters
- Git working/staged/unstaged/untracked/tracked/range selection
- Reverse-dependency + related-test impact mode
- Multi-entry AST tracing
- Deterministic query ranking + dependency expansion
- Hard token budgets with model-family profiles
- Secret redact/fail/allow policies
- Junk/generated/vendor/build filtering
- Markdown/XML/JSON renderers
- Relative/absolute/no-path output
- Provenance manifest + hashes
- stdout/clipboard/files + gzip/zstd
- Shell completion generation
- Global + project configuration
- Cross-platform release workflow retained; macOS tarballs added for the installer

## Implemented in 0.3

- Ratatui full-screen TUI (`--tui`)
- Collapsible file tree with aggregate directory selection state
- Interactive file toggles with keyboard, mouse-wheel and click navigation
- Fuzzy path/reason filtering, relevance scores, source preview and live token totals
- Git status badges
- Editable task query with ranked-candidate rebuild
- Editable token budget
- Interactive minify/format/secret-policy controls
- Clickable toolbar + model/format/path/secrets pickers and budget/query input overlays
- Animated large-repository preparation and CLI packing progress
- Non-fatal unreadable/binary/walk-error handling with an in-TUI skipped-item inspector
- Trace/impact refresh controls
- Shared prepared-selection API (`engine::prepare` + `engine::run_prepared`)
- Explicit `--no-minify` override

## Implemented in 0.4 — review workspace

- Six workflow views with Git patches/deletions, resolved imports and related-test suggestions.
- Shared safe snapshots and final CLI/TUI budgeting; exact reviewed output is delivered after freshness checks.
- Background planning with obsolete-result rejection, complete previews and editable context options.
- Pinned selections and missing-dependency/budget-exclusion diagnostics.
- Saved task options, selections and source fingerprints without source content.
- Repository-relative query ranking and local Rust library/module tracing.

## Source fidelity and export integrity (implemented; release gates required)

- Syntax-aware compaction preserving literal content, token boundaries and meaningful comment directives.
- Collision-free Markdown boundaries and valid, recoverable JSON/XML content.
- Parser-backed regressions for multiple languages, line endings and adversarial source.
- Accurate help/docs and explicit limits on fidelity, security and token claims.
- Release gate: full tests, format, strict Clippy, optimized build and runtime checks. Do not advertise universal losslessness or security guarantees.

## 0.5 — measurable retrieval

- Versioned, locally runnable task corpus with reviewed required-file/symbol labels; see [evaluation protocol](docs/EVALUATION.md).
- Record the 0.4.2 baseline before changing query ranking; evaluate at fixed budgets and publish omissions/failures.
- Initial symbol index with names, kinds, qualified identities and original byte/line ranges.
- Experiment with BM25 and symbol signals, retaining deterministic offline fallbacks and selection reasons.
- Exit gate: reproducible evaluation and language-coverage fixtures, not an unsupported superiority claim.

## 0.6 — focused context packets

- Map, focused implementation and complete-file representations; whole-file fallback when slicing confidence is low.
- Budget symbols/spans with imports, related tests, explicit exclusions and source provenance.
- Keep CLI, TUI and library consumers on the same reviewed plan.
- Exit gate: demonstrated required-symbol recall and relevant-context precision at fixed cost, plus source-fidelity checks.

## 0.7 — accurate accounting and incremental indexing

- Named offline exact tokenizers on the final rendered packet; visibly estimated fallback profiles.
- Content/parser/config-keyed cache invalidation for edits, deletions, renames and branch changes.
- Measure cold/warm latency and memory; optimize only with reproducible evidence.

Named tokenizers, hash-validated parsing and configuration-aware persistent dependency edges exist now. A generated 500-module/100,000-function workload verifies reuse/edit/rename behavior; public large-monorepo warm-query gains remain a separate evidence gate. Source rereads and fingerprinting are still required.

## 0.8 — secure agent integration

- Workspace-scoped MCP operations for search/context/symbol/trace/change/test queries.
- Stable structured responses with spans, costs, reasons, omissions and source fingerprints.
- Explicit roots, symlink/read/output limits and opt-in network behavior; never unrestricted filesystem access.

## 0.9 — ecosystem reliability

- Node package exports/workspaces, JSONC tsconfig inheritance and references.
- Python namespaces/src layouts, Cargo workspaces, Go modules/workspaces and other build-system roots.
- Framework-aware test discovery and unresolved-edge diagnostics.
- Arduino `.ino` language coverage, large-monorepo stress tests and strengthened secret-policy fixtures.
- Evaluate an established offline secret-scanning backend before raising security claims.

The current implementation covers common JSONC/Node/Python/local Cargo/Go workspace/Java source-root cases and optional Gitleaks. Cargo target tables and local Go replacements have dedicated regressions; full compiler/target/feature/version semantics, TypeScript build references and framework-aware discovery remain coverage gaps, not completed compiler-equivalence claims.

## Deliberately deferred

Themes, hosted accounts, cloud sync, built-in chat, a new autonomous coding agent and mandatory embedding/LLM retrieval. Optional reranking must earn its cost in the evaluation corpus. Competitor comparisons require pinned versions and measured results.

## 1.0

- Stable config/API/output schema
- Reproducible context manifests
- Polished cross-platform package distribution
- Public performance + retrieval-quality benchmarks

