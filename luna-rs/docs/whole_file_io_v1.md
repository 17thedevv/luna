# WHOLE-FILE-IO-v1

Status: implemented, pending design/freeze review. This document does not freeze
Phase 5.

## Public surface

The physical provider is `file`; its public logical exports are all under
`std::`:

```luna
std::FileError
std::read_file(path: &std::String) -> std::Result<std::Vec<u8>, std::FileError>
std::write_file(path: &std::String, bytes: &[u8]) -> std::Result<void, std::FileError>
std::copy_file(src: &std::String, dst: &std::String) -> std::Result<u64, std::FileError>
```

`std::FileError` has exactly four variants: `NotFound`, `PermissionDenied`,
`InvalidInput`, and `Io`. There is no `std::fs` module and no root/provider-name
aliases.

## Error mapping

The runtime status mapping is deliberately conservative:

| Runtime condition/status | Public result |
| --- | --- |
| `NOT_FOUND` (`-1`) | `FileError::NotFound` |
| `PERMISSION_DENIED` (`-2`) | `FileError::PermissionDenied` |
| `IO_ERROR` (`-5`) | `FileError::Io` |
| `INVALID_ARGUMENT` (`-4`) | `FileError::Io` |
| Embedded NUL detected in the stdlib before the runtime call | `FileError::InvalidInput` |

The runtime status type is not exposed. The stdlib does not claim finer-grained
OS error classification, and does not expose `AlreadyExists` or recoverable
`OutOfMemory` errors.

## Read and ownership

`read_file` returns the exact bytes read, without UTF-8 validation; zero bytes
inside file contents are ordinary data. An empty file produces an empty Vec.
The runtime reads sequentially from offset zero until it observes EOF. Initial
file-size metadata is only a capacity hint and never a read limit.

The runtime returns an owned `(ptr, len, cap)` buffer. The stdlib does not adopt
that allocation as Vec storage: it copies the bytes into an ordinary
`std::Vec<u8>` and frees the runtime buffer exactly once using `cap`, not `len`.
Private output cells for the existing C out-parameter ABI are separately
allocated and freed through the existing runtime allocator. No runtime ABI
signature or public allocator API is added by this feature.

This is not a filesystem snapshot API. If a file shrinks while being read, an
earlier EOF may yield a shorter prefix. If it grows before EOF is observed,
newly reachable bytes may be included. Bytes appended after EOF has already
been observed are not required to be included. No lock, atomic snapshot, or
concurrent-mutation consistency guarantee is provided.

## Write and copy

`write_file` accepts arbitrary bytes. A missing destination is created where
the OS permits; an existing destination is truncated before writing. `Ok`
means every requested byte was written and closing succeeded. A write failure
after truncation or partial progress may leave the destination modified. There
is no atomic replacement, rollback, or preservation guarantee.

`copy_file` reads the complete source, writes the bytes to the destination, and
returns the number of bytes copied as `u64`. This whole-file buffering is an
implementation detail. It rejects identical textual source and destination
paths with `InvalidInput`; it does not resolve aliases, normalize paths, inspect
symlinks, or compare filesystem identity.

## Path boundary and non-goals

Paths are UTF-8 `std::String` values. The frozen `std::path` module remains
lexical, uses `/` as its canonical separator, and does not normalize `.` or
`..`, interpret drive letters, or implement UNC semantics. At the OS boundary,
embedded NUL is rejected rather than truncated. Windows runtime code performs
the existing UTF-8-to-UTF-16 conversion. POSIX paths that are not representable
as UTF-8 are outside v1.

V1 does not include file handles, streaming, buffering APIs, directories,
metadata, permissions APIs, symlink operations, path canonicalization, rename,
remove, mkdir, environment, time, threads, async I/O, `OsString`/`OsPath`,
zero-copy Vec adoption, recoverable OOM, atomic writes, locking, or snapshot
semantics.

## Native platform verification status

Native POSIX evidence passed for commit `bb84ae2b9a0ad540d9552029b251b502752e15b3`
in [GitHub Actions run 36661193516](https://github.com/17thedevv/luna/actions/runs/36661193516):

- Ubuntu 24.04 and macOS 15 Intel runtime builds succeeded; `RuntimeAbiTests`
  passed 1/1 on each platform.
- Whole-File I/O source/fresh-artifact parity passed 2/2 on both platforms.
- Ubuntu AddressSanitizer runtime ABI suite passed 1/1, including partial-read
  cleanup coverage.
- `cargo test --workspace -- --test-threads=1` completed with **exit code 0**;
  the following `git diff --check` step also succeeded. The workflow run
  concluded `success`.
- The clean-checkout workspace setup builds canonical provider artifacts with
  `cargo run -p luna-cli -- build-sysroot --quiet` before the workspace suite;
  those generated artifacts are not committed.

The CI result establishes native Linux and macOS build/runtime evidence in
addition to the existing Windows verification. Phase 5 remains **NOT FROZEN**;
implementation is **CROSS-PLATFORM VERIFIED** and **READY FOR DESIGN/FREEZE
REVIEW**.
