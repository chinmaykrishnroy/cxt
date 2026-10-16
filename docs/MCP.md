# Read-only MCP integration

Launch cxt 0.5.0 or later for exactly one trusted project:

```bash
cxt --mcp /absolute/path/to/project
```

In a client that supports stdio MCP servers, configure the executable and arguments:

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

On Windows use an absolute executable path if the client does not inherit your shell PATH. Check `cxt --version`; pre-0.5 binaries do not support this server. Client configuration locations differ; the snippet describes the server entry, not every client's complete schema. For instruction-based integration without a server, see [agent skills](AGENT_SKILL.md).

## Tools

| Tool | Use |
| --- | --- |
| `get_context` | Complete/focused/adaptive source packet for optional paths/query. |
| `search` | Query-ranked symbol maps; requires `query`. |
| `trace` | Static dependencies from explicit entry-file `paths`. |
| `get_symbol` | Implementation/import spans; requires `symbol`. |
| `get_diff_context` | Current-checkout source selected by `git` mode/range, default working. |
| `get_related_tests` | Naming/import-based test suggestions for paths or Git changes. |
| `get_repo_map` | Structural orientation; not implementation coverage. |

Arguments: optional workspace-relative `paths` (1–32), `query` (up to 1024 UTF-8 bytes), `symbol`, `context` (`complete|map|focused|adaptive`), `max_tokens` (1–32000, default 8000), and `git`. Unknown keys and invalid types are rejected. Tool results contain text plus structured `cxt.context.v1` JSON; errors set `isError`. JSON-RPC initialization/initialized notification must precede tool operations. Transport is newline-delimited stdio, protocol negotiation for 2025-03-26 / 2025-06-18; no HTTP endpoint.

## Agent workflow and recovery

Start with a project subdirectory or a few entry files and leave `max_tokens`
at its 8000 default unless the task warrants another budget. `search` and
`get_repo_map` return maps, not implementation bodies. Follow promising names
with `get_symbol`, or request `get_context` with `context: "focused"` and a
task query. Inspect representations, original-source spans and omissions after
each call; a successful packet can still omit the implementation you need.

The token budget includes the **rendered JSON packet**, including source,
metadata, selection reasons and omission diagnostics. It is not a source-only
allowance or a measurement of a client's complete model/protocol bill. A
400-token search or 1200-token whole-repository map can refuse export even
when individual source snippets are small. When this happens, the error names
`max_tokens`, explains overhead and suggests increasing it (up to 32000) or
narrowing paths/query. Raising it to 32000 is not a universal fix: a client can
still truncate a large response. Prefer a focused follow-up over a repository dump.

`get_related_tests` uses filename similarity and test files reached through
bounded reverse-import edges. It does not understand every test framework,
fixture injection, HTTP-client call or dynamic test setup. No matches is not
proof that no relevant tests exist: try `search` using the behavior and test
terms, then inspect the resulting source. Naming/import associations remain
heuristics, not verified behavioral coverage.

For a suspected stall, record `cxt --version`, the exact request/command, elapsed
time, exit status and full sanitized error. Scan limits refuse export rather than
return partial context; MCP's limits are fixed, so narrow requests rather than
adding unsupported CLI limit flags. A `--mcp` process normally stays alive waiting
for the next stdio request; that alone is not a hung scan. CLI status messages on
stderr are not themselves failure evidence: distinguish PowerShell's handling of
native stderr from cxt's exit status and its explicit scan/error diagnostics.

## Boundaries

- One canonical directory; absolute, parent-relative and outside symlink paths are rejected. Git root must also be inside it.
- Built-in redaction is fixed; callers cannot choose `allow`, exceptions, output paths, cache writes, broad scans or external commands. Global/project cxt configuration is ignored; it cannot weaken server policy or alter selection.
- Exact cl100k accounting, 10,000 entries, 16 MiB eligible input, depth 24 and cooperative 10-second phase limits. Requests/responses are capped at 1 MiB; narrow the request when capped.
- No source mutations, network/model calls, clipboard delivery or persistent index writes. The server still invokes Git for Git requests.
- Secret scanning remains best effort; the authorized workspace must itself be trustworthy. Canonical path checks are not an OS sandbox. No full asynchronous cancellation or hard real-time interruption of parser/filesystem operations is promised.

The protocol is intentionally small. Do not grant a client your user directory or filesystem root. Review packet reasons, representations, warnings and omissions before treating it as task-complete context.
