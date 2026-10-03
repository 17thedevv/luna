# Luna 0.1 — standard library contracts

The stdlib is an ordinary consumer of Luna semantics. The baseline manifest has
32 component providers; its generated [inventory](stdlib-inventory.md) is derived
from `luna-rs/libs/external/sysroot.toml`, not a second hand-maintained manifest.
Provider counts describe this baseline and may change only with an explicit
manifest/spec update. Public APIs are not implicitly complete because all
providers build.

## Required invariants

| Capability | Contract |
|---|---|
| Box and owning containers | Transfer ownership exactly once; clean initialized values; use generic allocation/drop primitives |
| Vec | `len <= capacity`; storage must cover every admitted initialized element; check allocation/growth arithmetic before updating metadata |
| String | Every safe text-producing operation preserves valid UTF-8; byte validation rejects invalid encodings |
| Slice iteration | Yield exactly the logical element count, including zero-size element types; shared/mutable capabilities remain distinct |
| HashMap / HashSet | Equal keys obey the Hash/Eq contract; insert/replace/remove/growth preserve values and exact ownership cleanup |
| Iterators | next advances the logical stream; adapters/consumers respect their declared generic ownership and borrowing requirements |
| Generic predicates | A predicate must not consume a non-Copy value that the API subsequently returns or uses unless the contract explicitly allows duplication |
| Formatting | Ordinary Display/Writer dispatch, error propagation, UTF-8 text output; no compiler-selected library callees |
| Whole-file I/O | Preserve bytes, report the defined failures, honor buffer ownership and path preconditions |
| Path | Retain the defined lexical slash-based semantics; do not import Windows drive/UNC behavior into that contract implicitly |

An allocation size overflow must not produce usable container metadata for
storage that was not allocated. Library preconditions must distinguish unsafe
raw operations from safe public methods. Generic helpers must not reinterpret
arbitrary `T` storage as i32 or another particular library element type.

## Numeric, text and optimization boundaries

Convert and Display inherit correct generic integer widening; they must not
mask a compiler defect independently in every API. The existing char-to-text
boundary has unresolved scalar-domain debt: String/Writer text invariants remain
requirements while a decision about every integer-to-char cast is tracked
separately. No new cast semantics are frozen by this documentation refresh.

Optimization cannot weaken ownership, lifetime, overflow, text, ABI or parity
contracts. Identity hashes with low-bit masking have a measured clustering
problem; this does not redefine equality or promise a universal constant
complexity bound absent a specified hashing model. Enum niche layout changes,
buffer adoption and new allocator strategies require explicit compatibility
contracts and measurements.

## Conformance snapshot

The [2026-10-02 audit](../../audits/stdlib-2026-10-02/README.md) records 32 provider
builds, existing suites, CLI source/artifact probes and repeated benchmarks.
Its verdict is **BROKEN correctness / PARTIAL optimization and verification**.
The gap register retains seven confirmed library defects and compiler defects
that affect library behavior. Historical component freeze reports cannot be
used as current blanket release acceptance.

Float formatting, Debug, format-string macros and general streaming formatting
are not introduced by the formatting foundation. Rc/Arc, general threading
wrappers and other future APIs are not promised merely because runtime support
or a roadmap entry exists.
