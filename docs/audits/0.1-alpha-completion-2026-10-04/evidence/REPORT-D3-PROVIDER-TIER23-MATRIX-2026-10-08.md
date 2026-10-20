<!-- luna-doc-role: evidence -->

# D3 report — provider Tier-2 / Tier-3 matrix

Branch `d3-provider-tier23-matrix` (from candidate `35260b6b`).

## Method

Matrix certification first (no provider-boundary change). Lock the four fragile
boundaries: provider public interface -> serialized metadata -> dependency
reconstruction -> consumer semantic resolution.

## Tier-3 graph — `consumer -> a -> b -> c`

`provider_tiers_cli` builds and runs the consumer in every mode:

| Mode | Layout |
|---|---|
| all-source | `a.ln`, `b.ln`, `c.ln` |
| a-artifact | `a.llib`, `b.ln`, `c.ln` |
| ab-artifact | `a.llib`, `b.llib`, `c.ln` |
| all-artifact | `a.llib`, `b.llib`, `c.llib` |
| relocated | all-artifact graph moved to a new directory |

All five resolve and run to exit 0.

## Tier-2 feature surface (source and artifact)

`Box<T>` (generic type), `identity<T>` (generic fn), `Describe` + `impl`
(trait/impl), `STAMP` (comptime const) and a **private helper reached from a
public fn** all survive the artifact boundary: the consumer resolves through the
`.llib` and `use_helper()` still materializes the private body, while the private
symbol is not part of the public metadata surface.

## Fail-closed / stale

- missing dependency -> E1004, no source fallback, no published executable;
- stale interface/execution/version classes are covered by
  `interface_constraints_cli`, `comptime_dependency_precision_cli` and
  `fresh_artifact_classification_cli`;
- duplicate provider identity surfaces are covered by `alpha_modules_cli` /
  `provider_module_contract_acceptance_tests`.

## Async probe (excluded)

`FIND-ASYNC-PROVIDER-01` reproduces: exporting a **zero-parameter** `async fn`
from a provider and importing its `.llib` rejects with
`Invalid semantic metadata: CorruptedData`. The probe prints
`D3 external finding reproduced: FIND-ASYNC-PROVIDER-01`. Because the root cause
is a deeper serialization/decode issue rather than a Tier-2/3 provider-boundary
defect, it is **recorded and excluded** from D3 and remains open.

## Result

- `provider_tiers_cli`: 4 passed / 0 failed.
- `interface_constraints_cli`, `artifact_execution_dependencies_cli`,
  `alpha_modules_cli`: 1 passed each.
- Full workspace: 217 binaries, 1357 passed / 0 failed / 1 ignored, exit 0.

## Versioning

No bump: test-only, no provider serialization change
(compiler22 / metadata9 / MVIR5 / format2).

## Verdict

**D3: CONFORMANT IN TESTED SCOPE** — Tier-2/3 graphs (mixed, all-artifact,
relocated), the generic/trait/comptime/private surface, and fail-closed missing
dependencies all hold, with `FIND-ASYNC-PROVIDER-01` explicitly excluded and open.
