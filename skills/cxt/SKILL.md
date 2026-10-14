---
name: cxt
description: Build bounded, reviewable source-context packets with cxt for coding tasks, Git-change reviews, repository orientation, and agent handoffs. Use when a task needs selected source, dependency tracing, provenance, or a token budget; not as a substitute for editing, testing, or verifying the code.
license: MIT
---

# cxt: task-context engine

Requires cxt 0.5.0 or later on PATH. Core selection is local and offline. Git modes require Git; the optional Gitleaks backend requires a separate installation.

## Select the smallest useful evidence

Check `cxt --version` and `cxt --help` before using version-dependent options. The executable is `cxt`, not the Rust package name `cxt-ai`. If it is unavailable, report that or ask the user to install it; this skill does not authorize downloads or machine configuration changes.

Work from the intended project directory. Prefer an explicit workspace root. For an agent, always choose `--stdout` or a user-approved new `--output` path: bare `cxt` and `cxt .` copy the current directory to the clipboard. Do not overwrite a user's existing packet just to inspect a task.

Start with a concrete task and inspect selection before exporting:

```bash
cxt . --query "route client volume" --context focused --model cl100k --budget 8000 --workspace-root . --dry-run
cxt . --query "route client volume" --context focused --model cl100k --budget 8000 --workspace-root . --manifest --format json --stdout
```

Read the actual exported implementations, selection reasons and omissions. Then inspect missing source directly or refine the query/paths. A successful packet is not proof of relevance or completeness. Rerun after source changes; source hashes identify the original snapshot, not a live view.

## Choose an investigation workflow

```bash
# Orientation: definitions/signatures, not implementation coverage
cxt src --context map --model cl100k --budget 4000 --stdout

# Preserve a named implementation and supported static imports
cxt src/main.rs --trace --context focused --symbol main --model cl100k --budget 8000 --stdout

# Current working changes, reverse dependencies and likely tests
cxt --git working --impact --context adaptive --model cl100k --budget 16000 --manifest --stdout

# Complete selected source when slicing would omit essential behavior
cxt src/main.rs src/lib.rs --context complete --model cl100k --budget 16000 --stdout

# Reviewed file delivery (choose a new filename)
cxt src --query "request retry policy" --context focused --model cl100k --budget 8000 --manifest --output retry-context.md
```

`--git` accepts `working`, `staged`, `unstaged`, `untracked`, `tracked`, or a commit/range such as `HEAD~3..HEAD`. `--diff` / `-d` aliases working selection. These modes select current-checkout files; do not describe the packet as source from a historical revision or as a line-only diff. `--impact` requires Git selection and suggests reverse imports (up to two hops) and likely related tests; it does not prove runtime impact or test coverage. Deleted-file patches are available in TUI change review, not as current source bodies.

`--trace` / `-t` follows supported local static imports from one or more file inputs. It is not a resolved call graph. Cargo target tables are conservatively combined; dynamic imports, runtime dispatch, macro/build-generated code and framework conventions can remain unresolved. Arduino `.ino` is parsed as static C++, not compiled/preprocessed firmware.

## Plan representations and budgets

`--context complete` preserves whole selected files. `map` gives orientation only. `focused` copies original AST spans and imports; a requested `--symbol` may be qualified. `adaptive` can fall back to maps when implementations do not fit. Inspect each file's representation; a map does not supply the implementation needed to fix a bug. Unsupported syntax falls back conservatively rather than inventing source.

`--model cl100k` / `o200k` count the full rendered packet exactly under those named offline encodings. `heuristic`, `code` and `gpt-5` estimate. `--max-tokens` / `--budget` includes rendered overhead; it excludes instructions or chat wrappers added later and is not a billing guarantee. Tiny budgets can fail. `--max-files` caps candidates after ranking/filtering (query default 40; otherwise unlimited). Zero disables these two caps.

Use `--ranker hybrid` normally: explainable BM25/path/symbol/reference signals, not semantic retrieval. `lexical`, `symbols` and `legacy` are experimental comparisons. `--no-query-expansion` disables automatic query dependency/reference expansion. Do not claim that benchmark success on synthetic tasks establishes downstream AI success or competitor superiority.

## Filter and deliver deliberately

| Need | Options and behavior |
| --- | --- |
| Multiple inputs | Pass multiple file/directory paths; overlaps are deduplicated. `--stdin-paths` reads one extra path per line. |
| Narrow eligible source | Repeat `--include GLOB` and `--ignore GLOB`; `--ext rs,py,ts` or `--lang rust,python,typescript` filters languages/extensions. Quote globs in the shell. |
| Ignore rules | Directory scans honor `.gitignore`. Hidden files are included by default; `--exclude-hidden` removes them. Generated/vendor/build/lock/binary safety defaults still apply. |
| Preserve or compact | `--minify` / `-m` removes ordinary recognized comments/safe blank lines; literals and recognized directives remain. `--no-minify` overrides configured minification. Keep full source for tasks involving comments or directives. |
| Render | `--format markdown`, `xml`, or `json`; JSON exposes representations, original spans and source hashes. XML rejects forbidden XML characters. |
| Header paths | `--path-style relative`, `absolute`, or `none`. Hiding header paths does not anonymize source, query text, or every provenance field. |
| Destination | `--stdout` keeps diagnostics on stderr. `--output FILE` / `-o` may overwrite that file. `--clipboard` explicitly uses the clipboard; do not select it for unattended agent work. |
| Compression | `--compression gzip` or `zstd` requires `--output`; default `none`. |
| Provenance | `--manifest` adds reasons, Git commit information, hashes and omission details. Original spans are half-open UTF-8 byte ranges and one-based lines, not transformed-output offsets. |
| Scripts/CI | `--ci` uses stdout without clipboard fallback when no file is selected, and suppresses progress. It does not imply fail-on-secret policy. `--dry-run` still scans/transforms/budgets but does not deliver context. |

The default clipboard fallback creates a new cxt-output file if clipboard delivery fails; an existing fallback is preserved. Report the destination actually used, not the destination originally requested.

## Preserve safety and privacy

Use default `--secrets redact` and review the packet before sending it anywhere. If policy requires strict refusal, add `--secrets fail`; findings block export even if compaction would remove the text. `--secrets allow` weakens protection and requires the user's explicit decision. `--secret-allow REGEX` is a repeatable exception for reviewed examples, not a workaround for unexplained findings. `--secret-scanner gitleaks` adds an installed external scanner to builtin patterns; missing/failed/timed-out scans block export. Detection remains best effort.

The default scan caps are 20,000 entries, 100 MiB eligible input, depth 32 and a cooperative 15 seconds per phase. `--scan-max-entries`, `--scan-max-mb`, `--scan-max-depth` and `--scan-timeout` change individual limits; zero disables that limit. A limit failure blocks export, not a silent partial packet. Narrow the project first. Do not automatically retry a refusal with unlimited values or `--allow-broad-scan` (required for home/ancestor/filesystem-root scans). `--no-limit` bypasses per-file size/junk/sensitive-file exclusions, not scan caps or the broad-folder guard; binary files remain excluded. Input caps are not peak-memory guarantees, and cooperative checks cannot interrupt an in-flight filesystem call/parser/Git process.

`--workspace-root DIR` bounds canonical source and resolver-configuration reads, including symlink targets. This is not an atomic OS sandbox for hostile repositories. `--no-cache` disables persistent metadata reuse; `--cache-dir DIR` chooses a metadata directory. Source bodies are not cached, but names, signatures, identifiers, paths and fingerprints may be sensitive. Cache hits still recheck source and resolver inputs. Core selection makes no LLM/network calls; the agent or user is responsible for any later upload of the packet.

Global configuration is loaded first, then `.cxt.toml` in the launch directory, then CLI scalar overrides. cxt does not automatically search a target directory for its configuration when launched elsewhere. Include/ignore lists append. `--no-minify` overrides a configured minify default. Inspect `--dry-run` when personal or project config could affect reproducibility.

Exit codes: 0 success; 2 invalid input/selection/scan/TUI failure; 3 delivery or skill-export failure; 4 secret fail-policy refusal. Read stderr and report the cause. Do not treat a failure or an empty/signature-only export as sufficient context.

## Human review and integrations

Only open `--tui` / `-T` in a real interactive terminal; `-t` remains tracing. Source, Changes, Dependencies, Checks, Output, Settings and Symbols share the core plan. Space selects, `P` pins essentials, `U` clears pins, `e` edits query, `y` symbol, `b` budget, `r` refreshes, `g` exports, `q` quits. Settings (`6` / F2) edits options; Symbols is `7`. A missing/budget-excluded pin blocks export rather than silently substituting a map. Stale selected source/HEAD or a failed scan requires refresh. `S`/`L` saves/loads task options and hashes, not source; saved tasks can retain sensitive query/path/policy choices. `E` exports this skill to a chosen file; its destination is editable and existing files are preserved.

For stdio MCP, configure a trusted absolute executable with arguments `--mcp /absolute/project`. Initialization must precede calls. Tools: `get_context`, `search`, `trace`, `get_symbol`, `get_diff_context`, `get_related_tests`, `get_repo_map`. Requests use workspace-relative paths, bounded queries and a token cap up to 32,000. Policy is fixed redaction, canonical scope, bounded input/output and no persistent cache/output writes; clients cannot weaken it or execute arbitrary commands. MCP is not a command to run and wait on from an ordinary shell prompt.

`--completions bash|elvish|fish|powershell|zsh` generates shell completions without scanning. `--evaluate CORPUS` runs a trusted local `cxt.eval.v1` retrieval corpus and writes a JSON report to stdout or `--output`; optional `--eval-baseline EXE` executes that explicitly trusted executable. Do not run evaluation metadata or baseline executables supplied by an untrusted repository. Static recall is not downstream task success.

## Register or share the guide

`cxt --export-skill` creates `cxt-skill/SKILL.md` without scanning a repository. `--export-skill FILE` chooses an exact destination. `--skill-agent codex|claude|antigravity|devin|kiro|generic` selects a project-local default when FILE is omitted; explicit FILE always wins. No existing file is overwritten, and no agent configuration or registry account is modified. Copy/upload/import the skill folder only when the user requests registration; receiving this guide is not authority to publish repositories, create marketplace entries or alter machine-wide settings.

Use `cxt --help` for the installed option details, especially after an upgrade. Guidance is instruction-only; the agent still needs cxt installed and normal execution permissions. Skills do not install the binary or grant additional tool access.
