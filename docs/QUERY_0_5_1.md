# Query reliability in cxt 0.5.1

Local Windows x64 verification, 2026-10-14–15, Rust 1.97.1. This is a stronger verified cxt query implementation, not proof of best-in-class retrieval. The shared CLI/TUI/MCP engine remains local lexical/static retrieval; no model or network calls were added to selection.

## Changes driven by failures

- Bounded, bidirectional identifier relationships carry related implementations into focused/adaptive output, including shared-state handlers without query words. Names must be unique within their language family; ambiguity and high-fanout hubs do not become resolved references.
- Up to three implementation anchors precede cheap supporting context. Exact query-word matches in project-relative directory components take priority over generic API chatter, then file scores. Explicit review pins remain first. Anchors are heuristics, not guaranteed pins.
- Soft per-file span allocation leaves room for multiple primary implementations; an indivisible implementation may retry the full span budget. Final rendered-packet accounting still decides acceptance.
- Rust fields retain original declaration attributes/comments and trailing separators. Named field selection no longer requires a whole struct; the parser-cache schema is invalidated.
- Original-source fail-policy findings survive representation errors. Candidate-wide enforcement is deliberately unchanged, including candidates later omitted by budgeting. Failed scanners, scans, source freshness and omitted pins still block export.
- Evaluation reports classify failures and retain failed-child timing without storing raw stderr. These categories are diagnostic hints, not automatic proof of a root cause.

An intermediate candidate improved Click but regressed Afero by putting generic backend APIs ahead of its explicitly named module. The public replay caught that regression before tagging. A minimized public-CLI test now covers it. A separate exact-source check caught the missing Rust-field comma; the export was corrected, not the label. Intermediate reports and binaries remain preserved locally.

## Local release gate

- `cargo fmt -- --check`, **167 passing ordinary tests**, strict all-target/all-feature Clippy and optimized binary/example builds pass. One optional installed-Gitleaks test passes separately.
- Nine integration suites pass with the immutable optimized CLI selected: **130 ordinary cases**. Shared-core API assertions in those suites also run in their compiled test artifacts.
- Optimized help/version, named annotated-field export, import tracing, Git-impact selection and scan-cap refusal pass. The requested shared-state adaptive query with cl100k, an 8K budget and a manifest retains all three implementations in a 517-token Markdown packet.
- Workflow YAML and installer shell syntax parse. Hosted CI and asset publication are separate checks; consult the live repository workflows. No new physical-terminal TUI acceptance, ARM-device runtime test, package-manager submission or installed-PATH replacement is claimed by these local results.

Final local executable SHA-256: `3ed5bdd2c593ff828c3404ec2e761c93bdc251394e7559ac4cc05107ec47c185`.

Preserved copy: `target/evaluation/cxt-0.5.1-3ed5bdd2c593.exe`. This identifies the local measurement binary, not a claim that a separately compiled hosted asset has the same hash.

## Retrieval measurements

Every evaluation uses independently counted actual JSON stdout under offline cl100k. Source spans must contain the original implementation; maps do not count. Cold/repeat invocations are repetitions, not independent tasks.

### Shared-state regression

`eval/relationships.json`: two small Python/TypeScript tasks, focused/adaptive, 2K/4K budgets, expansion on/off, 32 invocations per tool. Labeled-source fingerprints match the actual published Windows 0.5.0 baseline.

| Expansion | Published 0.5.0 mean span recall | 0.5.1 mean span recall | Mean accepted packet tokens, baseline / 0.5.1 |
| --- | ---: | ---: | ---: |
| On, 16 invocations | 66.7% | 100% | 426.5 / 648 |
| Off, 16 invocations | 33.3% | 33.3% | 317.5 / 317.5 |

These deliberately small regression cases do not establish general semantic understanding. Better coverage can require more tokens.

### Existing synthetic corpus and ablations

The 540-invocation corpus replay retains 100% labeled-span recall in focused/adaptive mode at 8K, 16K and 32K: 60 accepted invocations per mode/budget. Mean packets are 1,141 tokens at 8K/16K and 2,854 at 32K. Complete mode accepts only 4/60 at 8K, with zero mean span recall and **56 retained token-budget failures**; at 16K/32K all 60 accept with 100% recall and mean 9,979 tokens. Overall, 484/540 invocations accept.

All 96 ranking-ablation invocations accept. Hybrid and symbol-only modes retain 100% labeled-span recall; lexical and legacy modes retain 0%. Those labels intentionally test symbol signals, not balanced real-world ranker quality. Expansion on/off does not change these one-file ablation cases.

### Provisional public-task replay

The same predeclared six-task subset, focused context, 8K/16K budgets and cold/repeat phases yields 24 invocations per executable. Queries, filters, original-source labels, fingerprints and strict security policy are unchanged. All six input checkouts were verified clean at their pinned base commits. No solution patches, upstream builds/tests or coding models were executed.

| Public task | 0.5.0 mean span recall, four invocations | 0.5.1 mean span recall, four invocations | 0.5.1 result |
| --- | ---: | ---: | --- |
| afero-tar-independent-handles | 50% | 100% | Both implementations retained at both budgets |
| click-black-color-zero | 0% | 100% | Both implementations retained at both budgets |
| fd-format-hyphen | 0% | 100% | Annotated field retained, including original separator |
| arduinojson-long-numeric-string | 0% | 0% | Four `secret_policy` refusals |
| jsoup-malformed-end-tag | 0% | 0% | Four `secret_policy` refusals |
| ky-json-error-utf8 | 0% | 0% | Four `secret_policy` refusals |

**Mean per-invocation span recall including failures: 8.33% → 50%.** Both tools accept 12/24 invocations. Within the 12 accepted 0.5.1 packets all labeled implementations are present, but dropping the refusals from the headline would hide an important limitation. No allowlist or policy relaxation was introduced for this replay.

Final packet counts at 8K / 16K are Afero 7,967 / 15,799, Click 5,882 / 15,152 and fd 7,642 / 15,731. Cold/repeat selections agree. The remaining failures are policy refusals in this replay, not evidence that retrieval in those repositories is now correct.

These cases informed debugging and tuning: **they are not held-out tasks, independently reviewed gold, downstream AI task success or competitor-superiority evidence**. See the [public materialization protocol](EVALUATION.md#materialize-public-exploratory-tasks). The unchanged subset configuration SHA-256 is `a9207af94f5709f47821464875a734aa1f938ac0cc2869434fb87213708c0ede`. The [first development pass](QUERY_RETRIEVAL_CYCLE.md) preserves the earlier misses.

### Incremental graph regression

The generated 500-module / 100,000-function workload passes reuse/mutation assertions: repeat reuses all 500 nodes, one source edit rebuilds one, and rename/import edit rebuilds two. Single-run times are 5,506 ms cold, 1,472 repeat, 1,449 after one edit, 1,498 after rename/import edit and 3,246 without cache. These are slower than some historical local runs; no controlled latency improvement is claimed. Enumeration/read/hash, metadata validation and workspace construction are included. This is not a public-monorepo query benchmark or a zero-read index.

## Reproduce and audit

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

Final local reports are `target/evaluation/{relationships,ablation,retrieval,public-six,graph}-0.5.1-verified.json`. Reports without `-verified` identify the earlier 0.5.1 candidate, whose binary hash begins `9c3ca4871492`; they are not final results. Baseline reports remain `relationships-published-0.5.0.json` and `public-six-published-0.5.0.json`, using published-binary SHA-256 `cc05db5c50ebfb63e8bee7668f421a9a3a30fb0c3df4fd154781df5d21cd7441`.

| Final report | SHA-256 |
| --- | --- |
| relationships | `bacc61ee3ed7b6c884644c28001119d65090e941e9d277ad67b7196a41a9368c` |
| ablation | `6d72520bdd160ab3c0d506364d6de7a2a97364ed1014559d0a3c7f94b37e4255` |
| retrieval | `eb7d57c23c1c6b173d74c3fe8c4597df7924d743c4158415ac505dac0a831cd6` |
| public-six | `3f0ec89e1ed03519f0cd7992ca1661a3a32401e1f27e3ddac573705d103cdad5` |
| graph | `ba73f38a740929404b4423c20f53ea318b434b199831cf84341d8ac61487ff8f` |

## What remains

Compiler/resolver-supported aliases and bindings, persistent searchable term data without cached source bodies, independently reviewed held-out labels and identical-model downstream coding experiments remain future work. Dynamic dispatch, framework wiring and terminology mismatch can still defeat these heuristics. A release number does not remove those limitations.
