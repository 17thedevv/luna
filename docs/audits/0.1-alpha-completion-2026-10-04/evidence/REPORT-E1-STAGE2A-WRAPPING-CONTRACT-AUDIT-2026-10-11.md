<!-- luna-doc-role: evidence -->

# E1 Stage 2A — wrapping intrinsic contract + stdlib audit

Branch `e1-checked-integer-arithmetic` (from `72787694`, Stage 1 frozen). Option A
approved: explicit `std::wrapping_add/sub/mul`, no function-wide unchecked
attribute, no hashing rewrite. This stage freezes the contract, audits the stdlib
and extends the freeze matrix — **no compiler change yet**.

## Intrinsic contract

| Operation | Semantics |
|---|---|
| `a + b`, `a - b`, `a * b` | checked, trap on overflow |
| `std::wrapping_add(a, b)` | modular addition |
| `std::wrapping_sub(a, b)` | modular subtraction |
| `std::wrapping_mul(a, b)` | modular multiplication |
| `/`, `%` | checked |
| `<<`, `>>` | checked shift contract |

`wrapping_*` accept integer operands only, operate at the target type's width for
both signed and unsigned, and are defined modulo 2^N with the bit pattern
interpreted by the target signedness. A legal modular wrap must **not** produce
`E4005`, in the runtime or in comptime.

### Recognition, lowering, parity

- **Recognition** is by compiler-owned language-item identity
  (`LanguageItemPath { lang_item, canonical_path }` in
  `luna-driver/src/lang_contracts.rs`), resolved after provider acquisition — not
  by comparing library function names. The semantic layer emits
  `Instruction::CallIntrinsic { kind, args }` at the resolved call site.
- **Lowering** maps `WrappingAdd/Sub/Mul` to unchecked modular LLVM integer ops
  (no trap). Float/pointer operands are rejected at resolution.
- **Comptime** implements the same modular semantics for the same kinds in the
  MVIR interpreter.
- **Parity.** The `.llib` writer already encodes the kind in a compiler-owned
  callee name (`writer.rs:296-298`: `__luna_intrinsic_{kind}`) and the reader
  decodes it as a plain `CallDirect` (`reader.rs:915`). Artifact-mode identity can
  therefore be **reconstructed** from that reserved prefix without changing the
  wire encoding, and must be proven by a source-vs-fresh-artifact parity test.
- **Wire-contract tripwire.** If reconstruction turns out to be insufficient
  (kind lost or ambiguous), the task stops for a wire-contract review. Intrinsics
  are never smuggled through `CallDirect` under a library name, and no opcode is
  added without versioning.
- **Docs count.** The docs validator asserts 16 `LanguageItemPath` entries and
  `docs/core-language-contract.md` says "16 manifest hook identities"; adding the
  three wrapping items makes that 19 and both must be updated in Stage 2B.

## Stdlib audit

| Site | Arithmetic | Class | Action |
|---|---|---|---|
| `core/hash.ln:7,8` `mix_u64` | u64 `*` twice | intentional modular | → `wrapping_mul` |
| `alloc/raw_table.ln:94,95` `mix_hash` | u64 `*` twice | intentional modular | → `wrapping_mul` |
| `alloc/string.ln:324` FNV step | u64 `*` | intentional modular | → `wrapping_mul` |
| `alloc/raw_table.ln:72,491,534,585` `cap * 8` | allocation size | must fail on overflow | keep checked |
| `alloc/raw_table.ln` `grow()` capacity | capacity growth | must fail on overflow | keep checked |
| `alloc/vecdeque.ln`, `alloc/vec.ln` size math | allocation size | must fail on overflow | keep checked |
| `core/num.ln` `checked_*` / `saturating_*` | pre-checked, then compute | already safe | unchanged |
| compiler-generated MVIR arithmetic (`generator.rs:675/681` index/element math, `2248` loop increment) | internal | classify per operation in Stage 2C | index/offset must not silently wrap; loop increments are ordinary integer arithmetic |

Hash algorithms and hash outputs must not change; regressions must cover hash
determinism and HashMap/HashSet insert/find/remove in source and artifact modes.

## Wrapping freeze matrix (baseline)

12 new reducers in `tests/luna/language/checked_arithmetic/`
(`wrapping_*`), covering u8/u16/u32/u64 and i8/i32/i64, `MAX`, zero and
signed-negative wrap. Baseline via `record_checked_arithmetic.py`
(`e1-stage2a-wrapping-baseline.json`):

- all 12 **COMPILE_ERROR** — `Symbol 'std::wrapping_*' not found` (the API does
  not exist yet): the expected Stage 2A baseline;
- the original 34 reducers are unchanged (30 silent-wrap, 4 comptime `E4005`).

Aggregate: 46 rows, 0 traps, 30 completed without a trap, 16 compile failures.

## Gate

Stage 2A = contract frozen + audit complete + matrix extended. No wire/format
change. Stage 2B implements the APIs and migrates the wrapping sites; Stage 2C
lands the checked lowering.
