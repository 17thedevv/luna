# Luna 0.1 — semantic contracts

Detailed rules A–K in [normative rules](../../normative-rules-p0-p1.md) remain
requirements. This chapter connects them; it does not weaken them to match a
compiler bug. Status and evidence are in [conformance](conformance.md).

## Types, initialization and ownership

Types determine permitted values and operations; LLVM layout is a later
representation choice. An initialized non-Copy value is moved when consumed.
A move transfers ownership and its destruction obligation, leaving the source
unavailable until reinitialized. Reads require definite initialization on every
reachable predecessor path. CFG joins conservatively combine possible origins
and initialization states.

A concrete type MUST NOT satisfy both Copy and Drop. A proper subplace under a
Drop-bearing owner MUST NOT be moved out; whole-owner moves remain legal.
Borrowing such a field is legal under the normal loan rules. Overwrite must
destroy the initialized old value and install the new value without a partially
initialized destructor invocation. Cleanup destroys each remaining initialized
owned value exactly once; moved values do not receive an additional drop.

Arrays and slice indexing SHALL enforce bounds instead of exposing an unchecked
out-of-bounds safe operation. Enum matches must be exhaustive; unreachable
patterns receive the appropriate diagnostic. Integer casts must preserve the
value for representable widening, using source signedness. Runtime overflow
policy outside already defined checked APIs remains specification debt; an
observed LLVM wrap is not by itself a language-wide arithmetic contract.

The recovered approved formatting contract defines char as a Unicode scalar,
excluding surrogates and values above U+10FFFF. Invalid known integer-to-char
casts diagnose, invalid dynamic values trap, and char arithmetic/bitwise/shift/
negation operators reject. This approval predates the integration; current
implementation conformance is separate from its exact-commit acceptance.

## References, lifetime relations and raw pointers

A safe reference carries **provenance, region constraints, and capability**.
Shared access and exclusive mutable access are distinct. Conflicting mutation,
move, or invalidation is prohibited while the relevant loan is live. Loans end
according to non-lexical use/escape requirements, not a blanket lexical lifetime.

Lifetime relations describe validity, not memory ownership. `life_from(source)`
binds output provenance; `requires life(a) >= life(b)` constrains validity.
Relations normalize to the REGION-SPEC-01 outlives graph. CFG joins retain all
possible source places; a later mutation cannot discard an inconvenient branch.
Loop-carried references cannot outlive their dynamic referent instances.

The retained LLE-v1 rules may infer only an unambiguous legal provenance or
conservative relation. Explicit contracts override inference by dimension.
Multiple candidates requiring a choice must not be resolved by declaration or
hash iteration order. Elision produces the same canonical semantic contract
that an equivalent explicit annotation would produce, including through `.llib`.

Raw pointers are unmanaged addresses, not implicit safe references and not
implicit lifetime extensions. Dereference, offset operations and raw-to-safe
promotion require their defined unsafe validity boundary. `unsafe` does not
disable moves, drops, loan conflicts, initialization or region validity.

RAW-STORAGE-ANCHOR-v1 binds a direct pointer field to the **logical owner**;
it does not assert that the allocation is physically inside that value. A move
rebases the owner's anchor, not every pointer previously extracted from it.
Unknown/mixed/foreign origins cannot establish a safe owner-bound reference
merely because `life_from` is present. Anchored fields preclude Copy. Metadata
stores canonical owner identity and field names, never session-local ordinals.

## Traits, coherence and monomorphization

At most one applicable impl may exist for a canonical trait/self-type pair.
Overlap is checked over unifiable patterns. An impl must own the trait or the
nominal head of its self type; a nested local generic argument does not make an
external wrapper local. Shared/mutable references remain semantically distinct.

Generic associated projections may be symbolic during generic analysis.
Concrete MVIR/backend input MUST contain no unresolved projection, generic
parameter or inference variable. Impl-, trait-, and method-level substitutions
must produce distinct canonical instances where semantics differ. Method
resolution and diagnostics must be deterministic. Under adopted
[METHOD-RESOLUTION-v1](method-resolution-v1.md), applicable inherent methods
take precedence; multiple distinct applicable trait methods without an inherent
candidate reject as ambiguous. Explicit trait qualification selects that trait.
Local and imported candidates follow the same rule; iteration order and expected
return type cannot silently choose between ambiguous traits.

`dyn Trait` follows the adopted object-safety restrictions: generic trait methods,
associated-type traits and supertrait composition are not silently admitted as
supported dynamic interfaces. Defined unsizing admits only the specified
array/slice and reference/trait-object coercions. Unsized values by value and
owning unsized Box support remain outside the retained baseline.

## FFI

Extern calls follow the target C ABI and the SPEC-HARDENING-02 restrictions.
Direct `&T`/`&rw T` parameters keep synchronous safe-borrow semantics. Direct
raw parameters permit call-scoped pointee checks without creating a safe loan
or proving foreign non-retention. An unbound safe-reference return is rejected.
By-value aggregates transitively carrying data-pointer/reference capabilities
are rejected until a recursive provenance contract is adopted. Function-pointer
values are code pointers; their callable signatures are checked separately.

An unsafe extern call still requires valid ownership and borrow state. Runtime
headers define their C signatures; a library wrapper must not reinterpret an
incompatible allocation layout merely to avoid a copy.

## Async, closures and compile-time evaluation

The already defined async contracts remain part of 0.1 even though this audit
does not establish complete async conformance. A future owns its execution
environment and destruction obligation. A move transfers them. Self-referential
future environments are prohibited; externally originated loans must remain
valid across suspension. Cancellation drops only initialized, live, owned,
unmoved state, cascades into active child futures, and frees the environment.
Completed-state destruction must not repeat normal-completion drops.

A closure borrowing local data must not escape beyond that referent. A moved
capture transfers ownership; it does not change an unsafe address into a valid
safe loan. Callable representation is not the source ownership contract.

`const` and `comptime` retain distinct admission rules but share MVIR evaluation.
Const admission is restricted, deterministic and pure. Comptime permits local
imperative computation under effect containment. Observable I/O, extern calls,
OS interactions and async suspension do not become compile-time fallbacks.
Cycles, invalid arithmetic, resource escapes, leaks and execution limits require
diagnostics. Runtime materialization must not expose compiler VM addresses.
Default step/depth limits and CLI overrides are implementation configuration,
not reasons to accept an invalid computation at runtime.
