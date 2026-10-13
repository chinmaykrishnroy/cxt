# Agent Notes for cxt

## Scope and references

For CLI/configuration semantics, defaults, output behavior or TUI changes, consult [docs/CLI_REFERENCE.md](docs/CLI_REFERENCE.md). For product positioning or retrieval improvements, consult [docs/PRODUCT_DIRECTION.md](docs/PRODUCT_DIRECTION.md). Keep `src/cli.rs` help and the option reference aligned; help/reference coverage is tested. `AGENT.md` is a pointer to this authoritative file.

## Validate

```bash
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Useful smoke tests

```bash
cargo run -- --help
cargo run -- . --dry-run
cargo run -- src/main.rs --trace --dry-run
cargo run -- . --query "context budget" --budget 20000 --dry-run
cargo run -- --git working --impact --dry-run
# Manual terminal smoke tests
cargo run --              # packs current directory to clipboard (CLI-first)
cargo run -- --tui        # explicit TUI
cargo run -- -T           # TUI short flag; -t remains trace
cargo run -- . --scan-max-entries 1 --dry-run # must fail, never export a partial pack
```

## Architecture

`src/main.rs` only owns CLI/TUI handoff, delivery, and exit behavior. `src/tui.rs` may edit prepared selections but must not duplicate the packing/security/rendering pipeline. Selection and transformation belong in `cxt_core` modules. Keep context selection deterministic and local-first; do not add network or LLM calls to core selection paths without an explicit opt-in design.

Directory, graph and tracing work must use the shared `scan` guard/budget. Home directories, their ancestors and filesystem roots require explicit broad-scan permission. Scan limits are independent of `--no-limit`; limit failures must block export, not silently truncate context. Time checks are cooperative and cannot interrupt an in-flight filesystem operation, parser or Git subprocess.

CLI invocation is non-interactive by default; only `--tui` / `-T` opens review. Preserve `-t` for tracing. Output defaults to clipboard unless a file/stdout/CI destination is selected. Report the actual fallback destination and preserve existing fallback/task files. Source previews, final plans and exports share core snapshots; failed scans, stale sources/HEAD, secret fail-policy findings and omitted pins block export.

Treat query selection, dependency/test suggestions, token counts and secret findings as heuristics. Keep current-checkout packing distinct from Git revision patch review. Preserve source-freshness checks and worker revision handling when editing TUI behavior.

## Release gate

Inspect Git changes and preserve unrelated work. Before a release, update Cargo.toml/Cargo.lock, CHANGELOG.md and relevant help/docs; run every validation command above plus applicable runtime smokes. Record local-platform verification separately from GitHub cross-platform build results.

Commit/push/tag only when authorized. Use an unused annotated `vMAJOR.MINOR.PATCH` tag on the verified commit; tag pushes trigger `.github/workflows/release.yml`. Verify remote branch/tag identities and that the release workflow starts. Do not move existing public tags or claim download assets exist until the workflow publishes them. Maintainer manual packaging/publication instructions are in [packaging/README.md](packaging/README.md).

