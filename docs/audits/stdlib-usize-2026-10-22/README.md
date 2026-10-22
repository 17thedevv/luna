<!-- luna-doc-role: evidence -->

# Stdlib API expansion — 2026-10-22

Status: focused Windows PASS at implementation pin
`28802f1258de156015c4ffec8a607e1dcfd6df4d` on stdlib-redesign. This record
does not qualify stable, a package, an installer or 32-bit native execution.

The maintainer selected usize memory counts, type constructors and continued
expansion, then requested a functional comparison with C++ and useful missing
facilities. See the [size/index map](../../proposals/stdlib-redesign-0.1/size-index.md)
and [C++23 gap map](../../proposals/stdlib-redesign-0.1/cpp-library-gap-map.md).

## Implemented slice

- Memory, slice, collection, text and consumer size/index migration to usize.
- Type-owned constructors for six additional collection families, preserving
  transitional factories and the compact std namespace.
- BTreeSet subset/superset/disjoint and four owned set operations. Inputs are
  borrowed; results clone elements, preserving sorted iteration. Relations
  require Ord only; owned results additionally require Clone.
- HashSet::is_disjoint under its existing Hash/Eq contract.
- std::bit u64 utilities in the existing num provider: count_ones, leading/trailing
  zeros, rotates, single-bit predicate, width/floor/ceil and byte swap. Names have
  the `_u64` suffix; results/counts use domain-width u32. Zero has 64 leading and
  trailing zero bits. Rotations reduce count modulo 64. bit_ceil returns None
  above the highest representable power of two, rather than shifting by 64.

The two [generic compiler repairs](compiler-gap-16.md) and
[temporary cleanup repair](compiler-gap-17.md) preserve the stdlib/compiler
boundary. No type-specific instruction, intrinsic or runtime ABI is introduced.
The coherent bundle versions are format 2 / compiler 26 / metadata 9 / MVIR 5 /
runtime ABI 2, with 50 providers.

## Verification boundaries

The [Windows receipt](evidence/windows-28802f12/receipt.json) and
[acquisition notes](evidence/windows-28802f12/README.md) preserve raw logs,
commands, binary identities and SHA-256 values. LLVM18/GNU results:

| Scope | Result |
| --- | --- |
| Eight CLI suites, including the new size/index matrix | 29 Rust tests PASS, zero failed |
| Existing ptr/mem, RawTable, Vec and String acceptance | 40 PASS, zero failed |
| Semantic/llib tests | 231 PASS, zero failed |
| Borrowck/semantic/llib unit run | 59 PASS, overlapping the semantic run |
| Final backend/MVIR unit replay | 9 PASS, zero failed |
| Fresh canonical sysroot | 50 LLIB headers: format 2 / compiler 26 / MVIR 5; 50 COFF objects 0x8664 |

Source-only and artifact-only roots are isolated; prior bundles are never
copied into a fresh build. The new matrix passes positive and negative
type/bound checks, exact non-Copy cleanup, borrowed views, overflow/allocation
controls, interface identity and old-protocol rejection. The generic projected
temporary fixture returns failure with the pre-repair compiler and success with
the repaired compiler. Failed intermediate attempts remain explicitly superseded.

The [parser source snapshot](evidence/windows-28802f12/source-surface-receipt.json)
records 50 providers, 2,411 declaration records and 219 explicit import edges.
Two acquisitions were byte-identical and all 50 raw source digests were checked.
These records include implementation declarations: they are not a resolved
export catalog, semantic coverage count or conformance percentage.

Earlier allocator results remain pinned to their recorded commits. In particular,
Ubuntu run 39634996889 completed focused tests but was cancelled during workspace
regression and skipped ASan; it is not workspace PASS for this implementation.
Fresh Ubuntu [run 39724850945](https://github.com/17thedevv/luna/actions/runs/39724850945)
targets the same implementation pin. Its runtime step passed; public CLI,
canonical sysroot, C-runtime ASan and complete workspace results are not yet
closed by this record. Full Windows/Ubuntu regression and final
package/installer gates remain open.

## API and release follow-up

The C++ comparison is a backlog of functional families, not a C++ conformance
target. Graph and basic CPU Matrix/Tensor remain planned extensions; regex,
Complex, packed bit collections, numeric scans, calendar/timezone and remaining
families are not implemented by this slice. Per-domain signatures, errors,
ownership and precision requirements must accompany their implementations.

No release merge, tag, installation or publication is justified by this audit.

## Semantic implementation report

| Field | Applied contract |
| --- | --- |
| Semantic Contract | Target-sized memory counts; generic ownership/drop, borrowing and explicit fixed-width bit operations |
| Luna Syntax | dec/rw, normal generic impl bounds, ordinary module/function APIs and life_from relations |
| Semantic Enforcement | TypeChecker and canonical interfaces enforce usize/bounds; borrowck and move/drop elaboration retain ownership/provenance |
| Implementation | Existing providers and generic validator/MVIR/backend machinery; provider IDs and source root retained |
| Feature Usage | Associated constructors, generic sets with Ord/Clone, unsafe memory boundaries, Result/Option |
| Foreign Assumptions Avoided | C++ headers are a functional comparison, not Luna semantics; rotations mask checked shifts explicitly |
| ABI / .llib | Compiler 26 rejects pre-amendment bundles; all providers/dependents rebuild coherently |
| Negative Tests | E2001 old u64 counts, E2021 missing Clone, existing borrow/move/escape/unsafe controls and stale artifacts |
| Source / Binary Parity | Isolated native source-only and fresh artifact-only CLI matrices |
| Verification | Receipts distinguish successful focused suites, superseded attempts and pending full qualification |
| Remaining Limitations | Full final-pin workspace/package qualification; general comptime callable gap retained; new library families remain planned |

SKILL IMPACT: CORRECTION. PTR-MEM-4 in luna-semantic-compliance now names usize
under the approved STD-D05 amendment; byte/element units and safety obligations
are retained. Related skills were searched for contradictory count signatures.
