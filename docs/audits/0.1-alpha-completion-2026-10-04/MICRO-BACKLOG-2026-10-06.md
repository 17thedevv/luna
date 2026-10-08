<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Micro-backlog thực thi cho 0.1-alpha.1. Không thêm
> contract/ngôn ngữ mới; chỉ phân rã công việc đã có trong tài liệu. Baseline và
> adopted amendment vẫn là authority; tài liệu này không phải nghiệm thu compiler.

# Micro-backlog thực thi — Luna 0.1-alpha.1 (2026-10-06)

Tách từ [kế hoạch còn lại](REMAINING-PLAN-2026-10-06.md) và
[kế hoạch gốc 2026-10-04](README.md). Tham chiếu ledger [EXECUTION](EXECUTION.md),
contract [CALL-ARGUMENTS-v1](../../spec/0.1/call-arguments-v1.md) và
[conformance gates](../../spec/0.1/conformance.md).

## 0. Quy ước thực thi

```
1 task → 1 mục tiêu → 1 finding/root cause hoặc 1 thay đổi
      → focused test → evidence → commit riêng nếu có code change
```

Không giao kiểu "fix toàn bộ ownership"; chỉ giao một lát nhỏ, ví dụ
"A1-06: Add MVIR cleanup emission for one unused owned closure capture".

## Definition of Done (template bắt buộc)

```
TASK:
PARENT TASK:
DEPENDENCIES:
BASE SHA:
IMPLEMENTATION SHA:
TARGET:
MODE:
REPRODUCER:
ORIGINAL FAILURE:
ROOT CAUSE:
CONTRACT:
CHANGED FILES:
FOCUSED COMMAND:
FOCUSED RESULT:
REGRESSION COMMAND:
REGRESSION RESULT:
ARTIFACT MODE:
REMAINING GAPS:
VERDICT:
```

`VERDICT=PASS` chỉ hợp lệ khi evidence đúng layer. Focused PASS không được dùng
để biến một workspace FAIL trước đó thành PASS; verdict mới phải chạy lại trên
exact candidate SHA.

## Phát hiện C1 (đổi backlog)

C1 **không** còn là feature "chưa triển khai". Nhánh `codex/call-arguments-0.1`
có 5 commit trên nền `3601d12e`:

```
2615b5f8 feat(calls): execute synchronous defaults with concrete binder and lifetime plans
ce417f19 feat(calls): check default declarations and preserve canonical binding identity
f36a5abc fix(comptime): admit prepared programs through ordinary loan checks
c0818477 fix(comptime): preserve concrete root call plans before early execution
1b398714 feat(calls): bind named arguments and preserve evaluation order across artifacts
```

Có 3 harness (`named_arguments_cli`, `default_arguments_cli`, `default_omission_cli`),
72 fixture (`tests/luna/language/named_arguments`, `default_arguments`), và bump
artifact compiler 19 / MVIR 5 / metadata 9. Do đó C1-01..C1-52 đổi trạng thái
thành `IMPLEMENTED_ON_SIDE_BRANCH` (chỉ trace) và thay bằng nhóm reconcile C1-R*.

## Execution sequence

```
PHASE 0  DOC-01
PHASE 1  R0-B01..R0-B24
PHASE 2  R0-H01..R0-H34
PHASE 3  C1-R01..C1-R30 + VR-01..VR-10
PHASE 4  baseline verdict vs candidate verdict
PHASE 5  A1 + A4 + D2, then A2/A3/A5/A6
PHASE 6  C2/C3/C4/C5
PHASE 7  D1/D3/D4
PHASE 8  E1/E2
PHASE 9  R4
PHASE 10 R5
```

Baseline (`codex/antigravity-repair-0.1` @ exact SHA) phải lấy trước mọi merge C1.
Reconcile C1 chỉ trong candidate branch `codex/luna-0.1-alpha.1-candidate`.

## Wave 0 — Quyết định maintainer

| ID | Micro-task | Output |
|---|---|---|
| DEC-01 | Đọc lại contract closure/callable hiện tại | Note 1 trang |
| DEC-02 | Liệt kê hành vi khi closure consume owned capture | Matrix |
| DEC-03 | Reducer cho closure gọi lại sau consume | .ln fixture |
| DEC-04 | So sánh 3 policy: allow / compile reject / runtime fail | Decision table |
| DEC-05 | Chọn D1 policy | Maintainer decision |
| DEC-06 | Gán E-code nếu D1 chọn compile reject | Contract amendment |
| DEC-07 | Xác định signed overflow cases | Matrix |
| DEC-08 | Xác định unsigned overflow cases | Matrix |
| DEC-09 | Xác định shift overflow/invalid shift cases | Matrix |
| DEC-10 | Xác định integer division edge cases | Matrix |
| DEC-11 | Chọn debug/release consistency policy | D2 decision |
| DEC-12 | Liệt kê targets compiler hiện chạy thực tế | Target inventory |
| DEC-13 | Chốt Windows GNU support | D3 |
| DEC-14 | Chốt Ubuntu support + ASan | D3 |
| DEC-15 | Chốt macOS support status | D3 |
| DEC-16 | Chốt freestanding advertised/not advertised | D3 |
| DEC-17 | Review NAMESPACE-USING-v1 scope | retain/defer |
| DEC-18 | Review PROVIDER-CONFIG-v1 scope | retain/defer |
| DEC-19 | Review MODULE-CONST-STORAGE-v1 scope | retain/defer |
| DEC-20 | Review CALL-ARGUMENTS-v1 scope | retain/defer |
| DEC-21 | Ghi D1–D4 vào authority docs | committed decision |

Gate: không bắt đầu patch phụ thuộc D1/D2/D3 trước khi decision được ghi lại.

## Wave 1 — R0 Baseline

| ID | Task | ID | Task |
|---|---|---|---|
| R0-B01 | Checkout exact candidate SHA | R0-B13 | Xác nhận provider count từ manifest |
| R0-B02 | Ghi git rev-parse HEAD | R0-B14 | Xác nhận 49 provider |
| R0-B03 | Ghi branch hiện tại | R0-B15 | CARGO_TARGET_DIR riêng baseline |
| R0-B04 | Ghi dirty/clean status | R0-B16 | CARGO_TARGET_DIR riêng candidate |
| R0-B05 | Ghi rustc --version | R0-B17 | Tạo source-only test root |
| R0-B06 | Ghi cargo --version | R0-B18 | Tạo artifact-only test root |
| R0-B07 | Ghi LLVM version | R0-B19 | Artifact-only không source fallback |
| R0-B08 | Ghi host triple | R0-B20 | Build canonical sysroot |
| R0-B09 | Ghi target triple | R0-B21 | Lưu command + exit code |
| R0-B10 | Hash compiler binary | R0-B22 | Workspace baseline --no-fail-fast |
| R0-B11 | Hash runtime binary/library | R0-B23 | Lưu full failure list |
| R0-B12 | Đọc artifact format versions | R0-B24 | Pin baseline report vào SHA |

## Wave 2 — R0 Harness repair

Provider oracle: R0-H01 search hardcode provider count · H02 liệt kê `36`, `35/36`
và biến thể · H03 xác định source of truth manifest · H04 helper đọc provider count ·
H05 chuyển Whole-File I/O oracle sang helper · H06 chuyển path suite · H07 focused
tests · H08 kiểm tra không còn hardcode.

Version oracle: R0-H09 search metadata version hardcode · H10 phân loại v3/v5/v7 ·
H11 tách stale-version khỏi current-version · H12 centralize current version
constants · H13 giữ incompatible-version rejection · H14 chạy stale artifact controls.

Raw-storage harness: R0-H15 reproduce `E_RAW_STORAGE_ANCHOR_FIELD` · H16 reproduce
`E_RAW_STORAGE_ANCHOR_COPY` · H17 fixture parse thành công · H18 diagnostic identity ·
H19 reason · H20 span · H21 thay text-only assertion bằng typed assertion.

Artifact isolation: R0-H22 search tests ghi vào canonical sysroot · H23 kiểm HT7 ·
H24 kiểm String parity · H25 chuyển sang temp root · H26 chạy hai lần liên tiếp ·
H27 xác minh run #2 không phụ thuộc run #1.

Temp/process: R0-H28 audit TEMP · H29 audit TMP · H30 chuẩn hóa temp root ·
H31 audit child timeout · H32 thêm timeout · H33 reproduce StorageFull · H34 harness stress.

Gate R0: mọi failure mới có reducer rõ; harness không nhiễm canonical state.

### R0-H verdict — 2026-10-06

```
VERDICT: PASS / MEASURING_INSTRUMENT_ESTABLISHED
REMAINING:
- H28..H34 temp/process hardening  -> STATUS: DEFERRED_HYGIENE, BLOCKING: NO
- H12 optional constant centralization -> deferred into VR-01..VR-10
```

- Provider-count oracle (R0-H01..H08): đã manifest/isolated-based, không còn hardcode
  `36`/`35/36` (`count_extension`/`count_files_with_extension` + fresh/isolated sysroot).
- Version oracle (R0-H09..H14): còn một assertion current-version tường minh
  (`SEMANTIC_METADATA_VERSION == 7`); rejection version cũ do reader tests phủ.
  Không refactor lúc này — sẽ xử lý trong VR-01..VR-10 khi version thành metadata 9.
- Isolation (R0-H22..H27): whole-file/path/String parity dùng sysroot riêng.
- Evidence: baseline workspace 197 target, chỉ 1 fail `generic_drop_cli` (A1) →
  mọi suite liên quan R0-H PASS.
- Nếu C1 reconcile xuất hiện hang/process leak: quay lại H28..H34 ngay.

## Wave 3 — R1 Ownership / Soundness

### A1 — Closure environment destruction

Investigation: A1-01 chạy riêng `generic_drop_cli::closure_capture` · 02 output
source · 03 output artifact · 04 reducer tối thiểu · 05 dump AST · 06 dump semantic
capture facts · 07 dump MVIR · 08 cleanup block closure scope · 09 capture nào có
ownership · 10 capture nào đã move · 11 trace backend drop glue · 12 root-cause note.

Tests trước patch: A1-13 unused owned capture · 14 used owned capture · 15 borrowed
capture · 16 multiple owned captures · 17 negative double-destroy · 18 negative use-after-move.

Implementation: A1-19 representation closure env · 20 drop order · 21 emit cleanup 1
capture · 22 generalize N captures · 23 skip captures transferred · 24 không fake loan ·
25 không fake provenance · 26 wire backend drop glue · 27 destructor chạy đúng một lần.

Verification: A1-28 run reducer source · 29 build fresh artifact · 30 run reducer
artifact-only · 31 run `generic_drop_cli` đầy đủ · 32 run ownership focused · 33 run
workspace candidate · 34 record remaining gaps.

A1 chỉ đóng khi `generic_drop_cli` pass toàn bộ, positives đếm đúng drop, negatives
giữ nguyên ở source + fresh artifact.

A1 status — 2026-10-06:

```
A1: CLOSED / CONFORMANT IN TESTED SCOPE
evidence: evidence/a1-closure-env-2026-10-06.json, evidence/REPORT-A1-CLOSURE-ENV-2026-10-06.md
progression: exit3 (leak) -> exit2 (double-drop, debugging evidence only) -> exit0
workspace: 1329 pass / 0 fail / 1 ignored (exit 0)
protocol: compiler20 / metadata9 / MVIR5 / format2
```

### A2 — Async suspended cleanup

A2-01 re-run initial cleanup controls · 02 re-run `future_initial_cancel` · 03 fixture
conditional suspend before init · 04 after init · 05 projected field initialized ·
06 projected field uninitialized · 07 generic owned resource · 08 cancel tại mỗi
suspension point · 09 count destructors · 10 E3001 negative · 11 E3005 negative ·
12 run source · 13 run artifact · 14 document cleanup state transitions.

A2 status — 2026-10-07:

```
A1: CLOSED
A2: CLOSED / CONFORMANT IN TESTED SCOPE
workspace: 1334 pass / 0 fail / 1 ignored (exit 0)
protocol: compiler21 / metadata9 / MVIR5 / format2
release: still BLOCKED
next correctness blocker: A3
remaining: A2-FU1 (shared DropFlagPlan/Transition), A2-FU2 (full CLI matrix)
```

A2-FU2 — 2026-10-07: DONE (commit f330edc5). CLI async cleanup matrix passes in
source + fresh artifact (value / borrow-only / generic-owned). A2 remains
CLOSED; coverage strengthened.

A2-FU1:

```
A2-FU1
STATUS: DEFERRED_NON_BLOCKING
CLASS: maintainability / anti-drift
MUST COMPLETE BEFORE: R5 final release gate
```

### A3 — Partial aggregate cleanup

Freeze result: `indexed_move` counterexample — `consume(arr[0])` double-dropped the
moved element (exit 3, expected 2), source + fresh artifact.

First mismatch: constant array indexing lowered to `PtrOffset` with an opaque
offset operand, which move analysis does not treat as a place-producing lvalue
address, and caller-side array cleanup emitted a single whole-array `Drop`. The
element subplace was untracked, so the moved element was dropped again.

Fix (generic): `emit_place_cleanup` descends arrays per element (`FieldPtr`); a
constant array index is emitted as a literal `PtrOffset` offset and move analysis
maps a constant `PtrOffset` on an array base to the element subplace (with `Load`
inheriting `PtrOffset` places). Indexing stays a raw pointer offset, so raw-slice
provenance is preserved; the whole-array drop glue is unchanged. A dynamic index
stays opaque; no partial-move support is claimed for it.

Regression caught by `raw_slice_provenance_cli`: an earlier variant lowering
constant indexing to `FieldPtr` lost `slice_middle_conflict`'s E3003 and was
rejected.

```
A3
status: CLOSED / CONFORMANT IN TESTED SCOPE (constant-index array element moves)
protocol: compiler22 / metadata9 / MVIR5 / format2
focused: generic_drop_cli 21 positives + 12 negatives PASS (source + artifact)
workspace: 207 binaries, 1335 passed / 0 failed / 1 ignored (exit 0)
dynamic index: unchanged / unsupported for partial move
release: still BLOCKED
next correctness blocker: A4
```

`FIND-ASYNC-PROVIDER-01`

```
FIND-ASYNC-PROVIDER-01
STATUS: OPEN
CLASS: artifact / async export
summary: a provider exporting a parameterless `async fn` yields
         `Invalid semantic metadata: CorruptedData` when its .llib is imported
reproducer: tests/luna/language/async_provider_export/
SCHEDULE: after A3 (or C2/C3, per root cause)
```

### A4 — Cast / poison containment

Freeze matrix found **no counterexample**: an invalid static cast already fails in
the semantic layer with one typed E2026 diagnostic that poisons the operand (no
cascade, no MVIR/backend, no published executable); runtime-trapping casts
(integer-to-char) stay well-typed and trap deterministically. Closes as a
coverage certification; no compiler change was required.

```
A4
status: CLOSED / CONFORMANT IN TESTED SCOPE (coverage certification)
protocol: compiler22 / metadata9 / MVIR5 / format2
focused: cast_poison_cli 5 passed (source + artifact)
workspace: 208 binaries, 1340 passed / 0 failed / 1 ignored (exit 0)
release: still BLOCKED
next correctness blocker: A5
```

### A5 — Raw slice / view contract

Freeze matrix found **no counterexample**. Invariant locked: a raw pointer/slice
projection is not an owned aggregate subplace. Conflicts (shared view + rw write,
mutable view + competing read, pointer-arithmetic-derived view + backing write)
are rejected with E3003; a view escaping its backing lifetime is rejected with
E3005; valid reads/mutations, multiple shared views, slice copy, projected
(sub-slice) and zero-length views behave; an OOB slice index traps. Closes as a
coverage certification; no compiler change was required. `raw_slice_provenance_cli`
(A3 regression guard) stays green.

```
A5
status: CLOSED / CONFORMANT IN TESTED SCOPE (coverage certification)
protocol: compiler22 / metadata9 / MVIR5 / format2
focused: raw_slice_view_a5_cli PASS + raw_slice_provenance_cli PASS (source + artifact)
borrowck: cargo test -p luna-borrowck -> 0 failed
workspace: 209 binaries, 1341 passed / 0 failed / 1 ignored (exit 0)
release: still BLOCKED
next correctness blocker: A6
```

### A6 — Unary logical NOT

Implementation fix. `!x` was rejected for every operand with E2012
(`Unary operator `Not` is not yet supported`). The parser already produced
`UnaryOp::Not`; the gap was in the typechecker (grouped with the unsupported
BitNot/PostInc/PostDec) and the MVIR generator (no lowering). Fix: typechecker
admits `bool -> bool` and rejects a non-bool operand with one E2012 (poisoned);
generator lowers `!x` to `Eq { x, false }` — no new instruction, no MVIR schema
change, so **no protocol bump**. Comptime and runtime share semantics.
Bitwise NOT on integers is deliberately not added.

```
A6
status: CLOSED / CONFORMANT IN TESTED SCOPE (implementation fix)
protocol: compiler22 / metadata9 / MVIR5 / format2 (no bump)
focused: unary_not_cli PASS + luna-semantic 0 failed + expression_diagnostic_spans_cli PASS
workspace: 210 binaries, 1342 passed / 0 failed / 1 ignored (exit 0)
release: still BLOCKED
next correctness blocker: C2
```

### C2 — Per-impl-header metadata

Freeze matrix found **no counterexample**. Each checked impl header keeps its
identity, binders, constraints, trait arguments, associated types and method
contracts through `.llib` serialization/decoding/reconstruction; two concrete
`Describe` impls and `Ranked<i32>`/`Ranked<bool>` on the same nominal head are
never merged; the graph resolves from a relocated artifact; an ambiguous method
call rejects with E1008 (never picks the first candidate). Interface-fingerprint
and stale-dependency semantics remain covered by `interface_constraints_cli`.
Closes as a coverage certification; no compiler change was required and the
protocol is unchanged.

```
C2
status: CLOSED / CONFORMANT IN TESTED SCOPE (coverage certification)
protocol: compiler22 / metadata9 / MVIR5 / format2 (no bump)
focused: impl_header_metadata_cli PASS + interface_constraints_cli PASS + test_artifact_metadata_parity 9 PASS
workspace: 211 binaries, 1343 passed / 0 failed / 1 ignored (exit 0)
release: still BLOCKED
next correctness blocker: C3
```

### C3 — Projection / fallback + proof depth

Implementation fix. Two counterexamples:

1. **Proof-depth guard leaked into the language** — a valid 70-deep blanket-impl
   chain was rejected with `E2021 does not implement L70` (depth 60 passed) because
   `prove_trait_goal` returned false at a fixed `stack.len() >= 64`. Fixed with a
   named `MAX_PROOF_DEPTH = 256`; exact-cycle detection unchanged, so a genuinely
   cyclic blanket bound still rejects.
2. **Chained projection not resolved** — `Module::Type::Assoc::Nested` was rejected
   with `E1001` because the prefix was resolved only through the symbol table.
   Fixed by resolving the prefix left to right (descend modules, stop at a type,
   then project the remaining segments).

Structured diagnostics confirmed: `E2008` cyclic projection, `E2019` ambiguous
projection (single, no cascade), `E2021` cyclic bound. Generic chained projection
(`T::Assoc::Nested`) needs associated-type bounds (`type Assoc: Trait`), which the
grammar does not accept — recorded, fail-closed. No protocol bump.

```
C3
status: CONFORMANT IN TESTED SCOPE (implementation fix)
protocol: compiler22 / metadata9 / MVIR5 / format2 (no bump)
focused: projection_fallback_cli PASS + trait_argument_bounds_cli PASS + interface_constraints_cli PASS + luna-semantic 0 failed
workspace: 212 binaries, 1344 passed / 0 failed / 1 ignored (exit 0)
release: still BLOCKED
next correctness blocker: C4
```

### C4 — Comptime execution-dependency precision

Freeze matrix (no counterexample in the tested scope). C4-1 direct body change
(interface unchanged) rejects with a typed stale diagnostic; C4-2 transitive leaf
change rejects; **C4-3 unused imported provider change does NOT invalidate** (no
false-positive); C4-5 covered by C4-1; C4-6 source/artifact parity; C4-7 typed
`dependency execution fingerprint mismatch`. `interface fingerprint != execution
fingerprint` holds. Verdict wording: **sound and sufficiently precise in the
tested scope** — the global claim "depends on exactly the executed providers" is
NOT made, because branch-sensitive selection is unconfirmed (C4-FU1).

Recorded limitation **C4-FU1** (non-blocking): branch-sensitive dependency
selection. The comptime dependency set is a static call-graph walk, so a provider
reachable only through an untaken branch can still be recorded — **sound but
imprecise** (over-approximation), never a missing (unsound) dependency.

```
C4
status: CONFORMANT IN TESTED SCOPE (coverage certification)
protocol: compiler22 / metadata9 / MVIR5 / format2 (no bump)
focused: comptime_dependency_precision_cli PASS (source + artifact)
workspace: 213 binaries, 1345 passed / 0 failed / 1 ignored (exit 0)
open follow-ups: C4-FU1 branch-sensitive dependency selection (non-blocking, precision)
release: still BLOCKED
next correctness blocker: C5
```

### C5 — Artifact freshness classification

Implementation fix. Format/compiler/MVIR/semantic-metadata version mismatches all
surfaced as the same `MlibError::VersionMismatch(u16)` (`invalid manifest:
VersionMismatch(N)`), so the failing boundary was not identifiable. Fix: classify
the boundary with `VersionMismatch { component, found, expected }` where
`component ∈ {format, compiler, mvir, semantic-metadata}`. Target mismatch
(`TargetMismatch`), object identity (`ObjectIntegrityMismatch`), section tampering
(`SectionChecksumMismatch`), and interface/execution fingerprint staleness were
already classified.

Harness `fresh_artifact_classification_cli` mutates one boundary per case
(header format/compiler/mvir/target triple, metadata section version, object
payload — with the section checksum refreshed so validation reaches the boundary).
Every stale class rejects deterministically with a classified reason; no
executable is published; a fresh artifact builds and runs.

`FIND-ASYNC-PROVIDER-01` stays independent (deeper serialization/`CorruptedData`
issue, not a freshness class).

```
C5
status: CONFORMANT IN TESTED SCOPE (implementation fix)
protocol: compiler22 / metadata9 / MVIR5 / format2 (no bump)
focused: fresh_artifact_classification_cli PASS + luna-llib 24 PASS + struct_lifetime_contract_acceptance_tests PASS
workspace: 214 binaries, 1346 passed / 0 failed / 1 ignored (exit 0)
release: still BLOCKED
next: D-wave
```

### D1 — DIAG-1..10 diagnostic conformance

Implementation fix + coverage. Counterexample **D1-1**: DIAG-10 requires parity
between `docs/diagnostics/diagnostics-v1.md` and `DiagnosticCode::ALL`, but `ALL`
did not exist. Fix: added `DiagnosticCode::ALL` (68 codes) + `registry_parity`
unit tests (doc<->enum parity, uniqueness, naming; range rows excluded).

Matrix: DIAG-1 typed code ✓, DIAG-2 source span ✓, DIAG-3 related spans ✓,
DIAG-4 poison containment ✓, DIAG-5 determinism ✓, DIAG-6 dedup ✓, DIAG-7 phase
ownership ✓, DIAG-8 origin traceability ✓, DIAG-9 source/artifact parity ✓,
DIAG-10 FIXED.

Non-blocking observations: D1-FU1 (E3001 message renders internal `%v5.0`);
D1-FU2 (comptime failure emits E1001 + E4005 for one root).

```
D1
status: CONFORMANT IN TESTED SCOPE (implementation fix + coverage)
protocol: compiler22 / metadata9 / MVIR5 / format2 (no bump)
focused: diagnostic_conformance_cli 4 PASS + luna-common 10 PASS
workspace: 215 binaries, 1352 passed / 0 failed / 1 ignored (exit 0)
release: still BLOCKED
next: D2
```

### A3 — Partial aggregate cleanup

A3-01 inventory fixtures · 02 nested tuple move · 03 enum-pattern partial move ·
04 indexed move · 05 projected aggregate move · 06 partially-moved destruction ·
07 verify drop count per field · 08 verify remaining fields drop · 09 negative move
field from Drop owner · 10 verify E3001 · 11 run source · 12 run artifact.

### A4 — Cast + poison containment

A4-01 re-run 5 cast rejection reducers · 02 isolate poisoned private-member secondary
diagnostic · 03 trace Type sau failed cast · 04 poison/Error path · 05 suppress
secondary diagnostic · 06 verify original diagnostic · 07 fixture imported provider
cast body · 08 build provider artifact · 09 relocate artifact · 10 run artifact-only ·
11 verify identity cast drop count · 12 verify E3001 controls.

### A5 — Raw slice/view

A5-01 inventory CStr raw constructors · 02 safe misuse probe #1 · 03 safe misuse
probe #2 · 04 borrowed-header coverage · 05 mixed ownership coverage · 06 opaque
callback coverage · 07 opaque view-return coverage · 08 source positives · 09 source
negatives · 10 artifact positives/negatives.

### A6 — Unary logical Not

A6-01 reproduce `boolean_not_observation.ln` · 02 confirm parser representation `!` ·
03 confirm adopted syntax contract · 04 trace E2012 origin · 05 bool positive
fixture · 06 non-bool negative fixture · 07 implement semantic behavior · 08 verify
expression span · 09 run source · 10 run artifact.

## Wave 4 — R2 Artifact + adopted features

### C1 — CALL-ARGUMENTS-v1 (trace-only + reconcile)

| ID range | STATUS | SOURCE | ACTION |
|---|---|---|---|
| C1-01..C1-52 | IMPLEMENTED_ON_SIDE_BRANCH | codex/call-arguments-0.1 | RECONCILE / AUDIT / VERIFY |

Không giao agent implement named/default args lần nữa. Các ID gốc giữ để trace:
C1-01..C1-13 named binding; C1-14..C1-21 lowering; C1-22..C1-28 artifact interface;
C1-29..C1-40 default parameters; C1-41..C1-46 portability; C1-47..C1-52 gate.

Reconcile tasks:

| ID | Task | ID | Task |
|---|---|---|---|
| C1-R01 | Verify branch ancestry từ 3601d12e | C1-R16 | Verify default_arguments_cli |
| C1-R02 | Review commit 1b398714 | C1-R17 | Verify default_omission_cli |
| C1-R03 | Review commit c0818477 | C1-R18 | Verify source mode |
| C1-R04 | Review commit f36a5abc | C1-R19 | Verify fresh artifact mode |
| C1-R05 | Review commit ce417f19 | C1-R20 | Verify relocated artifact-only |
| C1-R06 | Review commit 2615b5f8 | C1-R21 | Review comptime c0818477 độc lập |
| C1-R07 | Inventory named fixtures | C1-R22 | Review comptime f36a5abc độc lập |
| C1-R08 | Inventory default fixtures | C1-R23 | Check unrelated behavioral changes |
| C1-R09 | Inventory omission fixtures | C1-R24 | Run impacted artifact suites |
| C1-R10 | Map fixtures → CALL-ARGUMENTS-v1 | C1-R25 | Run impacted method-resolution |
| C1-R11 | Audit compiler 15→19 | C1-R26 | Run cast suites |
| C1-R12 | Audit MVIR 4→5 | C1-R27 | Run module/config suites |
| C1-R13 | Audit metadata 7→9 | C1-R28 | Produce reconciliation report |
| C1-R14 | Find all stale version oracles | C1-R29 | Integrate vào candidate branch |
| C1-R15 | Verify named_arguments_cli | C1-R30 | Workspace verdict trên exact candidate SHA |

Hai commit comptime (c0818477, f36a5abc) là task độc lập: original failure →
reducer → root cause → fix → affected semantic invariant → regression. Không suy
`C1 tests pass ⇒ comptime fixes hợp lệ`.

### VERSION-RECONCILE

VR-01 inventory version changes · VR-02 identify schema change cho mỗi bump ·
VR-03 verify mỗi bump có justification · VR-04 verify old artifact rejects ·
VR-05 verify current artifact accepts · VR-06 verify stale dependency rejects ·
VR-07 verify fresh sysroot rebuild · VR-08 eliminate test hardcodes không phù hợp ·
VR-09 preserve explicit incompatible-version tests · VR-10 record compatibility boundary.

Không sửa mọi `15` thành `19` máy móc.

### C2 — Per-impl-header metadata

C2-01 locate impl-header canonicalization · 02 identify grouping by nominal head ·
03 dump metadata hai impl header khác nhau · 04 compare fingerprints · 05 change
public bound · 06 verify dependent artifact stale · 07 rename binder · 08 verify
invariance · 09 reorder irrelevant syntax · 10 verify invariance · 11 change body
only · 12 verify interface fingerprint unchanged · 13 verify no session-ID dependency.

### C3 — Projection/fallback + 64-frame

C3-01 associated projection reducer · 02 fallback reducer · 03 valid deep graph below
64 · 04 graph at boundary · 05 graph above boundary · 06 observe current diagnostic ·
07 distinguish exhaustion vs unsatisfied bound · 08 deterministic diagnostic ·
09 verify valid deeper graph.

### C4 — Comptime dependency precision

C4-01 public-signature change · 02 private-body-only change · 03 generic-body change ·
04 comptime-body change · 05 compiler-version change · 06 target change · 07 record
current invalidation · 08 identify over-invalidation · 09 refine dependency selection ·
10 verify fail-closed · 11 run complete matrix.

### C5 — Freshness classification

C5-01 define interface identity inputs · 02 execution identity inputs · 03 target/compiler
identity inputs · 04 map artifact fields · 05 private non-inlined body test ·
06 materialized generic body test · 07 comptime body test · 08 inlined body test ·
09 run implementation-only validity test · 10 reconcile validator với contract.

## Wave 5 — R3 Diagnostics

### D1 — DIAG-1..10

D1-01 requirement inventory · 02 map requirement → constructor · 03 mark missing
constructor coverage · 04 mark optional code usages · 05 mark optional span usages ·
06 source diagnostics lacking span · 07 I/O diagnostics incorrectly given spans ·
08 structured related-label model · 09 enforce constructor rules · 10 deterministic
ordering · 11 deterministic dedup · 12 one negative CLI fixture per requirement ·
13 registry parity · 14 docs parity · 15 run full diagnostic suite.

### D2 — Expression spans

D2-01 exact-span assertion deref · 02 negation · 03 char cast · 04 tuple projection ·
05 scalar index positive control · 06 trace `get_expr_span_for_diag` · 07 add missing
deref shape · 08 add missing unary shape · 09 add missing cast shape · 10 TupleIndex
primary span · 11 return poison Error · 12 verify no file-start fallback · 13 run
`expression_diagnostic_spans_cli`.

### D3 — Provider conformance

D3-01 snapshot provider matrix · 02 enumerate Tier-2 · 03 enumerate Tier-3 ·
04 map Resources API · 05 Resources positives · 06 Resources ownership negatives ·
07 repeat concurrency · 08 repeat file · 09 repeat net · 10 repeat remaining
collections · 11 repeat error APIs · 12 repeat borrow APIs · 13 run source ·
14 build fresh artifacts · 15 run artifact-only · 16 run required targets ·
17 downgrade false CERTIFIED row · 18 mark legitimate UNKNOWN.

### D4 — Retained contracts

Nhóm: D4-LIFE, D4-FFI, D4-DYN, D4-CLOSURE, D4-ASYNC, D4-COMPTIME, D4-UTF8,
D4-ENTRY, D4-BACKEND. Mỗi nhóm: -01 inventory · -02 coverage map · -03 gap ·
-04 fixture · -05 source verification · -06 artifact verification.

## Wave 6 — Spec debt

E1 overflow: E1-01 implement D2 decision vào semantics · 02 signed add overflow ·
03 signed sub · 04 signed mul · 05 unsigned overflow · 06 invalid shift ·
07 division edge · 08 debug build · 09 release build · 10 compare · 11 normative evidence.

E2 formal grammar: E2-01 inventory adopted surface syntax · 02 inventory formal
productions · 03 diff surface vs grammar · 04 mark parser-only legacy · 05 add missing
adopted productions · 06 remove/mark non-adopted · 07 reproduce V01-GRAMMAR-01 ·
08 reproduce V01-GRAMMAR-02 · 09 reproduce V01-GRAMMAR-03 · 10 reconcile gaps register ·
11 reconcile repair ledger · 12 run docs/example validators.

## Wave 7 — R4 Provenance + docs

Ledger: R4-L01 freeze ledger backup · L02 define schema · L03 nullable
implementation_commit · L04 verified_at_commit · L05 separate contract status ·
L06 separate implementation status · L07 structured evidence · L08 remaining gaps ·
L09 preexisting_at_baseline · L10 write migration · L11 verify no evidence lost ·
L12 validate ledger.

Docs: R4-D01 Status.md · D02 current gaps · D03 spec chapters · D04 repair ledger ·
D05 provider ledger · D06 README · D07 quick start · D08 limitations · D09 runtime
compatibility · D10 artifact compatibility · D11 generated site · D12 move audit
3dac3ac to historical · D13 repair missing agent-handoff reference · D14 docs
validator · D15 example checker.

## Wave 8 — R5 Release candidate

R5-01 select candidate SHA · 02 push candidate · 03 fresh clone · 04 verify clean ·
05 record tested SHA · 06 build compiler · 07 build runtime · 08 build canonical
sysroot · 09 verify provider count · 10 run A1 focused · 11 ownership suite ·
12 cast suite · 13 call arguments suite · 14 diagnostics suite · 15 provider matrix ·
16 57-fixture module/config matrix · 17 formatting tests · 18 Unicode tests ·
19 artifact parity · 20 docs validator · 21 example validator · 22 workspace
--no-fail-fast · 23 fix all failures; do not tag yet · 24 select new candidate SHA
if code changed · 25 repeat clean checkout · 26 final release workspace · 27 require
exit 0 · 28 Windows GNU CI · 29 Ubuntu CI · 30 Ubuntu ASan · 31 macOS if advertised ·
32 save every command exit code · 33 freeze evidence · 34 verify report commit ≠
tested commit semantics · 35 tag exact verified SHA 0.1-alpha.1 · 36 verify tag points
to exact SHA · 37 merge only after gate · 38 if merge changes tree, rerun release gate.
