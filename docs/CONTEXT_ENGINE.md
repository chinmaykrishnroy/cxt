# Task-aware context engine

The development engine shares one selection, snapshot, security, representation, budgeting and rendering path across CLI, TUI and MCP. Its goal is reviewable task evidence at a bounded cost—not proof that every required dependency was found.

## Choose the right representation

```bash
cxt --query 'route client volume' --context focused --model cl100k --budget 8000 --manifest --output context.json --format json
cxt src/audio.rs --symbol route_volume --context focused --stdout
cxt --context map --model o200k --budget 4000 --stdout
cxt --git working --impact --context adaptive --model cl100k --budget 16000 --tui
```

Complete preserves full selected source. Map summarizes definitions and does not contain their bodies. Focused copies original AST units and imports, with bounded lexical-reference expansion. Adaptive can substitute maps when implementations do not fit; representation and reasons identify that omission. Maps must never be mistaken for implementation coverage. Unsupported/invalid syntax and unmatched task text use a complete fallback.

Focused selection is a deterministic greedy heuristic, not a globally optimal knapsack or compiler-resolved program slice. Imports, dynamic dispatch, macros, overloads and implicit framework behavior may be unresolved. Related tests are suggestions, not measured coverage. Pin essential files in the TUI; a pin that cannot fit blocks export rather than silently becoming a signature map. If adaptive preparation already produced a map, increase the budget and refresh, or choose complete context. Explicit map mode can pin maps because orientation was deliberately requested.

## Ranking and provenance

Hybrid combines length-normalized BM25, paths, symbol-name matches and approximate identifier-reference boosts. Static imports expand up to eight top query seeds. `--ranker lexical|symbols|legacy` and `--no-query-expansion` support controlled experiments. Selection reasons expose which signals contributed. Weights are experimental, not trained or claimed optimal; terminology mismatch remains a limitation.

JSON `cxt.context.v1` adds `representation`, `spans` and `source_sha256` to each file. Spans are half-open UTF-8 byte offsets and one-based original line locations. `source_sha256` fingerprints the original source; `sha256` under `--manifest` fingerprints exported transformed content. Complete representations have no sliced-span list. A map's locations identify definitions, **not copied implementation spans**. Redaction/minification can change exported offsets; source locations refer to the original snapshot.

JSON metadata records budget exclusions, skipped-input counts and up to 64 final-plan omissions (with a total omitted count). These describe candidates already selected; files rejected by initial ranking/filtering are not an exhaustive audit of the entire repository. Markdown/XML include omission details with `--manifest`. Omission metadata consumes budget too. Secret policy also scans task query metadata; source replacement totals do not include query replacements.

Named cl100k/o200k tokenizers operate offline on the complete rendered packet. Literal special-token-looking strings are ordinary text. Other profiles remain visibly estimated. Context/metadata serialization is rechecked after budget selection; a tiny budget can fail when even packet overhead cannot fit. Counts exclude chat wrappers, instructions added later and model billing rules.

## Index and privacy

The per-user symbol cache is keyed by canonical path, parser schema and cxt version, and validated against source SHA-256. Every source is reread and fingerprinted; edits and branch changes invalidate parsing, renames get separate keys, deleted files cannot be resurrected through enumeration. Invalid/oversized cache entries are reparsed. Failed cache writes do not prevent packing.

The dependency cache shares this directory under `dependencies/`, keyed by canonical source/scope, engine version and cache schema. Before edge reuse it verifies the source hash, resolver configuration hashes, successful and missing path probes, canonical targets and observed package-directory entries. Config changes can therefore affect unchanged importers in Git impact mode. Deletions, renames, branch/source changes, newly created targets and malformed cache files trigger reconstruction. CLI tracing, query expansion and the TUI workspace use this shared module.

Validation is bounded: 512 observations, 4 MiB cache entries, 16 MiB configuration validation reads and a cooperative two-second validation window per source. Beyond these bounds the resolver recomputes instead of trusting an incomplete observation set. Node workspace discovery remains uncached because its ignore-driven traversal has additional inputs not yet fully tracked. Failed cache writes preserve correctness through reconstruction. These are incremental static dependency edges, **not** compiler-resolved references, zero-read querying or filesystem watching. Source enumeration, reads, hashes and graph assembly still run. The Checks view reports reused/rebuilt/uncacheable source nodes.

No source bodies are persisted. Symbol names, signatures, identifiers, paths and locations can still reveal proprietary information. Built-in redaction sanitizes symbol metadata but is best effort; dependency paths/config fingerprints are metadata too. Use `--no-cache` for sensitive work or `--cache-dir` in a controlled location. Entries from deleted/renamed files or old engine versions are not automatically pruned; remove only the chosen cache directory when no scan is running. A user-owned cache is not an authenticated database or protection against an attacker controlling that account.

`--workspace-root` bounds source and resolver configuration reads. MCP enforces it and disables caches. Canonical checks reject outside symlink targets, but cannot provide atomic filesystem isolation against hostile concurrent path changes. Use process/OS isolation for untrusted workspaces. Secret handling covers source, represented content and TUI patch previews; optional Gitleaks adds provider patterns and fails closed on operational errors. Nothing here guarantees all credentials are detected.

## Resolver coverage

Supported static relationships include local Rust modules/library paths and Cargo path/workspace dependencies (including target-specific tables), Python relative imports, TypeScript/JavaScript JSONC config inheritance, alias fallback targets, selected Node workspace package exports, local Go modules and local-path `replace` directives in `go.mod`/`go.work`, Java standard source roots, and local C/C++ includes (including Arduino `.ino`). JSON config reads are bounded and extends chains are depth/cycle guarded.

This is not the TypeScript/Node/Python/Cargo/Go compiler resolver. Cargo target tables are conservatively combined, not evaluated against actual target/features. Go replacement parsing covers common local paths, not version-specific module selection or the complete Go grammar; no module downloads occur. Conditional exports may have multiple interpretations; runtime-generated imports, full TypeScript project references and framework-specific test discovery remain gaps. Broad scans, recursion and input volume remain bounded; inspect omissions and test suggestions rather than assuming compiler-level completeness. See the upstream [Cargo target dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies) and [Go replacement](https://go.dev/ref/mod#go-mod-file-replace) contracts for the broader semantics.

Additional resolver safety ceilings block CLI/TUI export rather than returning truncated dependency lists: Node workspace traversal 20,000 entries/depth 8, Go packages 1,000 entries, Go workspace configuration 1,024 lines/128 roots, and Cargo dependency tables 128 entries. These conservative implementation bounds are separate from configurable source-scan limits. Convenience library methods returning only vectors cannot communicate errors; use the engine API for guarded exports.
