<!-- luna-doc-role: evidence -->

# C-GAP-16: canonical raw-slice count was fixed to u64

The STD-D05 migration exposed a hardcoded u64 check in memory-intrinsic
validation and compiler-generated thunk admission. A renamed trusted `custom<T>(p: *T, count: usize) -> &[T]` reproduces
the mismatch independently of any container. Before repair, both canonical usize
declaration tests fail InvalidAttributeApplication.

The generic existing lang-item validator and exact thunk validator now require usize. Slice indexing also loads the length field as usize in MVIR instead of a u64 fallback. Positive shared
and mutable declarations and negative u64/u32/isize controls run at configured
32- and 64-bit pointer widths. Untrusted bindings, malformed pointer/return
shapes and wrong arity remain negative controls. These are compiler-internal
contract tests; the public stdlib migration acquires CLI/native source/artifact
evidence separately.

The [adopted memory-count amendment](../../spec/0.1/stdlib-memory-size-v1.md)
records the maintainer's STD-D05 authority. Compiler protocol 26 invalidates
pre-amendment bundles; no container identity, opcode or additional intrinsic is
introduced. This is not a stable or 32-bit native qualification claim.

Compiler Change
- Capability: target-sized canonical raw-slice count validation.
- Why stdlib exposed it: public memory counts are migrating to usize.
- Why it is generic: validation follows the existing lang-item identity.
- User-defined declaration benefiting: renamed trusted custom/shared/mutable hooks.
- Tests: memory_intrinsic_contract_tests, followed by public CLI size/index tests.
- New intrinsic/lang_item?: NO.
- Stdlib-specific branch?: NO.
