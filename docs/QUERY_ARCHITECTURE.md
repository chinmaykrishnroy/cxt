# Evidence-guided, model-free retrieval

This is cxt's development direction, not a claim that all stages are implemented
or that cxt outperforms other tools. The released 0.5.1 behavior is documented in
[CONTEXT_ENGINE.md](CONTEXT_ENGINE.md). Work must preserve the shared core used by
CLI, TUI and MCP, backward-compatible query options, and local deterministic
execution without model downloads, LLMs or network calls in selection.

## Current development status

| Architecture area | 0.5.2-dev.1 foundation |
| --- | --- |
| Resolution | Supported local scopes and Python/JavaScript/TypeScript import bindings; not complete language or type resolution. |
| Typed graph | Source-located `CALLS`/`REFERENCES` edges, inverse traversal and explicit binding/heuristic labels; effect/event/test edge kinds are not implemented. |
| Task interpretation | Planned; query terms and existing ranker controls remain. |
| Repository vocabulary | Planned; no general synonym or terminology-mismatch guarantee. |
| Retrieval | Bounded weighted best-first reference traversal, source-backed reasons and limited unresolved-target/hub diagnostics. |
| Span optimization | Existing focused/adaptive heuristics and final budget verification; global coverage optimization is planned. |
| Persistent search | Hash-validated parser/dependency metadata exists; lexical postings and persistent reference adjacency are planned. |

The binding implementation is deliberately limited. Re-export chains, wildcard
imports, Python global/nonlocal and comprehension semantics, general member/type
inference, overloads and dynamic dispatch are not fully modeled. Import/local
binding labels describe supported static syntax, not proof of runtime behavior.
Negative regressions prevent known scope confusions; they are not an exhaustive
language-conformance suite.

The [local verification report](QUERY_ARCHITECTURE_VERIFICATION.md) identifies the
tested development binary, comparisons with 0.5.1, retained failures and limits
on what these synthetic results establish.

## Retrieval pipeline

1. Guard the selected workspace and enumerate eligible sources under scan limits.
2. Read and hash sources; reuse only matching, versioned metadata.
3. Produce lexical/path/symbol anchors and an explicit task interpretation.
4. Resolve reference sites against lexical scopes, imports and optional trusted
   compiler-index evidence. Keep unresolved and ambiguous bindings explicit.
5. Traverse a bounded typed graph with evidence costs and deterministic tie breaks.
6. Choose original implementation spans and supporting evidence by marginal
   coverage per token, preserving essential symbols and deduplicating overlaps.
7. Apply the existing security policy, freshness and pin checks; verify the exact
   final rendered token budget and disclose omissions.

This is not a request to weaken candidate-wide fail-on-secret policy. Findings in
any original prepared candidate still block export, even when slicing or the
final budget would omit them. Operational scanner errors fail closed too.

## Module responsibilities

| Module | Responsibility |
| --- | --- |
| `symbols` | Original AST definitions, scopes, imports, reference sites and source spans. |
| `bindings` | Shared lexical/import binding decisions for graph expansion and focused helper retention, without filesystem reads. |
| `index` | Hash-validated parser and searchable lexical metadata; never raw source bodies. |
| `resolver` / `dependency_index` | Workspace-bounded module resolution and observed configuration/probe invalidation. |
| `query_graph` | Evidence-bearing relationships and bounded graph traversal. |
| `query` | Explainable anchor ranking, grounded vocabulary and task interpretation. |
| `engine` | Shared snapshots, policy enforcement, representation and final rendered budgets. |

Do not duplicate resolution or packing in CLI/TUI/MCP. Do not start an LSP server,
execute project hooks, download dependencies or invoke a compiler implicitly.

## Evidence contract

A relationship needs a source symbol, target symbol, kind, original reference
location, resolution method and evidence category. Confidence is categorical,
not an invented probability. An inferred path must not become "verified" merely
because its last edge is precise: explain every edge and the weakest evidence.

- **Compiler-resolved:** optional externally produced evidence, with its producer
  and source/configuration identity validated. No compiler adapter exists yet.
- **Syntax-bound:** a supported import/local binding identifies one target. This
  does not prove runtime dispatch or the absence of monkey patching.
- **Heuristic:** a unique name or convention suggests a target; uncertainty stays
  visible. Explicitly shadowed, ambiguous or unresolved imports must not fall back
  to unrelated global definitions.

Start with `CALLS` and `REFERENCES`; reverse traversal is a view over the original
edge, not a second independent observation. Add `IMPORTS`, `READS`, `WRITES`,
`IMPLEMENTS`, `EMITS`, `HANDLES`, `CONFIGURES` and `TESTED_BY` only when extractors
provide defensible evidence. A reference is not automatically a call, mutation,
event subscription, test coverage or runtime dependency.

Optional SCIP ingestion should precede live LSP orchestration. Both are adapters,
not requirements for ordinary use. Validate paths, encodings, source revisions,
producer identities and resource bounds. Reject stale indexes. Capability gaps
remain gaps rather than fabricated relationships.

## Grounding and coverage

Ground user terminology in visible repository evidence: UI strings, route names,
configuration keys, documentation, test descriptions and symbols. Record which
source established each bridge; cap expansion and avoid recursive synonym drift.
Documentation alone does not prove that a particular implementation is relevant.

For "changing one room's volume affects every room", useful coverage may include
the handler, mutation, shared-state definition, other consumers and tests. These
are evidence roles, not mandatory fictional nodes. Report which roles are found,
missing, unresolved or omitted by budget. A map is orientation, not implementation
coverage. No model-free engine can guarantee a terminology bridge without any
observable connection in the repository.

## Incremental milestones and gates

1. Reference-site extraction, import/member binding, lexical shadowing and
   explainable typed traversal. Regressions include positive and negative links.
2. Searchable lexical postings and graph reuse with configuration-aware
   invalidation, cold/repeat/edit/rename measurements and bounded storage.
3. Coverage-aware span selection and bounded repository-vocabulary/task adapters.
4. Optional compiler-index ingestion after the evidence contract is stable.

Each milestone must pass public CLI/core regressions, source fidelity/security
tests, synthetic retrieval and ranker ablations before claims are expanded.
Freeze code and weights before opening held-out task details; source-span labels
need independent review. Evaluate recall, token precision, false relationships,
latency, memory and refusals at equal final token budgets against frozen 0.5.1.
Synthetic examples and provisional labels do not establish superiority.

Downstream coding experiments are **offline harness only** until an explicit
model/agent, source-sharing policy and execution environment are approved. Do not
report coding-task success from retrieval recall. This document does not authorize
publication, a release tag or changes to existing public tags.
