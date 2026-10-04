# Luna 0.1-alpha.1 — implementation plan for using and provider configuration

The maintainer first approved these two bounded designs, then authorized
implementation on 2026-10-03. The earlier compiler-integrity repairs were
committed and pushed as `b330ee3` before feature work started. Contracts:
[NAMESPACE-USING-v1](namespace-using-v1.md) and
[PROVIDER-CONFIG-v1](provider-config-v1.md).
Status: **IMPLEMENTED / CLI SOURCE-ARTIFACT MATRIX PASS** on Windows x86_64 GNU;
workspace snapshots and final targeted regressions tracked in the [verification record](../../audits/alpha-modules-2026-10-03/README.md).

## Execution order

1. Preserve existing repairs and establish the exact implementation baseline.
   Resolve existing correctness blockers under the [repair ledger](../../audits/repair-0.1/README.md).
   These two features do not close S-06, DIAG, backend or target evidence gaps.
2. Implement namespace opening in AST/parser/resolver. Keep aliases separate
   from namespace openings; retain canonical IDs, visibility and macro hygiene.
   Define deterministic candidate lookup and ambiguity diagnostics before broad
   positive examples. Preserve invalid leaf-target and export rejection.
3. Implement typed configuration/binding inputs at the CLI/discovery boundary.
   Add nearest-entry configuration discovery and explicit/disabled selection.
   Reuse the existing provider loader and artifact validator; do not add a
   second source/artifact pipeline or provider-name branches in semantic code.
4. Preserve provider identity when a config key differs from a filename, and
   when imports reach the same provider through more than one route. Validate
   transitive configured discovery and portable dependency references before
   claiming artifact support.
5. Execute each contract's valid/invalid matrix through the CLI. Build artifact
   fixtures freshly, remove provider sources from search roots, and consume in
   another process. Verify cwd independence, import reorder, collisions and
   equivalent `check`/`build`/`run` behavior. Add focused internal invariant tests
   where CLI observation is insufficient.
6. Run relevant existing module/import, macro, provider/artifact and sysroot
   regressions, then the required workspace and native target gates. Record
   SHA, config, compiler/runtime identity, commands, expected/actual exits,
   diagnostics and freshness. Mark supported only within the verified scope.

## Completion boundaries

| ID | Owner | Acceptance | Current state |
|---|---|---|---|
| A1-USING-01 | Parser/AST/resolver | Full namespace-using matrix; aliases preserved; deterministic errors; source/artifact parity | IMPLEMENTED; source/fresh-artifact CLI matrix PASS on tested target |
| A1-CONFIG-01 | CLI/discovery/driver provider identity | Full provider-config matrix; old imports preserved; artifact validation unchanged | IMPLEMENTED; source/fresh-artifact CLI matrix PASS on tested target |

The [release gates](conformance.md) still apply. The requested additions are
explicit exceptions to the feature pause, not permission for a package manager,
new namespace keyword, auto-import, transitive config merging or backend shortcuts.
Update the [contract chapters](README.md), [gap register](gaps.md), status and
public documentation after evidence exists. Do not replace historical logs or
call these features implemented because their grammar/specification was updated.
