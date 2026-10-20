<!-- luna-doc-role: evidence -->

# C2 report — per-impl-header metadata

Branch `c2-per-impl-header-metadata` (from candidate `5939105d`).

## Method

Freeze matrix first (no compiler change). Prove that every checked impl header
keeps its identity, binders, constraints, trait arguments, associated types and
method contracts through `.llib` serialization, decoding and semantic
reconstruction. No counterexample was found, so C2 closes as a **coverage
certification**.

## Reducers (`luna-rs/crates/luna-cli/tests/impl_header_metadata_cli.rs`)

| Group | Check |
|---|---|
| Multiple impl headers | two concrete `Describe` impls (Alpha/Beta) survive with distinct `identity`, `self_type` and method symbol id |
| Generic impl | `impl<T: Describe> Pair<T>` impl-level constraint survives |
| Trait impl args | `Ranked<i32>` / `Ranked<bool>` on `Pair` are not merged (distinct trait args/identity) |
| Method identity | no collision between concrete self types |
| Artifact reconstruction | the consumer builds and runs from source **and** from a relocated artifact-only graph |
| Ambiguity | a type implementing two same-method-name traits rejects with E1008; `check` and `build` agree |

Interface-fingerprint and stale-dependency semantics are already covered by
`interface_constraints_cli` (public bound change -> different fingerprint;
private-body / alpha-rename / reversed -> same fingerprint; a dependency whose
interface moved rejects with a fingerprint mismatch), and by
`test_artifact_metadata_parity`.

## Result

- `impl_header_metadata_cli`: 1 passed / 0 failed.
- `interface_constraints_cli`: 1 passed / 0 failed.
- `test_artifact_metadata_parity`: 9 passed / 0 failed.
- Full workspace: 211 binaries, 1343 passed / 0 failed / 1 ignored, exit 0.

## Versioning

Test-only: the serialized semantic metadata is unchanged, so the protocol stays
**compiler22 / metadata9 / MVIR5 / format2** (no metadata 9 -> 10 bump).

## Unverified domains

- impl headers produced only by third-party providers with exotic generic nesting
  beyond the reducers;
- cross-provider impl ambiguity (two providers contributing the same trait+type).

## Verdict

**C2: CLOSED / CONFORMANT IN TESTED SCOPE** — per-impl-header identity,
constraints, method contracts, relocated artifact resolution and ambiguity
rejection all hold in source and fresh-artifact modes.
