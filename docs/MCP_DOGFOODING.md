# MCP-first development and findings

Use cxt to locate relevant code during work on this repository, then verify the
selected original source before editing it. This is a development practice, not
a claim that every operation is possible through MCP or a background monitor.
Contracts and client configuration examples are in [MCP.md](MCP.md).

## 2026-10-16: external agent feedback reproduced locally

No cxt connector was exposed, so the frozen trusted `cxt-dev-mcp-tui.exe` was
used over actual stdio. Private repositories mentioned in the feedback were
not accessed. An owned tiny Python fixture reproduced:

- `get_repo_map`, one short function, `max_tokens: 100`: the original
  `metadata/output overhead exceeds the configured limit` error. A public MCP
  red/green regression now requires the requested limit, overhead explanation,
  narrow/increase recovery and no exported packet, then retries successfully at
  2000. Byte limits remain distinguished from token refusals.
- `get_related_tests` on `app/leaves.py`: no tests, despite a supported project
  root and tests importing that module. The filter discarded reverse-import
  candidates before returning results. Public regression covers both matching
  and different filenames, plus an unrelated test excluded from the packet.
  The first fixture lacked a project marker; a second red run supplied
  `pyproject.toml` to distinguish root inference from the filter bug.
- Generic tool descriptions did not explain packet budgets or map-only search.
  A tools/list red/green regression now covers upfront overhead/default guidance
  and source follow-up direction.

Local logs: `target/evaluation/feedback-{budget,related,discovery}-{red,green}.txt`
and `feedback-related-project-red.txt`. Final candidate validation follows these
changes; earlier pre-feedback test/build attempts are not final-source proof.
The public evaluation check additionally caught new capitalized budget wording
being categorized as `cli_error`. The first focused-context test did not reach
metadata overhead; the complete-context counterpart reproduced it. The wording
now preserves the established `token_budget` category, with a public CLI
regression. Preserve `feedback-category-complete-red.txt` and
`feedback-category-green.txt`; do not count the initial ineffective probe as red.

The externally reported whole-repo stall remains unverified: exact version,
command/request and full stderr were requested. An idle stdio MCP process and
PowerShell rendering native status on stderr are not by themselves a hung scan.
No scan timeout, security policy, scope bound or response cap was weakened.

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

## 2026-10-15: DEV regression fixes and replay

The published prerelease is unchanged. The working DEV executable is frozen at
`target/evaluation/cxt-dev-mcp-tui.exe`, SHA-256
`736fc814ffb58523afab1c95e53f6bf8f5b060e5d59b465645d5637d22284cdf`.
It still reports `0.5.2-dev.1`; the hash distinguishes it from that release.
No cxt connector was exposed, so the replay used actual newline-delimited stdio
MCP, not CLI output relabeled as MCP. Source-only requests and replies are
summarized in the local `target/evaluation/dev-mcp-replay.jsonl`; source bodies
are not copied into that log. The local replay script is
`target/evaluation/dev_mcp_probe.py`.

Both executables read the same current source checkout. The baseline is the
checksum-verified published Windows binary, SHA-256
`ba1306a1d9e0453556c2c28c6560a1b7a19e295c131b26596b91cbfcf1a7a6de`.
Numbers below are server-reported cl100k packet counts, not client-visible
protocol traffic or model billing.

| Request | Published prerelease | DEV | Disposition |
| --- | --- | --- | --- |
| M1 search, 2,500 | One map, 2,269 tokens; 28 omissions | Unchanged | Open coverage/metadata limitation. |
| M1 search, 8,000 | `query.rs`, `cli.rs`, `git.rs` maps; 6,293 tokens | `query.rs`, `query_graph.rs`, `index.rs`, `bindings.rs` maps; 5,267 tokens; 25 omissions | All three input seeds retained, but still no implementation coverage. |
| M2 `IndexStore::load` | Symbol-not-found error | Focused implementation, 1,653 tokens | Fixed for tested Rust impl syntax; not full type/trait resolution. |
| M2 `rank` | 654 tokens with unrelated nested test imports | 559 tokens without those imports | Fixed scoped-import regression; not general import-usage minimization. |

Public MCP regressions went red before each fix, including a negative case
preventing bare Rust calls from binding unrelated impl members, and a three-file
Python case where import-only support displaced a genuine query match. Candidate-
wide fail-policy, fixed MCP redaction, workspace/scan bounds and no persistent
MCP cache writes are unchanged. The symbol metadata schema is invalidated.

M1 remains partial and M3 remains open: no persistent lexical postings or bounded
MCP session index was implemented in this slice. Process durations for four
requests are retained but are not a latency distribution or evidence of a speedup.
See [DEV verification](DEV_MCP_TUI_VERIFICATION.md) for local test gates and the
separate, still-open historical TUI deadline issue. No held-out task contents,
external model calls, downstream coding experiment or superiority claim is involved.

## 2026-10-16: stable-candidate scope review

No cxt connector was exposed; actual stdio MCP requests used the frozen DEV
executable identified above. `get_symbol` located `Workspace::load` (1,526 packet
tokens), `ScanBudget::new` (678), and the TUI test `ready` helper (1,668), with no
omissions. Original source was checked before editing. A request for
`TokenCounter::new` failed because that function does not exist; the narrow source
fallback found `TokenCounter::for_model`. This is a request correction, not a
product regression. The TUI helper's pre-poll timeout ordering went red in a
zero-wait queued-result test; polling first fixes that case without increasing
the deadline. The historical timeout cause remains unproven.

### M4 — Declared JS/TS class namespaces

A three-symbol local fixture reproduced incorrect helper selection:

```javascript
function helper() { return "global helper"; }
class Screen {
    helper() { return "member helper"; }
    changeVolume() { return helper(); }
}
```

`get_symbol` with `paths: ["screen.js"]`, `symbol: "Screen::changeVolume"`,
`max_tokens: 2000` returned the member helper instead of the global function.
The corresponding public MCP regression went red before the fix and green across
`js`, `jsx`, `mjs`, `cjs`, `ts`, `tsx`, `mts`, and `cts`. The correction marks
declared JS/TS class scopes non-lexical in shared parser metadata, used by both
graph and focused-helper binding. Schema `cxt.symbols.v17-js-class-namespaces-tree-sitter-0.20`
invalidates earlier metadata. This is not compiler/type or class-expression
resolution; no new resolver reads or persistent MCP writes are introduced.

A proposed Java method-call guard failed. A stdio probe of the unchanged frozen
baseline confirmed it already omits the bare-called helper; it was not caused
by the JS-specific fix. Keep that missing coverage in the backlog. A separate
baseline-validated Java implicit-field case guards against applying JS namespace
rules to other languages. The original failure remains in the local
`target/evaluation/stable052-mcp-final.txt`, rather than being overwritten by the
corrected guard in `stable052-mcp-corrected.txt`.

A focused `extract` request for `src/symbols.rs` at 2,400 tokens returned
`no selected file fit after including rendered metadata overhead`; narrow source
inspection was the fallback. This remains related to M1's budget/allocation gap.
Do not confuse a successful map-oriented `search` with implementation coverage,
or a fail-closed budget refusal with proof that a requested symbol was absent.

The candidate notes in [v0.5.2.md](releases/v0.5.2.md) disclose deferred scope.
No independent held-out data, external model execution, package publication or
new release tag was used in this review.
