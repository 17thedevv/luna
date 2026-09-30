# Phase 5 re-audit and closure record — 2026-09-30

## Re-audit finding

The Phase 5 whole-file I/O source was not yet safe to close even though native
POSIX gates passed. A failed destination write could be misclassified by
`copy_file` because MVIR generation treated a resolved fieldless enum-variant
identifier as an irrefutable binding. Nested enum fields had the same issue.
This was a generic compiler lowering defect, not an I/O runtime defect.

Permanent reproducers were added for failed `copy_file` destination writes and
nested enum-variant discrimination. The fix uses resolved semantic symbol
identity, handles nested variants and bindings separately, and gives enum tag
values the matching `u32` semantic type. No provider-name or enum-name
special-case was added.

## Closure evidence

Fix commit: `3dac3ac0e85204411bd80fac15b3294dd812e1be`.
It was fast-forwarded to `main` after the Phase 5 checks passed. Remote `main`
was verified at that exact commit.

GitHub Actions run `36670078044` completed successfully:

- Ubuntu 24.04 and macOS 15 Intel native RuntimeAbiTests passed.
- Whole-file I/O source/fresh-artifact parity passed on both platforms.
- Ubuntu AddressSanitizer partial-read cleanup test passed.
- Ubuntu `cargo test --workspace -- --test-threads=1` exited 0.
- Patch whitespace check exited 0.

The repair also has focused Windows enum and Whole-File I/O acceptance
evidence. The local Windows full-workspace attempt could not launch some test
children because this host lacked `LLVM-C.dll` (`STATUS_DLL_NOT_FOUND`,
`0xC0000135`); native Linux workspace CI is the successful full-workspace
evidence and the Windows limitation is environmental.

## Status

Phase 5 Whole-File I/O v1 is **IMPLEMENTATION COMPLETE** and
**CROSS-PLATFORM VERIFIED**. It remains **NOT FROZEN** pending design-authority
review. The initial audit findings were real and are recorded here as history;
they are resolved by the commit above, not current blockers.
