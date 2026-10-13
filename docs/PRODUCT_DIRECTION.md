# cxt 0.4: product direction

Research date: 2026-10-13. Competitor capabilities below are documented, not independently benchmarked. The cxt baseline was checked against current source; this document is not a runtime or release validation.

## Current position

cxt 0.4 provides deterministic, local lexical keyword ranking over paths and contents, followed by import expansion. It budgets whole transformed files, including rendered overhead, using approximate character-based token profiles—not exact model tokenization. Its safe-review TUI uses shared prepared/export snapshots, checks source freshness, supports essential-file pins, exposes dependency/test suggestions, and saves task options and fingerprints without source contents. These checks are heuristic, not proof of completeness or secret-free output. Evidence: [query](../src/query.rs), [engine](../src/engine.rs), [token profiles](../src/token.rs), [review/tasks](../src/review.rs), [TUI](../src/tui.rs).

This is a useful reviewable handoff workflow. There is no evidence yet that cxt is the best context selector or fastest packer; Rust, feature counts, and a polished TUI do not establish either claim.

## Competitive baseline

- Repomix offers experimental Tree-sitter compression that retains signatures/types while removing implementation details. It has configurable OpenAI-compatible token encodings via `gpt-tokenizer`, default-on Secretlint checks, and experimental MCP tools for local/remote packing plus partial reading/searching of packed outputs. Compression can omit the implementation needed for a fix; detection is not a security guarantee. [Compression](https://repomix.com/guide/code-compress), [token configuration](https://repomix.com/guide/configuration), [security](https://repomix.com/guide/security), [MCP](https://repomix.com/guide/mcp-server).
- Aider's repository map includes important symbols and signatures, using dependency-graph ranking to choose relevant portions within a chat-sensitive token budget. Unlike cxt's whole-file selection, this supplies structural orientation without entire implementations. [Repository map](https://aider.chat/docs/repomap.html).
- Gitingest documents local-directory/remote-repository text digests, structure/size/token statistics, CLI and Python interfaces, `.gitignore` filtering, and a convenient GitHub-to-Gitingest URL substitution. Convenience is already part of the comparison. [README](https://github.com/coderamp-labs/gitingest#readme).
- dotcommander/repomap documents local static analysis, import graphs, BM25 intent ranking, token-budgeted symbols/signatures, and symbol-centered bounded source/impact packets. Task-aware mapping and auditable handoffs are therefore not an uncontested niche. [README](https://github.com/dotcommander/repomap#readme).

## Priorities and decision gate

1. **Benchmark task retrieval first.** Label required files/symbols for 20–30 real fixes across representative repositories. Compare against these tools and simple search at equal exact-token budgets. Publish recall, irrelevant-context share, downstream task success, cold/warm latency, peak memory, versions, configurations, and failures.
2. **Add symbol-level context.** Combine a compact map with selected implementation spans, callers, tests, and configuration; preserve provenance and explicit omissions rather than blindly stripping bodies.
3. **Improve resolution.** Test framework-aware aliases, workspace/package exports, routing and configuration conventions. Add explicit Arduino `.ino` parsing/include/sketch fixtures; current [tracer extension dispatch](../src/tracer.rs) does not recognize `.ino`. Report unresolved relationships rather than implying semantic completeness.
4. **Make budgets exact where supported.** Use a named offline tokenizer on the final rendered packet; retain visibly approximate fallback profiles and avoid universal billing guarantees.
5. **Make scanning incremental.** The current patch adds broad-folder guards and configurable entry/byte/depth/cooperative-time caps; limit failures block export. Next, cache by content/config/parser identity and invalidate changed dependencies. Surface omissions explicitly without silently claiming completeness.
6. **Reduce adoption friction.** Provide convenient verified binaries/install paths and a short honest demo: task query, inspect exclusions, pin essentials, review checks, export, then demonstrate stale-source rejection.

Keep selection local-first. Continue investment only after repeatable retrieval or workflow gains; position cxt as a reviewable task-context tool, not an unmeasured category winner.
