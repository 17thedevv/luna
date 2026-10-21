<!-- luna-doc-role: evidence -->

# C-GAP-14: imported extern identity

2026-10-21, Asia/Bangkok. Generic compiler repair; no semantic freeze claim.

An output-pointer FFI declaration worked locally but produced E3003 after
moving the unchanged declaration into an imported source provider. The provider
interface copied unsafe-function identities but omitted extern-function
identities. Borrow analysis consequently treated the imported call as an opaque
ordinary function and retained a loan of the local pointer slot.

The repair transports/remaps extern identities in the source interface and
uses the existing `ExternFunction` kind in canonical metadata. Decoding restores
ordinary callable identity plus the independent extern flag. Safety remains
independent: an unsafe extern still requires an unsafe call site. The metadata
schema is unchanged; the canonical interface fingerprint now includes the
correct already-supported kind, and artifacts must be rebuilt.

[User-defined Buffer fixture](../../../tests/luna/language/imported_ffi_output/consumer.ln)
uses only raw FFI, a generic anchored owner and Result. The
[public CLI matrix](../../../luna-rs/crates/luna-cli/tests/imported_ffi_output_cli.rs)
checks native source/fresh artifact behavior, E3003 for a live borrow across the
FFI output write and E2025 for calling the unsafe extern outside unsafe.

Compiler Change
- Capability: preservation of extern declaration identity across providers.
- Why stdlib exposed it: an internal allocation bridge writes a raw output slot.
- Why it is generic: any imported extern with raw pointer arguments needs the same rules as its local declaration.
- User-defined type benefiting: Buffer<T>, independent of stdlib containers.
- Tests: CLI matrix above; final execution status is recorded in this audit's README.
- New intrinsic/lang_item?: NO.
- Stdlib-specific branch?: NO.

The borrow checker and its FFI safety rules are unchanged. The fix restores
their declared input facts; it does not grant no-escape effects or waive loans.
