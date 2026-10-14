# R4 — Diagnostic conformance inventory

Reviewed at base `3f8646c654b99b67bae57ee290b7a7e23d717b40`; regression source
`e4f9fb530d72628c81dfa29d117afeb99cc7664c`. **Full structural audit: PARTIAL;
R5 release precondition, not deferred.** This inventory does not execute tests.

Authority: [diagnostics-v1](../../../diagnostics/diagnostics-v1.md). The
[D1 report](../evidence/REPORT-D1-DIAGNOSTIC-CONFORMANCE-2026-10-08.md) and
comments in its harness use several different DIAG numbers. Preserve that dated
report; the mapping below uses the adopted contract's exact numbers. Its
source/artifact parity controls are useful evidence but are not a replacement
for the contract's event-origin or per-code coverage requirements.

| Invariant | Evidence established in tested scope | Uncertified remainder / R5 acceptance |
| --- | --- | --- |
| DIAG-1 Code Mandatory | Selected parser/resolver/borrowck CLI failures have stable codes; E1 admission cases have typed failures | Enumerate every user-visible error path, including quiet/build/provider/macro/infrastructure paths; prove no reachable untyped error |
| DIAG-2 Primary Span Mandatory for Source Errors | Grammar, expression-span and E1 admission matrices check source locations | Audit every source-error constructor and emission path, including failures from materialized AST; no absent/synthetic fallback replacing the actual source event |
| DIAG-3 Structural Related Spans | DiagnosticLabel and with_related exist; common unit test checks structural candidate labels; ambiguity CLI output has multiple locations | Verify typed related labels at each multi-position producer; rendered arrows alone do not prove structural storage; include conflicting loans, move causes, traits and provider origins |
| DIAG-4 Message Independence | Structured-code unit tests and registry parity exist | Audit categorization and assertions for dependence on message strings; several CLI tests inspect rendered code strings/wording, which cannot certify this whole invariant |
| DIAG-5 Poison Containment | Cast poison reducer emits one root; D1-FU2 suppresses comptime wrapper only with a visible error root | Audit all error-value joins and downstream consumers; prove no success, extra cascade or silently dropped failure after suppression |
| DIAG-6 Strict Determinism | Repeated check stderr and check/build identity agree for selected D1 cases | Expand ordering/dedup cases across phases, macro/provider paths and repeated processes; separate deterministic ordering from wording |
| DIAG-7 Regression Coverage | Registry and selected code/span tests run in final6 | Build an exhaustive DiagnosticCode::ALL-to-test map proving identity and primary span where required for every code; enum/spec parity does not establish per-code behavioral coverage |
| DIAG-8 Phase Ownership | Code owner partitions have unit assertions; E1 repairs Parser/Resolver/TypeChecker/Comptime admission and Infrastructure invariants | Trace all emitters and wrappers; optional in_phase assertions are not evidence that every emission uses owning admission |
| DIAG-9 Event Origin Traceability | Move/borrow and candidate-span controls exist; D2 refines selected primary spans | Prove both invalid event and antecedent cause in structured primary/related labels on all relevant move/loan paths; provider parity and a stale-provider message alone do not discharge this |
| DIAG-10 Stable Code Uniqueness & Parity | DiagnosticCode::ALL registry_parity tests establish documented enum equality, uniqueness and owner ranges in the tested pin | Rerun on exact R5 candidate; compare against adopted stable values and preserve any added-code amendment provenance; no cross-revision immutability claim from one current parity run |

## Concrete evidence and implementation boundary

- [Diagnostic representation and constructors](../../../../luna-rs/crates/luna-common/src/diagnostic.rs): code and span fields remain Option; Diagnostic::error starts without either. source_error attaches both and in_phase checks ownership when called. This is not universal constructor/emission enforcement.
- [D1 CLI reducers](../../../../luna-rs/crates/luna-cli/tests/diagnostic_conformance_cli.rs): selected code/span, repeatability, poison, ambiguity and parity controls. Its comment labels are historical shorthand, not authority for numbering.
- [D2 report](../evidence/REPORT-D2-EXPRESSION-SPANS-2026-10-08.md): expression precision evidence, not a second universal DIAG audit.
- [D1-FU2 report](../evidence/REPORT-D1-FU2-COMPTIME-DIAGNOSTIC-CASCADE-2026-10-09.md) and [harness closure](../evidence/REPORT-R5-HARNESS-01-TEMP-SYSROOT-ISOLATION-2026-10-09.md): cascade handling plus the later visible-root safeguard.
- E1 [debug](../evidence/e1-final5-diagnostic-debug-2026-10-13.json) and [release](../evidence/e1-final5-diagnostic-release-2026-10-13.json) check/build acquisitions, and [E-wave report](../evidence/REPORT-EWAVE-CLOSURE-2026-10-13.md): phase-owned annotation, macro, literal and comptime admission in their tested cases.
- [final6 summary](../evidence/e1-workspace-final6-2026-10-13.json) and its three lossless logs include diagnostic_conformance_cli, expression_span_precision_cli, comptime_diagnostic_cascade_cli and common registry tests. A green workspace does not create missing per-code or producer-path coverage.

R5 must record a producer/constructor → owning phase → code → primary/related
span → positive/negative test → target/mode → evidence map. Any discovered
implementation repair belongs to a separately tracked change and a new tested
pin. D1-FU1 presentation deferral cannot waive DIAG-1..10 structure.
