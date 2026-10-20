<!-- luna-doc-role: evidence -->

# E1 Stage 2B — closure: identity guards and artifact path

Branch `e1-checked-integer-arithmetic` (from `39b4cd6f`; implementation at
`080c5099`). This note closes the two remaining Stage 2B items.

## Artifact reconstruction — what actually happens

The serialized MVIR instruction stream is **not executed** in this build:

- executable definitions live in the portable AST, not in the artifact interface
  (`semantic_tables.rs:117`), and headers/data are reconstructed from
  source/portable AST (`registry.rs:111/118/124`);
- `MlibInstruction` (the type that carries `CallDirect("__luna_intrinsic_…")`)
  is referenced **only inside `luna-llib`** (reader/writer/ir + tests) — the
  driver and backend consume MVIR `Instruction`, not `MlibInstruction`.

Consequences:

1. In artifact mode the intrinsic identity is re-established by the **same
   resolved language-item path** as source mode, so parity holds by construction
   (verified: the wrapping matrix passes against a prebuilt/artifact sysroot with
   no reconstruction code at all).
2. There is **no path** where an unrecognised `__luna_intrinsic_*` `CallDirect`
   reaches the native linker, because that instruction stream is never executed.
   Nothing is "smuggled into CallDirect" at execution time.
3. **Security boundary (explicit).** A bare name in the wire cannot prove the
   authenticity of a deliberately forged artifact, and checksums are not
   signatures. What is guaranteed here is artifact/namespace **consistency**, not
   anti-forgery. If the serialized MVIR stream ever becomes executable,
   reconstruction from the reserved name must be added with strict kind/arity
   validation and fail-closed handling — recorded as a precondition, not a
   current claim.

## Reserved-namespace guards (implemented)

`__luna_intrinsic_*` is compiler-owned; user declarations may not claim it:

- a declared function name using the prefix is rejected (resolver);
- `#[link("__luna_intrinsic_…")]` is rejected, including on the inner function of
  an `extern` declaration (attribute processor).

Both produce a typed `E0005` (`InvalidAnnotation`) diagnostic.

## Tests

| Case | Result |
|---|---|
| `wrapping_apis_produce_modular_results_in_both_modes` (source, artifact, relocated) | PASS |
| `user_defined_same_name_is_not_reinterpreted_as_the_intrinsic` | PASS |
| `user_defined_wrapping_mul_keeps_ordinary_semantics` | PASS |
| `reserved_intrinsic_namespace_declaration_is_rejected` | PASS |
| `link_attribute_cannot_claim_reserved_intrinsic_namespace` | PASS |
| `comptime_wrapping_is_modular_without_overflow_diagnostic` | PASS |
| `wrapping_arithmetic_cli` total | 6 passed |
| Freeze matrix | 46 rows — 42 complete without a trap, 4 comptime diagnostics |
| HashMap / HashMap-storage / HashSet / String acceptance | 14 / 11 / 8 / 16 passed |
| docs validator | 0 errors |

## Workspace

`cargo test --workspace --no-fail-fast` → **225 binaries, 1390 passed / 0 failed /
1 ignored, exit 0** (parallel mode).

## Verdict

Stage 2B: **DONE.** Identity mechanism, migration, guards, negative tests and the
workspace gate all pass.
