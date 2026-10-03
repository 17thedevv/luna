<!-- luna-doc-role: historical -->

> **Luna 0.1 — historical.** Historical design, plan, or evidence. Original wording is preserved for context; it does not independently define current syntax, capability scope or release readiness. See the [versioned specification](../../docs/spec/0.1/README.md).

# Stage 7 — Phase 4C Freeze Audit

**Verdict: implementation and closure evidence complete; Phase 4C surface frozen.**

Namespace note: this record originally described the Phase 4C API before
STD-NAMESPACE-01. The canonical names for migrated ordinary-core symbols are
now `std::Convert`, `std::TryConvert`, `std::TryConvertError`, `std::Default`,
`std::FloatOps`, and `std::Result`; this updates naming only, not Phase 4C
semantics. Container namespace migration remains a separate slice.

## Frozen public surface

- `Convert<Target>` and `TryConvert<Target, TryConvertError>` are ordinary stdlib traits.
- `TryConvertError` is the enum `Overflow | Underflow | Invalid`.
- `Default` covers the primitive types (including `usize` and `isize`), `Option<T>`, `Vec<T>`, `HashMap<K, V>`, and `HashSet<T>`. Empty `HashMap`/`HashSet` construction has no `Hash`/`Eq` bound; bounds remain on operations that require them.
- `FloatOps` is an ordinary stdlib trait implemented by `f32` and `f64`. Method syntax uses normal trait dispatch; the compiler does not recognize float method names. No primitive inherent methods or associated constants were added.
- The Phase 4C float foundation is bit representation (`to_bits`/`from_bits`), classification and sign predicates, `abs`/`min`/`max`/`clamp`, and NaN/±Infinity factories.
- `sqrt`, `floor`, `ceil`, `round`, `trunc`, and `fract` are outside this frozen surface and deferred to the separate `CORE-GAP-18 Float Math` design, backed by the platform math ABI/compiler-supported primitive mechanism and its own accuracy/domain contracts.

## Conversion and cast evidence

`tests/stdlib_phase4c_numerics.rs::test_standard_conversions` covers the accepted integer and float success cases and the `Overflow`, `Underflow`, and `Invalid` paths, including NaN and both infinities. Float inputs are checked before an `as` cast; `std::TryConvert` returns `std::Result::Err` and does not delegate invalid inputs to a trapping cast. Runtime `as` conversion retains deterministic trapping behavior.

`test_float_to_int_comptime_casts` permanently covers `const VALUE: i8 = 128.0 as i8;` and requires a numeric conversion diagnostic before code generation. A valid comptime float literal is not claimed: current backend parsing rejects a valid `127.9` literal as an invalid numeric operand, so that behavior remains unsupported rather than being silently presented as verified.

The float-to-`i32` `TryConvert` lower-bound regression is covered at `f32` `i32::MIN`; the exact minimum converts successfully.

## Artifact and identity contract

`stdlib_phase4c_llib_parity_tests.rs::phase4c_fresh_session_source_vs_llib_parity` runs the same consumer against independently prepared sysroots in fresh compilation sessions. Source mode has no `.llib` or `.obj`; artifact mode is rebuilt from the isolated source copy and then has all `.ln` files removed (the test asserts the count is zero). It compares compile success, exit code, stdout, and stderr while exercising concrete and blanket `Convert`, all three `TryConvert` errors, `Default` for primitive/`Option`/`Vec`/`HashMap`, and `FloatOps`/bit/factory APIs.

Artifact-local declaration references are relocated into fresh session-local `DeclId`s during load. `ProviderInterface.method_to_impl_decl` contains only current-session IDs reconstructed after relocation. `DeclId` is not a portable artifact identity; raw `DeclId` values are not serialized as stable metadata.

## Gap bookkeeping and compiler repairs

- `C-GAP-10` remains **Loss of Instantiated Impl-Head Identity — RESOLVED & FROZEN**. Default has no C-GAP ID.
- C-GAP-09 runtime cast behavior remains covered; invalid comptime float-to-int casts now have a permanent diagnostic reproducer.
- The Phase 4C generic method-selection repair uses contextual result type and substitutes selected trait-impl generic arguments. The additional Phase 4C compiler repairs (provider provenance, recursive generic substitution/mangling, and COMDAT identity) are recorded as Phase 4C compiler defects; no existing C-GAP number is reused or redefined.
- Floating-point guarantees are described as Luna's frozen IEEE-754 floating-point semantics, not complete IEEE-754 compliance.

## Final verification snapshot

- Phase 4C numerics: **6 passed**.
- Fresh-session source/`.llib` parity: **1 passed**.
- Sysroot build invariants: **5 passed**.
- Workspace regression: `cargo test --workspace -- --test-threads=1` — **exit 0**.
- Runtime ABI regression: **PASS**, based on the supplied CTest log (`RuntimeAbiTests` passed in 35.82 seconds; 100% passed). It was not rerun because runtime code did not change.
- Canonical sysroot: **29 providers, 79 direct dependency edges**. The pre-Option-Default graph had 78; `Default for Option<T>` adds the required `core/default -> __lang_option` edge, so the post-change source-derived invariant is 79.
