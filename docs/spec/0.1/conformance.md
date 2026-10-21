# Luna 0.1 — conformance and release gates

Contract status and implementation status are separate. An adopted rule remains
a requirement while its implementation can be partial, broken or unverified.
This specification refresh is not a compiler repair or a release certification.

Current status — 2026-10-21: **v0.1.0-alpha.1 is published; stable is not yet qualified**.
The CPU-baseline repair completed eight alpha lanes and both clean package smokes;
see its [release closure](../../audits/r5-cpu-baseline-2026-10-20/release-closure.json).
The [stable execution ledger](../../stable/0.1.0/README.md) defines the new
integration baseline and remaining work. No alpha PASS is transferred to a new
candidate. Strict target identity and adopted language contracts remain mandatory.
Earlier qualification snapshots below retain their dated scopes.

Prior evidence reconciliation — 2026-10-14: see the
[R4 release ledger](../../audits/0.1-alpha-completion-2026-10-04/r4/README.md).
The E-wave qualification at `e4f9fb53`, stored by closure `3f8646c6`, is scoped
Windows GNU evidence. R5 exact-candidate qualification is still PENDING;
release status remains BLOCKED / NOT VERIFIED.

## Status vocabulary

| Status | Meaning |
|---|---|
| ADOPTED CONTRACT | Requirement retained from an approved/frozen rule or this explicitly requested baseline consolidation |
| PROPOSED / DECISION REQUIRED | A contract amendment or unresolved choice; not silently a release requirement |
| CONFORMANT IN TESTED SCOPE | Named positive/negative behavior verified on stated target and source/artifact paths |
| PARTIAL | Some required cases/stages have evidence; the whole contract does not |
| BROKEN | A reproduced defect contradicts a requirement |
| UNVERIFIED | Evidence is absent or could not finish |
| DEFERRED | Explicitly outside the adopted capability scope |

Historical FROZEN labels record previous decisions and dated acceptance, not
blanket current release completion. READY FOR DESIGN/FREEZE REVIEW is not FROZEN.

## Diagnostics

Retain DIAG-1–DIAG-10: typed stable codes, source spans for source errors,
structured related spans, poison containment, deterministic order/deduplication,
phase ownership, origin traceability and registry parity. Rendering wording is
not diagnostic identity. A timeout or stack overflow is not an appropriate
substitute for a source rejection. Registry documentation must distinguish
normative coverage requirements from a claim that every current error path
already satisfies them.

The [R4 diagnostic inventory](../../audits/0.1-alpha-completion-2026-10-04/r4/diagnostic-inventory.md)
uses the exact adopted DIAG-1..10 numbering. The historical D1 table mislabels
several numbers; its green reducers do not establish universal constructor,
emission, event-origin or per-code coverage. The complete structural remainder
is a mandatory R5 precondition, not an accepted deferral.

## Required acceptance boundaries

1. Language and library behavior: real `.ln` fixtures via the public Luna CLI,
   checking exit, stdout/stderr and appropriate rejection.
2. Provider parity: source-only and freshly built artifact-only roots with no
   relevant source fallback. Invalid artifact selection must reject.
3. Compiler invariants: focused Rust tests may inspect semantic/MVIR/backend
   state. They complement, not replace, executable evidence.
4. Ownership: positive cleanup counts plus negative move, borrow, escape and
   initialization cases. A single successful compilation is insufficient.
5. Determinism: repeated compilation for collision/order-sensitive cases,
   including user-defined types independent of the original stdlib trigger.
6. Target claims: runtime ABI, linker and executable evidence per advertised
   target/profile. A Windows pass cannot certify POSIX or freestanding behavior.

## Release 0.1-alpha.1 target matrix

Adopted 2026-10-06 (D3). Supported, release-gated targets:

| Target | Adopted scope | Qualification at the R4 input | Required R5 evidence |
|---|---|---|---|
| `x86_64-pc-windows-gnu` | SUPPORTED / RELEASE-GATED | VERIFIED IN E-WAVE TESTED SCOPE at e4f9fb53 | exact candidate clean native/full regression and CI |
| `x86_64-unknown-linux-gnu` (Ubuntu) | SUPPORTED / RELEASE-GATED | NOT VERIFIED (normal and ASan) | exact candidate normal CI + ASan with instrumented scope recorded |

The [detailed target matrix](../../audits/0.1-alpha-completion-2026-10-04/r4/target-matrix.md)
separates native evidence from job configuration and host-toolchain identity.

NOT ADVERTISED in 0.1-alpha.1: macOS, freestanding and every other target. A
target that currently compiles but is not listed here remains
experimental/unverified and is not release-supported. The host toolchain triple
(for example `x86_64-pc-windows-msvc`) is not a Luna target and MUST NOT be
advertised as one. The maintainer confirms the final advertised matrix before
tagging.

## Release 0.1 gate

Before declaring the release conformant, address confirmed compiler/library and
architecture gaps; resolve normative disagreements without silent syntax
changes; regenerate sysroot artifacts through the official path; run required
focused and relevant workspace regressions; verify diagnostics and parity; and
publish the exact supported target matrix and any accepted deferrals.

Stable readiness is **NOT YET QUALIFIED**; see the
[current stable backlog](../../stable/0.1.0/backlog.json). The historical
[R5 checklist](../../audits/0.1-alpha-completion-2026-10-04/r4/r5-checklist.md)
was completed for the published alpha in the later release closure. Its required
acceptance boundaries remain applicable when requalifying a new candidate.
E-wave completed three Windows workspace regressions and 236/236 arithmetic
acquisitions; this does not complete Ubuntu/ASan, the structural diagnostic
inventory, every generic/data domain, broader sanitizers or fuzzing. Historical
green counts do not supersede a later counterexample.

## Retained 0.1 contract scope

Adopted 2026-10-06 (D4). The following contracts remain part of 0.1-alpha.1 and
are NOT deferred to 0.2:

| Contract | Status |
|---|---|
| NAMESPACE-USING-v1 | KEEP |
| PROVIDER-CONFIG-v1 | KEEP |
| MODULE-CONST-STORAGE-v1 | KEEP |
| CALL-ARGUMENTS-v1 | KEEP |

Each is a release requirement. If any of them is not yet conformant, the release
remains **BLOCKED**; an implementation gap does NOT automatically move a contract
out of 0.1. Removing or deferring any of these requires a new explicit maintainer
decision.

## Documentation refresh validation

Local documentation consistency is checked with `python docs/tools/validate_v01_docs.py`.
It validates current links, authority roles, provider/hook inventories and generated
pages. `python docs/tools/check_v01_examples.py` records CLI examples and known
grammar failures in both source/artifact modes. These two commands intentionally
separate documentation consistency from compiler conformance; known failures are
retained in their evidence, not converted into success expectations.

The CLI recorder defaults to the Windows debug executable and the audited GNU
runtime archive. Other build locations can be selected with `LUNA_DOCS_CLI` and
`LUNA_RUNTIME_LIB`; `LUNA_DOCS_TOOLCHAIN_PATH` optionally prepends toolchain
directories to PATH, and `LUNA_DOCS_TEMP` selects temporary storage. Rebuild
the matching CLI, runtime and sysroot artifacts before collecting new evidence.
Use `LUNA_DOCS_EVIDENCE_OUT` to keep a later run separate from the original
audit evidence; the integration run is stored in `docs/integration/evidence`.
