# R5 — exact release qualification (final implementation pin db00b9ff)

**Ngày**: 2026-10-16
**Task**: R5 exact release qualification cho pin implementation cuối cùng sau repair wave DIAG.
**Base (pin R5 gốc)**: `9bda6f572fa5f9904379c08c625c28122fcd0c56` (candidate `luna-0.1-alpha.1-candidate` trước closure)
**Candidate sau closure**: `c085449b207b5860b1d7a79bee221c2ff3d357c3` — đã **fast-forward** từ `9bda6f57` (fast-forward, không force)
**Implementation SHA**: `db00b9ff1287c7a900d60e3d20c8f8340dd09091` (branch `r5-diag-repair`, đã push)
**Docs follow-up của pin**: `c085449b207b5860b1d7a79bee221c2ff3d357c3` (chỉ sửa BOM trong `docs/tools/diag_producer_inventory.py`)
**Code tree**: `luna-rs`, `runtime`, `tests` **giống hệt byte-for-byte** giữa `db00b9ff` và `c085449b` (tree objects `0861dcad004e`, `60cb194d21d5`, `ba52039a110e`) ⇒ pin implementation không bị vô hiệu, đây là docs-only follow-up.

**VERSIONS**: compiler **25**, semantic metadata **9**, MVIR **5**, format **2** — không đổi trong toàn bộ repair wave (wire/artifact compatibility không đổi).

---

## 1. Phạm vi đã sửa trước qualification

Repair wave DIAG trên `r5-diag-repair` (pin `9a98a25b`, rồi `db00b9ff`):

| Mã | Nội dung | Trạng thái |
| --- | --- | --- |
| DIAG-1 | Mọi lỗi CLI có typed code (E6005 read/open, E6004 sysroot, E6007 emit kind, E6008 provider config, E6009 run) | FIXED |
| DIAG-2 | Producer gắn primary span thật (tuple pattern, if/while/for, array literal, missing return); 11 chẩn đoán contract provider/manifest khai báo tường minh qua `Diagnostic::contract_error`; gate fail-closed `enforce_source_anchor_policy` ở boundary render; **không** bịa `Span(0..0)` | FIXED |
| DIAG-3/9 | Move/drop/loan gắn related label cấu trúc (track site theo place qua dataflow, join khi merge; borrow gắn site của loan) | FIXED |
| DIAG-4 | Identity = `(code, primary span)`, bỏ dedup theo message text | FIXED |
| DIAG-5 | Poison containment tại đúng boundary: operand `SemanticType::Error` không cascade qua member access; reducer mới `poison_consumer_forms_invalid.ln` | FIXED (counterexample thật: cast lỗi → member access từng báo thêm E2001) |
| DIAG-6 | Chọn antecedent + thứ tự báo conflict tất định (earliest location → value id) | FIXED |
| DIAG-8 | Phase ownership: E1008 dựng bởi Resolver, E2001/E2016 dựng bởi TypeChecker; phase phát hiện route qua constructor của owner | FIXED |
| DIAG-10 | Parity enum/number/name/owner/docs là test máy kiểm tra (`registry_parity`) | PASS |
| DIAG-7 | Test per-code identity + span policy cho **toàn bộ 72 code**; inventory producer regenerable + drift gate (`docs/tools/diag_producer_inventory.py`, `docs/diagnostics/diag-producer-inventory.json`) | PASS (kèm debt ghi rõ) |
| R5-FU-TARGET-01 | Canonical target selection: `canonical_supported_target` → `x86_64-pc-windows-gnu` / `x86_64-unknown-linux-gnu`; target ngoài matrix không canonicalize; `TargetConfig::default` ghi triple canonical | FIXED (giải quyết blocker target contract của R5) |

---

## 2. Kết quả theo lane (exact pin)

### 2.1 Windows GNU — lane local exact-pin (runner ngoài checkout)

| Stage | Kết quả |
| --- | --- |
| acquire (fresh remote clone + detach tại SHA) | exit 0 |
| runtime-config / runtime-build / runtime-abi | exit 0 / 0 / 0 |
| cli-debug / cli-release | exit 0 / 0 |
| sysroot (49 provider, target `x86_64-pc-windows-gnu`, COFF `0x8664`) | exit 0 |
| **focused (12 batch)** | **PASS** (mọi batch exit 0, gồm `diagnostics`, `arithmetic`, `grammar-harness`, **`artifact-target`** mới thêm) |
| **diagnostics (structural audit)** | **PASS** (registry parity + per-code identity/span policy + span-anchor contract + inventory drift) |
| docs-examples | exit 0 |
| docs-validator | **FAIL (history)** → docs-only fix `c085449b` → `docs-validator-fixed` **exit 0** |
| quickstart (build + run native) | **PASS** (PE64) |
| comptime/layout (R5 harness, plan pin patched) | exit 0 |
| **workspace-triage** (`--workspace --no-fail-fast`) | **exit 0 — 1409 passed, 0 failed, 1 ignored** (19116 s) |
| **workspace-final2** (default parallel) | **exit 0 — 1409 passed, 0 failed, 1 ignored** → `workspace-summary2.json` = **PASS** |

Ghi chú liveness: `workspace-final` lần đầu bị **host session kết thúc giữa run** (log stale 818 phút, không còn process) — phân loại **preserved partial attempt**, không phải FAIL, không phải evidence; đã chạy lại dưới label mới `workspace-final2` tại cùng pin, chỉ reuse kết quả triage đã hoàn tất. Trước đó lane preflight `9a98a25b` cũng chết giữa `workspace-triage` vì cùng nguyên nhân host session (đã phân loại tương tự, các label build/sysroot/focused/examples giữ nguyên là preflight evidence).

### 2.2 Ubuntu GNU — lane CI exact-pin (GitHub Actions)

| Lane | Run | Kết quả |
| --- | --- | --- |
| **Ubuntu GNU normal** (build + sysroot target contract + focused + docs + quickstart, `--skip-workspace`) | #10 `38859185307` @ `c085449b` | **SUCCESS — 43/43 gate** |
| **Ubuntu runtime ASan** (sanitized C runtime + RuntimeAbiTests) | #16 `39009473122` @ `c085449b` | **SUCCESS** |
| **Ubuntu comptime-vm** (portable boundary probe) | #16 `39009473122` @ `c085449b` | **SUCCESS** (sau khi align pin của harness lồng) |

**Điểm quan trọng (giải quyết blocker R5)**: trên runner Ubuntu host LLVM báo `x86_64-pc-linux-gnu`, nhưng artifact ghi **canonical** `x86_64-unknown-linux-gnu` và **native-integrity + MANDATORY TARGET_CONTRACT đều PASS** ⇒ contract target canonical đã được enforce đúng trên Linux.

Ba lần fail trước đó của lane CI đều là lỗi công cụ, đã sửa và ghi lại trong lịch sử:
- run #4: workflow invalid — `env` không dùng được trong `jobs.<job_id>.if`.
- run #6: harness còn kỳ vọng artifact target bằng spelling host LLVM (giả định pre-repair) → đã align theo contract đã adopt.
- run #7/#9: chỉ `docs-validator` fail do BOM trong tool DIAG-7 → sửa docs-only, đưa pin tested lên `c085449b`.
- run #7b (trước đó): job bị **cancel ở đúng 6h00m** do cap 6 giờ của GitHub-hosted runner khi lane còn chạy cả workspace → tách workspace khỏi lane normal (workspace evidence lấy từ lane Windows cùng pin).
- run #11..#15 (lane comptime-vm): gate `comptime-vm-boundary` fail vì **harness lồng còn hardcode pin `9bda6f57`** nên assert `HEAD == PIN` chết ngay khi khởi động (không sinh verdict nào: `FAIL verdicts=0`). Đã đưa pin harness lồng về đúng pin tested → lane **PASS** ở run #16. Việc đọc được nguyên nhân này là nhờ bổ sung bước đưa failure summary + verdict vào workflow annotation (log CI cần quyền admin nên không tải trực tiếp được).

---

## 3. Kiểm soát chất lượng evidence

- Mọi label kết quả **bất biến** (không ghi đè): label fail/partial được giữ nguyên trong lịch sử; lần chạy lại dùng label mới (`workspace-final2`, `docs-validator-fixed`, `diagnostics-inventory-fixed`).
- Không carry PASS sang SHA khác: mỗi SHA có bộ evidence riêng; pin tested của CI được ghi rõ theo run.
- Không sửa evidence để thành PASS; `docs-validator` FAIL vẫn nằm trong lane dir như lịch sử.
- Log CI không tải được vì thiếu quyền admin; để root-cause, harness được bổ sung bước **đưa failure summary vào workflow annotation** — nhờ đó đọc được verdict/gate lỗi mà không cần job-log.

---

## 4. TARGETS

| Target | Trạng thái |
| --- | --- |
| Windows x86_64 GNU (`x86_64-pc-windows-gnu`) | **VERIFIED** (build + 49 provider + focused + diagnostics + comptime + quickstart + workspace 1409/0) |
| Linux x86_64 GNU (`x86_64-unknown-linux-gnu`) | **VERIFIED** cho platform/ABI/target-contract/focused/docs/quickstart (lane normal 43 gate) + sanitized runtime ASan |
| Ubuntu comptime-vm (portable boundary probe) | **VERIFIED** (run #16 sau khi align pin harness lồng) |
| macOS / 32-bit / freestanding | **không advertised** (ngoài release matrix, không canonicalize) |

---

## 5. REMAINING GAPS

**Blockers**: không.

**Deferred debt** (đã ghi trong `r5-summary.json`):
- `UBN-CPT-01` — **RESOLVED**: pin của harness comptime lồng đã align về đúng pin tested; lane `ubuntu comptime-vm` PASS ở run #16.
- `DIAG-COMPTIME-CODE-01` — diagnostic-identity: E4002/E4003/E4004 đã registered nhưng **không có producer**; step/recursion limit hiện báo **E4005** với message nêu tên `E_COMPTIME_*_LIMIT` (prose code ≠ typed code). Ghi nhận riêng, không tự đổi hành vi. MUST-COMPLETE-BEFORE: next-diagnostic-amendment.
- `DIAG-7-COV-01` — behavioural-coverage: 17 code có coverage identity + span policy nhưng chưa có fixture/hành vi tham chiếu. MUST-COMPLETE-BEFORE: release-gate:per-code behavioural coverage.

**Khác**: 10 code registered nhưng không producer trong 0.1 được khai báo tường minh trong inventory (E0004, E1006, E1007, E2014, E2015, E2020, E4002, E4003, E4004, E5002) — không overclaim coverage.

---

## 6. VERDICT

**Release gate: SATISFIED trong phạm vi đã test.** Toàn bộ lane bắt buộc PASS tại pin implementation cuối:
- Windows GNU exact-pin: focused 12 batch + diagnostics + comptime/layout + quickstart + workspace triage/final **1409 passed / 0 failed**;
- Ubuntu GNU normal: **43/43 gate**, bao gồm canonical target contract;
- Ubuntu runtime ASan: PASS;
- Ubuntu comptime-vm: PASS.

**Candidate `luna-0.1-alpha.1-candidate` đã được fast-forward `9bda6f57 → c085449b`** (`c085449b` là docs descendant của `db00b9ff`, tree `luna-rs`/`runtime`/`tests` giống hệt byte-for-byte), đúng điều kiện đã thống nhất: chỉ fast-forward khi final implementation SHA đạt toàn bộ R5 gate.

---

## 7. NEXT

1. Xử lý debt còn lại theo thứ tự: `DIAG-COMPTIME-CODE-01` (typed code cho step/recursion limit) rồi `DIAG-7-COV-01` (behavioural coverage per-code).
2. Không cần chạy lại lane nào cho closure này; mọi thay đổi tiếp theo ngoài docs sẽ tạo pin mới và phải refresh qualification tương ứng.
