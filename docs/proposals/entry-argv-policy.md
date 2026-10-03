<!-- luna-doc-role: historical -->

> **Luna 0.1 — historical.** Historical design, plan, or evidence. Original wording is preserved for context; it does not independently define current syntax, capability scope or release readiness. See the [versioned specification](../spec/0.1/README.md).

# Decision Note: Luna 0.1 Entrypoint `main(args: [str])` Arguments & Executable Path Policy

## 1. Context and Problem Statement
In Luna 0.1, the program entrypoint may have signature:
1. `fn main() -> i32` or `fn main()`
2. `fn main(args: [str]) -> i32` or `fn main(args: [str])`

When lowering `main(args: [str])`, the runtime passes native `(int argc, char** argv)`.
A fundamental question arises:
**Does `args` contain the executable path at index 0 (as in C/POSIX `argv[0]`), or does `args` only contain the user-provided arguments (as in modern languages like C# `string[] args`)?**

---

## 2. Policy Analysis

### Option A: Standard POSIX / C-ABI Convention (`args[0]` is executable path)
- `args[0]` is the invocation command or path to the binary (`argv[0]`).
- `args[1..]` are the user-supplied arguments.
- `args.len()` is equal to native `argc`.
- **Pros**: Matches raw C ABI passed to `@main(i32 %argc, ptr %argv)`. No allocation or slicing needed at startup.
- **Cons**: User code must slice `args[1..]` to inspect arguments.

### Option B: User-Arguments Only (`args[0]` is first parameter, separate API for exe path)
- `args` is sliced from `argv[1..argc]`.
- If no CLI arguments were passed by user, `args.len() == 0`.
- Executable path is accessed via `std::env::current_exe()` or similar.
- **Pros**: Cleaner for command-line processing; `args` truly represents user input.
- **Cons**: If program is invoked with `argc == 0` (e.g. `execve` with empty argv), handling requires guarding against negative bounds.

---

## 3. Recommended Specification for Luna 0.1
Until an amendment standardizes Option B with dedicated `current_exe()` support in standard library, **Option A** governs Luna 0.1 low-level entry ABI:
- `args` in `main(args: [str])` reflects the complete argv array provided by the runtime environment (`args.len() == argc`).
- UTF-8 valid null-terminated strings are represented as borrowed `str` slices valid for the duration of `main`.

---

## 4. Acceptance Matrix

| Case | CLI Invocation | `args.len()` | `args[0]` | `args[1]` |
|---|---|---|---|---|
| A1 | `./app` | 1 | `./app` (or binary name) | OOB |
| A2 | `./app foo bar` | 3 | `./app` | `"foo"` |
| A3 | `./app "hello world"` | 2 | `./app` | `"hello world"` |
| A4 | `./app ""` (empty string) | 2 | `./app` | `""` |
