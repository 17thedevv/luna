<!-- luna-doc-role: evidence -->

# Luna 0.1 Repair Ledger & Verification Record

Baseline Commit: `97d5e8402cefca4cd7f99ffb031a5917d73acb1c`  
Branch: `codex/antigravity-repair-0.1`  
Executor: Antigravity  
Status: **COMPLETE / ALL PASS**  

## Scope & Mandate
This ledger tracks the systematic repair, compiler decoupling, stdlib verification, and release gating specified in `docs/agent-handoff/antigravity-repair-plan-2026-10-03.md`.
All work adheres to the Luna 0.1 architectural principles:
1. No compiler hardcoding of standard library types or macros.
2. Standard library is a consumer of the language; compiler gaps are fixed generically with user-defined reproducers.
3. Genuine Luna syntax and ownership/borrowing semantics.
4. Independent verification of source and artifact modes without fallback.

## Finding & Repair Ledger

| ID | Contract | Status | First Failing Stage | Reproducer / Trigger | Fix Commit | Evidence |
|---|---|---|---|---|---|---|
| W1-POSIX-BUILD | Complete C11 headers & portability for POSIX/Darwin | PASS | runtime_c_compile | `io.c` (`<signal.h>`), `net.c` (`<netdb.h>`) | `a4c83d5` | `runtime/src/io/io.c`, `runtime/src/platform/posix/net.c`, `RuntimeAbiTests` 100% pass |
| C-GAP-04 | Bounded import metadata identity propagation | PASS | `luna-driver::registry` | `tests/luna/language/audit_2026_10_02/provider_bounds/` | `19ba384` | `tests/luna/language/audit_2026_10_02/provider_bounds/` (12 levels dedup verified); `compiler_gap_c_gap_04_provider_chain_bounds_tests.rs` |
| C-GAP-01 | Deterministic inherent/trait method resolution | PASS | `luna-semantic` method selection | `tests/luna/stdlib/audit_2026_10_02/compiler_inherent_trait_dispatch.ln` | `d950f7f` | `docs/proposals/v01-design-01-method-collision.md`; `compiler_inherent_trait_dispatch.ln` (source/artifact pass) |
| C-GAP-03 | Entry argument bridge passing argv to `main(args: [str])` | PASS | `luna-backend` entrypoint lowering | `tests/luna/stdlib/audit_2026_10_02/compiler_main_args.ln` | `d950f7f` | `docs/proposals/entry-argv-policy.md`; `compiler_main_args.ln` (source/artifact pass); `main_entrypoint_tests.rs` |
| C-GAP-02 | Unsigned widening cast preservation | PASS | `luna-mvir` / backend | `tests/luna/stdlib/audit_2026_10_02/compiler_unsigned_cast.ln` | `eb72083` | `compiler_unsigned_cast.ln`; `integer_cast_signedness_tests.rs` |
| COMPOUND-ASSIGN | Compound assignment lowering with LHS expression evaluation | PASS | `luna-mvir/src/generator.rs` | `luna-rs/tests/luna/compiler/numeric_compound_assignment_diagnostic.ln` | `eb72083` | `numeric_compound_assignment_diagnostic.ln`; `compound_assignment_tests.rs` |
| S-04 | Unicode scalar char validation & UTF-8 String safety | PASS | `luna-semantic` / `alloc/string.ln` | `tests/luna/stdlib/audit_2026_10_02/string_invalid_char.ln` | `41484c2` | `crates/luna-cli/tests/char_scalar_cli_parity.rs`; `string_invalid_char.ln` (compile-time rejection) |
| LITERAL-TYPING-01 | Literal typing conformance (integer suffix, arrays) | PASS | `luna-semantic` typechecker | `luna-rs/tests/literal_typing_cli_parity.rs` | `6e8d278` | `crates/luna-cli/tests/literal_typing_cli_parity.rs` |
| S-01 | Vec capacity/reserve arithmetic overflow checks | PASS | `alloc/vec.ln` | `tests/luna/stdlib/audit_2026_10_02/vec_capacity_overflow.ln` | `1ba8518` | `vec_capacity_overflow.ln`; `vec_reserve_overflow.ln` (safe panic) |
| S-02 | RawTable capacity round-up without zero-wrap loop | PASS | `alloc/raw_table.ln` | `tests/luna/stdlib/audit_2026_10_02/hashmap_capacity_overflow.ln` | `1ba8518` | `hashmap_capacity_overflow.ln` (safe panic) |
| S-06 | Generic Vec.dedup without reinterpreting arbitrary T as i32 | PASS | `alloc/vec.ln` `dedup_i32` | `tests/luna/stdlib/audit_2026_10_02/vec_dedup_i32_generic.ln` | `1ba8518` | `vec_dedup_i32_generic.ln` (source/artifact pass) |
| V01-ARCH-02 | Removal of 8 hardcoded macro names & callee map in MacroEngine | PASS | `luna-semantic/src/macro_engine.rs` | MacroEngine inspection | `d950f7f` | `crates/luna-cli/tests/v01_arch_02_macro_decoupling_tests.rs` |
| A-01 | Removal of hardcoded SliceIter layout in compiler | PASS | `luna-semantic` typechecker | typechecker inspection | `6aa260d` | `crates/luna-driver/tests/stdlib_slice_iterator_acceptance_tests.rs`; `libs/external/core/slice.ln` |
| S-03 | Zero-Sized Type (ZST) slice iteration count | PASS | `core/slice.ln` | `tests/luna/stdlib/audit_2026_10_02/slice_zst.ln` | `1ba8518` | `slice_zst.ln`; `slice_zst_mut.ln` (source/artifact pass) |
| S-05 | Filter/find non-Copy item consumption | PASS | `core/iter_adapters.ln` | `tests/luna/stdlib/audit_2026_10_02/filter_owned.ln` | `6aa260d` | `core/iter_adapters.ln`; `core/iter_consumers.ln`; `filter_owned.ln`; `find_owned.ln`; `stdlib_iterator_adapters_acceptance_tests.rs`; `stdlib_iterator_terminal_consumers_acceptance_tests.rs` |
| S-07 | Range<T: Step> advancing without premature move | PASS | `core/iter_adapters.ln` | `tests/luna/stdlib/audit_2026_10_02/range_owned_step.ln` | `1ba8518` | `range_owned_step.ln` (source/artifact pass) |
| V01-GRAMMAR-01 | Comma-separated struct fields; semicolon rejected | PASS | `luna-parser` | `tests/luna/language/spec_v01/reject_semicolon_fields.ln` | `41484c2` | `crates/luna-cli/tests/v01_grammar_acceptance_tests.rs` |
| V01-GRAMMAR-02 | Parenthesized foreach head per Rule K.4 | PASS | `luna-parser` | `tests/luna/language/spec_v01/foreach_parenthesized_contract.ln` | `41484c2` | `crates/luna-cli/tests/v01_grammar_acceptance_tests.rs` |
| V01-GRAMMAR-03 | Receiver shorthand `&self`, `&rw self`, `self` | PASS | `luna-parser` | `tests/luna/language/spec_v01/receiver_shorthand_contract.ln` | `41484c2` | `crates/luna-cli/tests/v01_grammar_acceptance_tests.rs` |
| V01-DIAG-01 | Typed diagnostic codes DIAG-1..10 across compiler phases | PASS | `luna-parser` / error emitter | `tests/luna/language/spec_v01/` error reporting | `41484c2` | `crates/luna-cli/tests/v01_grammar_acceptance_tests.rs` |
| W7-COMPTIME | Portable comptime artifact without host session leakage | PASS | `luna-llib` / `luna-mvir` interp | `tests/luna/language/literal_comptime_provider_gap.ln` | `6e8d278` | `crates/luna-cli/tests/literal_typing_cli_parity.rs (comptime_provider_gap)` |
| W8-ALL-PROVIDERS | 49 stdlib providers contract & API audit matrix | PASS | provider coverage | `luna-rs/libs/external/sysroot.toml` | `8fa8429` | `luna build-sysroot` (49/49 providers verified, 0 orphans, sidecar DAG valid); `crates/luna-driver/tests/test_sysroot_build_invariants.rs` |
| W9-PERF | Profiling and evidence-backed optimization | PASS | benchmarks | `tests/luna/stdlib/audit_2026_10_02/hashmap_collision_pattern.ln` | `1ba8518` | `hashmap_collision_pattern.ln`; `hashmap_growth_control.ln`; `hashmap_sequential_pattern.ln` |

## Verification Environments
- Host 1: Windows 11 x86_64, MinGW-W64 GCC 12.1.0, LLVM 18.1.8, Rust 1.98.0
- Host 2: GitHub Actions `ubuntu-24.04`, GCC 13 / Clang 18, LLVM 18, Rust stable
- Host 3: GitHub Actions `macos-15-intel`, Apple Clang, LLVM 18, Rust stable
