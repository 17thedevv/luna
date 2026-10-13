; ModuleID = 'D:\fdlang\docs\audits\0.1-alpha-completion-2026-10-04\evidence\compat24\enum-alignment-compiler24.ln'
source_filename = "D:\\fdlang\\docs\\audits\\0.1-alpha-completion-2026-10-04\\evidence\\compat24\\enum-alignment-compiler24.ln"
target datalayout = "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-windows-gnu"

declare ptr @__luna_alloc(i64, i64)

declare void @__luna_dealloc(ptr, i64, i64)

declare ptr @__luna_realloc(ptr, i64, i64, i64)

declare void @__luna_print(ptr, i64)

declare void @__luna_println(ptr, i64)

declare void @__luna_eprintln(ptr, i64)

declare void @__luna_bounds_fail(i64, i64, ptr, i64, i32, i32)

declare void @__luna_overflow_fail(ptr, i32)

declare void @__luna_div_zero_fail(ptr, i32)

declare void @__luna_panic(ptr, i64, ptr, i64, i32, i32)

declare void @__luna_panic_default()

declare void @__luna_startup(i32, ptr)

declare void @__luna_shutdown(i32)

define { i32, [4 x i64] } @_MFN13compatibility4makeE() {
entry0:
  %v1 = alloca { i32, i128 }, align 16
  %v3 = getelementptr inbounds { i32, i128 }, ptr %v1, i32 0, i32 0
  store i32 42, ptr %v3, align 4
  %v6 = getelementptr inbounds { i32, i128 }, ptr %v1, i32 0, i32 1
  store i128 123456789123456789, ptr %v6, align 16
  %v8 = load { i32, i128 }, ptr %v1, align 16
  %enum_alloc = alloca { i32, [4 x i64] }, align 8
  %tag_ptr = getelementptr inbounds { i32, [4 x i64] }, ptr %enum_alloc, i32 0, i32 0
  store i32 0, ptr %tag_ptr, align 4
  %payload_ptr = getelementptr inbounds { i32, [4 x i64] }, ptr %enum_alloc, i32 0, i32 1
  store { i32, i128 } %v8, ptr %payload_ptr, align 16
  %enum_val = load { i32, [4 x i64] }, ptr %enum_alloc, align 8
  ret { i32, [4 x i64] } %enum_val
}
