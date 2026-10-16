# cxt delivery TODO

Status checked against the **0.5.2 stable release preparation** on 2026-10-16.
This is the actionable backlog; [ROADMAP.md](ROADMAP.md) provides historical
milestones and [the query architecture](docs/QUERY_ARCHITECTURE.md) defines the
target contracts. Unchecked items are not implemented or verified capabilities.

## Stable 0.5.2: bounded reference-resolution release

Release the current foundation with explicit experimental-retrieval limits,
rather than describe the whole DeepQuery architecture as complete. The
[verification report](docs/QUERY_ARCHITECTURE_VERIFICATION.md) records the tested
development binary and local results; it is not final cross-platform acceptance.

- [x] **R1 — Freeze and review scope.** Review the binding/resolution changes,
  supported syntax and negative cases. Triage the MCP findings in
  [the dogfooding log](docs/MCP_DOGFOODING.md); decide which are release blockers
  and document any deferred limitations. Keep candidate-wide secret fail-policy.
  DEV progress: qualified Rust impl lookup, non-lexical impl bindings and scoped
  imports have MCP regressions. Import-only support no longer inherits anchor
  relevance. The original 8K probe now retains maps for all three requested files,
  but the 2.5K probe still retains only one map; implementation coverage and
  metadata-aware allocation remain open, not a solved retrieval claim.
  Candidate scope: MCP `search` is intentionally map-oriented, so missing bodies
  in search alone are not an implementation-export failure. Low-budget multi-file
  coverage stays a disclosed heuristic limitation. Persistent lexical/session
  indexes and full behavior-oriented retrieval are deferred, not silently claimed.
- [x] **R2 — Investigate intermittent TUI worker timeout.** Retain the failed run,
  reproduce under controlled load, distinguish deadline sensitivity from a lost
  worker result, and record the disposition. A green rerun alone is not a fix;
  increasing the timeout alone is not a diagnosis.
  Include the prerelease's concurrent TUI scan/Gitleaks timeouts and the enclosing
  Git-root review-graph cost. Both isolated checks passed without deadline changes;
  their load-sensitive behavior still needs controlled measurement.
  DEV progress: reproduced and fixed output-only settings rereading reviewed
  sources, and a disconnected response channel leaving the TUI busy forever.
  Both have App/worker regressions; neither is proven to explain the historical
  deadline failure. The ten-second test deadline remains unchanged, with richer
  revision/workspace/plan diagnostics. Keep R2 open until that failure is resolved
  or explicitly accepted as a documented release limitation.
  A later optimized integration replay also retained three scan-time-limit
  refusals (`exact_budget_is_checked_on_actual_stdout_for_every_output_format`,
  `go_local_replacements_trace_the_replacement_package`, and
  `cargo_target_specific_local_dependencies_are_discovered_conservatively`).
  No partial context was exported. Distinguish these production scan deadlines
  from the ten-second TUI test helper deadline; serial replay results belong in
  the DEV verification report, not in a claim that the failures are fixed.
  2026-10-16: a zero-wait regression reproduced a completed current-revision plan
  being reported stalled because the helper checked elapsed time before polling.
  Poll ordering is corrected without widening the deadline; an obsolete-response
  counterpart guards against accepting unfinished work. This is not proof of the
  original failure's cause; repeat measurements and explicit disposition of
  load-sensitive scan refusals are recorded below, not a root-cause claim.
  Release disposition: ship the demonstrated polling/snapshot/disconnection
  fixes with the unchanged cooperative scan guards. Retain heavy-load refusals
  as an explicit fail-closed limitation, not a claim that all historical timeouts
  were repaired. The user subsequently authorized the bounded stable release;
  the external whole-repo stall remains an open reproduction request below.
- [x] **R3 — Finalize release identity.** Change Cargo.toml/Cargo.lock to 0.5.2,
  finalize CHANGELOG.md, add `docs/releases/v0.5.2.md`, and align help/docs with
  the actual supported scope. Identify the exact source and executable hashes.
  Cargo.toml/Cargo.lock identify 0.5.2; versioned stable notes are finalized at
  `docs/releases/v0.5.2.md`. The user authorized main merge and stable publication.
  The frozen local input/executable identities and completed local gates are in
  [candidate verification](docs/STABLE_0_5_2_CANDIDATE_VERIFICATION.md); hosted
  acceptance and published-asset verification remain separate.
- [x] **R4 — Revalidate the final source locally.** Run every AGENTS.md validation gate,
  resolver/public CLI/core regressions, synthetic corpus and ablations. Retain
  refusals, omissions, source fingerprints and exact final rendered-token counts.
  194 tests and strict Clippy passed; 153 integration tests also passed with the
  frozen optimized executable, and optional Gitleaks passed separately. Synthetic
  corpus retains 56 token-budget refusals; acceptance is not task success.
- [ ] **R5 — Runtime and distribution acceptance.** Exercise CLI defaults,
  explicit TUI, MCP, freshness/security/scan failures and install/uninstall/PATH
  behavior. Check native-host results separately from cross-compilation. Obtain
  hosted CI/build results for the release targets; resolve failures before
  claiming those platforms are supported by this release.
  The previously merged baseline `0c63db8` passed all six hosted CI jobs on
  2026-10-15, including native Windows/macOS/Linux tests, release builds and
  synthetic retrieval controls. This does not validate later candidate edits.
  Local candidate TUI/stdin MCP smokes, three unchanged-deadline worker repeats
  and the generated 500-by-200 graph/mutation check passed. These do not close
  the historical timeout or validate the external whole-repo stall. The user
  authorized DEV push and empty-tag six-target build-only CI after local checks;
  no main merge, public release or installed-binary replacement is included.
  Superseding authorization: the user requested final release to main. Main
  merge/tagging will follow the candidate six-target and final-source checks;
  package-manager submissions and installed-user-binary replacement are separate.
  The [candidate build-only run](https://github.com/chinmaykrishnroy/cxt/actions/runs/38864100676)
  passed source validation and all six target build/package jobs. Publication
  was skipped as intended. Final-main CI and tagged asset verification follow.
- [ ] **R6 — Publish only with authorization.** Verify the remote branch and unused
  annotated `v0.5.2` tag, workflow start, asset names/checksums and actual downloads.
  Update package-manager submissions only after immutable assets exist. Preserve
  public tags and unrelated work.

Full behavior-oriented retrieval is not required to ship this bounded patch.
Reviewed public evidence is required before making new real-world accuracy or
competitive claims, even if those experiments finish after the release.

### Agent-usability acceptance from external feedback

The 2026-10-16 pasted review is development feedback, not a controlled benchmark.
Do not optimize for an agent's numeric praise or call this a 9/10 product without
independent task evidence. Do not access the reviewer's private repositories.

- [x] Reproduce the vague small-budget failure through actual MCP on an owned
  fixture; add a red/green public MCP regression for limit, cause, recovery and
  a successful bounded retry. Keep failure atomic; do not drop omission/security
  information to manufacture acceptance.
- [x] Reproduce naming/import inconsistency in a supported Python project;
  retain import-derived test candidates with positive and unrelated negative
  controls. This is not universal framework/test discovery.
- [x] Add MCP initialization/tool-schema guidance for map-to-source follow-ups,
  the default 8000-token whole-packet budget, overhead and narrow retries.
- [ ] Obtain the sanitized exact version, command/request and complete scan
  failure from the external whole-repository stall. Separate idle MCP lifetime,
  native stderr handling, explicit scan refusals and an actually stuck operation.
  Do not increase limits and claim a performance fix without a timing loop.
- [ ] After freezing the candidate, measure first-call acceptance, retries,
  required implementation/test coverage, useful tokens versus total tool traffic,
  refusals, p50/p95 latency and peak memory on independently reviewed tasks.
  Compare equal-budget baselines and count missed/blocked tasks. A subjective
  6–7/10 review is a reason to investigate, not a measured 9+ release gate.

## DeepQuery gaps

Priorities apply to architectural development, separately from the release gates.
P0 is an evaluation-integrity prerequisite; P1 is the next engineering sequence.

| ID | Component | Current status | Next work |
| --- | --- | --- | --- |
| D1 | Scope-aware references | Partial; supported lexical binding cases, not complete language semantics | P1: broader syntax/languages with shadowing and ambiguity regressions. |
| D2 | Import-aware resolution | Python and JS/TS subset, direct declaration exports | P1: re-export chains, complex bindings and explicit unresolved results. |
| D3 | Weighted graph retrieval | Bounded evidence-bearing best-first traversal | P2: adaptive bounded depth and diverse supporting evidence; retain per-edge provenance and hub penalties. |
| D4 | Persistent searchable index | Parser/dependency metadata cached; lexical postings and reference adjacency rebuilt | P1, first implementation: normalized postings, lookup metadata and validated adjacency reuse. |
| D5 | State relationships | No behavioral `READS`/`WRITES` extraction | P1, second implementation: conservative AST-backed state identity and access edges. |
| D6 | Events and routes | No `EMITS`/`HANDLES`/`ROUTES_TO` relationships | P2: narrowly supported framework adapters with source evidence. |
| D7 | Natural-language task interpretation | Token normalization/ranking, not intent or symptom parsing | P2: deterministic entity/operation/symptom/intent rules. |
| D8 | Repository terminology | No general grounded vocabulary bridge | P2: bounded UI/config/route/docs/test terminology tied to source locations. |
| D9 | Global context optimization | Focused/adaptive heuristics and exact final budget checks | P2: coverage-aware span allocation across candidates. |
| D10 | Compiler-grade resolution | Not implemented | P3: optional, validated SCIP ingestion; retain offline AST fallback. |

### P0 — Independent evaluation before further tuning

- [ ] **E1 — Establish actual holdout isolation.** A file in this workspace is
  accessible to the implementation agent. The existing provisional protocol is
  therefore not an access-controlled holdout. Have an independent human/curator
  keep new tasks, identities and labels in a separate account/store/environment
  to which implementation agents have no read access or credentials. Git ignore
  rules, path filters, a hidden filename and instructions alone are not isolation.
  Do not inspect or relocate the current task details during implementation.
- [ ] **E2 — Freeze before disclosure.** Record source/config/ranking-weight and
  binary hashes, test protocol, budgets, baseline versions and pass/fail criteria.
  Independently review original source-span labels, including negative links.
  Run the sealed evaluation through the curator; tuning after disclosure turns
  that task set into development data and requires a fresh holdout.
- [ ] **E3 — Measure equal-budget usefulness.** Use unfamiliar public,
  redistributable repositories and fixed tasks. Report required-symbol recall,
  relevant-token precision, false-positive relationships, acceptance/refusals,
  timeouts, latency distributions and memory. Count failures in the denominator;
  distinguish maps from implementation coverage and include all rendered overhead.
- [ ] **E4 — Keep coding experiments offline-only.** Prepare a reproducible
  harness, frozen baselines and result schema. Model execution, source sharing,
  paid calls and unfamiliar upstream execution remain separately approval-gated.
  Retrieval recall is not downstream coding-task success.

### P1 — Searchable indexing, then behavior-oriented edges

- [ ] **D4a — Lexical postings.** Reuse normalized term frequencies, document
  lengths and lookup metadata instead of retokenizing every candidate on each
  query. Persist no raw source bodies; assess sensitive identifier/literal leakage
  in metadata, bound storage and preserve cache-free operation.
- [ ] **D4b — Validated graph reuse.** Persist/reuse symbol lookup and reference
  adjacency separately from task-specific traversal. Invalidate on source hashes,
  schema/parser/resolver changes, configuration, missing-target creation,
  deletions, renames, branch changes and workspace policy. Observe resolver probes.
- [ ] **D4c — MCP session reuse.** Design bounded in-memory workspace reuse while
  preserving MCP's no-persistent-cache-write contract and freshness checks.
  Validate cold/repeat/edit/rename/config-change/corruption behavior, deterministic
  equivalence with cache disabled, latency and memory on realistic workspaces.
  Faster repeated queries must not become a zero-source-read claim.
- [ ] **D5a — First behavioral extractors.** Start with explicit supported JS/TS
  state access/assignment syntax, then Python. Distinguish state identity, reads,
  writes and unresolved alias/member access; a call named `set` is not evidence
  of mutation. Record original AST locations and evidence categories.
- [ ] **D5b — Shared-state paths.** Retrieve handler -> mutation -> shared state ->
  other consumers, plus independently supported related tests. Preserve missing
  roles and ambiguous paths in explanations. Include query-word mismatch,
  shadowing, alias/reassignment, unrelated receiver and duplicate-name negatives.
- [ ] **D1/D2 — Resolution coverage.** Add supported cases incrementally with
  language-specific positive/negative regressions, config/probe invalidation and
  safe unresolved fallbacks. Measure false links, not just recovered links.
  0.5.2 candidate review reproduced/fixed declared JS/TS classes incorrectly
  capturing bare calls as member helpers, with all eight supported extension
  variants covered. Preserve Java's supported implicit fields separately.
  A baseline-confirmed Java bare method-call helper omission remains deferred;
  this candidate must not claim complete Java method/call resolution.

### P2/P3 — Grounding, coverage and compiler evidence

- [ ] **D3 — Adaptive traversal.** Allocate depth/work by task evidence and fanout;
  diversify handler/state/consumer/test evidence without rewarding redundant
  paths or silently expanding resource limits.
- [ ] **D6 — Event/API adapters.** Add explicitly recognized emission,
  subscription and route-registration forms. Carry event/route identity and AST
  locations; label convention-based associations as heuristics, not verified
  runtime dispatch. Unsupported frameworks remain unsupported.
- [ ] **D7/D8 — Intent and terminology.** Recognize investigation, discovery,
  state-change, API/test failure and dependency tasks. Ground expansions in
  repository strings, keys, routes, tests, docs and symbols; cap synonyms and
  retain the source establishing every bridge. Measure distractor expansion.
- [ ] **D9 — Coverage-aware allocation.** Select complete essential symbols and
  useful supporting spans by marginal role coverage/cost, deduplicate overlap,
  preserve original bytes/pins and report budget omissions. Verify the exact
  final rendered budget, including manifests; an approximate selector is not
  a proof of globally optimal context.
- [ ] **D10 — Optional SCIP adapter.** Validate producer/schema, source/config
  identity, paths/encodings and resource limits. Separate compiler evidence from
  syntax-bound and heuristic links; reject stale inputs and never run compilers,
  hooks, servers or downloads implicitly.
- [ ] **UX — Explain missing evidence.** Surface supported/ambiguous/unresolved/
  omitted roles and weakest-edge evidence consistently in CLI, TUI and MCP,
  without duplicating the core selection pipeline.
- [ ] **Trust/demo.** Record a reproducible repository-dump comparison at equal
  useful-context cost and improve signed provenance/install acceptance. Keep
  demonstration claims within measured results.

## Continuous MCP dogfooding

- [ ] During development, follow [the MCP-first procedure](docs/MCP_DOGFOODING.md).
  Add reproducible findings and explicit outcomes there; prioritize fixes here.
  Measure context reduction and coverage together rather than optimizing for
  small packets that miss the implementation. This is an in-task workflow, not
  an unattended scheduled monitor.
