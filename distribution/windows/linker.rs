//! Use the private LLVM linker driver with the pinned MinGW GNU sysroot.
use std::{env, ffi::OsString, io, path::Path, process::Command};

fn launch() -> io::Result<i32> {
    let executable = env::current_exe()?;
    let root = executable.parent().and_then(Path::parent)
        .ok_or_else(|| io::Error::other("Cannot locate the Luna installation"))?;
    let llvm = root.join("toolchains/llvm/bin");
    let gnu = root.join("toolchains/gnu/mingw64");
    let mut sysroot = OsString::from("--sysroot=");
    sysroot.push(gnu);
    let status = Command::new(llvm.join("clang.exe"))
        .arg("--no-default-config")
        .arg("--target=x86_64-w64-windows-gnu")
        .arg(sysroot)
        .arg("-fuse-ld=lld")
        .args(env::args_os().skip(1))
        .status()?;
    Ok(status.code().unwrap_or(1))
}

fn main() {
    let code = match launch() {
        Ok(code) => code,
        Err(error) => { eprintln!("Luna linker: {error}"); 1 }
    };
    std::process::exit(code);
}
