# METHOD-RESOLUTION-v1 — inherent priority and trait ambiguity

Adopted by the maintainer on 2026-10-04 during the 0.1 completion work:
Option A, applicable inherent methods first; ambiguous applicable trait methods
reject; explicit trait qualification disambiguates. Implementation/evidence are
separate; adoption does not declare the current resolver conformant.

## Candidate selection

An unqualified method call SHALL consider canonical candidates with a compatible
self type/receiver, satisfied generic bounds and access from the calling scope.
Ordinary type, visibility, ownership and lifetime checks remain requirements.
An inaccessible or inapplicable candidate does not win merely by sharing a name.

1. Applicable inherent candidates take precedence over trait candidates.
2. In the absence of an applicable inherent candidate, exactly one applicable
   trait candidate is selected.
3. Multiple distinct applicable trait candidates SHALL produce a typed
   ambiguity diagnostic (`E1008`), including the canonical candidate names and
   available related declaration spans. Repeated routes to the same declaration
   do not create ambiguity.
4. No candidate produces the ordinary appropriate rejection, such as a missing
   method, unsatisfied bound or access error; it must not become a successful
   call through an unrelated candidate.

Local source impls and imported metadata impls obey the same precedence.
Candidates SHALL NOT be selected by import order, declaration order, hash-table
iteration or numeric session-local symbol identity. An expected return type
SHALL NOT silently select between otherwise ambiguous trait methods. The chosen
method's return type is checked against the expected type normally.

Explicit `TraitName::method(receiver, ...)` selects that trait contract and
resolves its implementation under ordinary trait/generic rules. It is not
shadowed by a same-named inherent method. This contract adds no keyword or
alternate receiver syntax.

## Generic binder identity

Adopted by the maintainer on 2026-10-05: impl and method generic parameters
are independent binders. Repeating a spelling in the method declaration does
not constrain or alias the impl parameter. Substitution follows declaration
identity and parameter position within that declaration, never name matching
between unrelated binders.

Element requirements belong on the corresponding impl, for example
`impl<T: std::Eq> Vec<T> { fn dedup(self: &rw Self) ... }`. A method may declare
its own generic arguments independently, including a parameter that shadows an
impl parameter. An omitted method type argument must be inferred under ordinary
typing rules or rejected; a same-named impl parameter is not an inference source.
Generic caller parameters must establish required bounds from their contracts;
being symbolic does not automatically satisfy a bound.

The selected impl header, binder identities and method declaration must survive
provider reconstruction and monomorphization. Grouping candidates by nominal
head must not replace individual checked headers. Later stages must preserve
semantic selection rather than choosing another candidate by result type.

## Acceptance boundary

Require inherent/trait, two-trait ambiguity, explicit qualification, receiver
mutability, generic bounds, private/inapplicable controls and local/imported
combinations through the CLI. Reverse declarations/imports; verify source-only
and fresh-artifact-only modes. Definition/impl identities must survive portable
metadata. Update the gap register and current status only after these checks.
