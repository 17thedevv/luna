# Luna 0.1 — conformance and release gates

Contract status and implementation status are separate. An adopted rule remains
a requirement while its implementation can be partial, broken or unverified.
This specification refresh is not a compiler repair or a release certification.

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

## Release 0.1 gate

Before declaring the release conformant, address confirmed compiler/library and
architecture gaps; resolve normative disagreements without silent syntax
changes; regenerate sysroot artifacts through the official path; run required
focused and relevant workspace regressions; verify diagnostics and parity; and
publish the exact supported target matrix and any accepted deferrals.

Current readiness is **BLOCKED** by [gaps.md](gaps.md). The latest audit built
all providers and completed substantial checks, but did not complete all compiler
tests, sanitizer/fuzzing, all targets or all generic/data domains. Historical
green test counts do not supersede a later counterexample.


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
