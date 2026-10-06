<!-- luna-doc-role: evidence -->

# R0 Baseline report — 2026-10-06

Baseline immutable cho Luna 0.1-alpha.1. Đây là báo cáo evidence, không phải
nghiệm thu release.

## 1. Tóm tắt

- BASE SHA: `3601d12e8ae81f001a47a5523b819e7728b85c54`, branch
  `codex/antigravity-repair-0.1`.
- Sysroot: 49 provider build thành công, `build-sysroot` exit 0.
- Workspace: **197 target**, **1315 passed / 1 failed / 1 ignored**, exit 101.
- Failure duy nhất: `luna-cli --test generic_drop_cli` — test
  `generic_owned_values_drop_once_across_control_flow_and_provider_modes`,
  `closure_capture` exit 3 ở cả source và artifact mode (A1).
- Run StorageFull trước đó được phân loại riêng là **invalid environment**,
  không trộn vào verdict này.

## 2. Phạm vi và giới hạn (bắt buộc đọc)

- Source compiler/library tại baseline là đúng `3601d12e` (không có thay đổi
  source compiler/library).
- Tuy nhiên lúc chạy, working tree **không sạch tuyệt đối**: có thay đổi
  **doc-only** (`documentation-index.json`, `documentation-index.md` modified;
  `MICRO-BACKLOG-2026-10-06.md`, `REMAINING-PLAN-2026-10-06.md` untracked),
  `.commandcode/taste/taste.md` do hệ taste sửa, và các artifact tạm không liên
  quan (`.tmp-interface-debug/`, `luna-rs/target-lsp/`, `scratch/`).
- Vì vậy **KHÔNG được gọi run này là "clean-checkout release verification"**.
  Đây là **immutable compiler baseline** ở đúng SHA `3601d12e`; các thay đổi
  working-tree nêu trên không phải source compiler/library.
- Không cần chạy lại workspace chỉ vì sau đó commit docs.

## 3. Metadata baseline

| Hạng mục | Giá trị |
|---|---|
| BASE SHA | `3601d12e8ae81f001a47a5523b819e7728b85c54` |
| Branch | `codex/antigravity-repair-0.1` |
| cargo | 1.98.0 (797e8a9bc 2026-08-05) |
| rustc | 1.98.0 (88d9e12ae 2026-08-18) |
| Host triple | `x86_64-pc-windows-msvc` |
| Luna target | `x86_64-pc-windows-gnu` |
| LLVM-C DLL | `bin/LLVM-C.dll` (18.1.8) |
| Compiler version | 15 |
| MVIR version | 4 |
| Format version | 2 |
| Semantic metadata | 7 |
| Provider count | 49 |

Hashes (SHA256):

- `luna-rs/target/debug/luna.exe` (rebuilt): `6c7a0b07f6d2b2a41373dc35453caab8eb332b588818b4747579bdb12f1a2a9a`
- `bin/luna.exe`: `1d03736cde52297bf5927edc0dcb9b89bd5692ff95550da9f153924ad56bad0d`
- `runtime/build-gcc/libluna-runtime.a`: `02e7fab138160d91f1c50686558a1b2ed96bfeffca7e903824ca2d6eb47ac5c7`
- `runtime/luna-runtime.lib`: `7ad86e8735b22539a0d9d9ad3970c06ce38f9ec2766c9194d66d6a2d1c92f3ad`

## 4. Sysroot

- Command: `cargo run -p luna-cli -- build-sysroot`.
- Điều kiện: `bin/` phải có trên PATH (cho `LLVM-C.dll`), nếu không exit
  `0xC0000135` (STATUS_DLL_NOT_FOUND).
- Kết quả: **exit 0**, "Sysroot built successfully.", 49 provider.

## 5. Workspace

- Command: `cargo test --workspace --no-fail-fast`.
- Kết quả: **exit 101**; 197 target có kết quả; **1315 passed / 1 failed /
  1 ignored**; **0 lỗi StorageFull**.
- Target fail duy nhất: `luna-cli --test generic_drop_cli`.
  - Test: `generic_owned_values_drop_once_across_control_flow_and_provider_modes`.
  - Chi tiết: `source/closure_capture run: exit code 3` và
    `artifact/closure_capture run: exit code 3` (`generic_drop_cli.rs:98`).
  - Ý nghĩa: owned closure capture không được destroy khi environment ra khỏi
    scope (thuộc task A1).
- Mọi target còn lại PASS.

## 6. Sự cố môi trường (phân loại riêng)

- Run hợp lệ ban đầu không capture được output (redirect qua `&&` chain tạo log
  0 byte). Khắc phục bằng batch script `cmd /c *.cmd` với redirection trực tiếp.
- Một run batch trước đó **hỏng vì `D:` hết dung lượng (0 GB free)**: log có
  **60 lỗi `StorageFull (os error 112)`** khi "copy provider file", 10–15 target
  FAILED giả. Run này được lưu riêng trong
  `r0-baseline-3601d12e-attempt.json` và **không** dùng làm verdict.
- Xử lý: `cargo clean` (giải phóng ~59.09 GB từ `luna-rs/target`, 1211 file
  `.fingerprint` còn lock), giữ TEMP/TMP mặc định trên `C:` cho run hợp lệ.
- Sau run hợp lệ: `D:` còn 43.6 GB.

## 7. Evidence

- `evidence/r0-baseline-3601d12e.json` — verdict hợp lệ (ESTABLISHED).
- `evidence/r0-baseline-3601d12e-attempt.json` — run StorageFull (INVALID
  ENVIRONMENT), giữ nguyên.
- Log đầy đủ (forensic, local, không commit): `scratch/workspace-baseline.log`;
  wrapper log trong thư mục shellout.

## 8. Khối đóng task (DoD)

R0-B (baseline)
- TASK: R0-B immutable baseline
- PARENT TASK: PHASE 1
- DEPENDENCIES: DOC-01
- BASE SHA: `3601d12e`
- IMPLEMENTATION SHA: — (không sửa code)
- TARGET: Windows x86_64 GNU (host MSVC)
- MODE: source + artifact (workspace)
- REPRODUCER: `cargo test --workspace --no-fail-fast`
- ORIGINAL FAILURE: `generic_drop_cli::generic_owned_values_drop_once_across_control_flow_and_provider_modes` (closure_capture exit 3)
- ROOT CAUSE: owned closure capture không được destroy (điều tra sâu thuộc A1)
- CONTRACT: 0.1 conformance gates
- CHANGED FILES: 3 file evidence (không sửa source)
- FOCUSED COMMAND: `cargo run -p luna-cli -- build-sysroot`
- FOCUSED RESULT: PASS exit 0, 49 provider
- REGRESSION COMMAND: `cargo test --workspace --no-fail-fast`
- REGRESSION RESULT: FAIL exit 101 — 197 target, 1315 pass / 1 fail / 1 ignored
- ARTIFACT MODE: có (harness tự dựng source + fresh artifact root)
- REMAINING GAPS: version oracle/harness thuộc PHASE 2 (R0-H); chưa phải release verification
- VERDICT: BASELINE ESTABLISHED

## 9. Kết luận

- DOC-01: PASS.
- R0-B: BASELINE ESTABLISHED (FAIL exit 101 với đúng một failure A1 closure_capture).
- Đóng PHASE 0/1 sau hai commit (plan materialization và baseline evidence).
- Không reconcile C1 trước khi PHASE 2 (R0-H) sạch.
