# C4 withdrawn final-if spelling: qualification preflight

Exact source pin: `9bda6f572fa5f9904379c08c625c28122fcd0c56`.
This note records source observations, not an executed result or a language amendment.

The historical C4 report withdrew `comptime { if false { A } else { B } }`
after observing zero instead of the inner else expression's value. It shipped no
permanent reducer. The R4 ledger therefore retained this as UNKNOWN / NEEDS AUDIT.

The durable `inputs/if_expression_false.ln` and `if_expression_true.ln` reproduce
the shape and return the materialized constant as the process exit code. Their
name identifies the historical description, not a certified Luna if-expression
surface. They receive a CHARACTERIZED result, never an automatic conformance PASS.

Exact candidate source observations:

- `luna-rs/crates/luna-ast/src/expr.rs` declares `Expr::Match` and `Expr::Comptime`,
  while an if is represented by `Stmt::If` in `luna-ast/src/stmt.rs`.
- `luna-rs/crates/luna-parser/src/stmt.rs:17` dispatches a leading if to
  `parse_if_stmt`. `parse_block_stmt` at line 45 promotes only a final
  `Stmt::Expr { has_semicolon: false }` to the outer block's `tail_expr`. A final
  if remains in the block body.
- `luna-rs/crates/luna-semantic/src/typechecker.rs:3658`
  `infer_stmt_value_type` nevertheless recursively obtains an inner branch value
  type through `Stmt::If`, even when the outer block has no tail expression.
  The `Expr::Comptime` case at line 6147 uses that inference.
- `luna-rs/crates/luna-mvir/src/generator.rs:829`
  `generate_stmt_as_function` obtains its result from `generate_block_expr`.

These observations identify a possible admission/type versus materialization
disagreement to characterize. They do not themselves establish a normative
value-producing if-expression contract. An observed zero must remain explicit,
and must not be counted as successful selected-branch value evaluation.

Strict qualification uses canonical reducers with explicit value transport:

- `if_statement_control.ln`: statement branches assign a local; the local is the
  outer comptime block tail expression.
- `branch_function_values.ln`: each selected path returns the function result;
  the call is the outer comptime block tail expression.
- `match_expression_values.ln`: canonical value-producing match expressions,
  with ordinary Luna `->` arms.

Dependency correctness is independent: an executed provider body change must
invalidate a materialized dependent. Rejection after changing an untaken but
statically reachable provider is the already accepted C4-FU1 over-approximation,
not evidence that the untaken branch executed.
