<!-- luna-doc-role: evidence -->

# C3 report — associated projection / fallback / proof depth

Branch `c3-projection-fallback` (from candidate `22c33366`).

## Freeze result — two counterexamples

1. **Proof-depth guard leaks into the language.**
   `deep_bound_70.ln` — a valid chain of 70 blanket impls — was rejected with
   `E2021 The type \`Leaf\` does not implement trait \`L70\`` while the same graph
   at depth 60 passed. Root cause: `trait_bound_proof::prove_trait_goal` returned
   `false` at a fixed `stack.len() >= 64` bound, presenting an internal search
   limit as a false "unsatisfied". **Layer:** constraint / trait-obligation
   solving. **Fix:** a named termination guard `MAX_PROOF_DEPTH = 256`; the exact
   cycle check (`stack.contains`) is unchanged, and a genuinely cyclic blanket
   bound (`cyclic_bound.ln`) still rejects (E2021) — the guard remains a
   termination bound, not a language limit.

2. **Chained (nested) projection not resolved.**
   `nested_projection_concrete.ln` — `proj3::Top::Middle::Leaf` — was rejected
   with `E1001`. Root cause: the type-path prefix was resolved only through the
   symbol table, so a type segment preceding the final associated name aborted the
   walk. **Layer:** type-path resolution / projection normalization. **Fix:**
   resolve the prefix left to right — descend module segments, stop at a type (or
   type parameter), then project the remaining segments.

## Structured diagnostics confirmed

| Case | Diagnostic |
|---|---|
| Cyclic projection | `E2008 E_ASSOC_TYPE_CYCLE` |
| Ambiguous projection | `E2019 E_AMBIGUOUS_ASSOCIATED_TYPE` (one diagnostic, no cascade) |
| Genuinely cyclic bound | `E2021` (termination intact) |
| Associated projection (single level) | resolves and runs |

## Unverified / unsupported domain

A **generic** chained projection `T::Middle::Leaf` needs an associated-type bound
(`type Assoc: Trait`), which the grammar does not currently accept; it fails
closed with a structured diagnostic. This is recorded, not silently accepted.

## Result

- `projection_fallback_cli`: 1 passed / 0 failed (source + fresh artifact).
- `trait_argument_bounds_cli`, `interface_constraints_cli`: 1 passed each.
- `cargo test -p luna-semantic`: 0 failed.
- Full workspace: 212 binaries, 1344 passed / 0 failed / 1 ignored, exit 0.

## Versioning

No bump: both changes only widen the accepted program set; no previously valid
program changes codegen and no artifact schema changed (compiler22 / metadata9 /
MVIR5 / format2).

## Verdict

**C3: CONFORMANT IN TESTED SCOPE** — deep valid bound graphs are no longer
blocked, chained concrete projections resolve, and projection/cycle/ambiguity
failures stay structured and deterministic.
