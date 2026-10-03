<!-- luna-doc-role: adopted-contract -->

> **Luna 0.1 — adopted-contract.** Retained detailed contract. Prior acceptance and freeze claims remain dated evidence; current release conformance is tracked separately. See the [versioned specification](../spec/0.1/README.md).

# Luna runtime core ABI — retained contract revision 1

Language baseline: [Luna 0.1](../spec/0.1/README.md). ABI revision 1 is independent of the language version. Current target evidence and gaps are recorded in [runtime.md](../spec/0.1/runtime.md).

## 1. Scope & Status
- **Specification Version**: 1.0.0
- **Status**: adopted core contract; current conformance is not certified by this label
- **Target Profiles**: Hosted (Windows, Linux, macOS), Freestanding (bare-metal, embedded)
- **Calling Convention**: Platform standard C calling convention (`extern "C"`: Microsoft x64 on Windows, System V AMD64 on POSIX).
- **Symbol Namespace**: All runtime public symbols are strictly prefixed with `__luna_`. No auxiliary symbols or un-namespaced identifiers (`malloc`, `free`, `puts`, `printf`, `main`) are exposed by the Luna runtime library.

---

## 2. Type Mappings Across Compiler, C, and LLVM IR

| Logical Concept | Luna Source | C ABI Type | Rust FFI (`extern "C"`) | LLVM IR Type |
|:---|:---|:---|:---|:---|
| Size / Offset | `usize` | `size_t` | `usize` | `i64` (on 64-bit) |
| Alignment | `usize` | `size_t` | `usize` | `i64` (on 64-bit) |
| Raw Memory Pointer | `*u8` / `*void` | `void*` / `uint8_t*` | `*mut u8` / `*const u8` | `ptr` |
| UTF-8 Byte Sequence | `str` / `&[u8]` | `const uint8_t*, size_t` | `*const u8, usize` | `{ ptr, i64 }` or split `ptr, i64` |
| Exit Code / Status | `i32` | `int32_t` | `i32` | `i32` |
| Source Location Line | `u32` | `uint32_t` | `u32` | `i32` |
| Source Location Col | `u32` | `uint32_t` | `u32` | `i32` |

---

## 3. Core runtime ABI functions

### 3.1 Startup & Entrypoint Bridge (Model B: Compiler-Owned Entrypoint)

#### `__luna_startup`
```c
void __luna_startup(int argc, char** argv);
```
- **Direction**: **Exported** by runtime, **Invoked** by compiler-generated `@main`.
- **Role**: Initializes runtime subsystems (stores `argc`/`argv`, initializes standard handles).
- **Contract**: Non-terminal; returns cleanly to let execution proceed.

#### `__luna_start`
```c
int32_t __luna_start(int argc, char** argv);
```
- **Direction**: **Emitted** by compiler for executable targets, **Invoked** by compiler-generated `@main`.
- **Role**: The compiler-generated executable entry shim separating OS arguments from Luna semantics.
- **Contract**:
  1. If user `main` accepts `(args: [str])`: constructs slice `[str]` from `argv[0..argc]`.
  2. Invokes user-defined `__luna_user_main(...)`.
  3. If user `main` returns `void`: normalizes return code to `0`.
  4. If user `main` returns `i32`: propagates the return code directly.
  5. Drops temporary arguments allocation if any.
  6. Returns normalized `i32` exit code to `@main`.

#### `__luna_shutdown`
```c
LUNA_NORETURN void __luna_shutdown(int32_t exit_code);
```
- **Direction**: **Exported** by runtime, **Invoked** by compiler-generated `@main`.
- **Role**: Flushes stdio streams and terminates process via platform exit (`exit(exit_code)`).
- **Contract**: Terminal `[[noreturn]]`. In LLVM IR, must be followed immediately by `unreachable`.

---

### 3.2 Memory Allocation Subsystem

#### Alignment Limit
```c
#define LUNA_MAX_ALIGN 4096
```
- Every `align` parameter must satisfy:
  $$0 < \text{align} \le \text{LUNA\_MAX\_ALIGN} \quad \land \quad (\text{align} \mathrel{\&} (\text{align} - 1)) == 0$$
- Any alignment violation triggers an immediate deterministic abort (`LUNA_ERR_INVALID_STATE`).

#### Identity-Based Zero-Size Sentinel
- Zero-size allocations return a shared, runtime-owned sentinel object aligned to `LUNA_MAX_ALIGN`:
  ```c
  static _Alignas(LUNA_MAX_ALIGN) const unsigned char g_luna_zero_sentinel[1] = { 0 };

  static inline int is_zero_sentinel(const void* ptr) {
      return ptr == (const void*)g_luna_zero_sentinel;
  }
  ```

#### `__luna_alloc`
```c
void* __luna_alloc(size_t size, size_t align);
```
- **Parameters**:
  - `size`: Number of bytes to allocate.
  - `align`: Memory alignment requirement ($0 < \text{align} \le \text{LUNA\_MAX\_ALIGN}$, power of 2).
- **Return**:
  - If `size == 0`: Returns `(void*)g_luna_zero_sentinel`.
  - If `size > 0`: Returns non-null aligned pointer satisfying $ptr \pmod{align} == 0$.
- **Failure Contract**:
  - Round-up overflow (`size > SIZE_MAX - (align - 1)`) or OOM aborts immediately via `__luna_panic_code(LUNA_ERR_ALLOC_FAILURE, ...)`.
  - **Never returns `NULL`** under this retained allocation contract.

#### `__luna_dealloc`
```c
void __luna_dealloc(void* ptr, size_t size, size_t align);
```
- **Parameters**:
  - `ptr`: Pointer previously returned by `__luna_alloc` or `__luna_realloc`.
  - `size`: Allocation size metadata.
  - `align`: Allocation alignment metadata.
- **Contract**:
  - If `ptr == NULL || is_zero_sentinel(ptr)`: Returns immediately (guaranteed no-op).
  - Deallocating real heap memory calls platform-aligned free (`_aligned_free` / `free`).
  - Mismatched metadata or double-free is undefined behavior.

#### `__luna_realloc`
```c
void* __luna_realloc(void* ptr, size_t old_size, size_t old_align, size_t new_size);
```
- **Parameters**:
  - `ptr`: Original allocation pointer (or `NULL` / sentinel).
  - `old_size`: Exact size of current allocation.
  - `old_align`: Exact alignment of current allocation.
  - `new_size`: Desired new allocation size in bytes.
- **Contract**:
  - If `ptr == NULL || is_zero_sentinel(ptr)`: Delegates to `__luna_alloc(new_size, old_align)`.
  - If `new_size == 0`: Calls `__luna_dealloc(ptr, old_size, old_align)` and returns `(void*)g_luna_zero_sentinel`.
  - If `new_size > 0`: Allocates new aligned block of `new_size`, copies $\min(old\_size, new\_size)$ bytes (only if old block was not a sentinel), deallocates old block, and returns new pointer.
  - OOM failure triggers deterministic abort; old block is untouched prior to termination.

---

### 3.3 Panic & Diagnostic Traps

#### Public Primary Panic: `__luna_panic`
```c
LUNA_NORETURN void __luna_panic(
    const uint8_t* msg_ptr,
    size_t         msg_len,
    const uint8_t* file_ptr,
    size_t         file_len,
    uint32_t       line,
    uint32_t       col
);
```
- **Contract**: Writes diagnostic to `stderr` and aborts process with exit code `101`:
  ```text
  luna: PANIC at <file>:<line>:<col>: <message>
  ```
- Terminal `[[noreturn]]`.

#### Internal Error-Code Panic Helper: `__luna_panic_code`
```c
LUNA_NORETURN void __luna_panic_code(
    uint32_t       error_code,
    const uint8_t* msg_ptr,
    size_t         msg_len,
    const uint8_t* file_ptr,
    size_t         file_len,
    uint32_t       line,
    uint32_t       col
);
```
- Used internally by runtime allocator (OOM) and traps to emit standardized error codes (`[L001]`, `[L002]`, etc.) under its separately documented runtime service contract.

#### Bounds Check Trap: `__luna_bounds_fail`
```c
LUNA_NORETURN void __luna_bounds_fail(
    size_t         index,
    size_t         len,
    const uint8_t* file_ptr,
    size_t         file_len,
    uint32_t       line,
    uint32_t       col
);
```
- Writes out-of-bounds message to `stderr` and aborts process with exit code `102`:
  ```text
  luna: PANIC [M002] index out of bounds: index <index>, len <len> at <file>:<line>:<col>
  ```

---

### 3.4 Primitive Unformatted Stdio

```c
void __luna_print(const uint8_t* bytes, size_t len);
void __luna_println(const uint8_t* bytes, size_t len);
void __luna_eprintln(const uint8_t* bytes, size_t len);
```
- Raw unformatted byte writes.
- `println` appends `\n` to `stdout` and flushes.
- `eprintln` appends `\n` to `stderr` and flushes.

---

## 4. Summary of the Public Stable ABI Symbols

| Symbol | Signature | Owner | Role |
|:---|:---|:---:|:---|
| `__luna_startup` | `(int argc, char** argv) -> void` | Runtime | Process state initialization |
| `__luna_start` | `(int argc, char** argv) -> i32` | Compiler | Executable entrypoint shim |
| `__luna_shutdown` | `(i32 exit_code) -> !` | Runtime | Process flush & exit |
| `__luna_alloc` | `(size_t size, size_t align) -> void*` | Runtime | Infallible heap allocation |
| `__luna_dealloc` | `(void* ptr, size_t size, size_t align) -> void` | Runtime | Explicit heap deallocation |
| `__luna_realloc` | `(void* ptr, size_t old_size, size_t old_align, size_t new_size) -> void*` | Runtime | Infallible heap reallocation |
| `__luna_panic` | `(msg_ptr, msg_len, file_ptr, file_len, line, col) -> !` | Runtime | Terminal panic trap |
| `__luna_bounds_fail` | `(index, len, file_ptr, file_len, line, col) -> !` | Runtime | Terminal bounds violation trap |
| `__luna_print` | `(bytes, len) -> void` | Runtime | Unformatted stdout write |
| `__luna_println` | `(bytes, len) -> void` | Runtime | Unformatted stdout write + newline |
| `__luna_eprintln` | `(bytes, len) -> void` | Runtime | Unformatted stderr write + newline |

---

## 5. Boundary Classification: Public ABI vs Internal Symbols

To ensure future ABI stability and prevent accidental leakage of implementation details, symbols are categorized strictly into two tiers:

### 5.1 Tier 1: Public Stable ABI Contract (Frozen)
Section 4 describes the retained core subset. Additional documented runtime service headers and adopted file/I/O contracts extend that subset; it is not the complete current export inventory.
- Compiler lowering and standard library bindings must use a documented ABI contract, including separately documented service headers where applicable.
- These signatures are guaranteed stable across runtime minor versions.

### 5.2 Tier 2: Internal Runtime Symbols (Private Implementation Details)
The following symbols are internal to `luna-runtime` and **MUST NOT** be depended upon by compiler codegen or user libraries:
- `__luna_panic_code`: Internal diagnostic trap supporting numeric error codes.
- `__luna_is_zero_sentinel`: Internal sentinel pointer inspection helper for runtime unit tests.
- `__luna_abort`: Fallback process termination primitive.
- `__luna_argc`, `__luna_argv`: Internal accessors for runtime startup state.
- No legacy free alias is part of the current identity contract; callers use `__luna_dealloc`.
- `__luna_mem_copy`, `__luna_mem_move`, `__luna_mem_set`: Freestanding fallback primitives (compiler prefers LLVM intrinsics).
