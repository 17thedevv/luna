# R4 — Accepted deferrals and known limitations

These records limit claims; they do not amend normative contracts. Release
remains **BLOCKED / NOT VERIFIED** pending R5.

| Item | Classification / authority | Limit and reopening condition |
| --- | --- | --- |
| C4-FU1 | Maintainer DEFERRED to 0.2, non-blocking precision; decision 98b55952 is resolved exactly in the release ledger | Static call-graph over-approximation can retain an untaken branch's provider. Missing an executed dependency would be a correctness defect and reopen the gate; no exact branch-sensitive selection claim |
| D1-FU1 | Maintainer DEFERRED to 0.2, non-blocking presentation; same formal decision | E3001 may render an internal value name such as %v5.0. Code/span/ownership requirements remain mandatory. Reopen for message-quality work or a requirement forbidding placeholders |
| Standalone comptime statement | UNSUPPORTED execution, fail closed; adopted E1 decision | Statement-position comptime blocks reject E4005 with a primary span. Expression-form comptime remains supported in tested cases; parsing a statement does not establish execution |
| VM layout model | VERIFIED ONLY IN 64-BIT GNU TESTED SCOPE | Generic enum alignment repair and runtime/static/VM padding parity do not certify arbitrary target reflection, non-GNU layouts or other pointer widths |
| C4 withdrawn if-expression probe | UNKNOWN / NEEDS AUDIT, not a deferral of correctness | The C4 report records an unconfirmed branch-value probe withdrawn without a permanent reducer. It is not PASS or proof that all comptime if-expressions are broken. R5 must characterize any advertised behavior with a valid reducer and owning-stage evidence |
| V01-DIAG-01 / complete DIAG-1..10 | PARTIAL; mandatory R5 precondition, not deferred | Complete constructor/emitter enforcement, related labels, origin paths and per-code regression map remain uncertified; see diagnostic inventory |
| Ubuntu GNU normal / ASan | NOT VERIFIED; mandatory advertised-target R5 gates | Windows results and runtime-only CI configuration do not supply the missing native evidence |
| Harness last-process residual | Accepted bounded infrastructure limitation in R5-HARNESS-01 | One zero-byte owned temporary directory may remain for the next orphan sweep; no unbounded growth or shared mutable output is accepted |
| R0-H28..H34 / H12 | Historical DEFERRED_HYGIENE / version-oracle work, non-blocking in the R0-H measuring-instrument verdict | The later R5-HARNESS-01 closes the demonstrated storage leak and parallel-output collision; it does not certify every incidental process/timeout hygiene item. Current versions are 25/9/5/2; historical oracle numbers are not current protocol |
| V01-DESIGN-04 | Remaining formal-grammar/document debt | E2 reconciles adopted productions and characterizes legacy parsing. Parser-only syntax is not adopted language scope |
| Provider APIs / wider domains | Bounded coverage, not blanket certification | 49-provider builds and D3 graphs do not certify every API/generic/input domain, all POSIX stdlib semantics, full compiler sanitization, performance or fuzz coverage |

Exact deferral authority and original reopening conditions are in the
[formal defer section](../MICRO-BACKLOG-2026-10-06.md), preserved by
`98b55952` in the [exact ledger](release-ledger.json).
The short prefix above is descriptive only; the ledger's full 40-character
commit is authoritative.

The [C4 report](../evidence/REPORT-C4-COMPTIME-DEPENDENCY-2026-10-08.md)
preserves the withdrawn probe. The [diagnostic inventory](diagnostic-inventory.md)
and [target matrix](target-matrix.md) define the missing certification.
The [E-wave report](../evidence/REPORT-EWAVE-CLOSURE-2026-10-13.md) records
comptime admission and VM/target limits. The [harness report](../evidence/REPORT-R5-HARNESS-01-TEMP-SYSROOT-ISOLATION-2026-10-09.md)
records the bounded residual.

A2-FU1, A2-FU2, D1-FU2, D4-FU1 and FIND-ASYNC-PROVIDER-01 are closed in their
tested scopes and are not silently left as active blockers from older reports.
No fresh deferral is created by R4. The four retained contracts
NAMESPACE-USING-v1, PROVIDER-CONFIG-v1, MODULE-CONST-STORAGE-v1 and
CALL-ARGUMENTS-v1 remain 0.1 requirements. Their bounded acceptance does not
waive exact-candidate R5 requalification.
