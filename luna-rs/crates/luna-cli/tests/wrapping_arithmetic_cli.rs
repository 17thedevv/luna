//! E1 Stage 2B — explicit wrapping intrinsics.
//!
//! `std::wrapping_add/sub/mul` are recognised by compiler-owned language-item
//! identity (never by name), lower to modular arithmetic and never trap; ordinary
//! operators stay checked (their trap behavior lands in Stage 2C).
#[path = "support/stdlib.rs"]
mod support;

use std::fs;
use std::process::Command;
use support::{render, ProviderModes};

fn write_fixture(modes: &ProviderModes, name: &str, source: &str) -> std::path::PathBuf {
    let dir = modes.source.join("e1_wrapping");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.ln"));
    fs::write(&path, source).unwrap();
    path
}

fn run_in(modes: &ProviderModes, root: &std::path::Path, path: &std::path::Path, tag: &str) -> i32 {
    let check = modes.check(root, path);
    assert!(check.status.success(), "check failed for {tag}: {}", render(&check));
    let (exe, build) = modes.build(root, path, tag);
    assert!(build.status.success(), "build failed for {tag}: {}", render(&build));
    let run = Command::new(exe).output().unwrap();
    run.status.code().unwrap_or(-1)
}

#[test]
fn wrapping_apis_produce_modular_results_in_both_modes() {
    let modes = ProviderModes::fresh();
    let source = r#"
import <num>;

fn main() -> i32 {
    dec u8a: u8 = 255;
    dec u8b: u8 = 1;
    if std::wrapping_add(u8a, u8b) != 0 { return 1; }
    dec u8c: u8 = 0;
    if std::wrapping_sub(u8c, u8b) != 255 { return 2; }
    dec u8d: u8 = 200;
    dec u8e: u8 = 2;
    if std::wrapping_mul(u8d, u8e) != 144 { return 3; }

    dec i8a: i8 = 127;
    dec i8b: i8 = 1;
    if std::wrapping_add(i8a, i8b) != (0 - 127) - 1 { return 4; }

    dec u32a: u32 = 4294967295;
    dec u32b: u32 = 1;
    if std::wrapping_add(u32a, u32b) != 0 { return 5; }

    dec i32a: i32 = 65536;
    if std::wrapping_mul(i32a, i32a) != 0 { return 6; }

    dec u64a: u64 = 18446744073709551615;
    dec u64b: u64 = 1;
    if std::wrapping_add(u64a, u64b) != 0 { return 7; }
    return 0;
}
"#;
    let path = write_fixture(&modes, "modular", source);
    assert_eq!(run_in(&modes, &modes.source, &path, "wrap_mod_src"), 0, "source mode");
    assert_eq!(run_in(&modes, &modes.artifact, &path, "wrap_mod_art"), 0, "artifact mode");

    // Relocation: the intrinsic is compiled in, so a relocated fresh-artifact
    // executable must behave identically.
    let (exe, build) = modes.build(&modes.artifact, &path, "wrap_mod_reloc");
    assert!(build.status.success(), "relocation build failed: {}", render(&build));
    let relocated_dir = modes.artifact.join("relocated");
    fs::create_dir_all(&relocated_dir).unwrap();
    let relocated = relocated_dir.join(exe.file_name().unwrap());
    fs::copy(&exe, &relocated).unwrap();
    let run = Command::new(&relocated).current_dir(&relocated_dir).output().unwrap();
    assert_eq!(run.status.code(), Some(0), "relocated artifact run failed: {}", render(&run));
}

#[test]
fn user_defined_same_name_is_not_reinterpreted_as_the_intrinsic() {
    // Identity is the resolved language item, not the callee name: a user
    // function called `wrapping_add` must keep ordinary semantics.
    let modes = ProviderModes::fresh();
    let source = r#"
fn wrapping_add(a: u32, b: u32) -> u32 {
    return 999;
}

fn main() -> i32 {
    dec a: u32 = 255;
    dec b: u32 = 1;
    if wrapping_add(a, b) == 999 { return 0; }
    return 7;
}
"#;
    let path = write_fixture(&modes, "user_named", source);
    assert_eq!(run_in(&modes, &modes.source, &path, "wrap_user_src"), 0, "user function must win");
}

#[test]
fn reserved_intrinsic_namespace_declaration_is_rejected() {
    // The compiler-owned `__luna_intrinsic_*` namespace may not be claimed by a
    // user declaration, so a forged wire name cannot come from ordinary source.
    let modes = ProviderModes::fresh();
    let source = r#"
fn __luna_intrinsic_WrappingMul(a: u32, b: u32) -> u32 {
    return 1;
}

fn main() -> i32 {
    return 0;
}
"#;
    let path = write_fixture(&modes, "forged_name", source);
    let check = modes.check(&modes.source, &path);
    let rendered = render(&check);
    assert!(!check.status.success(), "reserved namespace must be rejected: {rendered}");
    assert!(
        rendered.contains("error[E1009]") && rendered.contains("compiler-owned intrinsic namespace"),
        "expected a typed rejection for the reserved namespace: {rendered}"
    );
}

#[test]
fn link_attribute_cannot_claim_reserved_intrinsic_namespace() {
    let modes = ProviderModes::fresh();
    let source = r#"
#[link("__luna_intrinsic_WrappingAdd")]
extern fn spoofed(a: u32, b: u32) -> u32;

fn main() -> i32 {
    return 0;
}
"#;
    let path = write_fixture(&modes, "forged_link", source);
    let check = modes.check(&modes.source, &path);
    let rendered = render(&check);
    assert!(!check.status.success(), "#[link] spoof must be rejected: {rendered}");
    assert!(
        rendered.contains("error[E1009]") && rendered.contains("compiler-owned intrinsic namespace"),
        "expected the reserved-namespace rejection message: {rendered}"
    );
}

#[test]
fn user_defined_wrapping_mul_keeps_ordinary_semantics() {
    let modes = ProviderModes::fresh();
    let source = r#"
fn wrapping_mul(a: u32, b: u32) -> u32 {
    return 4242;
}

fn main() -> i32 {
    dec a: u32 = 65536;
    dec b: u32 = 65536;
    if wrapping_mul(a, b) == 4242 { return 0; }
    return 7;
}
"#;
    let path = write_fixture(&modes, "user_named_mul", source);
    assert_eq!(run_in(&modes, &modes.source, &path, "wrap_user_mul_src"), 0);
}

#[test]
fn comptime_wrapping_is_modular_without_overflow_diagnostic() {
    // A legal modular wrap at comptime must not be reported as E4005.
    let modes = ProviderModes::fresh();
    let source = r#"
import <num>;

const WRAPPED: u8 = comptime { std::wrapping_add(255u8, 1u8) };

fn main() -> i32 {
    if WRAPPED == 0 { return 0; }
    return 7;
}
"#;
    let path = write_fixture(&modes, "comptime", source);
    let check = modes.check(&modes.source, &path);
    let rendered = render(&check);
    assert!(
        check.status.success(),
        "comptime wrapping must not diagnose overflow: {rendered}"
    );
    assert!(
        !rendered.contains("E4005"),
        "comptime wrapping must not emit E4005: {rendered}"
    );
    assert_eq!(run_in(&modes, &modes.source, &path, "wrap_comptime_src"), 0, "source mode");
    assert_eq!(run_in(&modes, &modes.artifact, &path, "wrap_comptime_art"), 0, "artifact mode");
}
