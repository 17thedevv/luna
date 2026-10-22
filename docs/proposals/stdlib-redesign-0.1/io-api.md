<!-- luna-doc-role: guidance -->

# Thiết kế API I/O của Luna

Ngày 2026-10-23. Trạng thái: **bản thiết kế để review, chưa triển khai/freeze**.
Maintainer yêu cầu thiết kế I/O trước các đợt math/matrix/graph và chọn nội suy
`$tenbien`, thay cho placeholder `{}`. Lựa chọn cú pháp này đã được xác nhận;
các chi tiết contract dưới đây là đề xuất cụ thể, không tự thay baseline 0.1.

Nguồn hiện trạng: code pin `28802f12`, cùng source ở recording pin `c56d9fae`.
[Bản đồ declaration](io-current-surface.json) gắn từng record với owner,
dependency, artifact và disposition; đây chưa phải resolved export catalog.
[Contract nội suy](io-interpolation.md) tách capability ngôn ngữ khỏi policy I/O.

## 1. Mục tiêu và ranh giới

| Tầng | Trách nhiệm | Surface chính |
| --- | --- | --- |
| Byte stream | Đọc/ghi byte bất kỳ, EOF, partial I/O, flush, seek | std::io::Read / Write / Seek / BufRead |
| Text | UTF-8, dòng, chuyển byte thành String có kiểm tra | std::io::read_line / write_text / write_str |
| Formatting | Giá trị -> Unicode scalar qua trait, nội suy tên biến | std::fmt::Writer / Display; các macro |
| Resource | File, TCP stream, standard streams | std::File, std::TcpStream, std::io::Stdin/Stdout/Stderr |
| Runtime | Native handle, syscall/CRT, status và OS error | Bridge private; không biết String/Vec/Display |

Không tạo API nhập số kiểu scanf hoặc một `read<T>` đoán cách parse. Đọc text
và parse số là hai việc độc lập. UDP giữ biên datagram, không implement byte
stream Read/Write chỉ để dùng chung tên. Async I/O là thiết kế khác.

Giữ gốc `luna-rs/libs/external`. Đợt này thiết kế API; không di chuyển provider.
`import <io>` và namespace std::io là hai khái niệm khác nhau. File/TcpStream
tiếp tục ở std; không suy ra std::file/std::net từ đường dẫn source.

## 2. Quy ước chung

- Độ dài buffer, capacity, index và số byte của một vùng nhớ: usize.
- Vị trí stream/file và tổng luồng có thể lớn hơn một buffer: u64;
  offset tương đối của seek: i64. Không cast cắt cụt khi bridge platform hẹp hơn.
- `&[u8]` cho input; `&rw [u8]` cho destination đã khởi tạo. Không yêu cầu caller
  truyền raw pointer hay tự cấp phát vùng chưa khởi tạo cho safe I/O.
- Thao tác có thể lỗi trả Result. EOF không phải lỗi của read thông thường.
- Lỗi I/O không âm thầm biến thành panic, thành số byte 0 hoặc thành Ok.
- Ordinary constructor có allocation giữ policy abort của STD-D06; constructor
  try_* phải trả lại owned input khi thất bại. Helper I/O đã trả Result sử dụng
  fallible growth và báo lỗi allocation trong Result.
- Literal lấy kiểu từ ngữ cảnh: `buf.len() != 0`, capacity `4096`, không cần rải
  `as usize`. Chuyển biến đã có kiểu hoặc bridge ABI là việc riêng.

## 3. Byte protocol

Các signature sau là **target contract**, không phải API mới đã chạy được:

```luna
export trait Read {
    fn read(self: &rw Self, dst: &rw [u8]) -> std::Result<usize, IoError>;
}
export trait Write {
    fn write(self: &rw Self, src: &[u8]) -> std::Result<usize, IoError>;
    fn flush(self: &rw Self) -> std::Result<void, IoError>;
}
export trait Seek {
    fn seek(self: &rw Self, from: SeekFrom) -> std::Result<u64, IoError>;
}
export trait BufRead: Read {
    fn fill_buf(self: &rw Self) -> std::Result<&[u8], IoError> life_from(self);
    fn consume(self: &rw Self, count: usize) -> std::Result<void, IoError>;
}
```

Traits chỉ có primitive tối thiểu. Helper là hàm generic ở std::io; không cần
thêm default method hoặc extension trait trước khi chứng minh nhu cầu.

### read và write

| Tình huống | Contract |
| --- | --- |
| Buffer rỗng | Ok(0), không gọi OS, không đổi vị trí stream |
| read Ok(n) | 0 <= n <= dst.len; prefix n byte là dữ liệu mới; phần còn lại giữ nguyên |
| read Ok(0), dst không rỗng | EOF tại thời điểm gọi; không đồng nhất với WouldBlock |
| write Ok(n) | Chỉ prefix n byte được sink chấp nhận; caller xử lý suffix |
| write Ok(0), src không rỗng | Không tiến triển; write_all chuyển thành WriteZero |
| Interrupted | Helper có thể retry phần còn lại; không ghi/đọc lại prefix đã tiến triển |
| WouldBlock / TimedOut | Trả caller; không busy-loop, không coi là EOF |

Primitive chỉ mượn buffer trong lời gọi, không giữ pointer sau khi return.
Native adapter có giới hạn DWORD/ssize_t phải chia request hoặc trả short count
đúng; không truncate usize để vừa ABI. Read/Write là safe trait: helper phải kiểm
count/progress <= slice length trước mọi offset/raw operation. Impl báo sai count
không được biến thành truy cập ngoài vùng nhớ trong code thư viện.

`flush` đẩy buffer do writer quản lý xuống tầng kế tiếp. Ok không có nghĩa
dữ liệu đã bền trên đĩa hoặc peer đã xử lý. File::sync_all là thao tác durability
riêng, cần primitive platform và acceptance riêng.

### SeekFrom

Giữ Start(u64), Current(i64), End(i64). Seek trả vị trí tuyệt đối. Tính signed
offset phải kiểm underflow/overflow; không tạo vị trí âm. Stream không seek được
trả Unsupported. Seek vượt EOF có thể hợp lệ; read sau đó trả EOF, write theo
contract của resource. Không âm thầm giới hạn public u64 thành i64.

## 4. Lỗi và tiến triển khi thất bại

Đề xuất giữ tên std::io::IoError nhưng đổi từ enum phẳng thành struct có field
private. IoErrorKind là enum phân loại; đây là breaking API cần migration.

| Accessor/constructor | Ý nghĩa |
| --- | --- |
| IoError::new(kind) | Lỗi không có OS detail, progress bằng 0 |
| kind() -> IoErrorKind | Phân loại portable, không phân nhánh theo chuỗi message |
| os_error() -> Option<OsError> | Domain Posix/Win32/WinSock và native code, nếu có |
| read_bytes() -> u64 | Byte đã lấy khỏi logical input bởi lời gọi thất bại này |
| written_bytes() -> u64 | Byte đã được logical sink chấp nhận bởi lời gọi này |
| with_read_progress(self: Self, usize) -> Self / with_write_progress(self: Self, usize) -> Self | Đặt counter của hướng tương ứng, không cộng dồn ngầm; helper cộng dồn bằng phép toán checked |
| fmt_error() -> Option<std::fmt::FmtError> | Giữ nguyên nguyên nhân formatter nếu có |

IoErrorKind đề xuất: NotFound, PermissionDenied, AlreadyExists, InvalidInput,
InvalidData, Interrupted, WouldBlock, TimedOut, UnexpectedEof, WriteZero,
BrokenPipe, ConnectionRefused, ConnectionReset, ConnectionAborted, NotConnected,
Unsupported, OutOfMemory, SizeOverflow, LimitExceeded, FormattingFailed, Other.
Không hứa mọi OS error có mapping riêng; Other vẫn giữ native detail.
OsError có domain() -> OsErrorDomain và code() -> i64; domain là Posix, Win32
hoặc WinSock. Native code chỉ có nghĩa trong domain đó, không phải portable enum.

Progress tính theo lời gọi ngoài cùng, không cộng trùng error từ nested helper.
Read-ahead vẫn nằm trong BufReader chưa tính là byte caller đã consume.
Primitive read/write Err có thể báo prefix khác 0; destination/prefix tương ứng
phải hợp lệ. Không có retry toàn bộ request sau một lỗi có progress.

`copy` có thể đọc nhiều hơn đã ghi; hai counter giữ rõ khác biệt đó. Khi copy
thất bại, pending scratch không được trả lại bởi API convenience này: không hứa
resume được một socket stream. Ứng dụng cần giữ pending bytes phải tự quản lý
buffer và vòng read/write. Cả thành công lẫn lỗi đều không hứa rollback stream.

Nếu BufWriter::write phải flush dữ liệu cũ rồi lỗi trước khi nhận input mới,
written_bytes đối với input mới bằng 0. State buffer cũ vẫn phản ánh phần đã
commit. Counter của flush nội bộ không được dùng để skip input của lời gọi write.

IoError không cần allocation để tạo hoặc đọc kind/code/counter. Error::description
và Display dùng để diễn giải, không điều khiển flow. `?` chỉ truyền cùng error
qua Try/FromResidual đang có; không thêm implicit conversion mọi FileError/
NetError/FmtError chỉ vì có trait Error.

Counter không wrap. Helper giới hạn request kế tiếp theo phần u64 còn biểu diễn
được và dừng SizeOverflow trước khi tiêu thụ thêm byte không thể hạch toán.

Mapping bắt buộc: invalid UTF-8 -> InvalidData; OOM -> OutOfMemory;
allocation/count overflow -> SizeOverflow; FmtError không có cause I/O cụ thể ->
FormattingFailed. Adapter có IoError thật giữ nguyên kind/cause của lỗi đó,
không đổi BrokenPipe thành FormattingFailed. Template/name/trait error là compile-time
diagnostic, không phải IoError runtime. write_str kiểm text trước khi ghi nên
invalid UTF-8 ở đó có written_bytes = 0.

## 5. Helper generic

Các hàm thuộc std::io. Bảng dùng tên type ngắn để dễ đọc signature.

| Hàm | Kết quả | Contract |
| --- | --- | --- |
| read_exact<R: Read>(&rw R, &rw [u8]) | Result<void, IoError> | Đủ buffer hoặc UnexpectedEof/lỗi; prefix đã đọc được giữ và báo progress |
| write_all<W: Write>(&rw W, &[u8]) | Result<void, IoError> | Ghi hết hoặc trả lỗi cùng prefix; không tự flush |
| read_to_end<R: Read>(&rw R, &rw Vec<u8>) | Result<usize, IoError> | Append đến EOF; trả số byte thêm, không xóa dữ liệu cũ |
| read_up_to<R: Read>(&rw R, &rw Vec<u8>, limit: usize) | Result<usize, IoError> | Append tối đa limit; 0 trả ngay; không consume quá limit |
| copy<R: Read, W: Write>(&rw R, &rw W) | Result<u64, IoError> | Chạy đến EOF, báo cả read/write progress khi lỗi; không tự flush |
| read_until<R: BufRead>(&rw R, delimiter: u8, &rw Vec<u8>, max_bytes: usize) | Result<usize, IoError> | Append đến và gồm delimiter hoặc EOF; max_bytes > 0, không consume quá giới hạn |
| read_line<R: BufRead>(&rw R, &rw String, max_bytes: usize) | Result<usize, IoError> | Một dòng UTF-8, giữ LF/CRLF; chỉ commit String sau khi toàn dòng hợp lệ |
| write_text<W: Write>(&rw W, &String) | Result<void, IoError> | Ghi UTF-8 theo độ dài String, kể cả NUL bên trong; không thêm LF |
| write_str<W: Write>(&rw W, str) | Result<void, IoError> | Quét đến NUL đầu tiên, kiểm UTF-8 rồi ghi; không tự đổi str thành slice |

read_to_end có thể chờ mãi trên input không EOF hoặc dùng nhiều memory; caller
cần giới hạn chọn read_up_to. Không ngầm đổi tên read_to_end thành đọc một chunk.
Khi growth lỗi, Vec giữ dữ liệu ban đầu và prefix đã append; không abort vì OOM.

read_until giữ prefix trong Vec khi lỗi/limit. read_line dùng staging fallible:
String cũ giữ nguyên nếu invalid UTF-8, OOM, limit hoặc I/O error. Input đã consume
không rollback; read_bytes báo số đó. Muốn giữ raw invalid bytes, dùng read_until
rồi tự chuyển UTF-8. LF là byte 0x0A; CRLF được giữ nguyên. Bỏ newline là bước
caller làm rõ ràng, không tự trim whitespace.

EOF trước byte đầu tiên trả Ok(0); dòng cuối thiếu LF vẫn trả Ok(n). Dòng trống
`\n` trả 1, không phải EOF. max_bytes bao gồm delimiter. Khi đã đạt giới hạn mà
chưa gặp LF, BufRead cho phép peek EOF mà không consume byte kế tiếp: EOF thì
thành công, còn dữ liệu thì LimitExceeded. max_bytes = 0 là InvalidInput.

## 6. Console và formatting

| Macro đích | Kết quả | Đích |
| --- | --- | --- |
| std::print!("...") | Result<void, IoError> | stdout |
| std::println!("...") / std::println!() | Result<void, IoError> | stdout, thêm đúng một LF |
| std::eprint!("...") | Result<void, IoError> | stderr |
| std::eprintln!("...") / std::eprintln!() | Result<void, IoError> | stderr, thêm đúng một LF |
| std::write!(&rw writer, "...") | Result<void, IoError> | Writer: std::io::Write |
| std::writeln!(&rw writer, "...") | Result<void, IoError> | Như write!, thêm LF |
| std::format!("...") | Result<String, FormatError> | Tạo owned UTF-8; không I/O |

FormatError thuộc std::fmt nhưng được sở hữu ở tầng có alloc: các cause
Formatting(FmtError), Allocation(AllocError), InvalidText. Không thêm dependency
alloc vào core/fmt. Đợt đầu io/io.ln sở hữu allocating macro/adapters/FormatError,
đóng góp vào std và std::fmt; thêm import string/alloc_error. Đây là placement
tạm có chủ đích để giữ provider/root hiện tại; tách provider là migration riêng.

Các ví dụ dưới đây mô tả **API đích, chưa phải executable hiện tại**:

```luna
import <io>;
dec name = "Luna";
dec count: usize = 4;
std::println!("Hello $name, count=$count");
std::println!("Giá: $$10");
std::write!(&rw output, "name=$name");
```

Đây không phải overload nhận thêm danh sách argument như `("x={}", x)`.
[Nội suy $name](io-interpolation.md) định nghĩa grammar, escaping, lookup,
evaluation, lifetime, diagnostics và generic compiler boundary.

Tên canonical là macro trong std. Short spelling println!/print! hiện có chỉ
là forwarding export từ provider io khi import, dùng chung một implementation;
không tạo prelude compiler tự nạp I/O. Alias out/outln/err/errln là compatibility
tạm thời, không tiếp tục nhân đôi macro body. Removal cần release note/migration.

Console macro render vào staging UTF-8 bằng fallible allocation trước khi ghi.
Formatting/encoding/OOM thất bại thì chưa có output từ lời gọi đó. Sau render,
giữ khóa của đúng standard stream qua write_all và flush; chỉ ngăn interleave
với lời gọi Luna dùng cùng khóa, không hứa atomic với OS/process khác. Không
giữ khóa khi chạy user Display, tránh deadlock khi Display gọi console.

Fallible staging dùng adapter riêng, không gọi lại Writer for String đang có
aborting growth rồi giả rằng Result đã xử lý OOM. Mỗi write_char reserve đủ
encoding của cả scalar trước khi append; lỗi không để lại nửa scalar trong String.

Đề xuất cả bốn console macro flush trước Ok để prompt không phụ thuộc newline.
std::write!/writeln! trên writer caller cung cấp không tự flush, không tự lock,
không cấp buffer staging bắt buộc; lỗi có thể để lại prefix. `println` thêm LF
kể cả template đã có LF. Không tự đổi LF thành CRLF; byte redirection giữ nguyên.

Console handles stdin()/stdout()/stderr() là capability mượn standard resource,
không sở hữu/close handle toàn tiến trình. Mỗi stream dùng chung state/khóa cho
mọi wrapper; Stdin dùng chung input buffer, không tạo private read-ahead rồi
vứt khi wrapper tạm drop. Stdin implement Read và có read_line/read_until method
giữ khóa trong lời gọi, nhưng **không implement BufRead trả slice của buffer
global**: wrapper Stdin thứ hai có thể mutate buffer đó mà borrow của wrapper
đầu không chặn được. Generic read_line cần BufRead có thể dùng BufReader<R> với
buffer thuộc chính reader. Không trộn nhiều reader trên cùng input nếu cần
ordering một luồng liên tục. Public lock guard API để đợt riêng sau borrow audit.

Trong console macro đã giữ khóa, dùng adapter private không acquire khóa lần
hai; không gọi ngược public Stdout::write đang tự lock rồi gây deadlock.

core/fmt giữ Writer::write_char và Display::fmt đã chốt. Byte Write không tự
trở thành text Writer. Adapter encode Unicode scalar thành UTF-8 và giữ IoError
thật khi Writer chỉ trả FmtError::WriteFailed; không được bịa Ok rồi mất lỗi.
String và literal Unicode phải decode đúng; không coi từng byte UTF-8 là char.
str vẫn là NUL-terminated value, không đổi thành borrowed text slice.

Không tự thêm Display cho byte vector, float, Debug, width/precision, locale
hoặc terminal control. Các implementation ngoài nền formatting đã chốt cần
contract riêng; không dùng nội suy để lén đóng CORE-GAP-18 về float formatting.

### Console và redirection trên hai platform

File/pipe/socket I/O giữ byte nguyên vẹn, không CRLF translation hay NUL
truncation. Khi stdout/stderr được redirect, console macro cũng ghi đúng UTF-8
bytes đã render. Với terminal thật, macro text phải hiển thị Unicode qua native
console mechanism phù hợp; không dựa vào codepage mặc định trên Windows.
Byte Write không hứa arbitrary invalid UTF-8 sẽ thành glyph có nghĩa trên TTY.
Không tự thay đổi codepage/CRT mode toàn tiến trình làm đổi behavior của code khác.
Stdin terminal text bridge cần giữ UTF-8 byte encoding và phần encoded pending
trong shared buffer; input redirect vẫn byte-exact. Terminal decoding/count và
OS status capture là native-boundary work cần prototype/acceptance riêng.

## 7. Buffer và Cursor

Tất cả type tiếp tục dưới std::io; constructor trên type.

| Type/API | Contract đích |
| --- | --- |
| Cursor<T>::new(inner: T) -> Self | Move inner một lần; không cấp phát chỉ để tạo cursor |
| Cursor<Vec<u8>>: Read/Write/Seek | Read EOF không grow; write/gap dùng try_reserve, overflow kiểm trước mutation |
| position()/set_position(u64) | Vị trí stream; chỉ đổi position không tạo memory hay truncate |
| BufReader<R>::new/with_capacity | Owned reader; capacity usize, request 0 được nâng thành tối thiểu 1; allocation thường có thể abort |
| BufWriter<W>::new/with_capacity | Owned writer; capacity usize, request 0 được nâng thành tối thiểu 1; không tạo mode pass-through ngầm |
| try_new / try_with_capacity | Result<Self, AllocFailure<R/W>>, trả lại nguyên inner nếu allocation lỗi |
| buffer() | Borrow phần unread/pending, life_from(self); hết borrow mới được mutate |
| get_ref() | &inner, life_from(self); không bypass mutable buffer state |
| into_parts(self) | Move (inner, Vec<u8> unread/pending), không I/O, không tự bỏ bytes |

Đợt đầu không thêm blanket Read/Write cho mọi T của Cursor; mỗi storage cần
contract và tests riêng. Cursor<Vec<u8>> giữ get_ref/get_mut/into_inner với
lifetime và ownership chính xác; buffer adapter không có lossy into_inner.

Default capacity là chi tiết implementation; with_capacity hứa ít nhất
max(request, 1), không hứa allocator cấp chính xác số byte yêu cầu. Quy tắc
request 0 áp dụng cả try_with_capacity, không tạo lỗi allocation giả cho input 0
và không khiến BufReader báo EOF khi underlying reader vẫn còn dữ liệu.

BufReader implement Read + BufRead. fill_buf không consume. consume(n) chỉ
chấp nhận n <= unread.len, sai trả InvalidInput và không đổi state. Không trả
borrow rồi cho phép refill/consume/move khi borrow còn sống.

BufWriter::write có thể trả short count. Buffer giữ đúng suffix chưa được inner
chấp nhận. flush sau khi inner đã nhận prefix rồi lỗi phải loại prefix đó ngay;
retry chỉ ghi suffix. Nếu inner.flush lỗi sau khi buffer rỗng, retry flush không
được phát lại dữ liệu. Helper cần cập nhật state trên cả Ok(n) lẫn Err có progress.

Đề xuất BufWriter không tự flush trong destructor; Drop vẫn drop inner và buffer
đúng một lần theo contract riêng của chúng. Caller phải flush để quan sát lỗi và
đảm bảo phát hết buffer. into_parts trả pending cho caller nếu cần recovery.
Không tuyên bố drop là successful write. Đây là policy cần review, không dựa vào
convention của ngôn ngữ khác. Buffered Seek/get_mut và consuming finish có
recovery wrapper để đợt sau; không hứa before feasibility/ownership audit.

## 8. File và mạng

- std::File implement Read/Write/Seek; inherent read/write/flush có cùng contract
  và IoError, không giữ một behavior khác chỉ vì gọi trực tiếp.
- File::seek nhận SeekFrom thay cho (offset, whence: i32). Numeric whence không
  thuộc safe public API đích.
- File::close(self: Self) -> Result<void, IoError> consume resource; không dùng
  lại sau cả Ok/Err. Drop là cleanup best-effort, không trả lỗi hay double-close.
  close bị interrupted cần policy platform riêng, không blind retry handle có
  thể đã được hệ thống tái sử dụng.
- File::flush khác File::sync_all. Whole-file helpers giữ tên/byte contract cũ
  trong khi chuyển errors; không tự thêm một std::read alias mơ hồ.
- Path input hiện là &String, kiểm NUL/precondition như WHOLE-FILE-IO-v1.
  Path/literal ergonomics và lexical path redesign không được ngầm đổi ở đây.
- FileError -> IoError là migration có breaking change, gồm Display/messages,
  match code và whole-file callers. Các message cũ đã chốt không tự đổi bởi doc.
- std::TcpStream implement Read/Write qua native adapter; connect/listen/resolve
  vẫn có NetError domain. Không ép tất cả lỗi mạng/địa chỉ thành IoError.
- std::UdpSocket vẫn send_to/recv_from; datagram rỗng không phải stream EOF.

## 9. Ví dụ sử dụng API đích

Các đoạn này chỉ minh họa thiết kế, chưa là chương trình được compiler hiện tại
hỗ trợ. Ví dụ dùng Result để caller chọn cách xử lý lỗi; không dùng unwrap ngầm.

```luna
fn greet() -> std::Result<void, std::io::IoError> {
    dec name = "Luna";
    dec count: usize = 4;
    std::println!("Hello $name, count=$count")?;
    return std::Result::Ok;
}
```

```luna
fn prompt() -> std::Result<void, std::io::IoError> {
    dec rw input = std::io::stdin();
    dec rw line = std::String::new();
    std::print!("Tên: ")?;
    dec n = input.read_line(&rw line, 65536)?;
    if n == 0 { return std::Result::Ok; }
    // read_line giữ newline, nên print không thêm một LF thứ hai.
    std::print!("Bạn nhập: $line")?;
    return std::Result::Ok;
}
```

```luna
fn copy_file(source: &std::String, target: &std::String)
    -> std::Result<void, std::io::IoError> {
    dec rw input = std::File::open(source)?;
    dec rw output = std::File::create(target)?;
    std::io::copy(&rw input, &rw output)?;
    output.flush()?;
    output.close()?;
    input.close()?;
    return std::Result::Ok;
}
```

Đây là streaming copy không atomic replace; source/target không được cùng file.
API atomic copy/replace cần filesystem contract riêng.

## 10. Dependency và feasibility

| Phần | Disposition | Điều kiện |
| --- | --- | --- |
| Helper loops, limits, error/progress, buffer state | IMPLEMENT | Ordinary generic Luna; không tin count của safe impl trước bounds check |
| Stream/file/console adapters, UTF-8 bridge | SAFE-API-VIA-AUDITED-UNSAFE | Native count/status, pointer validity, initialized output, NUL/encoding và ownership proof |
| Borrowed BufRead buffer | SAFE-API-VIA-AUDITED-UNSAFE | CLI borrow/escape tests và portable Result<&[u8]> contract; không blanket claim |
| $name trong template macro | LANGUAGE-BLOCKED cho machinery hiện tại | Cần generic opt-in literal segment capability; xem tài liệu nội suy |
| Float/precision/Debug/async/vectored/public lock guards | DEFERRED | Contract riêng; không tự sửa compiler vì thiếu API |

Không ghi C-GAP cho một feature ngôn ngữ mới chưa được triển khai. Nếu ordinary
Luna đúng contract lộ bug, phải có reproducer user-defined, sửa machinery chung,
regression rồi mới quay lại I/O. Runtime extension phải là thay đổi riêng với
ABI versioning/status/count tests; không đổi prototype hiện hành trong lặng lẽ.

## 11. Thứ tự triển khai và acceptance

1. Review IoError/progress và policy Result, newline, flush, buffer Drop.
2. Giữ provider hiện tại, sửa byte protocols/helpers và mock source fixtures.
3. Đồng nhất File/TCP adapters; ghi riêng nhu cầu runtime/ABI chưa đáp ứng.
4. Hoàn thiện buffer recovery/UTF-8/text, constructor và OOM ownership.
5. Thiết kế rồi triển khai capability nội suy generic; sau đó mới đổi console
   macro sang $name. Không đổi nghĩa macro đang dùng nối giá trị ở giữa release.
6. Migration consumer/docs/Lunar templates, rebuild coherent artifacts, rồi
   qualification Windows/Ubuntu và package/installer tại final pin.

| Acceptance dimension | Ca bắt buộc |
| --- | --- |
| Partial/zero/interrupt | Scripted reader/writer: 1-byte chunks, short count, Interrupted trước/sau prefix, WouldBlock, WriteZero, EOF giữa read_exact |
| Progress | Lỗi sau k byte, nested helpers không cộng trùng, copy read > write, count sai không vượt bounds |
| Buffer | Flush prefix rồi lỗi/retry không duplicate; into_parts giữ suffix; no hidden flush in Drop; inner drop đúng một lần |
| Borrow | fill_buf/buffer get_ref escape, consume/refill/move khi còn loan; gọi lại được sau loan chết |
| Text | ASCII, Unicode 2/3/4-byte qua chunk boundary, invalid UTF-8, NUL, LF/CRLF, EOF cuối dòng, limit đúng biên |
| Allocation | OOM ở mọi growth/staging boundary; giữ String cũ và owned inner; không abort ở helper fallible |
| Console | stdout/stderr đúng đích, exact bytes/LF, redirected output/broken pipe, flush error, không giả Ok |
| Formatting | User Display + failing sink, lỗi thật không bị mất, template/name/trait errors và lifetime/drop |
| Artifact | Isolated source-only/fresh artifact-only CLI check/build/native; stale dependent rejection và body-only controls |
| Platform/release | Windows + Ubuntu, Unicode console/redirected bytes, complete workspace, package + installer smoke |

## 12. Migration cần review

| API hiện tại | Đích đề xuất |
| --- | --- |
| io::print/println/eprint/eprintln(&[u8]) -> void | Retire sau migration; bytes qua Write/write_all, formatted output qua macro |
| io::*_str(str) -> void | write_str trên stream + macro cho literal; không giữ lỗi bị nuốt |
| read_line(&rw Vec<u8>) | read_until cho bytes; read_line(&rw String, max_bytes) cho text |
| print_val/eprint_val/newline helper public + duplicated aliases | Chi tiết implementation private/forwarding; không là catalog API mới |
| *_new / *_with_capacity buffer/cursor factories | Type::new / with_capacity và try_* theo STD-D03/D06 |
| BufReader/BufWriter::into_inner | into_parts giữ unread/pending, không silent loss |
| FileError riêng cho stream I/O | IoError; FileError compatibility phải có kế hoạch rõ |
| File::seek(i64, i32), close(&rw Self) -> void | seek(SeekFrom), consuming close -> Result |
| Macro nối 1..4 giá trị | Template $name; không giữ cùng spelling với hai cách hiểu mâu thuẫn |

Chưa xóa alias hoặc thay behavior nào bằng tài liệu này. Spec nền
[formatting v1](../../../luna-rs/docs/phase6_core_formatting_v1.md),
[whole-file I/O](../../../luna-rs/docs/whole_file_io_v1.md),
[macro boundary](../../spec/0.1/macros.md) tiếp tục là authority cho implementation
hiện hành; amendment/acceptance mới phải ghi riêng.

Tham khảo cơ chế platform, không dùng làm semantics của Luna:
[Microsoft WriteFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-writefile)
cho count/error và short write,
[Microsoft ReadFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-readfile)
cho native read status. API C không thay cho Result, ownership hoặc UTF-8 contract.

SKILL IMPACT: none. Đây là proposal và lựa chọn $name của maintainer; không sửa
skill thành một freeze hoặc một capability chưa được chứng minh.
