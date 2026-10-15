# MCP-first development and findings

Use cxt to locate relevant code during work on this repository, then verify the
selected original source before editing it. This is a development practice, not
a claim that every operation is possible through MCP or a background monitor.
Contracts and client configuration examples are in [MCP.md](MCP.md).

## Procedure

1. Check the available connector and executable version. Prefer the registered
   cxt MCP tools. If unavailable, exercise the real stdio server from a trusted
   local binary; disclose the fallback. Never substitute CLI output while calling
   it an MCP result. Global client registration is a separate configuration step.
2. Choose explicit task-relevant source paths and a bounded token budget. Keep
   held-out evaluation material outside retrieval. Path allowlists reduce
   exposure but do not create an access-control boundary; dependency expansion
   can read additional eligible files in the authorized workspace.
3. Use `search`/`get_repo_map` for orientation, then `get_symbol`/focused
   `get_context` for implementations. Inspect representations, original spans,
   source hashes, omissions and warnings. Retry narrowly when evidence is
   missing; a successful response alone does not establish task completeness.
4. Read exact original spans or use narrowly scoped filesystem/search fallbacks
   when MCP cannot supply the required evidence. Record why. Preserve security,
   freshness and workspace bounds; keep evaluation labels out of broad scans.
5. Log reproducible misses, false links, refusals, redundant spans, latency and
   overhead below, with version/hash, request and disposition. Promote actionable
   items to [TODO.md](../TODO.md), then add a public CLI/core or MCP regression
   at the appropriate seam before implementing a fix.
6. Compare equivalent tasks and coverage before claiming token savings. Separate
   selected source tokens, the complete rendered packet and actual client-visible
   tool traffic. Include schema/reason/omission overhead, duplicated text versus
   structured payloads, retries and fallback reads in end-to-end costs. Exact
   packet accounting does not measure every client's protocol/model token bill.

MCP currently performs no persistent cache writes. Any repeated-query cache work
must preserve that boundary, using bounded validated session memory unless an
explicitly reviewed policy changes it. This procedure does not authorize new
services, scheduled runs, model calls or global client configuration changes.

## 2026-10-15: local stdio probe

Executable: `target/evaluation/cxt-0.5.2-dev-f3d1c499b486.exe`.
SHA-256: `f3d1c499b486d63a97408b626c72de2a5e1d58559eb8657803456859740ad74f`.
Transport: initialize using `2025-03-26`, initialized notification, then
`tools/call` over newline-delimited stdio. All server processes exited zero;
no cxt connector was exposed in the development session. Held-out task contents
were not opened. These source-only requests were not a held-out evaluation.

### M1 — Search coverage under metadata and expansion pressure

Request:

```json
{"name":"search","arguments":{"paths":["src/query.rs","src/query_graph.rs","src/index.rs"],"query":"persistent lexical postings ranking reference graph","max_tokens":2500}}
```

- Response: success, exact-encoding packet count 2,276; only `query.rs` retained,
  as a map, with 28 omissions. Source paths in the result are relative to its
  selection root, so `query.rs` refers to `src/query.rs` here.
- Repeating with `max_tokens: 8000` retained maps for `query.rs`, `cli.rs` and
  `git.rs`, counted 6,300 packet tokens, and omitted 26 files including the
  explicitly supplied `query_graph.rs` and `index.rs`.
- Explicit paths are candidate seeds, not pins or a hard file allowlist.
  Dependency expansion enlarged the candidate set to 29. This is an observed
  selection-quality/overhead gap, not a workspace escape or a token-limit breach.
- **TODO / release triage:** test essential-file coverage and map/reason/omission
  overhead; improve adaptive representation/coverage and compact diagnostics
  without hiding omissions. Establish a same-task complete-source baseline
  before claiming measured token savings.

### M2 — Rust method identity and focused follow-up

```json
{"name":"get_symbol","arguments":{"paths":["src/index.rs"],"symbol":"IndexStore::load","max_tokens":4000}}
```

This returned `isError: true`, `symbol not found: IndexStore::load`. The same
file's map listed `load` without its impl owner; requesting `symbol: load`
succeeded with a focused 1,578-token packet. The returned method is not wrapped
in its `impl` owner; imports from an unrelated test scope were also visible in a
focused `src/query.rs` request. Focused text is source context, not necessarily
a standalone compilable translation unit.

- **TODO / release triage:** preserve Rust impl ownership in symbol identities,
  disambiguate same-named methods, and scope supporting imports. Add regressions
  before changing the extractor/cache schema. These observations do not yet
  establish the full affected-language/syntax scope.

### M3 — Indexing work confirmed through MCP

`get_symbol` for `rank_with_store_checked` in `src/query.rs` succeeded with a
focused 2,423-token packet. It shows per-candidate source reads, word extraction
and query-frequency computation. `get_symbol` for `load` in `src/index.rs`
shows hash-validated parser metadata, not persisted lexical postings.
The graph builder's original source confirms per-query adjacency construction.
This supports prioritizing D4 in TODO.md; it does not demonstrate a measured
repeated-query speedup from a new index.

No implementation changes or comprehensive MCP performance experiment were made
by these probes. Multi-request process timings are not per-tool latency metrics.
