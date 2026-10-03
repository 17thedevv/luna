<!-- luna-doc-role: historical -->

> **Luna 0.1 — historical.** Historical design, plan, or evidence. Original wording is preserved for context; it does not independently define current syntax, capability scope or release readiness. See the [versioned specification](../spec/0.1/README.md).

# Decision Note: V01-DESIGN-01 Inherent vs Trait Method Resolution and Ambiguity Policy

## 1. Context and Problem Statement
In Luna 0.1, a type may have:
1. Inherent methods defined directly on the type via `impl Type { fn method(&self) ... }`.
2. Trait methods implemented for the type via `impl Trait for Type { fn method(&self) ... }`.
3. Multiple traits implemented for the same type that define methods with identical names (e.g., `Display::fmt` and `Debug::fmt`).

Currently:
- Inherent methods take precedence over trait methods in `typechecker.rs` (search phase 1 checks inherent members, search phase 2 checks trait members).
- When multiple trait methods match, the compiler sorts candidates by `(is_trait, trait_sym_id, first_method)` and selects the first candidate without raising an ambiguity error, or depends on provider symbol declaration order.

This note documents the design options, semantic implications, and acceptance matrix for normative adoption.

---

## 2. Decision Options

### Option A: Inherent Priority + Trait-Trait Ambiguity Rejection (Recommended)
- **Inherent methods strictly shadow trait methods**: If `Type` defines an inherent method `foo`, `val.foo()` unconditionally dispatches to the inherent method.
- **Multiple Trait Ambiguity Error**: If a type implements two traits `TraitA` and `TraitB` both defining method `bar`, and neither is inherent, an unqualified call `val.bar()` is rejected with a typed diagnostic (`E1005: AmbiguousMethodCall`).
- **Disambiguation Syntax**: Callers must disambiguate using fully qualified syntax (e.g., `TraitA::bar(&val)`).

### Option B: Trait Precedence by Import/Declaration Order (Historical / Fragile)
- Unqualified calls pick the trait that was imported or declared first in source.
- **Flaw**: Violates Luna Artifact Identity Principle and causes subtle bugs when imports change or when comparing source vs `.llib` artifact modes.

### Option C: Uniform Ambiguity (Inherent and Trait collisions both error)
- If an inherent method and an imported trait method share a name, calling `val.foo()` is rejected as ambiguous unless qualified.
- **Flaw**: Adding a method to an imported trait can break downstream code that has existing inherent methods.

---

## 3. Recommended Policy: Option A
1. **Rule 1 (Inherent First)**: Inherent methods always take precedence over trait methods with the same name.
2. **Rule 2 (Trait Ambiguity)**: When no inherent method exists and two or more in-scope trait implementations provide the method, the compiler MUST reject the call as ambiguous.
3. **Rule 3 (Explicit Qualification)**: Ambiguous trait methods can be called via `TraitName::method(val, ...)`.

---

## 4. Acceptance Matrix

| Case | Scenario | Expected Outcome |
|---|---|---|
| P1 | Inherent method `Counter::drain` + Trait `Drain::drain` | Dispatches to `Counter::drain` (inherent) |
| P2 | Single trait `Display::fmt` on `Point` | Dispatches to `Display::fmt` |
| P3 | Fully qualified call `Display::fmt(&val, &rw writer)` | Dispatches to `Display::fmt` |
| N1 | Two traits `TraitA::run` and `TraitB::run` implemented on `Task`, no inherent `run` | Compile error: `AmbiguousMethodCall` with candidate spans |
| N2 | Trait method call when trait is not in scope | Compile error: `MethodNotFound` |
