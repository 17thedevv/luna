<!-- luna-doc-role: evidence -->

# A6 report — unary logical NOT

Branch `a6-unary-logical-not` (from candidate `54df3f43`).

## Freeze result

`!x` was rejected for every operand with:

```
error[E2012]: Unary operator `Not` is not yet supported
```

The parser already produced `UnaryOp::Not`; the gap was downstream, so A6 is an
**implementation fix** (not a coverage certification).

## Layer localization

| Layer | State |
|---|---|
| Lexer / parser | already correct (`!` -> `UnaryOp::Not`) |
| TypeChecker | **fix** — `Not` was grouped with the unsupported `BitNot`/`PostInc`/`PostDec` |
| MVIR generator | **fix** — no `Not` lowering existed |
| Interpreter / LLVM backend | unchanged — `Eq` already handles bool |

## Fix (minimal, at the real boundary)

- TypeChecker: `UnaryOp::Not` on `bool` -> `bool`; on any other type -> a single
  E2012 diagnostic and an `error`-typed (poisoned) result.
- MVIR generator: lower `!x` to `Eq { left: x, right: false }`. This reuses an
  existing instruction, so no MVIR instruction/schema change and therefore **no
  protocol bump** — and no pre-existing artifact could contain `!` (it was
  rejected before).

Bitwise NOT on integers is deliberately **not** added: the contract defines
logical NOT only.

## Reducers (`luna-rs/crates/luna-cli/tests/unary_not_cli.rs`)

Positives (build + run, exit 0): `!true`, `!false`, `!!value`, `!(a && b)`,
`!` in a condition, `!` in a generic function, `!` in comptime.

Negatives: `!` on `i32` -> E2012; a poisoned `!`-result feeding many downstream
uses -> exactly **one** E2012 (no cascade) and no published executable.

## Result

- `unary_not_cli`: 1 passed / 0 failed (source + fresh artifact).
- `cargo test -p luna-semantic`: 0 failed.
- `expression_diagnostic_spans_cli`: 1 passed (existing `neg_bool` E2012 intact).
- Full workspace: 210 binaries, 1342 passed / 0 failed / 1 ignored, exit 0.

## Verdict

**A6: CLOSED / CONFORMANT IN TESTED SCOPE** — bool-only logical NOT with typed
rejection, poison containment, comptime/runtime parity and source/artifact parity;
protocol unchanged.
