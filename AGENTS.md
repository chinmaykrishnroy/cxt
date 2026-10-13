# Agent Notes for cxt

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
cargo run --              # bare interactive invocation should open TUI
cargo run -- --tui        # explicit TUI still works
```

## Architecture

`src/main.rs` only owns CLI/TUI handoff, delivery, and exit behavior. `src/tui.rs` may edit prepared selections but must not duplicate the packing/security/rendering pipeline. Selection and transformation belong in `cxt_core` modules. Keep context selection deterministic and local-first; do not add network or LLM calls to core selection paths without an explicit opt-in design.

