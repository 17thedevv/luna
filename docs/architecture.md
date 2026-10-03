<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](spec/0.1/README.md).

# Luna 0.1 architecture

The [versioned specification](spec/0.1/README.md) defines contracts; code and tests
establish conformance. The canonical implementation is the 12-crate Rust
workspace under luna-rs, with a C runtime and component stdlib.

## Ownership of phases

AST belongs to the frontend; semantic identities, contracts and generic instances
belong to the middle end; MVIR belongs to lowering/optimization; LLVM representation
belongs to the backend. Every phase communicates through owned, stable interfaces.
Do not turn an implementation detail from a later phase into source semantics.

The driver orchestrates parsing, provider bootstrap/import, attributes/macros,
resolution/type analysis, concrete instances, MVIR analyses/ownership cleanup,
optimization, async/backend lowering and linking. Detailed ordering is compiler
implementation evidence, not a second language contract. The current optimizer
passes do not justify a blanket LLVM inlining/vectorization promise.

## Library, runtime and build tooling

Stdlib consumes generic traits, moves, drops, borrows, lifetime relations and
memory primitives. Container or printing-macro names/layouts cannot select
special compiler semantics. Runtime owns platform primitives and the documented
C ABI. Compiler owns entry lowering. Package/build tooling owns acquisition,
rebuild and caching policy; compiler validates or rejects selected artifacts.

Provider identity, physical path and module namespace remain distinct.
Portable .llib identity/metadata preserve canonical contracts without session IDs,
source fallback for invalid artifacts or duplicated transitive trait bounds.
See [modules](spec/0.1/modules.md), [macros](spec/0.1/macros.md),
[runtime](spec/0.1/runtime.md), and the [gap register](spec/0.1/gaps.md).
