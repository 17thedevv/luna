mod build;
mod manifest;

use clap::{Parser, Subcommand};
use manifest::{Graph, Result};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{self, Command},
};

#[derive(Parser)]
#[command(
    name = "lunar",
    version,
    about = "Offline project and build tool for Luna"
)]
struct Cli {
    /// Project luna.toml (otherwise search the current directory and ancestors)
    #[arg(long, global = true, value_name = "FILE")]
    manifest_path: Option<PathBuf>,
    /// Luna compiler executable (default: LUNA, sibling luna, or luna on PATH)
    #[arg(long, global = true, value_name = "EXE")]
    compiler: Option<OsString>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a project in a new directory
    New {
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        lib: bool,
    },
    /// Create a project in an existing directory without overwriting files
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        lib: bool,
    },
    /// Build libraries and binary targets with local dependencies
    Build {
        #[arg(long)]
        bin: Option<String>,
    },
    /// Check library, binary and test targets, using source dependencies
    Check,
    /// Build and execute one binary; arguments follow --
    Run {
        #[arg(long)]
        bin: Option<String>,
        #[arg(last = true)]
        args: Vec<OsString>,
    },
    /// Build and run declared [[test]] programs (exit 0 means pass)
    Test,
    /// Remove only this package's Lunar-owned target/lunar directory
    Clean,
}

fn create(path: &Path, name: Option<String>, library: bool, new: bool) -> Result<()> {
    if new && path.exists() {
        return Err(format!("directory {} already exists", path.display()));
    }
    let absolute = std::path::absolute(path).map_err(|e| e.to_string())?;
    let name = name.unwrap_or_else(|| {
        absolute
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .replace('-', "_")
    });
    manifest::name(&name, library)?;
    let source = if library {
        format!("src/{name}.ln")
    } else {
        "src/main.ln".into()
    };
    for file in ["luna.toml", &source] {
        if path.join(file).exists() {
            return Err(format!(
                "refusing to overwrite {}",
                path.join(file).display()
            ));
        }
    }
    let mut project = format!("schema = 1\n\n[package]\nname = {name:?}\nversion = \"0.1.0\"\n");
    if library {
        project.push_str(&format!("\n[lib]\npath = {source:?}\n"));
    } else {
        project.push_str(&format!("\n[[bin]]\nname = {name:?}\npath = {source:?}\n"));
    }
    let code = if library {
        format!("export module {name} {{\n    export fn answer() -> i32 {{\n        return 42;\n    }}\n}}\n")
    } else {
        "fn main() -> i32 {\n    return 0;\n}\n".into()
    };
    fs::create_dir_all(path.join("src")).map_err(|e| e.to_string())?;
    fs::write(path.join(&source), code).map_err(|e| e.to_string())?;
    fs::write(path.join("luna.toml"), project).map_err(|e| e.to_string())?;
    if !path.join(".gitignore").exists() {
        fs::write(path.join(".gitignore"), "/target/\n").map_err(|e| e.to_string())?;
    }
    println!(
        "Created {} project '{name}' in {}",
        if library { "library" } else { "binary" },
        path.display()
    );
    Ok(())
}

fn compiler(explicit: Option<OsString>) -> Result<OsString> {
    let selected = explicit
        .or_else(|| std::env::var_os("LUNA"))
        .unwrap_or_else(|| {
            let sibling = std::env::current_exe().ok().and_then(|p| {
                p.parent()
                    .map(|p| p.join(if cfg!(windows) { "luna.exe" } else { "luna" }))
            });
            sibling
                .filter(|p| p.is_file())
                .map(|p| p.into_os_string())
                .unwrap_or_else(|| "luna".into())
        });
    let path = Path::new(&selected);
    if path.is_relative() && path.components().count() > 1 {
        Ok(fs::canonicalize(path)
            .map_err(|e| format!("compiler {}: {e}", path.display()))?
            .into_os_string())
    } else {
        Ok(selected)
    }
}

fn execute() -> Result<i32> {
    let cli = Cli::parse();
    match cli.command {
        Commands::New { path, name, lib } => {
            create(&path, name, lib, true)?;
            return Ok(0);
        }
        Commands::Init { path, name, lib } => {
            create(&path, name, lib, false)?;
            return Ok(0);
        }
        _ => {}
    }
    let path = manifest::discover(cli.manifest_path.as_deref())?;
    let path = fs::canonicalize(path).map_err(|e| e.to_string())?;
    if matches!(cli.command, Commands::Clean) {
        build::BuildRoot::acquire(path.parent().ok_or("manifest has no parent")?)?.clean()?;
        println!("Removed target/lunar");
        return Ok(0);
    }
    let graph = Graph::load(&path)?;
    let compiler = compiler(cli.compiler)?;
    let root = &graph.nodes[graph.root];
    match cli.command {
        Commands::Build { bin } => {
            let output = build::build(&graph, &compiler, false, false, bin.as_deref())?;
            println!("Built {}", output.path.display());
        }
        Commands::Check => {
            build::build(&graph, &compiler, true, false, None)?;
            println!("Check passed");
        }
        Commands::Run { bin, args } => {
            let targets = build::select(&root.manifest.bin, bin.as_deref())?;
            if targets.len() != 1 {
                return Err("run requires exactly one binary; select one with --bin NAME".into());
            }
            let output = build::build(&graph, &compiler, false, false, Some(&targets[0].name))?;
            let status = Command::new(build::executable(&output.path, "bin", &targets[0].name))
                .args(args)
                .current_dir(&root.root)
                .status()
                .map_err(|e| format!("execute binary: {e}"))?;
            return Ok(status.code().unwrap_or(1));
        }
        Commands::Test => {
            if root.manifest.test.is_empty() {
                return Err("no [[test]] programs declared in luna.toml".into());
            }
            let output = build::build(&graph, &compiler, false, true, None)?;
            let mut failed = 0;
            for test in &root.manifest.test {
                let status = Command::new(build::executable(&output.path, "test", &test.name))
                    .current_dir(&root.root)
                    .status()
                    .map_err(|e| format!("execute test '{}': {e}", test.name))?;
                println!(
                    "test {} ... {}",
                    test.name,
                    if status.success() { "PASS" } else { "FAIL" }
                );
                if !status.success() {
                    failed += 1;
                }
            }
            println!(
                "{} passed; {failed} failed",
                root.manifest.test.len() - failed
            );
            return Ok(i32::from(failed != 0));
        }
        _ => unreachable!(),
    }
    Ok(0)
}

fn main() {
    match execute() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("lunar: {error}");
            process::exit(1);
        }
    }
}
