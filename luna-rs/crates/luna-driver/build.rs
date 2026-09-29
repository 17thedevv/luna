use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=LUNA_LLVM_LIB_DIR");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_180_PREFIX");
    println!("cargo:rerun-if-env-changed=LLVM_CONFIG_PATH");

    let library_dir = find_llvm_library_dir();
    println!("cargo:rustc-link-search=native={}", library_dir.display());
    println!("cargo:rustc-link-lib=LLVM-C");
}

fn find_llvm_library_dir() -> PathBuf {
    if let Some(path) = env::var_os("LUNA_LLVM_LIB_DIR") {
        return PathBuf::from(path);
    }

    if let Some(prefix) = env::var_os("LLVM_SYS_180_PREFIX") {
        return PathBuf::from(prefix).join("lib");
    }

    let llvm_config = env::var_os("LLVM_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("llvm-config"));
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
