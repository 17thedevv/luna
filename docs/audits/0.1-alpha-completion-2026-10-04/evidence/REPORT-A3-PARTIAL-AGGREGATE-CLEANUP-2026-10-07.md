# A3 report — partial aggregate cleanup

Branch `a3-partial-aggregate-cleanup` (from candidate `47f402ea`).

## Counterexample (freeze result)

`tests/luna/language/generic_drop/indexed_move.ln`: moving one array element
out (`resources::consume(arr[0])`) dropped that element **again** at scope exit.

| place/subplace | initial owner | transition | expected cleanup | actual |
|---|---|---|---|---|
| `arr[0]` | `arr` element 0 | moved by `consume(arr[0])` | drop `arr[1]` only | drop `arr[0]` **and** `arr[1]` (double-drop of `arr[0]`) |

Observed exit code 3 (consume 1 + `arr[0]` again 1 + `arr[1]` 1) instead of 2, in
**both source and fresh-artifact** modes.

## First mismatch

Not move analysis itself: constant array indexing lowered to
`Instruction::PtrOffset` with a **materialized-constant** offset operand, and move
analysis only propagates a place through a `Load` whose pointer is an
`Alloca | HeapAlloc | FieldPtr` value (and the offset was opaque). The element
pointer therefore had no tracked place, the moved element was never marked moved,
and caller-side array cleanup re-dropped it.

Contributing layer: `emit_place_cleanup` did not descend arrays (it emitted a
single whole-array `Drop` via the array drop glue), unlike structs/tuples, so no
per-element place existed to eliminate.

## Fix (generic)

1. `emit_place_cleanup` descends arrays per element (`FieldPtr` with a constant
   index) so a moved element is a distinct, eliminable place. The whole-array drop
   glue is unchanged and still used for dropping an entire array value.
2. A constant array index is emitted as a **literal** `PtrOffset` offset (rather
   than an opaque value) so move analysis can recognize it.
3. Move analysis maps a constant `PtrOffset` on an **array** base to the element
   subplace (`Field(idx)`) and lets `Load` inherit a `PtrOffset` place, so the
   moved element is marked moved and its cleanup is eliminated.
4. Indexing stays a raw pointer offset, so **raw-slice provenance is unaffected**.
   A dynamic index on a non-array base produces no place; no partial-move support
   is claimed for it (fail-closed / separate).

An earlier variant that lowered constant indexing to `FieldPtr` was rejected
because it lost raw-slice provenance (`slice_middle_conflict` stopped reporting
E3003); that regression is now guarded by `raw_slice_provenance_cli`.

## Reducers (source + fresh artifact, all PASS)

- `enum_partial_move` — move one payload field out of a variant; remaining payload
  dropped once; other variant untouched.
- `indexed_move`, `indexed_move_index1`, `indexed_move_middle` — constant-index
  element move; remaining elements dropped exactly once (index 0 / 1 / middle of 3).
- `projected_partial`, `projected_call_arg` — struct field move (let-binding and
  call-argument forms); remaining field dropped once.
- `path_dependent_partial` — conditional move merged at CFG join, then scope exit.
- `nested_partial` — move `outer.value.first`; cleanup descends and drops
  `inner.second` once.

## Versions / scope

Compiler protocol **21 → 22** (codegen place representation for array element
access changed). Metadata 9 / MVIR 5 / format 2 unchanged.

Focused `generic_drop_cli`: 21 positives + 12 negatives PASS in source and
artifact. Full workspace: see the JSON evidence.

## Separate finding (not A3)

`FIND-ASYNC-PROVIDER-01`: a provider exporting a **parameterless** `async fn`
produced `Invalid semantic metadata: CorruptedData` when its `.llib` was imported.
Minimal reproducer: `tests/luna/language/async_provider_export/` (`provider.ln`
with `export async fn helper() -> i32`, imported by `consumer.ln`). Filed as its
own backlog item; not used to conclude artifact parity of any other feature.
