# Agent Notes for cxt

## Build

```powershell
cargo build --release
```

## Run locally

```powershell
cargo run -- src/main.rs --trace
```

## Install globally

```powershell
cargo install --path .
```

The installed binary is placed in `%USERPROFILE%\.cargo\bin`.

## PATH notes (Windows)

- Cargo/rustc live in `%USERPROFILE%\.cargo\bin`.
- `winget` lives in `%LOCALAPPDATA%\Microsoft\WindowsApps`.
- Both should be on the user `PATH` to use `cargo`, `cxt`, and `winget` from a fresh terminal.

## Useful commands

- `cxt --diff` — pack only changed/untracked files.
- `cxt src/main.rs --trace` — AST trace dependencies.
- `cxt --minify` — AST-aware minification (strip comments + whitespace).
- `cxt --output <FILE>` — write output to a file instead of the clipboard.
- `cxt <path>` — directory scan respecting `.gitignore`.
