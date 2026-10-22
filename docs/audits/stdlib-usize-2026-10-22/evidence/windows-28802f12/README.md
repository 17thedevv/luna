<!-- luna-doc-role: evidence -->

# Windows API foundation acquisition

Implementation pin: `28802f1258de156015c4ffec8a607e1dcfd6df4d`.
Target: x86_64-pc-windows-gnu, GNU C runtime and LLVM 18. The code and permanent
fixtures were unchanged between final acquisition and this recording commit.

[receipt.json](receipt.json) records commands, code-tree identities, compiler and
production-runtime digests. All final commands exited zero. Eight public CLI
suites contain 29 Rust tests; several orchestrate many Luna check/build/native
cases in isolated source-only and fresh artifact-only roots. These counts are
test-harness units, not individual language features or a coverage percentage.
The four existing driver acceptance suites add 40 passing tests. The 231-test
semantic/llib run overlaps the 59-test internal unit run; do not sum them as
unique coverage. Final backend/MVIR units pass 9 tests.

[sysroot-headers.json](sysroot-headers.json) was read directly from all 50 fresh
canonical LLIB files and their objects after successful quiet build-sysroot.
Every header has format/compiler/MVIR 2/26/5 and x86_64-pc-windows-gnu; every
object has machine 0x8664. Metadata 9 and runtime ABI 2 are source contracts,
not fields inferred from that abbreviated header tuple. The quiet build log is
empty; exit status plus actual artifact inspection provide its evidence.

The source parser [receipt](source-surface-receipt.json) identifies the
[compressed snapshot](source-surface.json.gz), its raw/compressed SHA-256 and
byte-identical second acquisition. All 50 source digests were recomputed.
The 2,411 declarations / 219 explicit imports do not resolve effective exports,
implicit bootstrap dependencies, generated items or semantic conformance.
Enum payload types are not expanded by this mapper. Prior inventories remain
immutable historical snapshots at their original pins.

The independent projected-owner regression was compiled using both compiler
generations: the old native executable exits 1, the final executable exits 0.
The repeated-predicate probe deliberately returns its number of drops: the
intermediate executable returns 1, the repaired executable returns 3. The
permanent CLI fixture also covers repeated predicates, short-circuit paths,
partial moves, break and early return in both provider modes.

Raw failures in attempts/ are retained as FAILED_SUPERSEDED or
SUPERSEDED_NOT_QUALIFICATION in the receipt. The initial collection drop
failure, LLVM dominance failure, missing fixture imports and checked-shift
L005 failure are not counted as PASS. Rotations now mask left-shift pieces;
the final bit fixture covers high-bit wrap, count modulo 64, inverse rotation
and overflow boundaries. None of the original cleanup or type oracles was
weakened to obtain success.

This is focused Windows evidence. It does not establish full Windows workspace,
Ubuntu, native 32-bit, package/installer, general comptime callable support or
stable qualification. The new Ubuntu run is tracked in the parent audit.
