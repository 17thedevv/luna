<!-- luna-doc-role: evidence -->

# C-GAP-17: cleanup of projected owned temporaries

The size/index collection fixture observed 158 drops for 160 inserted values.
Two popped owners were read through `.unwrap().id` without a named local.
The independent `projected_temporary.ln` fixture uses a user-defined Owner,
ordinary `make`/`consume` functions, field/tuple projection and Drop; no container
implementation is required. The pre-repair executable returns 1 because the
projected owner is never cleaned up.

MVIR lvalue materialization stored rvalues in temporary storage without enrolling
that storage in cleanup. It now tracks owned storage in the existing lexical
temporary scope. Existing move/drop flags suppress cleanup of transferred fields
and values. This repairs projections without adding a library identity branch.

Conditional initialization then exposed a second generic invariant: an LLVM
stack address created only on the RHS of short-circuit logic did not dominate
its later guarded cleanup. LLVM verification rejected the executable. Static
stack storage now lives in the function entry block; value initialization and
drop remain controlled by their original branches and drop flags. Skipped RHS
expressions do not initialize or drop an owner.

Repeated loop predicates exposed the per-evaluation cleanup requirement as well:
an independent loop creates three owners but the intermediate repair dropped
only one at function exit. If/while predicates now snapshot their boolean result
and clean their evaluation scope before branching or reusing temporary storage.
Early-return paths retain the already emitted cleanup rather than duplicating it.

The permanent CLI regression covers field and tuple projection, partial move,
consuming calls, skipped/evaluated RHS paths, repeated predicates, break and
early return. The collection fixture keeps
the original 160-drop oracle; it was not weakened by rebinding popped values.
Native source/artifact results belong to the accompanying receipt, not this
description alone.

Compiler Change
- Capability: generic owned-temporary cleanup and dominating stack storage.
- Why stdlib exposed it: field projection of a popped non-Copy owner.
- Why it is generic: applies to arbitrary rvalues and stack allocations.
- User-defined type benefiting: Owner in projected_temporary.ln.
- Tests: stdlib_size_index_cli; broader ownership/drop/conditional controls.
- New intrinsic/lang_item?: NO.
- Stdlib-specific branch?: NO.
