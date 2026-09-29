# SEM-GAP-21: Type-Level Raw Storage Anchor Contract

**Status:** RESOLVED & FROZEN — 2026-09-26.
**Canonical contract:** RAW-STORAGE-ANCHOR-v1 in `docs/raw_storage_anchor_v1.md`.
**Registry:** SEM-GAP-21 is already allocated to this capability; no new gap ID is introduced.

## Defect

Luna distinguishes place provenance, raw-pointer origin, and safe-loan
provenance, but a generic struct contract cannot currently state that a direct
raw-pointer field is anchored to its owning `self`. Consequently a helper that
loads such a field from an abstract receiver cannot validate a raw-to-safe
conversion from the type's public semantic contract. The compiler must remain
conservative: the field-slot address is not evidence about the pointer value
stored in that slot.

The concrete consumer is `RawTable::get_or_insert` in
`libs/external/alloc/raw_table.ln`. C-GAP-12's earlier source/artifact
divergence is fixed, but after removal of an unsound field-address fallback,
the valid RawTable conversion reports `E3005 LocalBorrowEscape`. This is the
motivation for a type-level contract, not authority to weaken E3005.

## Required separation

The contract must be attached to the canonical owner type and direct canonical
field names. It must not serialize session-local IDs or infer pointer-value
origin from the field's storage place. Per-value anchor facts are independent
of raw origin and safe loans. Unknown, mixed, or unrelated stores cannot prove
the anchor; ordinary data dependencies do not create it. Raw-to-safe conversion
requires sufficient provenance evidence: either a compatible known raw origin
or a compatible declared owner anchor, together with an explicit lifetime
relation and compatible mutability. A declared anchor does not rewrite an
independently `Unknown` raw-origin fact. Unsafe establishment is explicit and
still obeys ownership, moves, mutation, borrow, and region rules.

Artifact metadata must include this type-level contract in its canonical
interface/fingerprint and reject unsupported older semantic metadata versions
explicitly. Decoding reconstructs session-local type/field associations from
stable owner identity and canonical field names.

For stripped non-generic helper bodies, the artifact also carries the separate
body-derived raw-pointer return channel (parameter-relative origin and
canonical direct-field names). This is not a safe-borrow/lifetime contract and
does not serialize `DeclId`, `ValueId`, or `PlaceId`. Semantic metadata schema
version is 3; the outer `.llib` container version remains 2.

## Implementation and closure evidence

Track implementation details and test results here only after verifying them
against source. Required evidence includes generic user-defined positive and
negative fixtures, RawTable acceptance, and source versus freshly rebuilt
artifact-only parity. SEM-GAP-21 is not closed by merely making RawTable pass.

**Closure:** ANCHORED-OWNER PATH RESOLVED & FROZEN. The `UnsafeRawRoot` path
permits local transient safe loans without upgrading unknown raw origin to an
owner anchor and rejects escaping
unknown-root references. Generic `Vec<T>` and `Box<T>` contracts anchor their
owned allocations to `self`; the built-in fat-slice data pointer preserves the
origin of its safe-reference input without treating ordinary pointer-field
slot addresses as pointer-value provenance. C-GAP-12 focused, negative,
source/artifact, collection, iterator, FFI, and full-workspace gates pass. The
final `cargo test --workspace -- --test-threads=1` run exited 0. See
`docs/compiler_gap_c_gap_12_safe_loan_effects.md` for the evidence summary.
