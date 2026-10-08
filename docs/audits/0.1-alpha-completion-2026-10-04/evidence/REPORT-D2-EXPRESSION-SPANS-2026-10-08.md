<!-- luna-doc-role: evidence -->

# D2 report — expression span precision

Branch `d2-expression-spans` (from candidate `39004d69`).
Scope: span precision only (not a second DIAG-1..10 pass). Identity is code +
span; wording is not asserted.

## Freeze result

No counterexample: the four targeted expression errors already carry precise,
stable, smallest-useful spans. D2 closes as a **coverage certification**.

| Reducer | Code | Primary span |
|---|---|---|
| invalid deref (`deref_scalar`) | E2002 | the deref base `value` of `*value` |
| invalid negation (`neg_bool`) | E2012 | the invalid `-value` operand (invalid operand type, not feature support) |
| invalid char cast (`char_invalid`) | E2026 | the cast source token `55296`, not the whole statement |
| invalid tuple projection (`tuple_scalar`) | E2003 | the projection base `value` |
| invalid index (`index_scalar`) | E2003 | the index base `value` |

## Invariants verified (`expression_span_precision_cli.rs`)

- correct stable code;
- primary span at the offending token (`file:line:column`);
- **caret width equals the offending token width** — the span is the token, not
  the enclosing statement;
- the rendered diagnostic block is identical between `check` and `build`;
- the rendered diagnostic block is identical between source and fresh-artifact
  modes;
- no executable is published.

The existing `expression_diagnostic_spans_cli` guard remains green.

## Result

- `expression_span_precision_cli`: 1 passed / 0 failed.
- `expression_diagnostic_spans_cli`: 1 passed.
- Full workspace: 216 binaries, 1353 passed / 0 failed / 1 ignored, exit 0.

## Versioning

No bump: test-only, no span propagation change
(compiler22 / metadata9 / MVIR5 / format2).

## Verdict

**D2: CONFORMANT IN TESTED SCOPE** — the four targeted expression errors carry
precise, stable, smallest-useful primary spans in both provider modes.
