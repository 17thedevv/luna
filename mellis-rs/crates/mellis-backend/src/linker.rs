use std::process::Command;
use std::fs;

pub fn compile_ll_to_exe(ll_file: &str, obj_file: &str, exe_file: &str) -> Result<(), String> {
    // Call llc.exe
    let llc_status = Command::new("D:/Programs/LLVM-DEV/bin/llc.exe")
        .arg("-filetype=obj")
        .arg("-o")
        .arg(obj_file)
        .arg(ll_file)
        .status()
        .map_err(|e| format!("Failed to invoke llc: {}", e))?;
        
    if !llc_status.success() {
        return Err(format!("llc exited with status {}", llc_status));
    }
    
    // Call clang.exe to link
    // Assuming MSVC environment or generic Clang
    let clang_status = Command::new("D:/Programs/LLVM-DEV/bin/clang.exe")
        .arg(obj_file)
        .arg("-o")
        .arg(exe_file)
        .status()
        .map_err(|e| format!("Failed to invoke clang: {}", e))?;
        
    if !clang_status.success() {
        return Err(format!("clang exited with status {}", clang_status));
    }
    
    // Cleanup temporary files
    let _ = fs::remove_file(ll_file);
    let _ = fs::remove_file(obj_file);
    
    Ok(())
}
