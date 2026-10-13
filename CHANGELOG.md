# Changelog

## 0.4.1 (2026-10-14)

- Made startup CLI-first: bare `cxt` packs the current directory to clipboard; `--tui` / `-T` explicitly opens interactive review, and `-t` remains tracing.
- Guarded home directories, their ancestors and filesystem roots behind explicit broad-scan permission.
- Added configurable entry, input-byte, depth and cooperative time limits shared by directory selection, tracing, impact analysis, snapshots and TUI graphs/patch review.
- Added scan controls to TUI Settings and blocked reuse of old plans after a failed scan.
- Corrected clipboard-fallback reporting and refused to overwrite an existing fallback output file.
- Added scan-safety regressions and documented current query limitations and product direction.
- Grouped CLI help, exposed the budget alias, and added usage examples and a complete option/configuration reference.
- Made parallel scan/review/TUI test fixtures collision-resistant on coarse system clocks.

## 0.4.0 (development baseline, included in 0.4.1)

- Rebuilt the TUI around Source, Changes, Dependencies, Checks, Output and Settings views.
- Added actual Git patches (including deletions), resolved import/reverse-import inspection, related-test suggestions, pinned selections, and final budget-exclusion explanations.
- Added editable selection/filter/security/delivery options, choice controls, cursor-aware text input, paste support, mouse navigation and responsive layouts.
- Moved scan/graph/plan/freshness work to a background worker with revision checks to discard obsolete results.
- Unified safe source previews and CLI/TUI exports through core file snapshots and one packing pipeline; removed the preview's 220-line cutoff.
- Export preserves the reviewed output and blocks stale sources, changed HEAD, secret fail-policy violations and omitted pinned files.
- Added saved task options/selections/source fingerprints, without storing source contents or overwriting existing task files.
- Fixed query ranking to use repository-relative paths, and added local Rust library/module/grouped-use tracing.
- Explicit zero budgets/file caps disable those limits, including project defaults.
- Added workflow regressions and terminal rendering tests.

## 0.3.6

- Bare `cxt` now opens the interactive TUI automatically when stdin/stdout are terminals.
- `cxt .` remains the explicit non-TUI current-directory pack command, while any other CLI arguments preserve normal CLI behavior unless `--tui` is supplied.
- Non-interactive/piped invocations do not auto-open the TUI, preserving scripting and CI behavior.
- Added startup-behavior tests for interactive vs non-interactive invocation.

## 0.3.5

- Fixed the final Rust 1.97 Clippy `-D warnings` findings reported after the full 64-test suite passed.
- Simplified `SelectionPreview` destructuring to use shorthand mutable-field syntax.
- Reduced the toolbar helper argument count without suppressing Clippy by grouping the x/y cursor into one position tuple.
- No runtime behavior change; this is a lint-clean maintenance release on top of 0.3.4.

## 0.3.4

- Fixed malformed/corrupted credential-like test fixtures that could become syntactically invalid when source sanitization rewrote them.
- Secret-detection tests now construct synthetic credential-shaped values at runtime instead of embedding credential-looking literals in the repository.
- Fixed the token/password integration test so it checks that the original synthetic values are absent rather than contradictorily asserting the redaction marker is both present and absent.
- No runtime behavior change; this is a compile/test-fixture hardening release on top of 0.3.3.

## 0.3.3

- Restored animated CLI progress for repository preparation and packing, including the current file, byte size, and a `large` marker while slower files are being processed.
- Made directory-walk/read failures non-fatal: permission-denied, vanished, binary, invalid-text, and other unreadable inputs are skipped while the rest of the context continues.
- Added structured skipped-item details to the engine and a `!`/clickable `skipped:N` TUI inspector.
- Added an animated TUI loading screen so large repositories no longer appear frozen during initial scan/token estimation.
- Reworked the TUI header into a clickable developer toolbar: model, budget, format, path style, secret policy, minify, trace, impact, manifest, preview wrap, query, skipped items, and Generate.
- Added picker/input overlays for model, budget, format, path style, secrets, and query; changing token-affecting options recalculates the live tree estimates.

## 0.3.2

- Fixed `--help` long description so the CLI branding assertion remains true under Clap's long-help rendering.
- Fixed all Clippy `-D warnings` findings reported with Rust 1.97: collapsed chained `replace` calls, derived enum defaults, removed explicit auto-deref, and removed a clone on a `Copy` value.
- Intended validation target: `cargo test --all-targets`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo build --release`.

## 0.3.1

- Fixed Rust type inference in `query_terms` by explicitly typing the term vector as `Vec<String>`.
- Fixed a partial-move ownership error in the TUI initializer by cloning `SelectionPreview` metadata before passing the prepared selection to `replace_prepared`.
- No CLI or TUI behavior changes; this is a compile-fix release.

## 0.3.0

- Added a full-screen Ratatui TUI via `cxt --tui`.
- Added a collapsible interactive file tree, fuzzy path/reason filtering, live token totals, per-file token estimates/relevance scores, selection reasons and source preview.
- Added Git porcelain status badges to the TUI.
- Added TUI controls for the task query, token budget, minification, output format, secret policy, tracing, impact expansion and refresh.
- Added mouse-wheel navigation, click-to-toggle/fold, keyboard navigation and help overlay.
- Split engine execution into `prepare()` and `run_prepared()` so CLI and TUI share the same selection/packing pipeline.
- Added `--no-minify` so project/global minification can be explicitly disabled, including from the TUI.
- TUI previews redact likely secrets unless the active policy is `allow`.
- Upgraded the package version to `0.3.0` and added Ratatui/Crossterm dependencies.

## 0.2.0

- Refactored the monolithic CLI into the reusable `cxt_core` engine/library.
- Added multi-path and stdin path selection.
- Added include/ignore, extension and language filtering.
- Added Git working/staged/unstaged/untracked/tracked/range modes.
- Added `--impact` reverse dependency and related-test expansion.
- Added deterministic `--query` ranking with dependency expansion.
- Added model-family token profiles and hard token budgets.
- Added secret policies (`redact`, `fail`, `allow`) and broader detection patterns.
- Added Markdown/XML/JSON renderers, path styles and provenance manifests.
- Added stdout/CI output, gzip/zstd file compression and shell completions.
- Added project-local `.cxt.toml` configuration merging.
- Added macOS `.tar.gz` release assets so the Unix installer works on macOS.
- Changed the Cargo package name to `cxt-ai`; the installed executable remains `cxt`.

