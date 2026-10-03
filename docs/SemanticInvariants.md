<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](spec/0.1/README.md).

# Luna 0.1 semantic invariants

Authority: [semantics](spec/0.1/semantics.md) and [retained detailed rules](normative-rules-p0-p1.md).
These are requirements, not claims that every current path already conforms.

- Definite initialization and moves are checked across all reachable CFG paths.
- Shared/exclusive capabilities remain distinct; CFG joins retain every possible origin.
- Copy and Drop are mutually exclusive for concrete types; proper subplace moves under Drop are forbidden.
- Safe indexing enforces bounds; cleanup destroys each initialized owned value once.
- Associated projections and generic/inference state resolve before concrete codegen.
- Lifetime, safe-loan provenance, raw address origins and owner anchors are distinct channels.
- FFI preserves its explicit reference/raw-pointer and recursive aggregate restrictions.
- Macro expansion and generic dispatch do not recognize stdlib container/printing names.
- Canonical artifact identities preserve contracts across source/.llib boundaries.
- Implementation layouts are target details, not language-wide u64/pointer assumptions.

Current counterexamples and unverified areas are listed in [gaps](spec/0.1/gaps.md).
