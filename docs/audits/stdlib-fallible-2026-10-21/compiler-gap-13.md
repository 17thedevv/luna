<!-- luna-doc-role: evidence -->

# C-GAP-13: qualified associated owner binders

2026-10-21, Asia/Bangkok. Focused generic compiler repair; not a release freeze.

`Storage<u8>::create` resolved its method but substituted the nominal type's
parameter only. An impl has independent binder identities, so the method's
parameter and nested `Outcome<Self, E>` remained rigid. After type checking was
repaired, monomorphization still discarded the impl substitution on qualified
direct calls and callable values because no receiver dispatch supplied an impl.

The repair matches the checked impl self pattern against the instantiated
nominal owner, retains owner substitutions when the method has no own generics,
checks impl bounds on direct calls, and derives the impl declaration when
ordering monomorphization substitutions. Binder spelling/ordinal equality is
not used; specialized self patterns and independent method binders are preserved.

The [user-defined fixtures](../../../tests/luna/language/associated_owner_generic/local.ln)
contain no allocation or stdlib containers. Native source and fresh provider-only
artifact runs pass for nested enum returns, u8/i64 instances, specialized self
patterns, method generics and constructor values called indirectly. Missing
bounds reject E2021; an incompatible owner rejects E2001. The permanent
[CLI matrix](../../../luna-rs/crates/luna-cli/tests/associated_owner_generic_cli.rs)
also isolates fresh source/artifact stdlib roots.

Compiler Change
- Capability: qualified generic associated calls and callable values.
- Why stdlib exposed it: a fallible constructor returns Result<Self, AllocError>.
- Why it is generic: nominal and impl binders are distinct for any user type.
- User-defined types benefiting: Storage, Pair, Restricted and Outcome.
- Tests: public CLI fixtures above; broader regression recorded separately.
- New intrinsic/lang_item?: NO.
- Stdlib-specific branch?: NO.

The allocator work resumes after these focused generic controls. This record
does not establish full workspace or stable qualification.
