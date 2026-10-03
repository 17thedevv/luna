# Luna 0.1 — macros and compiler/library boundary

## Generic declarative expansion

A declarative macro is a resolved declaration with matcher/transcriber rules,
not a keyword selected by its short name. Expansion must respect scopes,
qualified paths, hygiene, fragment parsing, repetitions, recursion limits and
structured diagnostics. Captured expressions are parsed using language syntax,
not guessed from regular-expression replacements.

```luna
macro twice {
    (@value: expr) => { @value * 2 }
}
fn main() -> i32 { return twice!(21) - 42; }
```

Matcher fragment kinds and repetition syntax follow the existing macro
contracts and parser tests. A proposed procedural or formatting macro feature
is not established merely because an annotation can be parsed.

## Prohibited stdlib mappings

The compiler MUST NOT trigger formatting because a macro is called `out`,
`outln`, `err`, `errln`, `print`, `println`, `eprint` or `eprintln`.
It MUST NOT infer an output stream from `err`/`eprint` prefixes, infer a newline
from an `ln` suffix, search library declarations by `print_val`/`eprint_val`
spelling, or fall back to an invented `std::io` callee path.

The same restriction applies at **all** pipeline stages, including AST macro
desugaring. Absence of a backend opcode does not make an AST library-name mapping
generic. An annotation such as `#[format_macro]` would not remove the coupling
if the compiler still chooses library-specific callees.

A library macro owns its output target, emission policy and newline policy.
Display/Writer are ordinary traits; user-defined types and sinks use ordinary
resolution, type checking, monomorphization and ownership checking.

## Formatting status and extension boundary

The adopted formatting foundation defines Writer, Display, FmtError and ordinary
implementations. It explicitly excludes formatting macros, format-string parsing,
width/alignment/precision syntax and general streaming writers from that phase.
Those exclusions remain in effect for the 0.1 baseline unless amended explicitly.
The original audit checkout at `3dac3ac` contained no implementation of the
eight-name mapping or `#[format_macro]` described in the earlier discussion.
All-worktree integration imports such a mapping from D:/fdlang in
`luna-semantic/src/macro_engine.rs`. This is an implementation boundary defect,
recorded as V01-ARCH-02; merging it does not amend or approve the contract above.

If `{expr}` interpolation is added, its language/macro capability must specify
literal/expression segmentation, escaping, evaluation order, evaluation count,
borrowing and temporary lifetimes, source spans, hygiene, failure behavior and
source/artifact parity. It must work with arbitrary library-defined names and
sinks. This document does not invent a new intrinsic or lang item to implement
that proposal.

## Other compiler-facing mechanisms

Only explicitly defined language primitives, generic intrinsics, and lang-item
contracts may carry compiler semantics. Recognition uses canonical semantic
identity, not the names/layouts of Vec, Box, String, HashMap, HashSet or a custom
container. Slice iterator implementation names/layouts are library-owned; the
existing compiler coupling is recorded as an architecture gap.
