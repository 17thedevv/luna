# MODULE-CONST-STORAGE-v1 — module constants and borrowed storage

Adopted by the maintainer on 2026-10-04 during the 0.1 completion work.
A module-level constant has immutable storage lasting for the program's
lifetime when a reference is taken to it. A function-local constant retains
its ordinary lexical storage lifetime. Implementation remains pending;
adoption does not certify current lowering.

## Scope and lifetime

The rule covers constants declared at file/module level. Visibility does not
determine storage lifetime: a private module constant can be used by its
defining functions. Existing visibility and const-admission rules still apply.

```luna
module values {
    const DEFAULT: i32 = 20;
    export fn default_ref() -> &i32 { return &DEFAULT; }
}
fn main() -> i32 {
    dec value = values::default_ref();
    return *value - 20;
}
```

Taking a shared reference must preserve the constant's module storage origin.
Returning it is valid under the ordinary global-origin lifetime rules.
Materializing a function-local temporary and returning a reference to that
temporary does not implement the contract. Declaration/provider identity must
govern storage resolution, including multiple generic instances and imports.

A local constant does not gain global storage because its initializer can be
evaluated at compile time:

```luna
fn invalid() -> &i32 {
    const LOCAL: i32 = 20;
    return &LOCAL;
}
```

This local escape rejects. Mutable references/writes to immutable constant
storage reject under the existing mutability rules. The amendment does not
promote arbitrary literals, local constants or default-expression temporaries
to global storage, and does not add mutable global-variable syntax.

## Pipeline and provider preservation

Type checking and borrow/region analysis must distinguish module storage from
local temporary origin. Lowering/codegen must create actual immutable target
storage with its valid initializer and concrete type/layout. Relabeling stack
storage as global to bypass escape checking is invalid. Compile-time VM
addresses cannot become runtime storage identities.

Imported native and portable generic bodies must retain declaration identity
and initializer dependencies without exposing private constants to ordinary
consumer lookup. Source-only and fresh artifact-only programs must agree on
lifetime, contents and rejection. Invalid/incompatible artifacts remain subject
to ordinary rejection without automatic rebuilding or source fallback.

## Required acceptance

Verify returned shared references from functions using private module constants;
direct/imported calls; repeated references; separate declarations and providers
with equal values; generic instances and relocated provider graphs. Exercise
scalar and admitted aggregate constants, borrowed field projections and native
execution after the defining function returns in source-only/fresh-artifact-only
modes. Reject local-constant escapes, mutable access, private consumer lookup
and invalid constant/runtime materialization.

The borrowed Option/Result provider parity failure is a regression target,
not the complete acceptance matrix for this amendment.
