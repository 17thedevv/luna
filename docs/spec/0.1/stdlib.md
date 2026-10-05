# Luna 0.1 — standard library contracts

The stdlib is an ordinary consumer of Luna semantics. The original audited
baseline has 32 component providers. All-worktree integration expands the
implementation manifest to 49; its generated [inventory](stdlib-inventory.md)
distinguishes the 32 audited components from 17 later additions. Adding a
provider is not automatic adoption or verification of every public API it
contains. The inventory is derived from `luna-rs/libs/external/sysroot.toml`,
not a second hand-maintained manifest.

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

`CStr` is a borrowed raw-backed view. `cstr_from_bytes_with_nul` and
`CStr::from_bytes_with_nul` retain the input byte-slice validity relation;
`CString::as_c_str` retains its receiver relation. These relations survive
Result extraction and generic forwarding. `cstr_from_ptr` and
`CStr::from_ptr` require `unsafe`: the caller proves readable, initialized,
NUL-terminated storage that remains valid for the view and all its borrows.
The raw constructor supplies no safe provenance proof by itself. A safe
view cannot escape local backing storage or permit invalidation of a live
borrow. See the [scoped repair audit](../../audits/0.1-alpha-completion-2026-10-04/RAW-SLICE-VIEWS.md)
for current verification rather than a blanket release claim.

## Numeric, text and optimization boundaries

Convert and Display inherit correct generic integer widening; they must not
mask a compiler defect independently in every API. The existing char-to-text
boundary retains the subsequently approved Unicode scalar/cast/operator contract
in the exact-commit formatting record. Merged changes still require fresh
verification; the original audit's counterexamples remain dated evidence.

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
