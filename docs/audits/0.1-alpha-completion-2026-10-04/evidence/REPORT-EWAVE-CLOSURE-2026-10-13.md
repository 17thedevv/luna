# E-wave completion — 2026-10-13

**TASK:** E-wave Completion. **VERDICT:** CLOSED trong phạm vi E-wave.

**BASE SHA:** `6017ba8d8db40cf0bc16498f639e5d830c0e827c` (E2 CLOSED).
**TESTED IMPLEMENTATION SHA:** `e4f9fb530d72628c81dfa29d117afeb99cc7664c`.
Closure commit chứa báo cáo này chỉ bổ sung documentation/evidence; code tree
`luna-rs`, `runtime`, `tests` phải giống SHA đã kiểm thử. Exact closure/candidate
SHA được xác nhận khi tích hợp Git; không dùng SHA chứa chính nó trong nội dung.

## Stage results và root causes

- **Stage 2C PASS:** ordinary integer `+ - * / % << >>` và signed `-MIN` checked
  trong debug/release. Overflow intrinsics và guard zero/MIN/-1/count loại bỏ
  LLVM UB/poison trước phép toán nguy hiểm; left-shift round trip kiểm miền
  biểu diễn toán học. Không dùng nsw/nuw làm checking. DCE trước đây xóa phép
  toán có kết quả không dùng, mất trap; nay giữ arithmetic có thể trap và operands.
  Float/pointer paths giữ semantics; compiler-generated decrement/loop increment
  dùng ordinary checked operations, pointer element offset dùng pointer path.
- **Stage 3 PASS:** runtime/comptime parity; bốn negative comptime có đúng một
  E4005. Explicit wrapping vẫn modulo 2^N, integer-only và theo lang-item identity.
  Hash/FNV migration không thay hash output; không thêm function-name matching.
- **Stage 4 PASS:** compiler25 từ chối artifact22, artifact23 và artifact24 thật. Probe
  compiler23 chứng minh old object đã mất arithmetic effect vẫn chạy exit0;
  artifact mới trap đúng. Maintainer đã duyệt boundary 24, sau đó 25 cho generic enum alignment; metadata9/MVIR5/format2
  giữ nguyên. Fresh source/artifact, relocation và macro-generated provider đều
  qua native tests, reader validation vẫn strict.
- Diagnostic precondition đã sửa bằng admission thật: Parser E0003/E0005;
  Resolver E1009/E1010; TypeChecker E2001/E2032; Comptime E4005; Infrastructure
  E6001. Macro/derive engines truyền structured failure. Spans và fail-closed
  được kiểm qua check/build, root/provider/macro cases, không publish artifact
  hay executable sai. Broader DIAG-1..10 vẫn là nghĩa vụ audit R5.
- Provider parity sửa alias identity, statement relocation, expansion publication,
  generated-node metadata ranges và lexical access của captured bindings.
  Entry `main` được nhận diện theo root/program identity, tránh nested helper
  bị áp ABI entry hoặc va chạm symbol. Windows CLI có stack 16MiB để tới recursion
  guard. Lượt full đầu ở SHA `1b5dfab0...` dừng compile vì unit test thiếu Arc;
  `b9841cfe...` thêm import trong module test. Lượt sau tại SHA đó dừng ở test
  trust cũ kỳ vọng source/artifact interface phải mismatch E6001; canonical
  identity đúng nay đưa copied bundle tới authorization E2006 trong cả hai mode.
  `939d53e0...` sửa expectation và giữ strict stale-interface controls. Full run
  sau đó tại pin này dừng ở RangeInclusive MAX (877 passed / 1 failed), vì iterator
  tạo successor chỉ để thay terminal value. Maintainer duyệt Option slot; một
  non-Copy i128 Probe tiếp tục phát hiện enum payload alignment8 sai. Repair chung
  dùng configured TargetData, một representation cho runtime/static, VM padding
  parity và Option/Result projections theo alignof(Self); compiler25 reject
  real24. Không thêm library-name branch, intrinsic hay runtime ABI entry point.
  Terminal slot và exact drops/steps, all ten integer limits, generic enum
  constants/pointer relocations và stale layout rejection PASS cả bốn mode.
  Test MVIR debug dump được đưa vào owned temp output để tránh đổi tracked file.
  Chi tiết: [alignment repair](REPORT-E1-ENUM-ALIGNMENT-2026-10-13.md).

Chi tiết sửa và reproducer: [repair checkpoint](REPORT-E1-REPAIR-2026-10-12.md).
Không sửa lịch sử các acquisition fail thành PASS.

## Matrix và regression

46 fixture gốc: baseline Stage2B **42 runtime không trap + 4 E4005 duplicate**
→ cuối **22 TRAP / 8 PASS / 12 WRAP / 4 COMPILE_ERROR**, mỗi comptime một E4005.
59 fixture hiện tại: **34 TRAP / 9 PASS / 12 WRAP / 4 COMPILE_ERROR** ở cả bốn
mode debug/source, debug/fresh-artifact, release/source, release/fresh-artifact
(236 lượt). Trap chỉ được tính khi có đúng Luna L003/L005 panic signature;
ICE, crash, timeout hoặc missing DLL không được tính là arithmetic trap.
Debug/release ở đây là hai Cargo profile của compiler CLI; native emission dùng
LLVM OptimizationLevel::Default hiện hành. CLI không cung cấp flag release riêng.
Đối chiếu từng fixture: [46-row reconciliation](e1-final5-46-row-reconciliation-2026-10-13.json).

Ba `cargo test --workspace` liên tiếp, default parallelism, không override
`--test-threads`, không `RUST_TEST_THREADS`; release CLI được cấp cho parity suite:

| Run | Binaries | Passed | Failed | Ignored | Exit | Seconds |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 219 | 1404 | 0 | 1 | 0 | 12191.303 |
| 2 | 219 | 1404 | 0 | 1 | 0 | 7910.464 |
| 3 | 219 | 1404 | 0 | 1 | 0 | 7840.3 |

Totals bao gồm doc-test groups được ghi riêng trong JSON. Lossless gzip logs
và hash nằm trong [workspace qualification](e1-workspace-final6-2026-10-13.json).
Các suite attribute ownership, checked arithmetic relocation/old-artifact,
wrapping, comptime cascade, FFI, memory identity, HashMap/HashSet/String và
E2 grammar đều nằm trong full runs; kết quả suite được dẫn trong raw logs.
49 fresh bundles và hashes: [sysroot provenance](e1-final5-sysroot-provenance-2026-10-13.json).
Current-source review: [generated arithmetic audit](e1-final5-generated-arithmetic-audit-2026-10-13.json).
Các acquisition trước (compile thiếu Arc, Windows khóa executable khi recorder
chạy cùng Cargo, native timeout dưới tải đó) đều được giữ là FAIL/NOT VERIFIED.
Không thay timeout/classifier; source không đổi khi thu lại các matrix final3.
Xem [acquisition ledger](e1-final-acquisition-ledger-2026-10-13.json).

## Targets, gaps và integration

**VERSIONS:** compiler25 / semantic metadata9 / MVIR5 / format2.
**Windows GNU:** VERIFIED trong tested scope: native executions trên host này,
target `x86_64-pc-windows-gnu`, COFF, 64-bit, little-endian. Rust host toolchain
không tự chứng minh Luna target; manifest và actual execution là bằng chứng.
Object/header thực của 49 bundle mới: [current native target](e1-final5-current-native-target-2026-10-13.json).
**Ubuntu GNU normal/ASan:** NOT VERIFIED; nghĩa vụ R5 còn lại.

Standalone `comptime { ... }` statement execution vẫn unsupported, từ chối
E4005 có primary span; đây là support gap riêng, không phải PASS cho execution.
Comptime expression tiếp tục hoạt động. Broader diagnostics structural audit,
Ubuntu target/ASan và release-wide conformance thuộc R4/R5; không có E-wave
blocker còn lại trong phạm vi gates trên. E2 giữ CLOSED, các hồ sơ lịch sử không
thay thế current evidence. **SKILL IMPACT:** none. VM layout model chỉ được kiểm trong phạm vi 64-bit GNU; không claim cross-target reflection.

Docs validator phải PASS 0 errors sau generation và trước closure commit.
Chỉ push implementation và fast-forward candidate tới cùng closure SHA khi
ancestry hợp lệ. Không merge main, không tag release, không tự chạy R4/R5.
**NEXT:** R4 documentation and release ledger.
