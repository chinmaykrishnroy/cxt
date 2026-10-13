<div align="center">
  <h1>cxt</h1>
  <p><strong>Build the smallest useful code context for the task in front of your coding agent.</strong></p>
  <p>Fast selection + AST tracing + Git impact analysis + query ranking + token budgets + secret protection.</p>
</div>

`cxt` is a local-first context engine for ChatGPT, Codex, Claude, Cursor and other coding assistants. The executable remains `cxt`; the crates.io package is named `cxt-ai` to avoid package-name collisions.

## What changed in 0.4

**Review changes, explore dependencies, and inspect the exact context before sharing it.** cxt now provides a terminal workspace with Git patches, import relationships, related-test suggestions, pinned files, final budget exclusions, and saved tasks. Source previews and exports share the same core transformation pipeline. Selection stays local and deterministic.

```text
CLI / CI / TUI
      |
      v
selection ---- git ---- query
   |             \       /
   v              impact
AST tracer <---- resolver
   |
   v
filter -> secrets -> minify -> token budget
   |
   v
Markdown / XML / JSON -> clipboard / stdout / file / gzip / zstd
```

### Highlights

- Multiple input paths and stdin path lists.
- `.gitignore`-aware directory scans with `--include`, repeatable `--ignore`, extension and language filters.
- Git modes: `working`, `staged`, `unstaged`, `untracked`, `tracked`, plus arbitrary commit/branch ranges.
- `--impact`: changed files + reverse dependencies (up to two hops) + likely related tests.
- Multi-entry AST dependency tracing.
- `--query`: deterministic lexical ranking, then dependency expansion around the strongest matches.
- Hard `--max-tokens` / `--budget` and `--max-files` limits.
- Model-family token estimation profiles (`heuristic`, `gpt-5`, `o200k`, `cl100k`). These are offline estimates, not billable-token guarantees.
- `--secrets redact|fail|allow`, including common API/token/JWT/private-key/credential URL patterns.
- Generated/vendor/cache/build/lock/binary safety defaults.
- Markdown, XML and JSON output.
- Relative, absolute or hidden path headers.
- Optional provenance manifest with Git commit, selection reasons and SHA-256 hashes.
- stdout/clipboard/file delivery plus gzip/zstd compression for file output.
- Shell completion generation.
- Project-local `.cxt.toml` layered over the global config.
- A reusable `cxt_core` library crate.
- **Interactive Ratatui workspace** with Source, Changes, Dependencies, Checks, Output and Settings views.
- Git patches including deleted files, resolved import/reverse-import inspection and likely related tests.
- Pinned essential files, explicit final budget exclusions, safe previews, and source-freshness checks before export.
- Background scans/planning, editable options, mouse navigation, keyboard choice controls and saved task selections.
- Animated loading/packing feedback for large repositories and structured reporting of unreadable/binary/walk-error files instead of aborting the whole pack.

## Install

### Release installer

```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/chinmaykrishnroy/cxt/releases/latest/download/cxt-installer.sh | sh
```

```powershell
# Windows PowerShell
irm https://github.com/chinmaykrishnroy/cxt/releases/latest/download/install.ps1 | iex
```

Both installers fetch `SHA256SUMS.txt` and verify the release asset before installation.

### From source

```bash
git clone https://github.com/chinmaykrishnroy/cxt.git
cd cxt
cargo install --path .
```

The installed executable is still named `cxt`.

## Quick start

```bash
# Open the interactive TUI in the current directory (interactive terminals)
cxt

# Pack the current project directly without opening the TUI
cxt .

# Multiple roots/files
cxt src Cargo.toml README.md

# Legacy diff mode still works (working tree + staged + untracked)
cxt --diff

# New Git selection modes
cxt --git staged
cxt --git unstaged
cxt --git untracked
cxt --git tracked
cxt --git HEAD~3..HEAD
cxt --git main...HEAD

# Changed code + reverse dependencies + related tests
cxt --git working --impact

# Trace more than one entry point
cxt src/main.ts src/admin.ts --trace

# Task-oriented context selection
cxt . --query "Snapcast volume slider does not synchronize" --budget 40000 --model gpt-5

# Filters
cxt . --include "src/**" --ignore "**/*.generated.ts" --lang typescript
cxt . --ext rs,toml,md

# Render to stdout without status noise mixed into the document
cxt . --stdout

# JSON manifest
cxt . --format json --manifest --output context.json

# Compressed context artifact
cxt . --format json --manifest --output context.json.zst --compression zstd

# Fail CI if a likely secret would be packed
cxt . --secrets fail --ci > context.md

# Inspect selection/reasons without copying or writing output
cxt . --query "ticket pricing bug" --dry-run
```

## Interactive TUI

Launch it simply with:

```bash
cxt
```

On an interactive terminal, bare `cxt` opens the TUI immediately. Use `cxt .` when you explicitly want the non-interactive current-directory packing flow. `cxt --tui` remains available when combining the TUI with other selectors/options.

Or seed the TUI with the same selectors used by the CLI:

```bash
# Start from current Git changes and automatically add impacted code/tests
cxt --git working --impact --tui

# Start from a ranked task-oriented candidate set
cxt . --query "Snapcast volume slider does not synchronize" --budget 40000 --tui

# Trace one or more explicit entry files, then refine visually
cxt src/main.ts src/admin.ts --trace --tui
```

Use the six clickable tabs, or keys `1`–`6`:

| View | Purpose |
| --- | --- |
| Source | Select files/subtrees, pin essential files, inspect full transformed source |
| Changes | Inspect Git patches, including deletions; untracked text uses the safe source preview |
| Dependencies | Explore resolved imports, reverse imports and related-test suggestions |
| Checks | Inspect missing resolved dependencies, suggested tests, secret findings, skipped inputs and final budget exclusions |
| Output | Review the final Markdown/XML/JSON document, including rendered overhead |
| Settings | Edit input paths, Git mode/range, query, budget, model, format, paths, secrets, minify, trace, impact, manifest, include/ignore/ext/lang filters, hidden files, file cap, safety limits, destination, compression, dry-run and CI mode |

Source and patch previews redact detected secrets unless `allow` is explicitly selected. `fail` still sanitizes previews but blocks export. Dependency/test suggestions describe static imports and filename heuristics; they are not complete call graphs or coverage guarantees. Git comparisons show revision patches; packed contents come from the current checkout.

Failure-report line navigation uses original source lines and requires minification to be disabled. Minified previews have transformed line numbers.

Scanning and plan changes run in the background. The tree marks selected files with `●`, pins with `★`, and selected files excluded from the final plan with `!`. A pin gives a file priority; export is blocked if a pinned file cannot fit. Narrow terminals switch to stacked panes; the minimum size is 52 × 15.

Useful keys:

| Key | Action |
| --- | --- |
| `↑/↓`, `j/k`, mouse wheel | Move through the tree (wheel over Preview scrolls source) |
| `Tab`, `1`–`6`, tab click | Switch workflow views |
| `o` / `F2` | Open Settings; Enter/click edits an option |
| `←/→`, `Enter` | Fold/unfold the current directory |
| left click | Select a row; click its checkbox area to toggle selection |
| `Space` | Toggle a file or the whole directory subtree selection |
| `a` / `n` | Select all / none matching the path filter |
| `/` | Filter paths using substring terms |
| `e` | Edit the task query and rebuild ranked candidates |
| `b` | Edit the token budget (`0` = unlimited for this run) |
| `c` | Choose a Git mode or enter a comparison range (`all` disables Git selection) |
| `P` / `U` | Pin/unpin current file / clear all pins |
| `F` | Locate a `file:line[:column]` from a failure report |
| `S` / `L` | Save/load a named task (options, selections and hashes; no source contents) |
| `r` | Refresh from disk/Git |
| `J` / `K` | Scroll source preview down / up |
| `w` | Focus preview; arrows scroll vertically/horizontally |
| `v` | Toggle preview wrapping |
| `g` | Export the reviewed plan after source/HEAD freshness checks |
| `↑/↓` in option dialogs | Choose a suggested enum/boolean value |
| `Ctrl+u` in dialogs | Clear input; cursor keys, Delete/Backspace and paste are supported |
| `?` | Help |
| `q` / `Ctrl+C` | Quit without generating |

`g` respects the normal destination flags. For example, `cxt --tui --output context.md` opens the selector first and writes the final selection to `context.md`. Without an output destination, the normal clipboard/fallback behavior is used.

The engine exposes `prepare()`, `snapshot_file()` and `run_snapshots()`. The TUI uses the same transformations and final budget calculation as the CLI, and hands the reviewed result to normal delivery. Changed sources or Git HEAD require refresh before export. Save tasks under `.cxt-task*.json` (ignored by this repository); saving refuses to overwrite an existing file. Loaded tasks report changed/unavailable files, and unavailable pins remain visible in Checks until cleared.

### Large repositories and unreadable files

Interactive terminals show preparation activity and CLI packing progress. cxt treats permission-denied paths, directory-walk errors, vanished files, binary inputs, and text-read failures as non-fatal skips; the remaining files continue packing. The CLI prints a compact skipped-item summary; the TUI includes recorded details in Checks. The import graph excludes common build/vendor directories and source files over 1 MiB even when packing limits are disabled. Progress rendering is disabled for CI/non-terminal output.

## Query mode

`--query` does not call an LLM or send code anywhere. It combines filename/repository-relative path matches with content matches, ranks candidates, then traces dependencies from the strongest matches. The checkout's parent directory does not affect ranking. Query mode defaults to at most 40 files unless `--max-files` is provided (`0` disables the cap).

Example:

```bash
cxt . \
  --query "Snapcast volume slider doesn't synchronize" \
  --budget 40000 \
  --model gpt-5 \
  --manifest \
  --output context.md
```

With `--dry-run`, each selected file shows its score and reasons, such as `query-path:volume`, `query-content:2-terms`, or `dependency-of:src/VolumeSlider.tsx`.

## Git impact mode

```bash
cxt --git working --impact
```

The initial seed is the Git selection. `cxt` scans supported source files, constructs reverse import edges, follows dependents up to two hops, and adds likely matching tests. This is intentionally conservative; build-system-aware monorepo resolution remains a 0.x roadmap item.

## Security

The default is:

```bash
cxt --secrets redact
```

Policies:

- `redact`: replace likely secret material with `[REDACTED_SECRET]`.
- `fail`: exit with code 4 before producing context if a likely secret is detected. Useful in CI.
- `allow`: keep source unchanged. Use only when the output remains trusted/local.

Secret detection is defense-in-depth, not a formal security boundary. Keep `.env`, credentials, private fixtures and dumps out of your context selection whenever possible.

## Token budgets

```bash
cxt . --max-tokens 50000 --model gpt-5
# alias
cxt . --budget 50000 --model gpt-5
```

0.3 uses offline model-family estimation profiles and enforces the result as a hard context budget, including rendered document overhead. Lower-ranked files are removed first in query mode. Exact tokenizer backends are planned separately so the fast default can stay dependency-light.

## Output formats

```bash
cxt . --format markdown --output context.md
cxt . --format xml      --output context.xml
cxt . --format json     --output context.json
```

Path handling:

```bash
cxt . --path-style relative
cxt . --path-style absolute
cxt . --path-style none
```

Compression is available for file output:

```bash
cxt . --output context.md.gz  --compression gzip
cxt . --output context.md.zst --compression zstd
```

## Configuration

`cxt` merges a global config and a repository-local `.cxt.toml`. Project-local values override global scalar values; include/ignore lists are appended.

Global locations include `~/.config/cxt/config.toml` (or the platform equivalent) and the legacy `~/.cxtrc`.

```toml
# .cxt.toml
default_minify = true
model = "gpt-5"
max_tokens = 60000
max_output_mb = 100
secret_policy = "redact"
format = "markdown"
path_style = "relative"

include = ["src/**", "tests/**", "*.toml"]
global_ignore = ["fixtures/**", "generated/**"]
```

## Shell completions

```bash
cxt --completions bash > cxt.bash
cxt --completions zsh > _cxt
cxt --completions fish > cxt.fish
cxt --completions powershell > cxt.ps1
```

## Library

The package publishes a library named `cxt_core` in addition to the `cxt` executable. The current API is still pre-1.0 and may evolve.

```rust
use cxt_core::token::TokenCounter;

let counter = TokenCounter::for_model("gpt-5");
println!("{}", counter.count("fn main() {}"));
```

## Language coverage

Tree-sitter parsing/minification is wired for JavaScript/JSX, TypeScript/TSX, Python, Rust, Go, C, C++, Java, C#, Ruby and PHP. Dependency extraction currently handles the import/include forms implemented by the tracer (strongest for JS/TS, Python, C/C++, Java/C#, Ruby and PHP); ecosystem-aware Rust/Go/workspace resolution is explicitly tracked for 0.4.

## Exit codes

- `0`: success
- `2`: invalid selection/configuration/general engine failure
- `3`: output delivery failure
- `4`: blocked by `--secrets fail`

## Development

```bash
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

See `ROADMAP.md` for the remaining ecosystem-resolver, graph, symbol-slicing, exact-tokenizer, MCP and caching work toward the larger 1.0 vision.
