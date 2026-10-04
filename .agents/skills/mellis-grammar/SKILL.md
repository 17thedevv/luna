---
name: mellis-grammar
description: Operational guidance for the versioned Luna Language Grammar and Syntax (formerly Mellis). Provides strict guidelines, forbidden syntax, and canonical examples for the language surface.
---

<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](../../../docs/spec/0.1/README.md).

# Luna Grammar and Syntax Guidelines

This skill is operational guidance for the versioned Luna 0.1 syntax and adopted contracts; it is not an independent specification. Do not infer Luna syntax from Rust, C++, or any other language. Always refer to this document and the accompanying `grammar.ebnf` when modifying the parser, AST, or semantic analysis, or when writing `.ln` code.

## Grammar Authority Rule

Canonical fixed-array types use `[T; N]`; slices use `[T]`. See the
[versioned syntax contract](../../../docs/spec/0.1/syntax.md). A stale auxiliary
grammar spelling must not override that contract.

Agents MUST adhere to the following rules at all times:
1. **Read this skill** before modifying parser, AST, or semantic syntax.
2. **Never introduce a new keyword** without first updating the grammar in `docs/grammar.md`, `grammar.ebnf`, and this skill.
3. **Never reuse deprecated or removed keywords**.
4. **Never infer Luna syntax from Rust/C++ syntax**. (e.g., Do not use `mut`, `let`, `mod`, `use`).
5. **Add or update parser tests** when changing grammar.
6. **Reuse, don't redesign**: If a syntax already exists, use it. If not, explicitly propose a grammar change to the user before implementation.

## Forbidden Syntax

The following keywords and syntax constructs are strictly **FORBIDDEN** in Luna:
- `mut`: Luna uses `rw` (rewritable) for mutability.
- `let`: Luna uses `dec` and `const` for variable bindings.
- `use`: Luna uses `import` for providers and `using ... as ...` for namespace aliases. The keyword `use` does not exist in Luna.
- `mod`: Luna uses `module` for namespaces, or `import` for module providers. (Avoid Rust's `mod` assumptions).
- `module foo;`: File-level module declarations without a body are forbidden. Luna only allows inline `module foo { ... }`. Provider identity is determined by file name / artifact resolution.
- `using namespace`: Luna does NOT have `using namespace` (C++ style). Luna uses `using path;` or `using path as alias;` under NAMESPACE-USING-v1.
- `export using` / `export import`: Luna v1.0 does not support module re-export. Both are hard syntax errors.
- `import <a::b>`: Mellis `import <...>` only accepts a single logical provider name. No `::`.
- `import "foo.ms"` or `import "foo.mlib"`: File paths in local imports must not contain extensions (use `import "foo";`).
- Prefix `await`: Luna 0.1 strictly uses postfix `.await` (e.g. `fut.await`). Prefix `await fut` is forbidden.
- Semicolons `;` as struct field delimiters: Struct fields must strictly use comma `,` delimiters (e.g. `struct Point { x: f64, y: f64 }`).
- `=>`: Mellis uses `->` for match arms and lambda return types. Note: `=>` is strictly used as the macro rule separator (`macro foo { (pattern) => { template } }`).
- `$`: Mellis uses `@` for macro placeholders/metavariables (`@name: expr`). `$` is forbidden in Mellis macro definitions.

## Canonical Examples

### Variable Bindings & Mutability
```rust
dec x = 10;
dec rw y = 20;

// References
dec r: &i32 = &x;
dec rw r2: &rw i32 = &rw y;

// Raw Pointers
dec p: *i32 = &x as *i32;
dec rw p2: *rw i32 = &rw y as *rw i32;
```

### Raw Pointers and Unsafe Blocks / Functions
```rust
dec x = 10;
dec rw y = 20;

// Creating and passing raw pointers is safe
dec p: *i32 = &x as *i32;
dec p_mut: *rw i32 = &rw y as *rw i32;
dec q = p; // trivially copyable

// Dereferencing and pointer arithmetic require unsafe
unsafe {
    dec val = *p;         // read through immutable raw pointer
    *p_mut = 30;          // write through mutable raw pointer
    *rw p_mut = 40;       // explicit mutable dereference syntax
    dec next = p_mut + 1; // pointer arithmetic (element stride)
}

// Unsafe functions
unsafe fn danger(ptr: *rw i32) {
    *ptr = 999;
}

// Calling unsafe functions requires unsafe
unsafe {
    danger(p_mut);
}
```

### Generics
Mellis utilizes `<...>` for explicit generic arguments at BOTH type-level and value-level.
`@<...>` is NOT generic syntax.

- Type generic arguments: `<T>`
- Value generic arguments: `<T>`

```rust
struct Box<T> { ... }

impl<T> Box<T> { ... }

fn identity<T>(x: T) -> T {
    return x;
}

// Usage
dec x: Box<i32>;
dec y = identity<i32>(42);
```

### Struct Declarations
Struct fields **strictly use comma `,` delimiters**. Semicolons `;` are forbidden inside the field list in Luna 0.1. Trailing comma is permitted.

**Struct Declaration Terminator Invariant**:
A struct declaration is terminated by exactly one `;` after all postfix type-level contracts.
- `}` closes the struct body.
- `;` closes the struct declaration.
Therefore, `struct A {};` and `struct B { field: &T } requires life(field) >= life(self);` are canonical.

Fields are **public by default** under approved Visibility-02. `export` explicitly means public; `private` means private. Containing type accessibility remains independently required:
```rust
export struct Point {
    export x: f64,
    export y: f64,
};

export struct User {
    export name: str,
    private password_hash: str,
};

// A private containing type remains inaccessible externally, independently of its field visibility.
struct InternalBuffer {
    capacity: usize,
    len: usize,
};

// Struct with lifetime contract:
struct Holder {
    value: &i32,
} requires life(value) >= life(self);

// RAW-STORAGE-ANCHOR-v1: a direct raw-pointer field may carry an explicit
// owner-relative storage invariant. This is not a lifetime relation.
struct RawOwner {
    private data: *rw u8,
} requires anchor(data) = self;

// Multiple contracts use one requires and comma-separated entries:
struct RawPair {
    private first: *rw u8,
    private second: *rw u16,
} requires anchor(first) = self, anchor(second) = self;
```

The canonical struct contract spelling is one `requires` followed by a
non-empty comma-separated list. Anchor and lifetime constraints may be mixed
in that list. A trailing comma after the last contract is not accepted.
Existing repeated `requires` groups remain accepted; they have the same
semantics as the single-group spelling. This does not change function
lifetime clauses or the raw storage anchor contract.

`requires anchor(field) = self` is valid only for a direct field whose type is
`*T` or `*rw T`. The canonical identity is the owning type plus the field
name, and the contract is preserved in `.llib` semantic metadata. Nested field
paths and non-pointer fields are rejected. Establishing or re-establishing the
per-value fact requires an explicit `unsafe` field construction/store boundary;
ordinary stores cannot preserve it. See the RAW-STORAGE-ANCHOR-v1 contract for
move, mutation, and raw-to-safe conversion rules.

### Imports & Module Architecture (Provider vs Namespace)
Mellis strictly separates **artifact providers** from **module namespaces**:
- `import` selects a **provider** (file/artifact name, no file extensions).
- `module` defines a **namespace / declaration container** inside a provider file.
- Symbols are accessed via their namespace (`geometry::Point`), NOT via their provider name.
- Multiple providers can contribute to the same module namespace (e.g. `alloc` and `core` contributing to `module std`).

```rust
// In math.ln (provider = "math"):
module geometry {
    export struct Point {
        x: f64,
        y: f64,
    };
}

// In consumer.ln:
import "math";     // Provider loaded (finds math.ln / math.llib)
import <mem>;     // External sysroot provider loaded

// Accessed via namespace, NOT provider name:
dec pt = geometry::Point { x: 1.0, y: 2.0 };
```

### Namespace Openings and Aliases (`using`)
`using path;` opens the accessible direct members of an existing namespace for unqualified lookup. `using path as alias;` creates a local namespace alias. Neither form loads providers or exports directives. Both are restricted to file/module scope; distinct competing candidates produce E1008. See NAMESPACE-USING-v1 for lexical precedence, visibility and declaration identity.

**Grammar:** `using_decl ::= "using" module_path [ "as" IDENTIFIER ] ";"`

**Rules:**
- Target must be a qualified module/namespace path, not a leaf symbol (type, function, etc.)
- `using` aliases are compilation-local — they are NOT exported via .llib
- `export using` is a compile-time syntax error
- `using path;` opens direct namespace members without copying declarations
- Standard duplicate-scope rules apply

```rust
import <vec>;

// Alias a namespace
using std as library;

dec v: library::Vec<i32>;       // Same as std::Vec<i32>

// Deep path alias
using application::network::protocol as proto;

dec r: proto::Request;
dec s: proto::Response;

// INVALID — target is a type, not a namespace:
// using std::Vec as V;       // ERROR

// Valid namespace opening:
using std;

// INVALID — cannot export:
// export using std as s;     // ERROR
```

### Methods & Receiver Parameters
Mellis defines canonical shorthand for method receivers, with strict semantic equivalence to explicit forms:

| Canonical Shorthand | Explicit Equivalent | Semantics |
|:---|:---|:---|
| `&self` | `self: &Self` | Immutable reference receiver |
| `&rw self` | `self: &rw Self` | Mutable / rewritable reference receiver |
| `self` | `self: Self` | By-value ownership receiver |

```rust
impl Point {
    fn distance(&self, other: &Self) -> f64 {
        // ...
    }

    fn translate(&rw self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
    }

    fn consume(self) {
        // consumes ownership
    }
}
```

Trait methods may declare method-level type parameters using the same
`generic_params` syntax as ordinary functions. Static calls are monomorphized;
generic trait methods remain excluded from `dyn Trait` by object-safety rules.

### Async & Await
Mellis strictly uses **postfix `.await`** (`expr.await`). Prefix `await expr` is strictly forbidden to eliminate operator precedence ambiguity and harmonize with the member-access family (`.field`, `.method()`, `[i]`, `.await`).

```rust
async fn fetch() -> i32 {
    dec fut = get_data();
    dec res = fut.await;
    return res;
}
```

### Loops (Foreach & C-style)
Mellis supports both foreach and C-style index loops as complementary systems constructs:

```rust
// 1. Foreach loop: for ( <pattern> in <expression> )
for (item in collection) {
    process(item);
}

// 2. C-style index loop: for ( <init> ; <condition> ; <step> )
for (dec rw i = 0; i < n; i += 1) {
    buffer[i] = 0;
}
```

### Match Statements
Match arms use `->` instead of `=>`.
```rust
match opt {
    MyOption::Some(x) -> x,
    MyOption::None -> 0,
}
```

### Functions
```rust
fn do_something(x: i32) -> i32 {
    return x + 1;
}
```

### Declarative Macros (Phase 14B)
Declarative macros use `@` for placeholders, `=>` to separate pattern and transcriber, and `!` for invocation.
```rust
macro foo {
    (@x: expr) => {
        @x * 2
    }

    (@x: expr, @y: expr) => {
        @x + @y
    }
}

// Invocations:
dec a = foo!(21);
dec b = foo![10, 20];
dec c = foo!{30};
```

## Module System Primitives

Luna 0.1 has exactly 4 module system primitives:

| Primitive | Purpose | Example |
|-----------|---------|---------|
| `import`  | Make an artifact provider available | `import <vec>;` / `import "math";` |
| `module`  | Define an inline namespace container | `module std { ... }` |
| `using`   | Namespace opening or local alias | `using std;` / `using std as library;` |
| `::`      | Qualified namespace lookup | `library::Vec<i32>` |

## Lifetime Contracts (REGION-DESIGN-01)

Luna uses explicit lifetime relation contracts rather than lifetime generic parameters.

| Syntax Primitive | Usage | Example |
|------------------|-------|---------|
| `life_from(x)` | Reference acquires lifetime provenance from `x`. | `fn id(x: &i32) -> &i32 life_from(x)` |
| `requires life(a) >= life(b)` | Explicit outlives constraint. | `requires life(owner) >= life(value)` |
| `life(self)` / `life(return)` | Special query targets for the current instance or return value. | `requires life(return) <= life(owner)` |

These contracts are appended at the end of function, struct, and extern declarations before the block/semicolon.

## Known current conformance disagreements

The versioned gap register preserves dated grammar characterization; the repair ledger and current verification records track later checks separately. Do not treat historical parser failures as current language restrictions. Formal EBNF and complete receiver-equivalence coverage remain separate acceptance obligations. Do not infer completeness from the invariant below.

## Grammar Layer Invariant

$$\text{AST accepted by parser} \iff \text{Exactly defined by grammar.ebnf} \iff \text{Exactly supported by parser} \iff \text{No accepted syntax silently ignored}$$

## Related Documents
- [grammar.ebnf](../../../docs/grammar.ebnf): Machine-readable formal EBNF definition of the Mellis syntax.
- [docs/grammar.md](../../../docs/grammar.md): Original comprehensive grammar specification.


Luna 0.1 alpha amendment: `using_decl = "using", namespace_path, [ "as", identifier ], ";" ;`
See [NAMESPACE-USING-v1](../../../docs/spec/0.1/namespace-using-v1.md) for lookup, visibility and ambiguity rules.
