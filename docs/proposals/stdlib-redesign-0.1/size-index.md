<!-- luna-doc-role: guidance -->

# STD-D05: memory size/index migration

2026-10-22, Asia/Bangkok. **IMPLEMENTED ON BRANCH; QUALIFICATION IN PROGRESS.** The maintainer selected
`usize` for memory/container lengths, capacities and indices. This map applies
that decision; it does not adopt new numeric, iterator or operating-system APIs.

The [current parser snapshot and first-wave map](../../audits/stdlib-redesign-2026-10-21/size-index-41416737/wave1.json)
records 50 providers and 2,375 declaration/impl/module records at 41416737. Two
independent acquisitions are byte-identical; all 50 source digests match. The
compressed snapshot preserves the full parser inventory. Its 66 first-wave
declarations contain 97 candidate type positions, including private helpers and
fields: these are not 66 public API or conformance claims.

## First coherent change

| Provider | Size/index positions | Constraints |
| --- | --- | --- |
| ptr | unsigned element counts for add/copy | Signed offsets and pointer differences need their own isize review |
| mem | raw slice lengths, byte counts, size/align queries, Layout fields/accessors | Preserve unsafe and lifetime contracts; no new primitive |
| __alloc_global | existing word-sized C allocation/memory arguments | Match C size_t; retain runtime ABI 2 semantics and extern safety |
| slice | lengths, indices, partition/search results and sorting bounds | Keep element types and i32 comparison results unchanged |
| vec | len/cap/index fields, iterator storage indices and public size/index APIs | Include constructors/factories and every dependent call site |
| string | byte lengths/capacities, finite text counts and byte offsets | Keep UTF-8 and borrowed C-string contracts; hash outputs remain u64 |

Change source signatures, stored fields, arithmetic/literals/casts, consumers
and fixtures together. A provider-only intermediate change is not a compilable
migration checkpoint if its dependents still pass u64. Existing factory names
are currently transition entries; their final disposition belongs to the API
migration map rather than an implicit numeric conversion added to the compiler.

The first-wave JSON identifies positions from parser declarations, not every
implementation expression. Review original source and implicit compiler
contracts before editing; textual replacement is not the migration algorithm.

## Expanded implementation and exclusions

The branch now includes RawTable, map/set, deque, heap and tree storage.
The RawTable `ProbeResult::Found(usize)` / `Vacant(usize)` payloads are memory indices,
while its hash parameters and hash array remain u64. Enum payload types are not
serialized in the inventory's declaration records, so source/AST review is
required. B-tree child indices/stacks and sentinel arithmetic need equivalent
review, independently of signed balance factors/heights.

Do not change every fixed-width integer into usize:

- Hash/crypto/encoding values and fixed-width numeric operations retain their
  domain-defined bit widths.
- File positions/lengths and total bytes copied across a stream can exceed one
  in-memory buffer. Review their u64 contracts independently of buffer lengths.
- Runtime status codes, process IDs, socket representations and numeric flags
  remain ABI/domain values.
- General iterator counters may describe unbounded streams. Their width and
  overflow behavior need a contract; a memory-index decision alone does not
  resolve them.
- Rc/Arc reference counts need an overflow/concurrency review. Arc currently
  stores AtomicU64; no AtomicUsize surface is established by this audit.

## Acceptance before migration completion

Use public CLI fixtures with native source-only and fresh artifact-only roots.
Cover zero/nonzero sizes, element vs byte units, indexing boundaries, checked
overflow, non-Copy drops, borrowed views and failure-preserving allocation.
Retain negative type/borrow/move controls rather than adding implicit coercions.

Verify changed public signatures/layout reach canonical interface identity and
reject stale dependent artifacts, with unchanged-contract controls. Keep
intrinsic size/align results and raw slice length representation consistent with
their language contracts. Rebuild coherent compiler/provider/runtime bundles;
the earlier allocator or alpha qualification does not qualify this migration.
The current native scope is Windows/Ubuntu x86_64 GNU, not untested 32-bit support.

## Implementation slice — 2026-10-22

The [memory-count amendment](../../spec/0.1/stdlib-memory-size-v1.md) and
[C-GAP-16](../../audits/stdlib-usize-2026-10-22/compiler-gap-16.md) cover canonical
raw-slice validation and target-sized MVIR lengths. Compiler protocol is now 26;
format 2, metadata 9, MVIR 5 and runtime ABI 2 are unchanged.

The migration includes ptr/mem/slice/algo, allocator ABI declarations, Vec,
String/CStr/CString and all map/set/deque/heap/tree storage. Thread/sync scratch
allocations now use size_t-compatible signatures; B-tree child storage uses
size/align_of<usize> rather than hardcoded eight-byte index slots. File/I/O,
encoding, crypto buffering, JSON and lexical path consumers were updated.

Hash arrays/parameters and SHA bit lengths remain u64; file/stream coordinates,
general iterator counters and Rc/Arc counts retain their domains. Network C
buffer-length parameters remain the existing uint64_t ABI with explicit bridges;
this does not silently change runtime ABI 2. Signed ptr offset/diff remain i64.
The retained source inventory above is a historical planning snapshot, not the
new surface or an automatic migration result.

Added type constructors: HashMap/HashSet/VecDeque/BinaryHeap new/with_capacity,
BTreeMap/BTreeSet new. Existing factory names remain migration entry points.
New public CLI fixtures in tests/luna/stdlib/size_index exercise typed returns,
byte/element units, UTF-8 byte offsets, growth, non-Copy cleanup, old u64 rejection,
interface fingerprint changes, stale dependents, coherent rebuild and old
protocol rejection. The [implementation audit](../../audits/stdlib-usize-2026-10-22/README.md)
tracks actual test outcomes and remaining release gates.
