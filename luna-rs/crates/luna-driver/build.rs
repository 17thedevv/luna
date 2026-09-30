use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=LUNA_LLVM_LIB_DIR");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_180_PREFIX");
    println!("cargo:rerun-if-env-changed=LLVM_CONFIG_PATH");

    let library_dir = find_llvm_library_dir();
    println!("cargo:rustc-link-search=native={}", library_dir.display());
    if cfg!(windows) {
        println!("cargo:rustc-link-lib=LLVM-C");
    } else {
        emit_llvm_link_flags();
    }
}

fn llvm_config_path() -> PathBuf {
    if let Some(path) = env::var_os("LLVM_CONFIG_PATH") {
        return PathBuf::from(path);
    }

    if let Some(prefix) = env::var_os("LLVM_SYS_180_PREFIX") {
        let executable = if cfg!(windows) {
            "llvm-config.exe"
        } else {
            "llvm-config"
        };
        let path = PathBuf::from(prefix).join("bin").join(executable);
        if path.is_file() {
            return path;
        }
    }

    PathBuf::from(if cfg!(windows) {
        "llvm-config.exe"
    } else {
        "llvm-config"
    })
}

fn emit_llvm_link_flags() {
    let llvm_config = llvm_config_path();
    let output = Command::new(&llvm_config)
        .args(["--link-shared", "--libs", "--system-libs"])
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "Could not run {} to determine LLVM link libraries ({error}). Set LLVM_CONFIG_PATH or LLVM_SYS_180_PREFIX.",
                llvm_config.display()
            )
        });

    if !output.status.success() {
        panic!(
            "{} --link-shared --libs --system-libs failed: {}",
            llvm_config.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let flags = String::from_utf8(output.stdout).expect("llvm-config link flags must be UTF-8");
    let mut flags = flags.split_whitespace();
    while let Some(flag) = flags.next() {
        if let Some(path) = flag.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={path}");
        } else if let Some(library) = flag.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={library}");
        } else if let Some(path) = flag.strip_prefix("-F") {
            println!("cargo:rustc-link-search=framework={path}");
        } else if flag == "-framework" {
            let framework = flags.next().unwrap_or_else(|| {
                panic!(
                    "{} emitted -framework without a name",
                    llvm_config.display()
                )
            });
            println!("cargo:rustc-link-lib=framework={framework}");
        } else if flag == "-pthread" {
            println!("cargo:rustc-link-arg=-pthread");
        } else if flag.starts_with("-Wl,") {
            println!("cargo:rustc-link-arg={flag}");
        } else {
            panic!(
                "Unsupported LLVM linker flag from {}: {flag}",
                llvm_config.display()
            );
        }
    }
}

fn find_llvm_library_dir() -> PathBuf {
    if let Some(path) = env::var_os("LUNA_LLVM_LIB_DIR") {
        return PathBuf::from(path);
    }

    if let Some(prefix) = env::var_os("LLVM_SYS_180_PREFIX") {
        return PathBuf::from(prefix).join("lib");
    }

    let llvm_config = llvm_config_path();
    let output = Command::new(&llvm_config)
        .arg("--libdir")
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "Could not run {} to locate LLVM libraries ({error}). Set LUNA_LLVM_LIB_DIR, \
                 LLVM_SYS_180_PREFIX, or LLVM_CONFIG_PATH.",
                llvm_config.display()
            )
        });

    if !output.status.success() {
        panic!(
            "{} --libdir failed: {}",
            llvm_config.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let path = String::from_utf8(output.stdout)
        .expect("llvm-config --libdir must return UTF-8")
        .trim()
        .to_owned();
    if path.is_empty() {
        panic!("{} --libdir returned an empty path", llvm_config.display());
    }
    PathBuf::from(path)
}
