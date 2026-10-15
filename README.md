<div align="center">
  <img src="assets/cxt-icon.svg" alt="cxt: focused source context" width="104" height="104">
  <h1>cxt</h1>
  <p><strong>The code your task needs. The context you choose to share.</strong></p>
  <p>A local-first source-context engine for developers and coding agents.</p>
  <p>
    <a href="https://github.com/chinmaykrishnroy/cxt/actions/workflows/ci.yml"><img src="https://github.com/chinmaykrishnroy/cxt/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI status"></a>
    <a href="https://github.com/chinmaykrishnroy/cxt/actions/workflows/release.yml"><img src="https://github.com/chinmaykrishnroy/cxt/actions/workflows/release.yml/badge.svg" alt="Release workflow status"></a>
    <a href="https://github.com/chinmaykrishnroy/cxt/releases/latest"><img src="https://img.shields.io/github/v/release/chinmaykrishnroy/cxt?color=6c83ff" alt="Latest published release"></a>
    <a href="LICENSE"><img src="https://img.shields.io/github/license/chinmaykrishnroy/cxt?color=4fbfa8" alt="MIT license"></a>
  </p>
  <p><a href="#install">Install</a> · <a href="#quick-start">Quick start</a> · <a href="docs/CLI_REFERENCE.md">User guide</a> · <a href="docs/AGENT_SKILL.md">Agent skills</a> · <a href="docs/MCP.md">MCP</a> · <a href="CHANGELOG.md">Changelog</a></p>
</div>

---

**cxt is short for context.** Select source for a coding task, follow supported imports, review Git changes, fit a token budget, and inspect exactly what will leave your machine. Export Markdown, XML or structured JSON—or review the same plan in a terminal UI.

```bash
cxt --query "request retry policy" --context focused --model cl100k --budget 8000 --manifest --output context.md
```

0.5.1 strengthens query retrieval with bounded caller/reference expansion, implementation-first budgeting, and annotated Rust-field selection. Symbol-level packets, exact named-tokenizer budgets, read-only MCP and portable agent skills remain shared across the CLI/TUI engine. **Task-aware retrieval remains experimental:** local lexical/structural ranking is not semantic search, and a successful export is not proof that every required implementation was found. See [measured results and limitations](docs/IMPLEMENTATION_STATUS.md).

Want to test the query-resolution preview? Read the explicit opt-in instructions
and known limitations for [0.5.2-dev.1](docs/releases/v0.5.2-dev.1.md).
The stable installation commands below remain on the latest stable release.

## Why cxt?

Coding agents can already search repositories. cxt gives you an explicit, inspectable **handoff boundary**: choose source, enforce limits, review omissions, and reuse the same packet across tools, issues and teammates.

| Need | What cxt offers |
| --- | --- |
| Less irrelevant source | Task ranking, named symbols, import tracing and focused AST spans. |
| Predictable packet size | Exact final-packet accounting under offline cl100k/o200k encodings; explicit omissions. |
| Review before upload | Git patches, source previews, pinned essentials, provenance and freshness checks. |
| Local selection | No model/API calls in the core; configurable filters and best-effort secret protection. |
| Any workflow | CLI/CI, seven-view TUI, stdio MCP and instruction-only agent skills. |

Focused selection can save tokens by omitting unrelated code. Savings depend on the task; tiny packets that miss the implementation are not a success. Named encoding counts exclude later chat wrappers and are not billing guarantees.

## Install

### Windows

```powershell
irm https://github.com/chinmaykrishnroy/cxt/releases/latest/download/install.ps1 | iex
```

### macOS / Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/chinmaykrishnroy/cxt/releases/latest/download/cxt-installer.sh | sh
```

The installers download from the latest published GitHub release, verify its `SHA256SUMS.txt`, and install for the current user. Inspect [install.ps1](install.ps1) / [install.sh](install.sh) before executing remote scripts. Open a new terminal if PATH was updated, then run:

```bash
cxt --version
cxt -h
```

**[Browse release downloads](https://github.com/chinmaykrishnroy/cxt/releases/latest)** for Windows `.exe` (x64/ARM64), macOS `.dmg`/tarballs (Intel/Apple silicon), and Linux `.deb`/tarballs (x64/ARM64). Assets appear only after the tagged release workflow succeeds. A raw Windows `.exe` is portable: merely running it does **not** install it or update PATH. A macOS DMG contains a CLI binary, not a GUI installer. Debian packages install system-wide and normally require administrator access; the scripts offer per-user installation.

<details>
<summary>Build from source</summary>

```bash
git clone https://github.com/chinmaykrishnroy/cxt.git
cd cxt
cargo install --locked --path .
```

The package is named `cxt-ai`; the executable is **`cxt`**, not `cxt-ai`. This source installation uses Cargo's executable directory. Package-manager publication is separate from GitHub releases; see the [maintainer packaging guide](packaging/README.md).

</details>

## Quick start

```bash
# Pack the current project to your clipboard (same as cxt .)
cxt

# Review the selection without copying/writing a packet
cxt --query "request retry policy" --context focused --budget 8000 --dry-run

# Preserve a named implementation and supported local imports
cxt src/main.rs --trace --symbol main --context focused --stdout

# Changed code, reverse dependencies and likely related tests
cxt --git working --impact --context adaptive --model cl100k --budget 16000 --tui

# Multiple inputs, filters and machine-readable provenance
cxt src tests --ignore "**/*.generated.*" --format json --manifest --output context.json

# Keep diagnostics off stdout; fail if a likely secret would be exported
cxt src --secrets fail --ci > context.md
```

No destination means clipboard. Use `--stdout`, `--output FILE` or `--ci` for scripts; explicit output files may be overwritten. Clipboard failure is reported and creates a **new**, non-overwriting fallback file. `cxt --help` lists every option; the [CLI reference](docs/CLI_REFERENCE.md) explains configuration, defaults, exit codes, formats, compression and completion scripts.

### Choose your representation

| Mode | Purpose |
| --- | --- |
| `--context complete` | Whole selected files; the backward-compatible default. |
| `--context map` | Definitions/signatures for orientation—not implementation coverage. |
| `--context focused` | Original AST implementation spans with supporting imports. |
| `--context adaptive` | Focused source with map fallback when implementation does not fit. |

`--model cl100k` / `o200k` count the rendered packet exactly under those encodings. `heuristic`, `code` and `gpt-5` estimate. Inspect per-file representations and omissions; a map is not a substitute for code needed to fix a bug.

## Review in the terminal

```bash
cxt --tui
```

| View | Review |
| --- | --- |
| **1 Source** | File tree, selection, pinned essentials and transformed source. |
| **2 Changes** | Git patches, including deleted files. |
| **3 Dependencies** | Supported static imports, reverse imports and likely related tests. |
| **4 Checks** | Secrets, unresolved inputs, safety failures and budget omissions. |
| **5 Output** | The actual final packet, including rendered overhead. |
| **6 Settings** | Selection, filters, representation, budgets, scan limits and delivery. |
| **7 Symbols** | Definitions and original source locations. |

Space selects; `P` pins; `e` edits the query; `y` selects a symbol; `b` edits the budget; `r` refreshes; `g` exports; `E` exports the agent skill; `?` opens help. Settings has editable values and choice controls. Mouse navigation and paste are supported. `S`/`L` saves/loads task options and source fingerprints, not source bodies.

CLI and TUI share snapshots, security, rendering and final budgeting. Stale selected source/HEAD, failed scans, secret fail-policy findings and omitted essential pins block context export. Only `--tui` / `-T` opens the TUI; `-t` remains tracing. [Full TUI guide](docs/CLI_REFERENCE.md#tui-workflow).

## Give your coding agent the skill

```bash
# In your project, choose the agent you use
cxt --export-skill --skill-agent codex
# Or: claude, antigravity, devin, kiro

# Portable folder, or an exact destination of your choice
cxt --export-skill
cxt --export-skill "guides/cxt/SKILL.md"
```

The bundled skill teaches the application's workflows, limits and safe operating practices. Export is local and does not scan source, overwrite a guide, modify agent configuration, or install cxt. Default destinations follow each agent's project conventions; hosted clients may require committing/importing the folder. In the TUI, choose the agent in Settings and press **E**. [Registration and personal/global paths](docs/AGENT_SKILL.md) · [Read the skill](skills/cxt/SKILL.md).

## Connect through MCP

cxt also serves **seven scoped, read-only stdio tools**: context, search, trace, symbol, Git context, related tests and repository map.

```json
{
  "mcpServers": {
    "cxt": {
      "command": "cxt",
      "args": ["--mcp", "/absolute/path/to/project"]
    }
  }
}
```

This is a server-entry example, not a universal client configuration file. Use an absolute executable path when your client does not inherit PATH. MCP fixes redaction, canonical workspace scope and request budgets; clients cannot request arbitrary commands, output files, policy weakening or persistent cache writes. Do not scope it to your home directory. [MCP tools and boundaries](docs/MCP.md).

## Safety and language support

- Directory scans honor `.gitignore`; include/ignore, extension and language filters narrow input. Common generated/vendor/build/binary/sensitive files are excluded by default.
- Home directories, their ancestors and filesystem roots require explicit `--allow-broad-scan`. Shared entry/input-byte/depth/cooperative time limits fail closed. `--no-limit` does **not** disable these guards.
- Secret handling is best effort: `redact` by default, `fail` for refusal, `allow` only deliberately. An optional separately installed Gitleaks backend adds checks; scanner failure blocks export. Review output before sharing.
- Minification preserves literals, token boundaries and recognized directives; unsupported/invalid syntax remains unchanged. JSON preserves exact transformed content. Redaction and minification intentionally change source.
- Tree-sitter grammars cover Rust, Python, JavaScript/JSX, TypeScript/TSX, Go, C, C++, Java, C#, Ruby and PHP; Arduino `.ino` uses static C++ parsing. Dynamic relationships, build-generated code and some ecosystem layouts remain unresolved.
- Git modes select **current-checkout source**, not historical file bodies or a line-only patch. Hash-validated caches persist parser/resolver metadata, not source bodies; metadata can still be sensitive.

[Engine contracts and privacy](docs/CONTEXT_ENGINE.md) · [All scan/configuration options](docs/CLI_REFERENCE.md).

## Evidence, not leaderboard claims

Synthetic tests demonstrate that focused selection can preserve labeled implementations within small budgets. Exploratory public cases expose missed implementations, safety refusals and timeouts. Labels are provisional; neither downstream AI coding success nor competitor superiority has been established. See the [recorded measurements](docs/IMPLEMENTATION_STATUS.md), [reproduction protocol](docs/EVALUATION.md) and [next milestones](ROADMAP.md).

CI tests Windows, Linux and macOS. The release workflow separately builds six target architectures; cross-compilation is not the same as native runtime testing. The live badges above report GitHub's actual workflow status.

## Develop and contribute

```bash
cargo fmt -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --locked --release
```

Bug reports are most useful with `cxt --version`, a small redistributable fixture, the exact command and expected versus actual output. Do not attach private source or credentials. Agent contributors should read [AGENTS.md](AGENTS.md). The reusable `cxt_core` library and CLI remain pre-1.0; API contracts may evolve.

MIT licensed. Created by **Chinmay Krishn Roy**. See [LICENSE](LICENSE).
