//! Give only Luna's child process the installed compiler and toolchain paths.
use std::{env, io, path::Path, process::Command};

fn launch() -> io::Result<i32> {
    let executable = env::current_exe()?;
    let root = executable.parent().and_then(Path::parent)
        .ok_or_else(|| io::Error::other("Cannot locate the Luna installation"))?;
    let package = root.join("packages").join(env!("LUNA_PACKAGE_DIR"));
    let llvm = root.join("toolchains/llvm");
    let gnu = root.join("toolchains/gnu/mingw64/bin");
    let compiler = package.join("bin/luna.exe");
    for required in [&compiler, &llvm.join("bin/LLVM-C.dll"), &llvm.join("bin/llc.exe"),
                     &llvm.join("bin/clang.exe"), &llvm.join("bin/ld.lld.exe"), &root.join("bin/luna-cc.exe"), &gnu.join("gcc.exe")] {
        if !required.is_file() {
            return Err(io::Error::new(io::ErrorKind::NotFound,
                format!("Missing installed component: {}. Run Luna Setup to repair this installation.", required.display())));
        }
    }
    let mut paths = vec![llvm.join("bin"), gnu.clone()];
    paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
    let path = env::join_paths(paths).map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let mut child = Command::new(compiler);
    child.args(env::args_os().skip(1));
    // An old development sysroot/CC must not override the installed bundles.
    for (key, _) in env::vars_os() {
        if key.to_string_lossy().to_ascii_uppercase().starts_with("LUNA_") {
            child.env_remove(key);
        }
    }
    child.env("PATH", path)
        .env("LLVM_HOME", llvm)
        .env("CC", root.join("bin/luna-cc.exe"))
        .env("LUNA_HOME", &package)
        .env("LUNA_SYSROOT", &package)
        .env("LUNA_RUNTIME_LIB", package.join("runtime/luna-runtime.lib"));
    let status = child.status()?;
    Ok(status.code().unwrap_or(1))
}

fn main() {
    let code = match launch() {
        Ok(code) => code,
        Err(error) => { eprintln!("Luna launcher: {error}"); 1 }
    };
    std::process::exit(code);
}
