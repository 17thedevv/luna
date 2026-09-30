use std::{env, ffi::OsString, fs, path::PathBuf, process::Command};

fn native_library_name() -> &'static str {
    if cfg!(windows) {
        "luna-runtime.lib"
    } else {
        "libluna-runtime.a"
    }
}

fn native_platform_name() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn native_triple() -> String {
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };
    if cfg!(windows) {
        format!("{arch}-pc-windows-gnu")
    } else if cfg!(target_os = "macos") {
        format!("{arch}-apple-darwin")
    } else {
        format!("{arch}-unknown-linux-gnu")
    }
}

fn native_compiler() -> OsString {
    env::var_os("CC").unwrap_or_else(|| {
        if cfg!(windows) {
            OsString::from("gcc.exe")
        } else if cfg!(target_os = "macos") {
            OsString::from("clang")
        } else {
            OsString::from("cc")
        }
    })
}

fn tool(name: &str) -> PathBuf {
    env::var_os("LLVM_HOME")
        .map(PathBuf::from)
        .map(|root| root.join("bin").join(name))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}

pub fn runtime_library() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    let library_name = native_library_name();
    let platform_name = native_platform_name();
    let triple = native_triple();
    let build_configs = ["", "Debug", "Release"];

    // 1. Explicit override via LUNA_RUNTIME_LIB
    if let Some(path) = env::var_os("LUNA_RUNTIME_LIB") {
        candidates.push(PathBuf::from(path));
    }

    // 2. Explicit home directory via LUNA_HOME
    if let Some(home) = env::var_os("LUNA_HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join("runtime").join(library_name));
        candidates.push(
            home.join("runtime")
                .join("hosted")
                .join(platform_name)
                .join(library_name),
        );
        candidates.push(
            home.join("build")
                .join("runtime")
                .join("Release")
                .join(library_name),
        );
        candidates.push(home.join("build").join("runtime").join(library_name));
        for config in build_configs {
            candidates.push(
                home.join("build")
                    .join("host")
                    .join(&triple)
                    .join(config)
                    .join(library_name),
            );
        }
    }

    // 3. Relative to current compiler executable and its ancestors
    if let Ok(exe_path) = env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            candidates.push(dir.join(library_name));
            for ancestor in dir.ancestors() {
                candidates.push(ancestor.join("runtime").join(library_name));
                candidates.push(
                    ancestor
                        .join("runtime")
                        .join("hosted")
                        .join(platform_name)
                        .join(library_name),
                );
                candidates.push(
                    ancestor
                        .join("build")
                        .join("runtime")
                        .join("Release")
                        .join(library_name),
                );
                candidates.push(ancestor.join("build").join("runtime").join(library_name));
                for config in build_configs {
                    candidates.push(
                        ancestor
                            .join("build")
                            .join("host")
                            .join(&triple)
                            .join(config)
                            .join(library_name),
                    );
                }
            }
        }
    }

    // 4. Relative to current working directory
    candidates.push(PathBuf::from("runtime").join(library_name));
    candidates.push(
        PathBuf::from("runtime")
            .join("hosted")
            .join(platform_name)
            .join(library_name),
    );
    candidates.push(
        PathBuf::from("build")
            .join("runtime")
            .join("Release")
            .join(library_name),
    );
    candidates.push(PathBuf::from("build").join("runtime").join(library_name));
    for config in build_configs {
        candidates.push(
            PathBuf::from("build")
                .join("host")
                .join(&triple)
                .join(config)
                .join(library_name),
        );
    }

    candidates.into_iter().find(|path| path.is_file()).ok_or_else(||
        "luna runtime library not found; set LUNA_HOME or LUNA_RUNTIME_LIB or place it alongside the compiler".into())
}

pub fn compile_ll_to_exe(ll_file: &str, obj_file: &str, exe_file: &str) -> Result<(), String> {
    // Emit a native object using LLVM's llc.
    let llc_name = if cfg!(windows) { "llc.exe" } else { "llc" };
    let llc_status = Command::new(tool(llc_name))
        .arg("-filetype=obj")
        .arg("-o")
        .arg(obj_file)
        .arg(ll_file)
        .status()
        .map_err(|e| format!("Failed to invoke llc: {}", e))?;

    if !llc_status.success() {
        return Err(format!("llc exited with status {}", llc_status));
    }

    let runtime = runtime_library()?;
    let link_status = Command::new(native_compiler())
        .arg(obj_file)
        .arg(runtime)
        .arg("-o")
        .arg(exe_file)
        // Optionally try ASan:
        // .arg("-fsanitize=address")
        .status()
        .map_err(|e| format!("Failed to invoke native linker: {}", e))?;

    if !link_status.success() {
        return Err(format!("native linker exited with status {}", link_status));
    }

    // Cleanup temporary files
    let _ = fs::remove_file(ll_file);
    let _ = fs::remove_file(obj_file);

    Ok(())
}

pub fn link_objs_to_exe<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    obj_files: &[P],
    exe_file: Q,
) -> Result<(), String> {
    let runtime = runtime_library()?;
    let mut cmd = Command::new(native_compiler());
    for obj in obj_files {
        cmd.arg(obj.as_ref());
    }
    cmd.arg(runtime).arg("-o").arg(exe_file.as_ref());

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to invoke native linker: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "native linker exited with status {}: {}",
            output.status, stderr
        ));
    }

    Ok(())
}

pub fn link_obj_to_exe(obj_file: &str, exe_file: &str) -> Result<(), String> {
    link_objs_to_exe(
        &[std::path::Path::new(obj_file)],
        std::path::Path::new(exe_file),
    )
}
