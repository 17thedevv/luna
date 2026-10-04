# Luna 0.1 — syntax

This is the source-level contract. Formal productions are in
[grammar.ebnf](../../grammar.ebnf); coverage limitations are stated there.
Parser deviations are tracked separately in [gaps.md](gaps.md).

## Identity, names and literals

Canonical source files use `.ln`; libraries use `.llib`. Legacy `.ms` and
`.mlib` reads are compatibility behavior, not the spelling for new examples.
Names are case-sensitive. `Self` is the current implementing type; `self` is a
receiver value. Literals include integers, floats, booleans, characters,
strings, byte literals/strings, and raw strings. Numeric separators and bases
follow the lexer contract. Literal acceptance does not prove complete backend
support for extreme 128-bit values.

Primitive types: `i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `u8`, `u16`, `u32`,
`u64`, `u128`, `usize`, `f32`, `f64`, `bool`, `char`, `str`, and `void`.
Pointer-sized integer width follows the target. Types also include references
`&T`/`&rw T`, pointers `*T`/`*rw T`, arrays `[T; N]`, slices `[T]`, tuples,
function types `fn(T) -> U`, nominal generic types and restricted `dyn Trait`.
Unsized types are subject to the semantic value-position restrictions.

## Bindings and functions

```luna
dec value: i32 = 10;
dec rw counter: i32 = 0;
counter = counter + 1;
const LIMIT: i32 = 20;

fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

fn identity<T>(value: T) -> T { return value; }
```

`dec` creates a binding; `rw` grants reassignment. Omitted annotations use
type inference where the language permits it. `const` requires compile-time
admission and evaluation; it is not a synonym for an immutable runtime binding.
Blocks and declarations use braces; statement terminators use `;`.

## Structs, enums and visibility

```luna
export struct Point {
    x: i32,
    y: i32,
    private cache: i32,
};

enum State { Ready, Value(i32), }

impl Point {
    fn sum(self: &Self) -> i32 { return self.x + self.y; }
}
```

Struct fields SHALL be separated by commas. A struct declaration ends with one
semicolon after its contracts. A declaration is private unless exported.
Under the approved **Visibility-02** amendment, a field without a modifier is
public; `export` explicitly says public and `private` says private. Access
requires accessibility of both the containing type and the field. A public
field does not make its containing private type accessible. Construction and
destructuring must respect private-field restrictions.

Receivers `self`, `&self`, and `&rw self` retain the defined equivalence to
`self: Self`, `self: &Self`, and `self: &rw Self`; a current parser limitation
must not silently remove that contract.

## Traits and generics

```luna
trait Measure { fn measure(self: &Self) -> i32; }
struct Item { amount: i32, };
impl Measure for Item {
    fn measure(self: &Self) -> i32 { return self.amount; }
}
fn read<T: Measure>(value: &T) -> i32 { return value.measure(); }
```

Both type and value generic arguments use `<...>`, e.g. `identity<i32>(3)`.
Associated types and method-level generics follow the retained trait contract;
their symbolic projections must resolve before concrete code generation.
Trait object restrictions are stated in [semantics.md](semantics.md).

## Control flow and expressions

`if`/`else`, `while`, `for`, `match`, `return`, `break`, and `continue` have
their ordinary structured control-flow roles. Match arms use `->`, not `=>`.
The retained dual-loop contract defines:

```text
for (pattern in expression) { ... }
for (dec rw i = 0; i < limit; i = i + 1) { ... }
```

**Known parser gap:** current accepted fixtures use `for pattern in expression`
without the surrounding parentheses; the previously defined parenthesized
foreach form is not implemented correctly. The baseline retains that contract
pending reconciliation rather than claiming both forms are supported.

Calls, member/index access, qualified paths, arithmetic, comparisons, boolean
operations, assignments, pointer/reference operators, casts, aggregate
construction, closures, and `?` are governed by their type/ownership rules.
Async suspension is postfix `future.await`. `unsafe { ... }` and `unsafe fn`
mark unsafe operations; they do not disable other semantic checks.

## Modules, macros and contracts

```luna
import <vec>;
import "geometry";
using std as library;
using std;

module application { export fn answer() -> i32 { return 42; } }

macro twice { (@value: expr) => { @value * 2 } }

fn borrow(value: &i32) -> &i32 life_from(value) { return value; }
struct View { value: &i32, } requires life(value) >= life(self);
struct Owner { private data: *rw u8, } requires anchor(data) = self;
```

Import selects a provider, never a synthetic namespace. Macros use `@` captures,
`=>` rule separators, and `!` invocations. Attributes use `#[...]`; parsing an
attribute is not a promise that any arbitrary attribute has executable meaning.
Supported attribute contracts must be stated independently.

`life_from(a | b)` supplies alternative provenance. Relations use
`requires life(a) >= life(b)` or the equivalent `<=` spelling. A projected
`life(...)` target does not grant a raw pointer a safe origin. Anchor contracts
apply to direct pointer fields, separately from lifetime relations.

## Rejected spellings

New Luna 0.1 code MUST NOT use `let`, `mut`, `use`, `mod`, Rust lifetime generic
parameters, `::<>`, `@<...>` generics, `$` macro captures, prefix `await`,
`using namespace`, `export using`, `export import`, or
file-level `module name;`. `where outlives(...)` is a removed surface spelling;
its relation is written using `requires life(...)`. Imports omit source/artifact
extensions. Historical examples using these spellings are not current examples.
