# Luna 0.1 — runtime and ABI

Runtime ABI contract revisions are independent of language version 0.1. The
canonical identity is `__luna_*`, `LUNA_*`, headers `luna/runtime/*`, and the Luna
runtime library. Historical `__mellis_*` aliases are not current ABI promises.

## Layer ownership

The compiler owns the executable entry shim and language lowering. The C runtime
owns platform startup state, allocation, panic/traps, byte I/O and native services.
Stdlib owns safe APIs and semantic adapters. Runtime support for a service does
not imply a public language or stdlib feature is already complete.

The core [ABI contract](../../runtime/abi-v1.md) retains detailed allocation,
startup, shutdown, panic and primitive output rules. Current C declarations are
under `runtime/include/luna/runtime/`. Newer service headers must be read with
their own ownership and failure contracts; the historical core subset is not
the complete exported-symbol inventory.

## Allocation

The retained hosted allocation contract is **infallible or terminal failure**,
not a nullable allocator silently mixed with a library panic policy. Alignment
is a nonzero power of two bounded by LUNA_MAX_ALIGN. Zero-size allocation uses
the aligned runtime-owned sentinel; it does not provide writable storage for
nonzero elements. Deallocation and reallocation require valid metadata and
ownership. Runtime round-up checks cannot detect a multiplication that already
wrapped in the caller; safe container arithmetic must be checked beforehand.

## Entry and process arguments

Startup receives argc/argv. The already defined language entry contract permits
`main` returning void or i32, with the defined argument-slice bridge when used.
The baseline retains construction from OS arguments as defined by the detailed
ABI contract; it does not bless an empty-slice replacement. C-GAP-03 records
that current backend failure. Runtime argument accessors distinguish user
arguments from the executable path as documented in the process header; a future
entry-policy revision must reconcile those two views explicitly.

## Output, files and failure

Primitive output writes raw bytes and has no knowledge of Display or macro names.
Formatting belongs to ordinary library code. File/line operations return their
declared status and buffer ownership; callers release transferred buffers using
the matching API, not an assumed Vec layout. Panic and bounds traps are terminal;
the defined contract does not silently gain exception unwinding or recovery.

## Target evidence

Under the adopted 0.1-alpha.1 target matrix (see [conformance](conformance.md)),
the supported release-gated targets are `x86_64-pc-windows-gnu` (native/full
regression) and `x86_64-unknown-linux-gnu` (Ubuntu; normal CI + ASan). The audit
verifies Windows x86_64 GNU with LLVM 18 and a CMake Release runtime. macOS,
freestanding, other architectures and the MSVC host triple are **not advertised**
for 0.1-alpha.1; a target that currently compiles is not thereby release-supported.
CTest passing is runtime ABI evidence, not proof of language argument lowering,
every safe stdlib invariant or every target ABI. The contracts retain their
stated hosted/freestanding intent.
