---
name: luna-lang-contracts
description: Defines the architecture for compiler-required language contracts stored in the Luna stdlib.
---

<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](../../../docs/spec/0.1/README.md).

# Luna Language Contract Architecture

## Purpose

Define the architecture for compiler-required language contracts stored in the Luna stdlib.

Current contracts:

- Drop
- Option
- Iterator
- IntoIterator
- Copy
- Try / FromResidual / ControlFlow
- Explicit wrapping arithmetic (wrapping_add / wrapping_sub / wrapping_mul)

The result provider additionally contributes controlled qualified std::OptionExt visibility; it is not a compiler-contract family. The explicit-import ptr/mem providers carry checked memory-hook identities without becoming broad auto-visible prelude providers.

These contracts are represented by normal Luna source/artifacts but participate in language semantics.

---

## 1. Language Contracts Are Not Compiler-Native Types

The compiler knows the CONTRACT, not an implementation-specific type.

The compiler MUST NOT introduce special semantic behavior such as:

```text
if type == Option
if type == Vec
if type == String
```

Instead it recognizes explicit language-level contracts.

For example:

```text
#[lang("drop")]
#[lang("drop_fn")]
```

identify the existing Drop protocol. Proposed future role markers are not examples of currently supported attributes.

The actual implementation remains Luna code.

## 2. Controlled Auto-Loading

Language contracts are automatically available because language semantics depend on them.

Users should not be required to write:

```text
import <core>;
```

merely to declare:

```text
dec x: std::Option<i32>;
```

or implement:

```text
impl std::Iterator<i32> for MyIterator {
    ...
}
```

This is NOT general implicit prelude behavior.

Only explicitly registered language-contract and controlled-prelude providers are auto-loaded. Bindings remain qualified under std; no unqualified root prelude is implied.

## 3. Contract Manifest

The set of auto-loaded providers MUST be explicit.

Conceptually:

```toml
[language_contracts]
drop = "lang/drop"
option = "lang/option"
iterator = "lang/iterator"
into_iterator = "lang/into_iterator"
copy = "core/copy"
try = "core/try"
wrapping_arithmetic = "core/num"
```

The compiler MUST NOT scan arbitrary files for `#[lang(...)]` and automatically load them.

The manifest defines the authoritative mapping.

## 4. Auto-Visible Bindings

Foundational contracts themselves may be auto-visible.

Examples:

Option
Option::Some
Option::None
Iterator
IntoIterator

This does NOT imply that all declarations from the corresponding provider become visible.

A provider may contain internal implementation details.

The visibility surface is controlled.

## 5. Language Syntax vs Language Contracts

Do not create artificial stdlib files for compiler-native syntax.

For example:

if
match
while
for
break
continue

are language syntax/semantics.

There is no need for:

lang/branch.ln

unless a future explicit semantic contract requires it.

The for expression itself is compiler syntax.

Its iteration protocol is supplied by:

IntoIterator
Iterator
Option

## 6. Current Contract Set

The current foundational set is:

Drop

Used by ownership/drop semantics.

Option

Used by iterator advancement and related language protocols.

Iterator

Defines iterator protocol.

IntoIterator

Defines the conversion from an iterable value into an iterator.

Copy, Try/FromResidual/ControlFlow and explicit wrapping arithmetic are also current families, represented by core/copy, core/try and core/num. Controlled std::OptionExt visibility is supplied by result. Exact bootstrap hook and qualified path lists live in luna-driver/src/lang_contracts.rs; the complete registered key table is in luna-semantic/src/lang_item.rs. Memory intrinsic identities are additionally carried by explicit ptr/mem imports. A key in a registry or a design proposal does not establish end-to-end support.

Do NOT add Clone to lang/ as part of this migration.

Future contracts require their own architectural decision.

## 7. Contract Provider Structure

Each contract should live in its own source component.

Example:

lang/drop.ln
lang/option.ln
lang/iterator.ln
lang/into_iterator.ln

Do not recreate a monolithic:

lang.ln

## 8. Compatibility

Changing the physical location of a language contract MUST NOT change:

its semantic identity
its lang-item identifier
its trait identity
its canonical symbol identity
its lifetime semantics
its runtime ABI

Provider paths are organizational metadata, not semantic identities.

## 9. Bootstrap Safety

The compiler must be able to bootstrap these contracts deterministically.

The load sequence should conceptually be:

language-contract manifest
        ↓
resolve provider
        ↓
load source or valid .llib
        ↓
register required contract
        ↓
expose controlled bindings
        ↓
compile user program

The implementation MUST NOT rely on incidental directory scanning order.

## 10. Validation

The implementation must prove:

language contracts are available without ordinary imports
unrelated core symbols are not automatically visible
contract providers load deterministically
source and .llib representations agree
duplicate contract registration is rejected or handled deterministically
moving contract source files does not alter their semantic identity
