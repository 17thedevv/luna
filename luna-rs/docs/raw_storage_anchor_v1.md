# RAW-STORAGE-ANCHOR-v1 — Raw Storage Anchor Contract

**Status:** FROZEN — 2026-09-26.
**Related gaps:** SEM-GAP-21 (RESOLVED & FROZEN); C-GAP-12 (RESOLVED & FROZEN).

## Contract

A struct may declare that one or more direct raw-pointer fields are anchored to
the owning `self` value:

```luna
struct RawOwner {
    data: *rw u8,
} requires anchor(data) = self;
```

For multiple fields, the canonical spelling uses one `requires` and a
comma-separated list; each entry declares the same independent field anchor:

```luna
struct RawPair {
    first: *rw u8,
    second: *rw u16,
} requires anchor(first) = self, anchor(second) = self;
```

This spelling changes no anchor semantics or artifact metadata. Existing
repeated `requires` groups remain accepted.

The comma-list parser extension was verified on 2026-10-01: the complete
`luna-parser` suite passed 32 tests (exit 0), including multi-anchor, mixed
anchor/lifetime and malformed-list cases. The public CLI regression
`struct_contract_cli_parity` passed (exit 0): a generic user-defined two-field
owner compiles and executes with source-only and fresh `.llib/.obj`-only
providers; wrong-owner use of either field, duplicate anchors, a non-pointer
anchor and an unknown field all reject through both `check` and `build` with
matching semantic diagnostics. This scoped verification does not claim a new
full-workspace run.

Compiler Change
    Capability: comma-separated struct contract entries after one requires.
    Why stdlib exposed it: a multi-field owner repeated the same keyword.
    Why it is generic: the parser handles every struct and both contract kinds.
    User-defined type benefiting: owner_api::Owner<T, U>.
    Tests: struct_syntax_acceptance_tests; struct_contract_cli_parity.
    New intrinsic/lang_item?: NO.
    Stdlib-specific branch?: NO.

This contract is type-level, not a property inferred merely from the address of
the field slot. It states that the pointer's validity is governed by the
logical owner instance's permitted lifetime; it does not claim the allocation
is physically inside the owner's bytes. It applies only to a direct field whose declared type is `*T`
or `*rw T`; nested projections are outside v1.  Canonical identity is the
owner type identity plus the canonical field name.  Field ordinals and
session-local `DeclId`, `SymbolId`, `PlaceId`, and `ValueId` are not portable
artifact identities.

The anchor invariant is separate from place provenance, raw-pointer
provenance, and safe-loan provenance. A pointer stored in an anchored field
does not keep a safe loan alive. A later raw-to-safe conversion requires a
unique compatible anchor owner, a valid explicit lifetime relation (for
example `life_from(self)`), and compatible pointer/owner mutability. The
declared anchor invariant or an explicit unsafe establishment is the evidence
that the pointer value is valid under that logical owner; the independent raw
address-origin fact remains unchanged and may be `Unknown`. An uncontracted
unknown raw pointer may create a temporary local safe loan only inside its
unsafe validity domain; that `UnsafeRawRoot` is not a `FromPlace` origin or
owner anchor and cannot justify a return, aggregate escape, persistent store,
or call without a proven non-escaping effect. Known null sentinels may satisfy
the field's logical owner anchor for an empty state, but carry a distinct
`Null` raw-origin fact and can never be promoted to a safe reference. A
lifetime annotation alone, or an uncontracted unknown raw pointer, cannot
establish an anchor. `*T` may produce only a shared reference. `*rw T` may
produce shared or mutable references only where the owner/receiver grants the
corresponding capability.

For each value, the compiler tracks which field values satisfy the declared
anchor.  Construction and unsafe re-establishment are explicit boundaries;
they do not bypass move, ownership, mutation, borrow, or region checks. A safe
write to an anchored field must prove that its value is compatible with the
same owner; unknown, other-owner, or mixed origins are rejected. Moving an
owner rebases the field's owner-relative anchor
with the value; a raw pointer extracted before that move is not implicitly
rebased. Types with anchor-bearing fields cannot implement `Copy`. User code
may implement `Clone`, but a clone must establish its own valid anchors.

The contract is part of the exported type's canonical semantic interface and
fingerprint. The `.llib` schema version is explicit: artifacts with an older
semantic metadata version are rejected; missing anchor metadata is not
silently defaulted. Decoding reconstructs fresh session-local identities from
stable owner identity and canonical field names.

Interprocedural propagation must also handle raw-pointer fields returned in an
aggregate: summaries are field-wise and use canonical field names plus
parameter-relative origins, never session IDs. Function-level `life_from`
relations remain a distinct safe-loan/lifetime channel; they may not invent
raw origins. An iterator may use itself as the logical anchor only when its
existing `life_from(source_owner)` return contract makes the iterator value
valid for no longer than that source owner; its unsafe construction explicitly
establishes that relation. This does not claim physical ownership of the
allocation or introduce a new owner/lifetime relation.

## Scope and non-goals

V1 supports direct fields only. It adds no public intrinsic and no compiler
branch for a standard-library type. Raw pointers do not become safe references
implicitly, and `life_from(...)` does not manufacture origin evidence. FFI
classification and existing pointer/reference representation rules are
unchanged. RawTable may declare the contract and mark only the minimum unsafe
construction/store boundary required by this contract; its representation is
otherwise unchanged.

## Acceptance gates

Tests cover generic user-defined owner types as well as RawTable: valid
construction, safe and unsafe stores, move/rebase behavior, pointer
copy/move/offset propagation, valid raw-to-safe conversion, shared/mutable
capability, and live-loan invalidation. Negative tests reject unknown origin,
owner B claimed as A, safe incompatible overwrite, anchored `Copy`, wrong
mutability, and branch merges containing unknown or unrelated origins. Source
and fresh `.llib`-only compilation must agree. C-GAP-12's independent
provenance, adversarial, artifact-parity, and full-workspace gates have passed.
The final command `cargo test --workspace -- --test-threads=1` exited 0.
SEM-GAP-21 and C-GAP-12 are **RESOLVED & FROZEN** by maintainer decision.
