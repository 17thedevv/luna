<!-- luna-doc-role: evidence -->

# Luna 0.1 Repair Ledger & Verification Record

Baseline Commit: `97d5e8402cefca4cd7f99ffb031a5917d73acb1c`  
Branch: `codex/antigravity-repair-0.1`  
Executor: Antigravity  
Status: **PARTIAL — IN PROGRESS (Full diagnostic contract and native multi-platform CI pending)**

Latest compiler integrity follow-up: [2026-10-03 repair and verification](compiler-integrity-repair-2026-10-03.md). The three independently reproduced defects are addressed there; this does not certify the whole diagnostic contract or all provider APIs.

## Scope & Mandate
This ledger tracks the systematic repair, compiler decoupling, stdlib verification, and release gating specified in `docs/agent-handoff/antigravity-repair-plan-2026-10-03.md`.
All work adheres to the Luna 0.1 architectural principles:
1. No compiler hardcoding of standard library types or macros.
2. Standard library is a consumer of the language; compiler gaps are fixed generically with user-defined reproducers.
3. Genuine Luna syntax and ownership/borrowing semantics.
4. Independent verification of source and artifact modes without fallback.

## Finding & Repair Ledger

| ID | Contract | Status | First Failing Stage | Reproducer / Trigger | Fix Commit | Evidence / Gap Analysis |
|---|---|:---:|---|---|---|---|
| W1-POSIX-BUILD | Complete C11 headers & portability for POSIX/Darwin | PARTIAL | runtime_c_compile | `io.c` (`<signal.h>`), `net.c` (`<netdb.h>`) | `a4c83d5` | Headers added; `RuntimeAbiTests` passes on Windows MinGW GCC; native Ubuntu/macOS CI evidence pending |
| A1-P0 | Backend fail-closed without silent dummy value fallback | PASS | `luna-backend/src/llvm_codegen.rs` | Wildcard fallback at `llvm_codegen.rs:1836` | - | Wildcard fallback removed; unhandled instructions fail closed with `BackendError::InvariantViolation`; `backend_fail_closed_tests.rs` pass |
| C-GAP-04 | Bounded import metadata identity propagation | PASS | `luna-driver::registry` | `tests/luna/language/audit_2026_10_02/provider_bounds/` | `19ba384` | 12-level provider bounds dedup verified; `compiler_gap_c_gap_04_provider_chain_bounds_tests.rs` pass |
| C-GAP-01 | Deterministic inherent/trait method resolution | PASS | `luna-semantic` method selection | `tests/luna/stdlib/audit_2026_10_02/compiler_inherent_trait_dispatch.ln` | `d950f7f` | `docs/proposals/v01-design-01-method-collision.md`; inherent method priority verified |
| C-GAP-03 | Entry argument bridge passing argv to `main(args: [str])` | PASS | `luna-backend` entrypoint lowering | `tests/luna/stdlib/audit_2026_10_02/compiler_main_args.ln` | `d950f7f` | `docs/proposals/entry-argv-policy.md`; `main_entrypoint_tests.rs` pass |
| C-GAP-02 | Unsigned widening cast preservation | PASS | `luna-mvir` / backend | `tests/luna/stdlib/audit_2026_10_02/compiler_unsigned_cast.ln` | `eb72083` | `compiler_unsigned_cast.ln`; `integer_cast_signedness_tests.rs` pass |
| COMPOUND-ASSIGN | Compound assignment lowering with LHS single-evaluation | PASS | `luna-mvir/src/generator.rs` | `numeric_compound_assignment_diagnostic.ln` | `eb72083` | `compound_assignment_tests.rs` (3/3 pass) |
| S-04 | Unicode scalar char validation & UTF-8 String safety | PASS | `luna-semantic` / `alloc/string.ln` | `tests/luna/stdlib/audit_2026_10_02/string_invalid_char.ln` | `41484c2` | `char_scalar_cli_parity.rs` pass; dynamic and static surrogate casts rejected |
| LITERAL-TYPING-01 | Literal typing conformance (integer suffix, arrays) | PASS | `luna-semantic` typechecker | `luna-rs/tests/literal_typing_cli_parity.rs` | `6e8d278` | `literal_typing_cli_parity.rs` pass |
| S-01 | Vec capacity/reserve arithmetic overflow checks | PASS | `alloc/vec.ln` | `tests/luna/stdlib/audit_2026_10_02/vec_capacity_overflow.ln` | `1ba8518` | `vec_capacity_overflow.ln`; `vec_reserve_overflow.ln` (safe panic exit 3) |
| S-02 | RawTable capacity round-up without zero-wrap loop | PASS | `alloc/raw_table.ln` | `tests/luna/stdlib/audit_2026_10_02/hashmap_capacity_overflow.ln` | `1ba8518` | `hashmap_capacity_overflow.ln` (safe panic exit 3) |
| S-06 | Generic Vec.dedup without reinterpreting arbitrary T as i32 | PASS | `alloc/vec.ln` `dedup_i32` | `tests/luna/stdlib/audit_2026_10_02/vec_dedup_i32_generic.ln` | - | `dedup_i32` delegates safely to `self.dedup()` with nominal Eq comparison; verified with `vec_dedup_4byte_nominal.ln` and `vec_dedup_i32_generic.ln` |
| V01-ARCH-02 | Removal of 8 hardcoded macro names & callee map in MacroEngine | PASS | `luna-semantic/src/macro_engine.rs` | MacroEngine inspection | `d950f7f` | `crates/luna-cli/tests/v01_arch_02_macro_decoupling_tests.rs` pass; user-defined macros expand cleanly |
| A-01 | Removal of hardcoded SliceIter layout in compiler | PASS | `luna-semantic` typechecker | typechecker inspection | `6aa260d` | `SliceIter` hardcoded struct synthesis removed; `SemanticContext.is_slice_authorized` capability securely restricts slice inherent impls to authorized sysroot build context; impersonation rejected; `p1a_tests.rs` (Cases 19-21) and CLI tests pass |
| S-03 | Zero-Sized Type (ZST) slice iteration count | PASS | `core/slice.ln` | `tests/luna/stdlib/audit_2026_10_02/slice_zst.ln` | `1ba8518` | `slice_zst.ln`; `slice_zst_mut.ln` pass in both modes |
| S-05 | Filter/find non-Copy item consumption | PASS | `core/iter_adapters.ln` | `tests/luna/stdlib/audit_2026_10_02/filter_owned.ln` | `6aa260d` | RFC `docs/proposals/v01-filter-step-dedup-api.md` adopted; borrowed predicate `fn(&Item) -> bool` implemented; `filter_owned.ln` and `find_owned.ln` pass |
| S-07 | Range<T: Step> advancing without premature move | PASS | `core/iter_adapters.ln` | `tests/luna/stdlib/audit_2026_10_02/range_owned_step.ln` | `1ba8518` | `range_owned_step.ln` pass in both modes via `mem::replace` |
| V01-GRAMMAR-01 | Comma-separated struct fields; semicolon rejected | PASS | `luna-parser` | `tests/luna/language/spec_v01/reject_semicolon_fields.ln` | `41484c2` | `v01_grammar_acceptance_tests.rs` pass |
| V01-GRAMMAR-02 | Parenthesized foreach head per Rule K.4 | PASS | `luna-parser` | `tests/luna/language/spec_v01/foreach_parenthesized_contract.ln` | `41484c2` | `v01_grammar_acceptance_tests.rs` pass |
| V01-GRAMMAR-03 | Receiver shorthand `&self`, `&rw self`, `self` | PASS | `luna-parser` | `tests/luna/language/spec_v01/receiver_shorthand_contract.ln` | `41484c2` | `v01_grammar_acceptance_tests.rs` pass |
| V01-DIAG-01 | Typed diagnostic codes DIAG-1..10 across compiler phases | PARTIAL | `luna-parser` / error emitter | `tests/luna/language/spec_v01/` error reporting | `41484c2` | Typed diagnostic codes E0001..E0003 (parser), E1001 (unresolved method), E2001 (type mismatch), E2006 (coherence orphan), E2026 (invalid cast) verified across compiler phases in `v01_grammar_acceptance_tests.rs` |
| W7-COMPTIME | Portable comptime artifact without host session leakage | PASS | `luna-llib` / `luna-mvir` interp | `literal_comptime_provider_gap.ln` | `6e8d278` | `literal_typing_cli_parity.rs` pass |
| W8-ALL-PROVIDERS | 49 stdlib providers contract & API audit matrix | PARTIAL | provider coverage | `luna-rs/libs/external/sysroot.toml` | `8fa8429` | Inventory & build infrastructure: 49/49 providers build in sysroot DAG; `test_sysroot_build_invariants.rs` pass (6/6); Tier-1 Core/Alloc certified; Tier-2/3 detailed API mapping in `docs/audits/repair-0.1/provider_matrix.md` |
| W9-PERF | Profiling and evidence-backed optimization | PASS | benchmarks | `hashmap_collision_pattern.ln` | - | SplitMix64 bit mixer in `core/hash.ln` and `alloc/raw_table.ln`; raw_table insert/insert_if_absent/get_or_insert probe first before grow, eliminating redundant rehash/growth on overwrite and hits; clear() retains capacity; benchmark in `docs/audits/repair-0.1/evidence/hashmap-benchmark.json` shows 132x speedup on low_bits_collision n=12000 (0.0259s vs 3.4264s) |

## Verification Environments
- Host 1: Windows 11 x86_64, MinGW-W64 GCC 12.1.0, LLVM 18.1.8, Rust 1.98.0 (Verified locally: all Windows suites PASS)
- Host 2: GitHub Actions `ubuntu-24.04`, GCC 13 / Clang 18, LLVM 18, Rust stable (Pending CI run)
- Host 3: GitHub Actions `macos-15-intel`, Apple Clang, LLVM 18, Rust stable (Pending CI run)
