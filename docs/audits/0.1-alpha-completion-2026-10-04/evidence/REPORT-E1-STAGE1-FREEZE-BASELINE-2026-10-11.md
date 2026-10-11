<!-- luna-doc-role: evidence -->

# E1 Stage 1 — freeze baseline (checked integer arithmetic)

Branch `e1-checked-integer-arithmetic` (from `6017ba8d`). Stage 1 records the
pre-fix behavior of the freeze matrix **before** any compiler change, so Stage 2
has a concrete counterexample for every row.

Reducers: `tests/luna/language/checked_arithmetic/*.ln` (34 fixtures).
Recorder: `docs/tools/record_checked_arithmetic.py`.
Baseline: `e1-stage1-baseline.json`.

Each reducer returns 0 only when the operation currently yields the wrong
wrapped/UB result, so **exit 0 = completed without a trap**; an abnormal exit
after Stage 2 = the checked trap.

## Observed baseline

| Fixture group | Fixtures | Expected after E1 | Baseline observation |
|---|---|---|---|
| signed add/sub/mul (i8/i32/i64) | `signed_add_overflow_i8`, `signed_add_overflow_i32`, `signed_add_overflow_i64`, `signed_sub_underflow_i32`, `signed_mul_overflow_i32` | overflow trap | exit 0 — silent wrap |
| unsigned add/sub/mul (u8/u32/u64) | `unsigned_add_overflow_u8`, `unsigned_add_overflow_u32`, `unsigned_add_overflow_u64`, `unsigned_sub_underflow_u32`, `unsigned_mul_overflow_u32` | overflow trap | exit 0 — silent wrap |
| division / remainder by zero | `signed_div_zero_i32`, `signed_rem_zero_i32`, `unsigned_div_zero_u32`, `unsigned_rem_zero_u32` | division-by-zero trap | exit 0 — **no trap at all** |
| signed `MIN / -1`, `MIN % -1` | `signed_min_div_neg_one_i32`, `signed_min_rem_neg_one_i32` | overflow trap | exit 0 — silent wrap |
| shift count ≥ width | `signed_shl_count_at_width_i32`, `signed_shr_count_at_width_i32`, `unsigned_shl_count_at_width_u32` | shift trap | exit 0 — count silently masked |
| unsigned left shift loses a bit | `unsigned_shl_loses_bit_u32` | overflow trap | exit 0 — silent wrap |
| signed left shift exceeds range | `signed_shl_overflow_i32` | overflow trap | exit 0 — silent wrap |
| unary signed negation of MIN | `signed_neg_min_i32` | overflow trap | exit 0 — silent wrap |
| float (unchanged) | `positive_float_overflow_no_trap` | no trap (inf) | exit 0 |
| in-range positives | `positive_signed_in_range`, `positive_unsigned_in_range`, `positive_division_in_range`, `positive_shifts_in_range`, `positive_shl_negative_operand_in_range`, `positive_shl_signed_boundary`, `positive_negation_in_range` | no trap | exit 0 |
| comptime / const | `comptime_signed_overflow`, `comptime_division_by_zero`, `comptime_shift_out_of_range`, `comptime_unary_neg_min` | `E4005` + span | `E4005` — but **emitted twice** per root |

Totals: 34 rows, 0 traps, 30 completed without a trap, 4 compile-time diagnostics.

## Classification (required before Stage 2)

- **No signedness or promotion surprises.** Every reducer compiled and produced
  exactly the wrapped value the fixture asserts, so checked lowering only has to
  add traps — no type semantics need to change.
- **Constant folding is not currently masking the runtime rows** because the
  reducers use mutable locals (`dec rw`). Explicit folding-sensitive reducers
  (literal-only expressions and optimizer const-fold cases) are added in Stage 2
  so the "no unchecked path" gate covers both constant and runtime operands.
- **Pre-existing duplicate diagnostic:** each comptime failure prints the same
  `E4005` twice for one root cause. This is a diagnostic-duplication observation
  in the D1-FU2 cascade family; Stage 3 must not introduce a new cascade and must
  decide this one (fix or record).

## Stage 1 gate

Every matrix row has a reproducer, its expected post-fix behavior, and a recorded
baseline observation. No compiler change has been made yet.
