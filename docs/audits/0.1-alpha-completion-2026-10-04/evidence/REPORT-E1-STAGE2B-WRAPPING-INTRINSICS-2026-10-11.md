<!-- luna-doc-role: evidence -->

# E1 Stage 2B — explicit wrapping intrinsics + stdlib migration

Branch `e1-checked-integer-arithmetic` (from `39b4cd6f`). Option A implemented.
**Status: mechanism + migration + focused tests green; two gate items remain
(artifact-side reconstruction guard, full workspace run).**

## What is implemented

- **Three APIs** `std::wrapping_add/sub/mul` declared as compiler-owned language
  items in `core/num.ln` (`#[lang("wrapping_add")] export intrinsic fn …<T>(a: T, b: T) -> T;`)
  and registered through the manifest (`luna-driver/src/lang_contracts.rs`,
  provider `num`, contract `wrapping_arithmetic`). Hook identities 16 → 19; the
  docs validator and `docs/core-language-contract.md` are updated.
- **Recognition by resolved lang-item identity** — the semantic layer resolves the
  call to the language item and the MVIR generator emits the existing
  `Instruction::CallIntrinsic { kind: WrappingAdd|Sub|Mul }`. No name matching of
  library symbols, no new MVIR opcode, no wire/format change. A CLI guard proves a
  user function named `wrapping_add` keeps ordinary semantics.
- **Backend lowering** is honest modular arithmetic: plain LLVM `add/sub/mul`
  (no `nsw`/`nuw`, no trap). Arity and integer-only operands are checked;
  unknown intrinsic kinds fail closed with a backend invariant error.
- **Comptime** evaluates the same three kinds with `wrap_to_width` (2^N at the
  target width, signedness respected) and does **not** emit `E4005` for a legal
  modular wrap.
- **Stdlib migration** (wrapping intent only): `core/hash.ln` `mix_u64`,
  `alloc/raw_table.ln` `mix_hash`, `alloc/string.ln` FNV step → `std::wrapping_mul`
  (each file imports `<num>`). Capacity, allocation-size and index arithmetic were
  left checked.

## Evidence

| Check | Result |
|---|---|
| `wrapping_arithmetic_cli` | 3 passed — modular results in source **and** artifact mode; user-named function not reinterpreted; comptime modular without `E4005` |
| Freeze matrix (`e1-stage2b-wrapping.json`) | 46 rows: 42 complete without a trap (the 12 wrapping reducers now pass; the 34 Stage 1 reducers preserved), 4 comptime diagnostics retained |
| HashMap acceptance | 14 passed |
| HashMap storage acceptance | 11 passed |
| HashSet acceptance | 8 passed |
| String acceptance (FNV) | 16 passed |
| docs validator | 0 errors |

Hash outputs are unchanged: `std::wrapping_mul` lowers to the same modular LLVM
multiply that the migrated plain `*` already produced.

## Remaining before Stage 2B can close

1. **Artifact-side intrinsic reconstruction.** The consumer path is verified in
   both sysroot modes, and provider bodies that now call `wrapping_mul` execute
   correctly in the acceptance suites. Still required by the contract: reconstruct
   `CallIntrinsic` when loading artifacts from the compiler-owned
   `__luna_intrinsic_{kind}` name with strict kind/arity validation, fail closed on
   unknown values, and refuse a user symbol that forges the reserved prefix.
2. **Full workspace run** in the gate's parallel mode with the migrated stdlib.

Checked lowering for ordinary operators (2C), the E4005 duplicate (Stage 3) and
the compiler 22 → 23 bump (Stage 4) remain out of this stage's scope.
