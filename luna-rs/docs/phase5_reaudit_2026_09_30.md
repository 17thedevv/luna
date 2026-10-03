<!-- luna-doc-role: historical -->

> **Luna 0.1 — historical.** Historical design, plan, or evidence. Original wording is preserved for context; it does not independently define current syntax, capability scope or release readiness. See the [versioned specification](../../docs/spec/0.1/README.md).

# Phase 5 re-audit — 2026-09-30

Initial verdict: **PARTIAL — MERGE BLOCKED — NOT FROZEN**. This was superseded
by the resolution below: the enum-pattern lowering defect was fixed, tested,
and merged to `main`; native POSIX verification passed. Current status is
**IMPLEMENTATION COMPLETE — CROSS-PLATFORM VERIFIED — NOT FROZEN**, pending
design-authority review.

Native POSIX execution evidence now exists. The remaining blocker is not a
missing Linux runner: this audit reproduced incorrect public error behavior
caused by generic enum-pattern lowering, plus an ineffective error-variant
test oracle. No compiler/runtime/stdlib behavior was changed by this audit.

## Audited versions and isolation

- Branch: `codex/phase5-posix-gate`.
- Audited HEAD: `cbf26b19558034b6074ae4e23bfa7e167601e940`.
- Successful native CI code: `bb84ae2b9a0ad540d9552029b251b502752e15b3`.
  HEAD differs from that code only by a Phase 5 documentation update.
- Local and remote `main`: `5ecebdd7d99fa78bf90c9fb97896c3f6a7da7231`.
- Built a separate compiler from the clean audited checkout with
  `cargo build -p luna-cli`: **exit 0**. The checkout did not include the
  uncommitted Phase 6 enum fix or FileError formatting changes.
- The relevant committed `generator.rs` and `file.ln` are unchanged between
  `main` and audited HEAD. These are pre-existing defects, not regressions
  introduced by the POSIX verification branch.

## P1 — copy_file can report success after its destination write failed

`libs/external/file/file.ln`, in `copy_file`, matches `write_file` with the
fieldless `std::Result::Ok` arm first. At audited HEAD,
`crates/luna-mvir/src/generator.rs::generate_pat_match` treats every
`Pattern::Identifier` as `true`, even when semantic resolution identifies an
enum variant. The parser represents a fieldless qualified variant with this
pattern form.

Consequently a destination write error enters the success arm and returns
`Ok(bytes.len())`. This violates the public write/copy error contract.

Permanent audit reproducer:
`tests/luna/stdlib/file/copy_file_write_failure.ln`.

The fixture runs in a new isolated directory. It creates a one-byte source,
checks that a direct `write_file` to `.` fails, then requires `copy_file` to
that same destination to fail. Its error oracle puts the payload-bearing
`Err(_)` arm first, avoiding the defective fieldless success pattern.

| Clean compiler / provider mode | Build | Expected run | Actual run |
| --- | --- | --- | --- |
| Source-only providers, zero `.llib` files | exit 0 | exit 0 | **exit 1** |
| Fresh artifact-only providers | exit 0 | exit 0 | **exit 1** |

Exit 1 means `copy_file` returned `Ok`; the direct-write failure control and
source creation controls both passed. The artifact sysroot was rebuilt by the
same clean compiler using `build-sysroot` (**exit 0**), then copied without
source files: **32 `.llib`, 32 `.obj`, zero `.ln`**. Thus this is wrong behavior
in both representations, not a source/artifact divergence.

The uncommitted Phase 6 work already contains a generic top-level variant
check. It was deliberately excluded from the audited commit and has not been
silently bundled into a Phase 5 merge.

## P1 — nested FileError assertions can pass for the wrong error

In `generate_pat_match`, the `Pattern::Enum` field loop skips every child
`Pattern::Identifier`. That includes resolved fieldless enum variants, not
only bindings. The uncommitted top-level variant fix does not change this
skip, so it is not sufficient to close the nested case.

`tests/luna/stdlib/file/whole_file_io_v1.ln` uses patterns such as
`Result::Err(FileError::NotFound)` and `Result::Err(FileError::InvalidInput)`.
Those assertions check the outer `Err` tag but skip the inner error tag.
Their existing green results therefore do not prove the claimed taxonomy.

Permanent compiler-only audit reproducer:
`tests/luna/compiler/nested_enum_variant_discrimination.ln`.

It supplies `Failure(Invalid)` to a `Failure(Missing)` pattern. Expected
executable exit is 0; the clean audited compiler builds successfully but the
executable returns **1**, identifying the false match. The existing local
compiler binary also returned 1. No stdlib names or I/O are involved.

Both new fixtures express the intended successful behavior (exit 0). They
are retained as failing audit reproducers, not tests that celebrate the bug
by expecting exit 1. Harness integration belongs to the subsequent repair.

## Native evidence that remains valid

[GitHub Actions run 36661193516](https://github.com/17thedevv/luna/actions/runs/36661193516)
was rechecked through the GitHub API: completed, conclusion `success`, at the
code commit listed above.

| Gate | Recorded result |
| --- | --- |
| Ubuntu 24.04 native runtime / RuntimeAbiTests | PASS, 1/1 |
| macOS 15 Intel native runtime / RuntimeAbiTests | PASS, 1/1 |
| Ubuntu Whole-File I/O source/fresh-artifact suite | PASS, 2/2 existing tests |
| macOS Whole-File I/O source/fresh-artifact suite | PASS, 2/2 existing tests |
| Ubuntu runtime partial-read cleanup under ASan | PASS, 1/1 |
| Ubuntu full workspace | `cargo test --workspace -- --test-threads=1`, exit 0 |
| CI whitespace check | exit 0 |

Source inspection confirms read-until-observed-EOF, size-as-capacity-hint,
growth, partial-buffer cleanup, POSIX EINTR handling, and write-until-complete.
The shared read-loop test injects interruption/growth/error through a callback;
it is not evidence of an actual signal interrupting a POSIX `read` syscall.
Existing exact-byte/empty/truncation and runtime tests remain useful evidence.
Their success does not override the new public API counterexample.

Both `git diff --check origin/main...HEAD` and the working-tree
`git diff --check` passed during this audit. Existing formatting warnings and
unrelated dirty Phase 6 work were not rewritten. No full workspace rerun was
performed for this audit: it changes no production behavior, and the focused
counterexamples already invalidate closure.

## Initial merge decision and next step

At the time of this initial audit, **no merge, main update, or push was
performed**. The user authorized merging only if the audit passed; the public
API counterexample prevented that at that stage.

Repair generic pattern lowering and strengthen the Phase 5 error matrix
before merging. Do not reorder stdlib match arms or replace them with a
special-case workaround. Do not assign a new gap ID without checking the
existing registry/history. See [the Phase 6 completion plan](phase6_completion_plan.md)
for the prerequisite repair, commit boundaries, and subsequent formatting work.

SKILL IMPACT: none. The findings are task-specific defects; existing audit
and capability-validation guidance already requires checking executable
behavior and negative-test oracles.

## Resolution addendum — 2026-09-30

The generic enum-lowering defect described above was fixed and independently
regressed in commit `3dac3ac0e85204411bd80fac15b3294dd812e1be`:

- Enum variant patterns now use the resolver's `pat_symbols` identity instead
  of global symbol-name suffix matching.
- Nested field patterns distinguish resolved enum variants from identifier
  bindings and wildcards.
- MVIR enum `Tag` values are represented as `u32`, matching the backend tag
  representation.
- Permanent compiler fixtures cover nested matching and duplicate terminal
  variant names; the Phase 5 executable acceptance now requires `copy_file` to
  return an error when its destination write fails.

Focused Windows verification passed:

- `enum_tag_codegen_tests`: 1/1.
- `whole_file_io_v1_acceptance_tests`: 2/2, including source/fresh-artifact
  parity and failed destination write propagation.
- `git diff --check HEAD^ HEAD`: exit 0.

Native CI for commit `3dac3ac` completed successfully in run
`36670078044`:

| Gate | Result |
| --- | --- |
| Ubuntu 24.04 RuntimeAbiTests | PASS |
| macOS 15 Intel RuntimeAbiTests | PASS |
| Ubuntu Whole-File I/O source/fresh-artifact parity | PASS |
| macOS Whole-File I/O source/fresh-artifact parity | PASS |
| Ubuntu partial-read cleanup under ASan | PASS |
| Ubuntu full workspace regression | `cargo test --workspace -- --test-threads=1`, exit 0 |
| Patch whitespace check | exit 0 |

The clean-worktree Windows full workspace attempt could not execute CLI test
children because the local machine lacked `LLVM-C.dll` (`STATUS_DLL_NOT_FOUND`,
exit `0xC0000135`). The same full workspace gate completed successfully on the
native Ubuntu runner, so this local environment limitation did not substitute
for or weaken the required regression gate.

After verifying `origin/main` was an ancestor and the branch worktree was
clean, the passing commit was fast-forwarded to `main`. Remote `main` now points
at `3dac3ac0e85204411bd80fac15b3294dd812e1be`. Phase 5 Whole-File I/O v1 is
implementation-complete and cross-platform verified, but remains NOT FROZEN
pending design-authority review. The initial findings above remain as a dated
record of the defect that the resolution commit closed.
