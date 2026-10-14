mod provider_config;
use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf, process};

#[derive(Parser, Debug)]
#[command(name = "luna", about = "Official Luna Compiler CLI", version)]
struct Cli {
    /// Maximum comptime evaluation steps
    #[arg(long = "comptime-steps", global = true)]
    comptime_steps: Option<usize>,

    /// Maximum comptime call recursion depth
    #[arg(long = "comptime-depth", global = true)]
    comptime_depth: Option<usize>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Args, Debug)]
struct ProviderConfigArgs {
    /// Use an explicit optional provider configuration
    #[arg(long, value_name = "FILE", conflicts_with = "no_config")]
    config: Option<PathBuf>,
    /// Disable nearest luna.toml discovery
    #[arg(long, conflicts_with = "config")]
    no_config: bool,
}

/// Every user-visible CLI failure carries a typed diagnostic code (DIAG-1).
/// Infrastructure failures are not source diagnostics, so they carry the code
/// without a fabricated span.
fn fail(code: luna_common::DiagnosticCode, message: String) -> ! {
    let diagnostic = luna_common::Diagnostic::error(message).with_code(code);
    eprintln!("{}", diagnostic.render(&luna_common::source::SourceManager::new()));
    process::exit(1);
}

fn configured_bindings(file: &std::path::Path, args: &ProviderConfigArgs) -> luna_driver::provider_binding::ProviderBindings {
    provider_config::load(file, args.config.as_deref(), args.no_config).unwrap_or_else(|error| {
        fail(luna_common::DiagnosticCode::ProviderConfigurationError, error.to_string());
    })
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Build a target (.exe or .llib)
    Build {
        /// The input source file
        file: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Emit intermediate representations: mvir, llvm, llib
        #[arg(long, value_name = "TYPE")]
        emit: Option<String>,

        /// Suppress console output
        #[arg(short, long)]
        quiet: bool,

        /// Build as a library (do not link into an executable)
        #[arg(long)]
        lib: bool,

        /// Add a directory to the module search path
        #[arg(short = 'I', long = "search-path", value_name = "DIR")]
        search_paths: Vec<PathBuf>,
        #[command(flatten)]
        provider_config: ProviderConfigArgs,

    },
    /// Check a source file for errors without emitting output
    Check {
        /// The input source file
        file: PathBuf,

        /// Suppress console output
        #[arg(short, long)]
        quiet: bool,

        /// Add a directory to the module search path
        #[arg(short = 'I', long = "search-path", value_name = "DIR")]
        search_paths: Vec<PathBuf>,
        #[command(flatten)]
        provider_config: ProviderConfigArgs,

    },
    /// Compile and run an application
    Run {
        /// The input source file
        file: PathBuf,

        /// Suppress console output
        #[arg(short, long)]
        quiet: bool,

        /// Add a directory to the module search path
        #[arg(short = 'I', long = "search-path", value_name = "DIR")]
        search_paths: Vec<PathBuf>,
        #[command(flatten)]
        provider_config: ProviderConfigArgs,

    },
    /// Inspect the manifest of a given .llib file
    Manifest {
        /// The .llib file to inspect
        file: PathBuf,
    },
    /// Build canonical .llib artifacts for every provider in the sysroot
    BuildSysroot {
        /// Suppress per-provider build output
        #[arg(short, long)]
        quiet: bool,
    },
}

fn parse_emit(emit_opt: &Option<String>) -> (bool, bool, bool) {
    let mut emit_mvir = false;
    let mut emit_llvm = false;
    let mut emit_llib = false;

    if let Some(emit) = emit_opt {
        for e in emit.split(',') {
            match e.trim() {
                "mvir" => emit_mvir = true,
                "llvm" => emit_llvm = true,
                "llib" => emit_llib = true,
                other => {
                    fail(luna_common::DiagnosticCode::InvalidArtifactOutput, format!("Unknown emit type: {other}"));
                }
            }
        }
    }
    (emit_mvir, emit_llvm, emit_llib)
}

fn read_source(file: &PathBuf) -> String {
    fs::read_to_string(file).unwrap_or_else(|err| {
        fail(luna_common::DiagnosticCode::ProviderReadFailure, format!("Error reading file {}: {}", file.display(), err));
    })
}

fn main() {
    // Compiler recursion is bounded by admission limits. The Windows process
    // main stack is too small to reach the macro limit in debug builds; use
    // an explicit compiler stack so the limit produces a diagnostic.
    let worker = std::thread::Builder::new().name("luna-compiler".into())
        .stack_size(16 * 1024 * 1024).spawn(run_cli).unwrap_or_else(|error| {
            let diagnostic = luna_common::Diagnostic::error(format!("Cannot start compiler worker: {error}"))
                .with_code(luna_common::DiagnosticCode::ProcessExecutionFailure);
            eprintln!("{}", diagnostic.render(&luna_common::source::SourceManager::new()));
            process::exit(1);
        });
    if let Err(panic) = worker.join() { std::panic::resume_unwind(panic); }
}

fn run_cli() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Manifest { file } => {
            let mut f = std::fs::File::open(file).unwrap_or_else(|err| {
                fail(luna_common::DiagnosticCode::ProviderReadFailure, format!("Error opening {}: {}", file.display(), err));
            });
            match luna_llib::reader::LlibReader::read_manifest(&mut f) {
                Ok(manifest) => {
                    let json = serde_json::to_string_pretty(&manifest).expect("Failed to serialize manifest to JSON");
                    println!("{}", json);
                    process::exit(0);
                }
                Err(e) => {
                    fail(luna_common::DiagnosticCode::ProviderReadFailure, format!("Failed to read manifest: {e:?}"));
                }
            }
        }
        Commands::BuildSysroot { quiet } => {
            let sysroot = luna_driver::sysroot::Sysroot::discover(None).unwrap_or_else(|err| {
                fail(luna_common::DiagnosticCode::SysrootConfigurationFailure, format!("Failed to locate Luna sysroot: {err:?}"));
            });
            let builder = luna_driver::sysroot_builder::SysrootBuilder::new(sysroot);
            if let Err(err) = builder.build_all(*quiet) {
                fail(luna_common::DiagnosticCode::SysrootConfigurationFailure, format!("Failed to build Luna sysroot: {err}"));
            }
        }
        Commands::Build { file, output, emit, quiet, lib, search_paths, provider_config } => {
            let source = read_source(file);
            let (emit_mvir, emit_llvm, emit_llib) = parse_emit(emit);
            
            let options = luna_driver::CompilerOptions {
                output_path: output.as_ref().map(|p| p.to_string_lossy().to_string()),
                emit_llvm,
                emit_mvir,
                emit_llib,
                emit_mlib: emit_llib,
                quiet: *quiet,
                search_paths: search_paths.iter().map(|p| p.to_string_lossy().to_string()).collect(),
                no_link: *lib,
                comptime_steps: cli.comptime_steps,
                comptime_depth: cli.comptime_depth,
                is_sysroot_build: false,
                provider_bindings: configured_bindings(file, provider_config),
            };

            if let Err(rendered_diagnostics) = luna_driver::compile_and_render(file.to_string_lossy().as_ref(), source.clone(), &options) {
                eprintln!("{}", rendered_diagnostics);
                process::exit(1);
            }
            if !*quiet {
                println!("Build successfully.");
            }
        }
        Commands::Check { file, quiet, search_paths, provider_config } => {
            let source = read_source(file);
            
            let options = luna_driver::CompilerOptions {
                output_path: None,
                emit_llvm: false,
                emit_mvir: false,
                emit_llib: false,
                emit_mlib: false,
                quiet: *quiet,
                search_paths: search_paths.iter().map(|p| p.to_string_lossy().to_string()).collect(),
                no_link: false,
                comptime_steps: cli.comptime_steps,
                comptime_depth: cli.comptime_depth,
                is_sysroot_build: false,
                provider_bindings: configured_bindings(file, provider_config),
            };

            if let Err(rendered_diagnostics) = luna_driver::check_and_render(file.to_string_lossy().as_ref(), source.clone(), &options) {
                eprintln!("{}", rendered_diagnostics);
                process::exit(1);
            }
            if !*quiet {
                println!("Check successfully (no errors found).");
            }
        }
        Commands::Run { file, quiet, search_paths, provider_config } => {
            let source = read_source(file);
            let bindings = configured_bindings(file, provider_config);
            match run_application(file, source, *quiet, search_paths, bindings, cli.comptime_steps, cli.comptime_depth) {
                Ok(code) => process::exit(code),
                Err(error) => fail(luna_common::DiagnosticCode::ProcessExecutionFailure, error),
            }
        }
    }
}

struct RunOutput(std::path::PathBuf);
impl Drop for RunOutput {
    fn drop(&mut self) {
        if self.0.parent() == Some(std::env::temp_dir().as_path())
            && self.0.file_name().is_some_and(|name| name.to_string_lossy().starts_with("luna_run_")) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

fn run_application(file: &std::path::Path, source: String, quiet: bool, search_paths: &[PathBuf],
    bindings: luna_driver::provider_binding::ProviderBindings, steps: Option<usize>, depth: Option<usize>) -> Result<i32, String> {
    let directory = std::env::temp_dir().join(format!("luna_run_{}_{}", process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir(&directory).map_err(|error| luna_common::Diagnostic::error(format!("Cannot create run output: {}", error)).with_code(luna_common::DiagnosticCode::OutputWriteFailure).render(&luna_common::source::SourceManager::new()))?;
    let owned = RunOutput(directory);
    let executable = owned.0.join("application").with_extension(std::env::consts::EXE_EXTENSION);
    let options = luna_driver::CompilerOptions {
        output_path: Some(executable.to_string_lossy().into_owned()), quiet,
        search_paths: search_paths.iter().map(|path| path.to_string_lossy().into_owned()).collect(),
        provider_bindings: bindings, comptime_steps: steps, comptime_depth: depth, ..Default::default()
    };
    luna_driver::compile_and_render(&file.to_string_lossy(), source, &options)?;
    let status = std::process::Command::new(executable).status()
        .map_err(|error| luna_common::Diagnostic::error(format!("Cannot run application: {}", error)).with_code(luna_common::DiagnosticCode::ProcessExecutionFailure).render(&luna_common::source::SourceManager::new()))?;
    Ok(status.code().unwrap_or(1))
}
