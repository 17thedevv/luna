<!-- luna-doc-role: evidence -->

# Luna 0.1 Repair Ledger & Verification Record

Baseline Commit: `97d5e8402cefca4cd7f99ffb031a5917d73acb1c`  
Branch: `codex/antigravity-repair-0.1`  
Executor: Antigravity  
Status: **IN PROGRESS**  

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
| W1-POSIX-BUILD | Complete C11 headers & portability for POSIX/Darwin | IN_PROGRESS | runtime_c_compile | `io.c` (`<signal.h>`), `net.c` (`<netdb.h>`) | pending | `evidence/w1-runtime-posix.log` |
| C-GAP-04 | Bounded import metadata identity propagation | OPEN | `luna-driver::registry` | `tests/luna/language/audit_2026_10_02/provider_bounds/` | - | - |
| C-GAP-01 | Deterministic inherent/trait method resolution | OPEN | `luna-semantic` method selection | `tests/luna/stdlib/audit_2026_10_02/compiler_inherent_trait_dispatch.ln` | - | - |
| C-GAP-03 | Entry argument bridge passing argv to `main(args: [str])` | OPEN | `luna-backend` entrypoint lowering | `tests/luna/stdlib/audit_2026_10_02/compiler_main_args.ln` | - | - |
| C-GAP-02 | Unsigned widening cast preservation | OPEN | `luna-mvir` / backend | `tests/luna/stdlib/audit_2026_10_02/compiler_unsigned_cast.ln` | - | - |
| COMPOUND-ASSIGN | Compound assignment lowering with LHS expression evaluation | OPEN | `luna-mvir/src/generator.rs` | `luna-rs/tests/luna/compiler/numeric_compound_assignment_diagnostic.ln` | - | - |
| S-04 | Unicode scalar char validation & UTF-8 String safety | OPEN | `luna-semantic` / `alloc/string.ln` | `tests/luna/stdlib/audit_2026_10_02/string_invalid_char.ln` | - | - |
| LITERAL-TYPING-01 | Literal typing conformance (integer suffix, arrays) | OPEN | `luna-semantic` typechecker | `luna-rs/tests/literal_typing_cli_parity.rs` | - | - |
| S-01 | Vec capacity/reserve arithmetic overflow checks | OPEN | `alloc/vec.ln` | `tests/luna/stdlib/audit_2026_10_02/vec_capacity_overflow.ln` | - | - |
| S-02 | RawTable capacity round-up without zero-wrap loop | OPEN | `alloc/raw_table.ln` | `tests/luna/stdlib/audit_2026_10_02/hashmap_capacity_overflow.ln` | - | - |
| S-06 | Generic Vec.dedup without reinterpreting arbitrary T as i32 | OPEN | `alloc/vec.ln` `dedup_i32` | `tests/luna/stdlib/audit_2026_10_02/vec_dedup_i32_generic.ln` | - | - |
| V01-ARCH-02 | Removal of 8 hardcoded macro names & callee map in MacroEngine | OPEN | `luna-semantic/src/macro_engine.rs` | MacroEngine inspection | - | - |
| A-01 | Removal of hardcoded SliceIter layout in compiler | OPEN | `luna-semantic` typechecker | typechecker inspection | - | - |
| S-03 | Zero-Sized Type (ZST) slice iteration count | OPEN | `core/slice.ln` | `tests/luna/stdlib/audit_2026_10_02/slice_zst.ln` | - | - |
| S-05 | Filter/find non-Copy item consumption | OPEN | `core/iter_adapters.ln` | `tests/luna/stdlib/audit_2026_10_02/filter_owned.ln` | - | - |
| S-07 | Range<T: Step> advancing without premature move | OPEN | `core/iter_adapters.ln` | `tests/luna/stdlib/audit_2026_10_02/range_owned_step.ln` | - | - |
| V01-GRAMMAR-01 | Comma-separated struct fields; semicolon rejected | PASS | `luna-parser` | `tests/luna/language/spec_v01/reject_semicolon_fields.ln` | `41484c2` | `crates/luna-cli/tests/v01_grammar_acceptance_tests.rs` |
| V01-GRAMMAR-02 | Parenthesized foreach head per Rule K.4 | PASS | `luna-parser` | `tests/luna/language/spec_v01/foreach_parenthesized_contract.ln` | `41484c2` | `crates/luna-cli/tests/v01_grammar_acceptance_tests.rs` |
| V01-GRAMMAR-03 | Receiver shorthand `&self`, `&rw self`, `self` | PASS | `luna-parser` | `tests/luna/language/spec_v01/receiver_shorthand_contract.ln` | `41484c2` | `crates/luna-cli/tests/v01_grammar_acceptance_tests.rs` |
| V01-DIAG-01 | Typed diagnostic codes DIAG-1..10 across compiler phases | PASS | `luna-parser` / error emitter | `tests/luna/language/spec_v01/` error reporting | `41484c2` | `crates/luna-cli/tests/v01_grammar_acceptance_tests.rs` |
| W7-COMPTIME | Portable comptime artifact without host session leakage | PASS | `luna-llib` / `luna-mvir` interp | `tests/luna/language/literal_comptime_provider_gap.ln` | `6e8d278` | `crates/luna-cli/tests/literal_typing_cli_parity.rs` |
| W8-ALL-PROVIDERS | 49 stdlib providers contract & API audit matrix | OPEN | provider coverage | `luna-rs/libs/external/sysroot.toml` | - | - |
| W9-PERF | Profiling and evidence-backed optimization | OPEN | benchmarks | `tests/luna/stdlib/audit_2026_10_02/hashmap_collision_pattern.ln` | - | - |

## Verification Environments
- Host 1: Windows 11 x86_64, MinGW-W64 GCC 12.1.0, LLVM 18.1.8, Rust 1.98.0
- Host 2: GitHub Actions `ubuntu-24.04`, GCC 13 / Clang 18, LLVM 18, Rust stable
- Host 3: GitHub Actions `macos-15-intel`, Apple Clang, LLVM 18, Rust stable
