# Kế hoạch hoàn tất Luna 0.1

Cập nhật 2026-10-20: **qualification tại pin sửa CPU `b5cb77c3` đang chạy lại**.
Tám lane tại `8525067f` giữ nguyên scope lịch sử; clean Windows package smoke
đã phát hiện CPU identity phụ thuộc máy, nên verdict release cũ bị thay thế.
[Trạng thái hiện tại](../r5-cpu-baseline-2026-10-20/qualification-state.json):
2/8 lane mới đã xác minh độc lập; còn sáu lane. Hai package Windows/Ubuntu từ
source `cd9cd188` đã build và clean hosted smoke PASS, đủ sáu ZIP đã xác minh.
Candidate vẫn `8525067f`; PR #7 draft; main/tag/publication còn pending.
Bước 3–5 phải đóng tại pin mới trước khi tiến hành merge và phát hành đã được
maintainer cho phép. [Hồ sơ sửa và bằng chứng](../r5-cpu-baseline-2026-10-20/README.md).
Goal đầy đủ vẫn active. Phần dưới giữ kế hoạch và baseline ngày 2026-10-18.

Kế hoạch này tiếp nối [kế hoạch hoàn tất gốc](../0.1-alpha-completion-2026-10-04/README.md),
[ledger R4](../0.1-alpha-completion-2026-10-04/r4/README.md) và
[checklist R5](../0.1-alpha-completion-2026-10-04/r4/r5-checklist.md).
Không thu hẹp các yêu cầu đó xuống riêng diagnostics hoặc một bộ test xanh.

## Mốc xuất phát đã kiểm tra

- Candidate: `c085449b207b5860b1d7a79bee221c2ff3d357c3`;
  main: `401a95a872f0fdfb9840640f41cc4ac629cb17cd`.
- Cả tám job CI tại implementation `d3d0ffe8d6495aee1f56257a1db817ce4107e992`
  đã SUCCESS. Pin đó đã bị thay thế vì finding DIAG-2/3; SUCCESS của job không
  thay cho kiểm tra archive và không được chuyển sang implementation mới.
- Bản sửa hiện tại chưa commit/chưa có pin qualification mới. Local preflight:
  **206 semantic + 1 ma trận CLI + 7 hồi quy = 214 passed / 0 failed**.
  Ma trận CLI có 19 ca âm, hai provider mode, 114 lệnh check/build âm và hai
  lượt chạy native hợp lệ. Runtime được tái sử dụng, nên đây chưa phải R5.
  [Log, hash và lịch sử các lần thử](evidence/declaration-preflight.json).
- Các sửa E3004, typed comptime E4002/E4003, giữ diagnostic gốc dưới `run`,
  Linux math linkage, move origins và identity/order vẫn nằm trong nhánh sửa.
- Protocol đang giữ compiler25 / metadata9 / MVIR5 / format2.

## 1. Đóng audit và các sửa còn lại trước khi chọn pin

**Công việc**

1. Đối chiếu từng requirement/gap của kế hoạch gốc, specification 0.1 và ledger
   R4: đã đóng trong phạm vi nào, được defer bằng quyết định nào, unsupported,
   hay còn cần bằng chứng. Bao gồm retained contracts, provider/target claims,
   grammar/document debt như V01-DESIGN-04; không lấy parser acceptance làm
   quyết định mở rộng ngôn ngữ.
2. Hoàn tất DIAG-1..10 theo từng producer/constructor/emission boundary:
   typed code, primary span thật, related locations, owner phase, poison,
   identity/order/dedup, event origins và registry parity. Inventory 72 code
   và coverage-debt rỗng là đầu vào, không tự chứng minh toàn bộ contract.
3. Chốt bản sửa E1002/E2005/E2006/E2019 và source anchors: declarations,
   fields/variants, import bindings, language-item registration, impl/candidate
   origins, bare returns, tuple/array expressions. Phân loại các fallback span
   còn lại theo đường chạy thực; dữ liệu nội bộ/synthetic phải được phân biệt
   với source error đang hiển thị cho người dùng.
4. Mỗi defect thật phải có reducer, nguyên nhân ở owning stage, sửa generic,
   positive/negative controls và source/fresh `.llib` parity phù hợp. Rà các
   source/portable-provider và check/build/run routes liên quan. Không thay
   expectation để hấp thụ lỗi hay tự defer một vi phạm contract 0.1.

**Đóng khi:** ma trận requirement → implementation → regression → evidence
không còn blocker hoặc claim 0.1 thiếu căn cứ. Mọi finding mới đã sửa và kiểm
chứng, hoặc có quyết định maintainer cụ thể trước khi coi là ngoài phạm vi.
Audit nguồn được làm trước vòng qualification tiếp theo.

## 2. Chốt bản sửa và implementation pin

- Hoàn tất hồi quy tại đúng tầng quan sát: public Luna CLI cho hành vi ngôn ngữ,
  Rust tests cho invariant nội bộ; valid controls phải build/run khi áp dụng.
- Tái tạo và kiểm tra diagnostic inventory; cập nhật map producer/owner/span,
  các reserved/unreachable conditions cùng lý do, không che debt.
- Lưu log gốc/nén, command, exit, hash, scope và cả các lần thất bại; đối chiếu
  thay đổi compiler/stdlib boundary và artifact identities.
- Đồng bộ tài liệu trước pin; docs generation phải idempotent và validator PASS.
- Commit các sửa cần thiết, review diff, chọn full immutable implementation SHA;
  cập nhật ba workflow cùng trỏ SHA đó. Phân biệt implementation, harness và
  report commits. Bổ sung suite declaration-location vào gate diagnostics.

**Đóng khi:** có một SHA triển khai cụ thể, preflight PASS, source/runtime/tests
và documentation baseline đã ổn định; chưa gán trạng thái release-qualified.

## 3. Chạy đủ tám lane tại đúng pin mới

| Lane bắt buộc | Điều kiện chấp nhận |
| --- | --- |
| Windows native normal | Fresh clone/detach; runtime ABI; CLI debug/release; 49 providers; 12 focused batches và diagnostic additions; docs/examples/quickstart; comptime/layout; strict L002 replay |
| Windows workspace triage | Full workspace với `--no-fail-fast`, terminal exit 0, zero failures |
| Windows workspace final | Chạy sau triage, full workspace default parallelism, terminal exit 0, zero failures |
| Ubuntu native normal | Native GNU build/runtime/CLI/49 providers; focused suites, diagnostics, canonical target, Whole-File I/O, formatting/Unicode, docs và quickstart |
| Ubuntu runtime ASan | RuntimeAbiTests và partial-read cleanup; chứng minh libasan instrumentation, exit 0, không sanitizer/leak report |
| Ubuntu comptime/VM | VM/runtime/static layout parity, target conflicts và branch/dependency characterizations giữ đúng phân loại |
| Ubuntu workspace triage | Full workspace với `--no-fail-fast`, terminal exit 0, zero failures |
| Ubuntu workspace final | Chạy sau triage, full workspace default parallelism, terminal exit 0, zero failures |

Giữ nguyên các focused A/C/D/E, module/config matrix, retained contracts,
artifact integrity/freshness và source/artifact controls trong checklist gốc.
Arithmetic traps phải có Luna L003/L005; bounds replay phải có L002 cùng
index/length thực. Crash, timeout hay thiếu DLL không phải expected trap.
Ghi số test và ignored cases thực tế, không dùng số đếm của pin cũ làm oracle.

**Đóng khi:** cả tám lane hoàn thành và tạo đủ evidence tại cùng implementation
pin. Nếu compiler/runtime/tests thay đổi, chọn pin mới và qualify lại cả tám;
không carry PASS. Trong lúc chạy chỉ hoàn thiện evidence/report không đổi code.

## 4. Xác minh evidence và nghiệm thu toàn bộ yêu cầu

- Tải và kiểm tra ZIP digest, manifest, compressed/raw log hashes, command và
  expected exit của từng gate; xác minh SHA, detached state, clean initial/final
  checkout và source tree identity. Job SUCCESS riêng lẻ chưa đủ.
- Kiểm tra canonical Windows GNU/Linux GNU target, native object headers và
  đúng 49 provider bundles với version tuple đã chọn; không suy từ host triple.
- Kiểm tra từng named suite, per-code/structural diagnostic coverage,
  source/artifact negatives, native valid outputs, docs/example rows và README
  quickstart. ASan chỉ chứng nhận C runtime trong scope đã chạy.
- Đối chiếu requirement-by-requirement với kế hoạch gốc và checklist R5;
  resolve mọi missing/weak/contradictory evidence trước khi đóng.

**Đóng khi:** từng yêu cầu có pin/target/mode/command/result/evidence đủ mạnh,
không còn blocker trong adopted scope; kết quả có thể kiểm tra lại độc lập.

## 5. Closure report, candidate và hồ sơ bàn giao

- Hoàn tất closure report và machine-readable ledger; tách introducing fix,
  tested implementation, harness và report SHA, bảo toàn lịch sử thất bại.
- Đồng bộ README, Status, gaps/conformance, provider/target matrix, limitations,
  generated site và release notes. Các deferral đã adopt, unsupported comptime
  statements, layout/ASan scope và diagnostic exceptions phải được mô tả đúng.
- Sau khi bước 1–4 thực sự đóng, fast-forward candidate đến implementation đã
  qualify; kiểm tra remote refs và tree identity. Không force-push.
- Bàn giao final SHA, CI links, verified archives, test counts, limits,
  nội dung PR/release notes và đề xuất nội dung gói phát hành có thể review.

**Đóng khi:** **READY FOR RELEASE REVIEW trên exact SHA**. Đây chưa phải tag,
merge main hay release publication; giữ các trạng thái đó riêng biệt.

## 6. Phát hành dưới quyền maintainer

- Sau lệnh maintainer, tạo PR/merge main, chọn tag/version và công bố release.
  Ưu tiên giữ implementation đã qualify. Nếu merge làm thay đổi release
  implementation hoặc artifact identity, kiểm tra/qualify commit phát hành
  trước khi tag; không âm thầm chuyển kết quả pin cũ.
- Build gói compiler/runtime/sysroot từ commit phát hành đã xác minh, tạo
  manifest/checksums và hướng dẫn sử dụng đúng supported targets.
- Cài/thử gói trên môi trường sạch Windows GNU và Ubuntu GNU: version/identity,
  public quickstart và smoke sử dụng runtime/sysroot từ chính gói đó.
- Kiểm tra tag trỏ đúng verified release commit, assets/checksums/release notes
  khớp nhau; ghi closing record cuối.

**Hoàn thành Luna 0.1** khi nghiệm thu kỹ thuật đã đóng và các bước phát hành
đã được maintainer cho phép, thực hiện và xác minh. Không đánh dấu toàn bộ goal
complete chỉ vì candidate hoặc CI đã xanh.

## Thứ tự thực hiện và xử lý finding mới

Đường găng: **audit/repair → preflight/pin → tám lane → xác minh/closure →
candidate/handoff → maintainer release và clean-package smoke**.
Không đặt ngày phát hành cứng khi audit chưa đóng. Mỗi finding mở lại phải
ghi requirement bị vi phạm và bằng chứng; không tự thêm feature 0.2 hoặc lặp
các gate đã hoàn tất nếu không có thay đổi/failure/concern cụ thể.

Việc lập kế hoạch này không tạo PR, merge main, tag hay publication. Các hành
động maintainer chỉ được xin quyết định sau khi hồ sơ cụ thể đã sẵn sàng.
