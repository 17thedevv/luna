# Luna 0.1-alpha.1 — namespace using amendment

Contract ID: **NAMESPACE-USING-v1**. Design scope requested by the maintainer on
2026-10-03; `using math;` was explicitly selected over C++ `using namespace math;`.
Status: **IMPLEMENTED / CLI SOURCE-ARTIFACT MATRIX PASS** on Windows x86_64 GNU.
Workspace snapshots and final targeted regressions are tracked in the [verification record](../../audits/alpha-modules-2026-10-03/README.md).
This status does not certify other targets or the overall alpha release.
This amendment supersedes only the former prohibition of bare namespace using.
Existing namespace aliases retain their contract. Implementation evidence is
separate from contract authority; this amendment makes no freeze claim.

## Syntax and purpose

```luna
import "geometry";
using geometry as geo;
using geometry;

fn main() -> i32 {
    dec point = Point { x: 10, y: 32 };
    return point.x + point.y - 42;
}
```

The imported provider must explicitly declare an accessible namespace
`geometry` containing the accessible type `Point`. The filename alone cannot
supply that namespace. `geo::Point`, `geometry::Point` and the successfully
resolved unqualified `Point` denote the same declaration.

```ebnf
using_decl = "using", namespace_path, [ "as", identifier ], ";" ;
```

- `using path as alias;` creates a namespace alias, as before.
- `using path;` adds the accessible direct members of that namespace as
  candidates for unqualified lookup in the containing scope.
- Both forms require an existing namespace target. A type, function, variable
  or provider name without a corresponding namespace is not a valid target.
- Alpha admits these declarations at file scope and within an inline module.
  Function/block-local directives and selective `using path::leaf;` are outside
  this amendment. `using namespace`, `export using` and `export import` remain
  rejected.

## Lookup, visibility and identity

1. Resolve each directive's target through existing qualified namespace/alias
   lookup after required providers have been acquired. A directive never loads
   a provider. Bare directives do not resolve each other's target paths, so
   mutually dependent namespace openings cannot bootstrap a missing target.
2. At each lexical scope, search its ordinary declarations, parameters and
   explicit aliases first. If no eligible direct binding exists, inspect that
   scope's opened namespaces. If neither yields a binding, continue outward.
   An inner direct binding or opening therefore takes precedence over an outer
   scope. An inaccessible declaration follows existing visibility diagnostics;
   an opening cannot make it accessible.
3. One distinct accessible candidate resolves normally. Two or more distinct
   candidates reject an unqualified use as ambiguous, with the competing
   qualified paths and related locations. Never choose the first imported,
   declared or iterated provider. Explicit qualification resolves the collision.
   This amendment does not introduce overload resolution.
4. Deduplicate candidates by canonical declaration identity. Opening the same
   namespace twice, or through two aliases, does not create ambiguity. Preserve
   namespace composition when multiple providers contribute to one namespace.
5. Only direct namespace members participate. An exported child module can be
   reached through its member name, but its contents are not recursively opened.
   Directives or aliases inside the target are not re-exported into the caller.
6. Visibility, trait/coherence rules, generic substitutions, lifetimes and macro
   hygiene remain governed by their existing contracts. Accessible macro members
   may use ordinary macro lookup; an opening cannot capture definition-site names
   or grant additional trait implementations.
7. The directive adds lookup candidates rather than copied/redeclared symbols.
   Provider, symbol and semantic identities remain stable. Neither form is a
   public `.llib` export; resolved public signatures and portable body references
   must retain their canonical identities without depending on a consumer using
   the same directive.

## Required acceptance

| Case | Required outcome |
|---|---|
| Existing alias syntax, with no bare using | Existing accept/reject behavior preserved |
| Open imported namespace; use type, function and exported child module | Correct check/build/run and declaration identity |
| Open namespace through an alias; repeat opening | Same binding, no artificial ambiguity |
| Two namespaces contain distinct `Point` declarations | Unqualified use rejects; qualified use succeeds |
| Direct local binding plus an opened name; nested module scope | Lexical precedence above, deterministic under import reorder |
| Private member, missing namespace, leaf target, exported using | Reject for the correct reason, with typed diagnostic and spans |
| Provider acquired under a different configured name | Namespace remains the one declared in provider source |
| Public generic API/body built with a directive | Fresh source/artifact consumers agree; no provider source fallback |
| Exported macros and definition-site references | Existing hygiene preserved; ambiguous lookup rejects |

Implementation owners: lexer/parser admission where applicable, AST,
resolver/symbol lookup, visibility/macro lookup, metadata/portable body identity,
CLI acceptance. No dedicated backend instruction or stdlib-name branch is needed.
Use [the acceptance boundaries](conformance.md) and [the implementation plan](alpha-module-plan.md).
