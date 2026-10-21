<!-- luna-doc-role: guidance -->

# STD-D05: memory size/index migration

2026-10-21, Asia/Bangkok. **PLANNED, NOT IMPLEMENTED.** The maintainer selected
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

## Later passes and exclusions

Container follow-up includes RawTable, map/set, deque, heap and tree storage.
The RawTable `ProbeResult::Found(u64)` / `Vacant(u64)` payloads are memory indices,
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
