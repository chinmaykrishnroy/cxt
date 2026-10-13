# cxt 0.4.1 user guide and CLI reference

cxt (short for **context**) builds reviewable code context locally. It selects files, resolves supported imports, ranks task keywords, applies safety/secret rules and renders Markdown, XML or JSON. It does not call an LLM or upload source. Clipboard contents and exported files can contain sensitive data; inspect them before sharing.

## Start here

```bash
cxt                         # current directory -> clipboard
cxt --dry-run               # inspect final packed files/reasons; no delivery
cxt --tui                   # interactive review; short flag -T
cxt src/main.rs --trace      # entry file plus resolved imports; short flag -t
cxt --output context.md      # write context; the named file may be overwritten
cxt --stdout                # context only on stdout; status on stderr
cxt --help                  # expanded option descriptions
cxt --version               # verify which executable is running
```

`cxt` and `cxt .` select the same directory. Paths are resolved relative to the launch directory. Supply multiple files/directories; overlapping selections are deduplicated. Flags do not auto-open the TUI. On Windows, `cxt.exe` is the executable; `cxt-ai` is the Rust package name, not a shell command.

For an existing project in PowerShell:

```powershell
cd C:\Users\morph\PoKo
cxt --dry-run
cxt --tui
cxt --query 'power shutdown audio activity' --budget 20000 --tui
cxt PowerManager.cpp --trace --output context.md
```

Arduino `.ino` files can be packed/searched as text, but currently lack AST tracing/comment stripping. A C/C++ header does not automatically pull in its paired implementation file. Include necessary `.cpp` files explicitly.

## All parameters

`[PATH]...` is optional. Defaults below apply unless overridden by launch-directory/global configuration. Quote globs, task text and Git ranges when needed by your shell.

### Input and selection

| Parameter | What it does |
| --- | --- |
| `[PATH]...` | Files/directories to scan; defaults to `.`. Multiple inputs are supported. |
| `--stdin-paths` | Read one additional path per stdin line; relative paths use the launch directory. Use CLI mode, not interactive TUI input. |
| `-d`, `--diff` | Alias for `--git working`: staged + unstaged + untracked files; ignored and deleted files are not packed. Conflicts with `--git`. |
| `--git MODE_OR_RANGE` | Select checkout files using `working`, `staged`, `unstaged`, `untracked`, `tracked`, or a comparison such as `HEAD~3..HEAD` / `main...HEAD`. Requires a Git repository. Selection is repository-wide, not scoped by additional path arguments; narrow it with filters. |
| `--impact` | Requires `--git` or `--diff`. Add resolved reverse dependencies up to two hops and filename-related tests. Suggestions are heuristic, not complete coverage or runtime call graphs. |
| `-t`, `--trace` | Follow supported local static imports from explicit file inputs. Directory inputs are scanned normally; supply entry files to trace. Unsupported/external/dynamic relationships may be missed. |
| `--query TEXT` | Local lexical path/content ranking and dependency expansion from strong matches. Not semantic search. Defaults to at most 40 candidates; `--max-files` overrides that cap. |

Git mode selects names from the requested revision/staging comparison, but exported source comes from the **current checkout**, not historical or staged blobs. Deleted paths appear as patches in TUI Changes rather than source files in the packet. TUI Changes uses its chosen Git comparison, or `working` when no mode is set.

When combined, Git selects the initial candidates and query ranks that set. Query already expands imports from its strongest matches; if both `--query` and `--trace` are set, query takes precedence over the separate entry-trace mode. Impact expansion follows ranking/tracing, and candidate/token caps apply afterward.

```bash
cxt --git staged --dry-run
cxt --git HEAD~3..HEAD --impact --tui
cxt --diff --include 'src/**' --output changed-context.md
cxt src/app.ts src/admin.ts --trace --tui
```

Path lists:

```bash
printf '%s\n' src/main.rs Cargo.toml | cxt --stdin-paths --stdout
```

```powershell
@('src/main.rs', 'Cargo.toml') | cxt --stdin-paths --output context.md
```

### Filters

| Parameter | What it does |
| --- | --- |
| `--include GLOB` | Keep matching paths; repeat for alternatives. Match against selection-root-relative paths or filenames. Configuration and CLI include lists are combined. |
| `--ignore GLOB` | Exclude matching paths; repeatable. Configuration and CLI ignore lists are combined; exclusions take precedence. |
| `--ext EXT` | Comma-separated extensions, e.g. `rs,toml,md`; leading dots are accepted. |
| `--lang LANG` | Comma-separated language families, e.g. `rust,python,typescript`. Their extensions are combined with `--ext` as alternatives, not an intersection. |
| `--exclude-hidden` | Exclude dotfiles/hidden entries during directory selection; off by default. Graph inspection can still inspect hidden source. Use `--ignore` to exclude a path consistently from packing. |

Directory walks respect applicable Git ignore rules. Sensitive credential files, common generated/build/vendor/lock files, binary content and files over 1 MiB are excluded by default. Explicit files still pass packing filters. Normal file exclusions and read failures are skipped; scan-limit failures instead stop export. Selection filters do not prevent the graph from inspecting otherwise eligible repository source.

### Transformation and budgets

| Parameter | What it does |
| --- | --- |
| `-m`, `--minify` | Strip recognized AST comments and blank lines. This changes source line positions and may remove useful documentation; inspect the result. |
| `--no-minify` | Disable minification, including a configured default. Conflicts with `--minify`. |
| `--model MODEL` | Offline token estimate profile: `heuristic` (default), `code`, `gpt-5`, `o200k`, `cl100k`. Recognized aliases exist; unknown names fall back to the heuristic ratio. No model is called. |
| `--max-tokens TOKENS`, `--budget TOKENS` | Enforce an **estimated** final-document token budget, including rendered metadata. Whole files are dropped until it fits. No default budget; `0` overrides configuration with unlimited estimated tokens. Not an exact tokenizer or billing guarantee. |
| `--max-files N` | Cap candidates after selection/ranking/filtering, before final token budgeting. Query default is 40; otherwise no cap. `0` disables the cap. This does not reduce enumeration work; scan caps are separate. |

### Output

| Parameter | What it does |
| --- | --- |
| `--format FORMAT` | `markdown` (default), `xml`, `json`. |
| `--path-style STYLE` | `relative` (default), `absolute`, `none` in output headers. This changes presentation, not file selection. |
| `-o`, `--output FILE` | Write to a file relative to launch directory unless absolute; existing named output can be overwritten. Conflicts with `--stdout` and `--clipboard`. The output file is excluded from normal initial selection. |
| `--stdout` | Write only context to stdout; diagnostic status goes to stderr. Conflicts with `--output` and `--clipboard`. |
| `--clipboard` | Explicit clipboard destination; also the default when no destination/CI mode is selected. Conflicts with `--output`, `--stdout`, `--ci`. |
| `--compression COMPRESSION` | `none` (default), `gzip`, `zstd`. Compression requires `--output`; filename extension alone does not enable it. |
| `--manifest` | Include provenance metadata: reasons, hashes, Git commit and related packet metadata. This is audit information, not a proof of completeness. |

If clipboard access fails, cxt attempts a **new** `cxt-output.md`, `.xml` or `.json` in the launch directory. It reports this actual destination and refuses to overwrite an existing fallback. If neither destination works, exit code is `3`; use an explicit output filename or stdout. Clipboard availability depends on your desktop/session.

```bash
cxt --format json --manifest --output context.json.zst --compression zstd
cxt --stdout > context.md
cxt --secrets fail --ci > context.md
```

Shell redirection opens its destination before cxt executes: the redirected file can exist even when cxt fails. Prefer `--output` when a failed scan must not create its destination. Always check exit status in scripts.

### Safety

| Parameter | What it does |
| --- | --- |
| `--secrets POLICY` | `redact` (default), `fail`, `allow`. Best-effort pattern detection: redact masks matches; fail blocks export when findings exist but sanitizes TUI previews; allow explicitly preserves raw matches. Not all secrets are detectable. |
| `--no-limit` | Bypass per-file size and generated/junk/sensitive-file exclusion rules. Custom include/ignore rules and binary detection remain. Does not bypass scan caps or the broad-folder guard. Use cautiously. |
| `--allow-broad-scan` | Explicit permission for home-directory, home-ancestor and filesystem-root scans. Off by default; these paths are refused before traversal. Scan caps still apply. Saved tasks can retain permission; review their options before loading them. |
| `--scan-max-entries N` | Maximum visited entries per traversal phase / unique processed files per processing phase. Default 20,000; `0` disables it. |
| `--scan-max-mb MB` | Aggregate eligible input limit per phase in MiB (1,048,576 bytes). Default 100; `0` disables it. |
| `--scan-max-depth N` | Directory traversal depth relative to an input root. Default 32; `0` disables it. This is not import-graph hop depth. |
| `--scan-timeout SECONDS` | Cooperative elapsed-time budget per phase. Default 15; `0` disables it. It cannot interrupt an in-flight filesystem call, parser or Git subprocess. |

Limit failures block export, rather than quietly truncating a scan. Multiple roots/phases have separate traversal/time budgets. Input size is not a peak-memory guarantee; Git patch bytes are checked after capture. Ignore/pruning rules affect visited entries; custom filters are applied after enumeration, so accounting can include later-excluded files. Graph phases always exclude common junk paths, unsupported source extensions and source files over 1 MiB, even with `--no-limit`.

```bash
cxt /path/to/project --scan-max-entries 50000 --scan-max-mb 200 --scan-timeout 60 --dry-run
```

Prefer a project path over broad-folder permission. `--dry-run` still performs selection, transformation and budgeting; it is not a free/metadata-only scan.

### Review and scripting

| Parameter | What it does |
| --- | --- |
| `--dry-run` | Report final packed files, reasons, estimated tokens and exclusions without copying/writing context. In TUI, preview/review still runs; exporting returns the dry-run report. |
| `--ci` | No clipboard fallback; use stdout when no output filename is supplied. Progress animation is suppressed. Conflicts with `--clipboard`. It does not imply `--secrets fail`. |
| `-T`, `--tui` | Interactive review with Source, Changes, Dependencies, Checks, Output and Settings. Requires terminal stdin/stdout. Quit without export with `q`; export with `g`. |
| `--completions SHELL` | Generate completions for `bash`, `elvish`, `fish`, `powershell`, `zsh` and exit without scanning. Load the emitted script using your shell's usual completion mechanism. |
| `-h`, `--help` | Show all parameters, examples and safety summary; long form expands descriptions. No scanning. |
| `-V`, `--version` | Show this executable's version and exit without scanning. |

## TUI workflow

1. Start with an entry file, Git comparison or query, optionally with a budget.
2. In **Source**, inspect transformed source; Space toggles files/subtrees. `P` pins essentials; `U` clears pins. Pinning prioritizes files but does not exceed the budget.
3. Inspect **Changes** and **Dependencies** for relevant patches/imports/dependents/tests. These are static/heuristic relationships, not proof that every dependency is included.
4. Inspect **Checks** for missing resolved imports, suggested tests, secret findings, skipped inputs and budget exclusions. Export is blocked for secret fail-policy findings, unavailable/omitted pinned files or a failed plan/scan.
5. Review **Output**: this is the transformed packet, including final metadata and estimated budget overhead.
6. Use **Settings** (`6`, `o` or F2) to edit inputs, query, Git mode, budgets, all scan limits, filters, transformations and delivery. Enter/click edits; Ctrl+u clears; arrows choose enum/boolean values.
7. Press `g` to export. Changed selected sources or Git HEAD require `r` to refresh first. Failed scans cannot reuse a previous workspace to export.

Additional controls: Tab / `1`–`6` changes views; `/` filters paths; `a` / `n` selects all/none matching the path filter; `e` edits query; `b` budget; `c` Git mode (`all` disables Git selection). `w` focuses preview, `v` wraps it, J/K scroll, `F` locates `file:line[:column]`. Disable minification for original-source line navigation. Mouse navigation and paste are supported. Minimum terminal is 52 × 15.

`S` / `L` saves/loads task options, selected/pinned paths and source fingerprints—not source contents. Save under `.cxt-task*.json`; an existing task is not overwritten. Restored files can be changed/unavailable; inspect Checks. Saving a task is not an encrypted storage mechanism and can retain query text, paths and security/broad-scan options.

## Configuration and precedence

Load the first existing global config (platform config directory `cxt/config.toml`, then `~/.config/cxt/config.toml`, then legacy `~/.cxtrc`), then `.cxt.toml` **in the launch directory**. cxt does not walk upward or automatically load the target project's config when launched elsewhere. Project scalar values override global values; CLI scalar flags override both. Include/ignore lists append; CLI globs do not clear configured globs. Malformed configuration logs a warning and falls back to other/default values; it is not a strict config validator.

Supported configuration keys:

| Key | Default / purpose |
| --- | --- |
| `default_minify` | `false`; overridden by `--minify` / `--no-minify`. |
| `model` | `"heuristic"`; offline estimate profile. |
| `max_tokens` | No cap; `0` disables it. |
| `max_output_mb` | 100 MiB aggregate transformed-file-content cap; `--no-limit` bypasses it. Separate from input scan limits and rendered metadata overhead. |
| `secret_policy` | `"redact"`; `"fail"`, `"allow"` are alternatives. |
| `format` | `"markdown"`; `"xml"`, `"json"` are alternatives. |
| `path_style` | `"relative"`; `"absolute"`, `"none"` are alternatives. |
| `include` | Array of include globs; empty means all eligible paths. |
| `global_ignore` | Array of exclude globs. |
| `scan_max_entries` | 20000; `0` disables this cap. |
| `scan_max_mb` | 100; `0` disables this cap. |
| `scan_max_depth` | 32; `0` disables this cap. |
| `scan_timeout` | 15 seconds; `0` disables this cap. |

Broad-scan permission is deliberately not a configuration key. Start from [the configuration example](../.cxt.toml.example), remove filters you do not want, and inspect selection with `--dry-run`.

## Installation and troubleshooting

Use [README installation instructions](../README.md#install). Downloading a standalone `.exe` or opening a `.dmg` does not install a CLI command or add it to PATH. The release scripts install into a user-writable location and configure PATH; reopen the shell if needed. Debian packages install system-wide through the package manager and normally require elevated privileges. Package-manager publication status is separate from GitHub release assets; see [packaging instructions](../packaging/README.md).

```powershell
Get-Command cxt -All       # find shadowing/older copies on Windows
cxt --version
.\target\release\cxt.exe --version  # source build, from repo root
```

```bash
command -v cxt
cxt --version
```

| Exit code | Meaning / next action |
| --- | --- |
| 0 | Success, help/completions/version, or TUI quit without export. |
| 2 | CLI parsing, invalid selection/Git mode, no query matches, scan guard/limit, terminal requirement or general engine/TUI failure. Read stderr; narrow inputs or correct options. |
| 3 | Delivery failed. Close apps holding clipboard, choose a new `--output` filename, or use `--stdout`. |
| 4 | Secret fail-policy blocked context. Remove findings or explicitly review policy; do not blindly switch to allow. |

Release limitations and next priorities are in [product direction](PRODUCT_DIRECTION.md). Local Windows gates do not establish success on every other platform; GitHub's tagged build must finish before all release assets/download links become available.
