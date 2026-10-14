# Query reliability: first post-0.5.0 development pass

Historical first-pass snapshot: local Windows x64 verification, 2026-10-14, Rust 1.97.1. At measurement time these changes were uncommitted and unreleased; the development binary still reported package version 0.5.0. Identify this experiment by its hash, not that version string. Subsequent fixes and final 0.5.1 measurements are recorded separately in [the release verification report](QUERY_0_5_1.md); the results below are preserved rather than retroactively replaced.

## Implemented

- Shared-core, bidirectional identifier relationships up to two hops, including another handler using the same state helper without query words in its own implementation.
- Exact AST identifier matching instead of split-word/string/comment coincidences for local helper expansion. Ambiguous names are skipped within language families; JavaScript/TypeScript variants share a family.
- Related-symbol hints carried into focused/adaptive representation. Hint seed limits do not discard other ordinary lexical matches. High-fanout traversal and expansion volume are bounded; see [engine contracts](CONTEXT_ENGINE.md).
- Per-definition reference metadata in the hash-validated parser cache, with schema invalidation. This is **not** a persistent searchable BM25 term index, compiler-resolved references or zero-read querying.
- Evaluation failure categories, exit codes and failed-child timing without retaining raw stderr. Empty-plan errors preserve preparation reasons rather than mislabeling every failure as an unreadable file.
- Candidate-wide original-source fail-policy retained by explicit maintainer choice. A regression also fixes findings being discarded when focused-span preparation fails. This was a refusal-handling gap; the reproduction did not export the detected credential. Scanner failures, scope, freshness and pin protections remain blocking.

The test-first public CLI/core approach kept selection and export behavior shared by CLI, TUI and MCP rather than adding separate client rankers. Source spans, representation labels and final rendered token checks remain authoritative.

## Verification

- Format check, 162 ordinary tests, strict all-target/all-feature Clippy and optimized binary/example builds pass. One optional installed-Gitleaks test passes separately.
- All nine integration suites pass with the final optimized CLI selected at the public executable boundary: 125 ordinary cases. Core API checks in those suites also execute the compiled source under test.
- Optimized scan-entry-cap smoke refuses export; help/version and the shared-state CLI exports work.
- CI YAML parses and the build matrix now exercises the shared-state corpus in addition to the existing ranking ablations. Hosted Windows/Linux/macOS jobs have not run for this unpushed tree. No new physical TUI acceptance run or downstream coding-agent experiment is claimed.

Final local executable SHA-256: `c85b1687f56ea04a62ad9ba6157292b238d34dbd02414703c3720c54a4186728`.

Preserved copy: `target/evaluation/cxt-query-c85b1687f56e.exe`. Baseline: actual published Windows x64 0.5.0 binary, SHA-256 `cc05db5c50ebfb63e8bee7668f421a9a3a30fb0c3df4fd154781df5d21cd7441`, under `target/published-0.5.0/`.

## Measured results

### Targeted shared-state regression

Two synthetic Python/TypeScript tasks, focused/adaptive modes, 2K/4K exact final-packet budgets, expansion on/off and cold/repeat subprocesses: 32 attempts per executable. Each expansion group below contains 16 attempts, not 16 independent tasks. Labeled source fingerprints match between tools.

| Configuration | Published 0.5.0 span recall | Development span recall | Mean accepted packet tokens, baseline / development |
| --- | ---: | ---: | ---: |
| Expansion on | 66.7% | 100% | 426.5 / 648 |
| Expansion off | 33.3% | 33.3% | 317.5 / 317.5 |

Every attempt fits its requested budget. The additional handler costs tokens; this is recovered implementation coverage, **not** a token-reduction result or evidence of general semantic retrieval.

### Existing synthetic corpus and ablations

The 30-task corpus completes 540 development attempts across three representations, three budgets and cold/repeat phases. Focused/adaptive retain 100% labeled-span recall at 8K/16K/32K, unchanged from the preceding release measurements. Complete mode still accepts only 4/60 attempts at 8K with zero required-span recall; its other 56 attempts remain recorded as token-budget failures. Complete mode at 16K/32K retains 100% recall. Overall acceptance is 484/540; failures are not omitted from scores.

The 96-attempt ranking ablation remains unchanged: hybrid/symbols recover the required span on its simple fixtures; lexical/legacy do not. Expansion on/off has no effect on that older one-file fixture. That is why the separate shared-state control was added.

### Six public provisional tasks: no gain demonstrated

Same predeclared tasks as the earlier six-repository bootstrap, focused context, 8K/16K budgets and two subprocess phases: 24 attempts per executable. All six source checkouts were verified clean at their pre-fix pinned commits. Labeled source fingerprints match. These are provisional static-source labels, **not independently reviewed gold**.

Both tools accept 12/24 attempts and average 8.33% required-span recall with failures scored zero. Development classifications distinguish the other 12 attempts as secret-policy refusals; no scan timeout occurred in this final replay. Older retained experiments did contain scan timeouts, so this is not proof that timeouts are solved.

| Task | Accepted attempts, each tool / 4 | Mean required-span recall, each tool | Development failure category |
| --- | ---: | ---: | --- |
| afero-tar-independent-handles | 4 | 50% | — |
| click-black-color-zero | 4 | 0% | — |
| fd-format-hyphen | 4 | 0% | — |
| arduinojson-long-numeric-string | 0 | 0% | secret_policy |
| jsoup-malformed-end-tag | 0 | 0% | secret_policy |
| ky-json-error-utf8 | 0 | 0% | secret_policy |

No security allowlist, source filter or policy relaxation was introduced to improve these scores. Refusal is not the same failure as an accepted packet missing its required implementation. This result does **not** support a broad accuracy improvement or competitive-superiority claim.

### Incremental graph regression

The generated 500-module / 100,000-function workload passes reuse/mutation assertions: repeat reuses all 500 nodes; one source edit rebuilds one; rename plus import edit rebuilds two. Current single-run phase times: cold 4,608 ms, repeat 1,188 ms, edit 1,138 ms, rename/import edit 1,130 ms, cache-disabled 2,609 ms. This includes enumeration/read/hash and workspace construction. It is not a monorepo query-latency result, zero-read index or statistically controlled speed comparison.

## Reproduce and inspect

```powershell
cargo fmt -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --bins --examples
.\target\release\cxt.exe --evaluate eval/relationships.json --output target/relationships.json
.\target\release\cxt.exe --evaluate eval/ablation.json --output target/ablation.json
.\target\release\cxt.exe --evaluate eval/corpus.json --output target/retrieval.json
cargo run --release --example graph_benchmark -- 500 200
```

Local retained reports use `target/evaluation/{relationships,ablation,retrieval,public-six,graph}-query-verified.json`. Baseline reports are `relationships-published-0.5.0.json` and `public-six-published-0.5.0.json`. Older `*-query-cycle.json` and `*-query-final.json` files are intermediate snapshots, retained for audit, not the final guard-fixed executable.

The six-task public configuration is a derived subset of the existing byte-verified materialized corpus, with only `contexts=[focused]` and `budgets=[8000,16000]`; see [the materialization protocol](EVALUATION.md#materialize-public-exploratory-tasks). Its SHA-256 is `a9207af94f5709f47821464875a734aa1f938ac0cc2869434fb87213708c0ede`. Public source remains in local ignored checkouts; no upstream builds/tests or models were executed. Timing/memory fields are retained, but no comparative latency claim is made from these small experiments.

## Still needed

1. Resolver/compiler-supported binding evidence for aliases, shadowing, overloads and framework relationships. Unique names alone remain a heuristic.
2. Persistent searchable term data to reduce repeated lexical work, with explicit privacy, invalidation and resource-bound tests. Source bodies must not become an implicit cache.
3. Independently reviewed real-task labels, including terminology mismatch and meaningful negatives. Do not tune away the retained public failures or promote provisional labels to gold.
4. Identical-model downstream coding experiments before claiming task-success gains. No model calls, paid API use or private-source upload were performed in this pass.

Continue retrieval work; this pass is useful hardening, not the final claim that query is best-in-class.
