# Agent Notes for cxt

## Scope and references

For CLI/configuration semantics, defaults, output behavior or TUI changes, consult [docs/CLI_REFERENCE.md](docs/CLI_REFERENCE.md). Retrieval/index contracts are in [docs/CONTEXT_ENGINE.md](docs/CONTEXT_ENGINE.md), MCP in [docs/MCP.md](docs/MCP.md), evaluation in [docs/EVALUATION.md](docs/EVALUATION.md). Keep `src/cli.rs` help and the option reference aligned; help/reference coverage is tested. `AGENT.md` is a pointer to this authoritative file.

Portable agent instructions live in `skills/cxt/SKILL.md`; registration/export conventions are in [docs/AGENT_SKILL.md](docs/AGENT_SKILL.md). `skill` is the shared non-overwriting writer for CLI/TUI. Skill export never scans source, edits client configuration or runs through MCP. Keep the guide's option coverage test passing and preserve its instruction-only, permission-neutral behavior.

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
cargo run -- src/engine.rs --symbol run_snapshots --context focused --model cl100k --budget 8000 --stdout
cargo run -- --evaluate eval/ablation.json --output target/ablation.json
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

Treat query selection, dependency/test suggestions and secret findings as heuristics. Named cl100k/o200k counts are exact for the rendered packet only; other profiles estimate. Keep current-checkout packing distinct from Git revision patch review. Preserve source-freshness checks and worker revision handling when editing TUI behavior.

`symbols` owns original AST spans and representation; `index` caches hash-validated parser metadata, never source bodies; `query` owns explainable local rankings; `engine` verifies final rendered budgets. Spans are original half-open UTF-8 byte ranges and one-based lines, not positions in minified/redacted output. Maps do not count as implementation coverage. Pinning must not silently substitute maps for essential implementations. Unsupported syntax must fall back without inventing code.

`dependency_index` owns persistent static edges and resolver-input observations. New resolver filesystem reads/probes must be observed or explicitly mark resolution uncacheable; config changes and missing-target creation must invalidate affected edges. Apply junk-directory rules relative to the explicitly selected project, not its ancestors. For incremental/performance changes, run `cargo run --release --example graph_benchmark -- 500 200`; see the evaluation guide for provisional public inputs and competitor tracks.

MCP serves one canonical workspace over stdio with fixed redaction, no cache writes and bounded requests; never expose CLI policy weakening, output writes or arbitrary commands as tool arguments. Resolver source/configuration reads must honor the workspace boundary. Scanner failures block export. Cache corruption reparses safely; changes in indexing logic require a cache schema/version invalidation.

Use public CLI exports and core transformation/rendering APIs as regression boundaries. Run the checked-in synthetic corpus and ranking ablations for retrieval changes; retain failures, exact final-token counts and source fingerprints. Synthetic recall is not downstream AI task success, competitor superiority or a large-monorepo performance result. Real tasks require reviewed labels and public/redistributable sources; do not use private/customer source without explicit permission.

## Release gate

Inspect Git changes and preserve unrelated work. Before a release, update Cargo.toml/Cargo.lock, CHANGELOG.md and relevant help/docs; run every validation command above plus applicable runtime smokes. Record local-platform verification separately from GitHub cross-platform build results.

Commit/push/tag only when authorized. Use an unused annotated `vMAJOR.MINOR.PATCH` tag on the verified commit; tag pushes trigger `.github/workflows/release.yml`. Verify remote branch/tag identities and that the release workflow starts. Do not move existing public tags or claim download assets exist until the workflow publishes them. Maintainer manual packaging/publication instructions are in [packaging/README.md](packaging/README.md).

