# cxt roadmap

`cxt 0.4` adds a review workspace over the reusable engine. The long-term identity remains: **do not dump a repository into an LLM; determine the smallest useful slice for the task.**

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

## Next Git/TUI work

- Rich commit/history browser and better rename/deletion representation
- Incremental graph refresh and larger real-world retrieval benchmarks
- Theme/config customization

## Next — ecosystem-aware resolvers

- Node package exports/workspaces, tsconfig references
- Python `pyproject.toml`, namespace packages and editable/src layouts
- Cargo workspaces/modules
- `go.mod` / `go.work`
- Maven/Gradle source roots
- `.csproj` / solutions
- CMake/include paths

## 0.5 — persistent code graph

- Files, symbols, imports, reverse dependencies, entry points and tests
- Incremental graph cache keyed by file hashes
- `cxt --reverse <file>` and graph inspection

## 0.6 — symbol-level context

- Symbol index across supported languages
- `cxt file --symbol updateVolume`
- Slice a symbol with required imports/types/callers/callees
- Whole-file fallback when slicing confidence is low

## 0.7 — ranking and budget optimizer

- Exact tokenizer backends in addition to fast offline estimates
- Lexical + symbol + graph + Git recency scoring
- Budget optimizer that can choose slices rather than whole files
- Retrieval quality benchmark corpus

## 0.8 — tests and change impact

- Framework-aware test discovery
- Coverage/test metadata integration where available
- Configurable impact depth and graph explanations

## 0.9 — agents and embedding

- MCP server: `get_context`, `search`, `trace`, `get_symbol`, `get_diff_context`, `get_related_tests`
- Stable library API
- Incremental cache and monorepo hardening

## 1.0

- Stable config/API/output schema
- Reproducible context manifests
- Polished cross-platform package distribution
- Public performance + retrieval-quality benchmarks

