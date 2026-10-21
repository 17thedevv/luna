use crate::manifest::{self, Graph, Result, Target};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs::{self, File},
    path::{Component, Path, PathBuf},
    process::{Command, ExitStatus},
};

const MARKER: &str = "Lunar disposable build directory, schema 1\n";

fn no_link(path: &Path) -> Result<()> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        let linked = meta.file_type().is_symlink();
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            linked || meta.file_attributes() & 0x400 != 0
        };
        if linked {
            return Err(format!(
                "refusing linked/reparse build path {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn remove_tree(path: &Path) -> Result<()> {
    no_link(path)?;
    if !path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        no_link(&path)?;
        if path.is_dir() {
            inspect_tree(&path)?;
        }
    }
    fs::remove_dir_all(path).map_err(|e| format!("remove {}: {e}", path.display()))
}

fn inspect_tree(path: &Path) -> Result<()> {
    no_link(path)?;
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        no_link(&path)?;
        if path.is_dir() {
            inspect_tree(&path)?;
        }
    }
    Ok(())
}

pub struct BuildRoot {
    pub path: PathBuf,
    _lock: File,
}
impl BuildRoot {
    pub fn acquire(root: &Path) -> Result<Self> {
        let target = root.join("target");
        no_link(&target)?;
        fs::create_dir_all(&target).map_err(|e| e.to_string())?;
        let lock_path = target.join(".lunar.lock");
        no_link(&lock_path)?;
        let lock = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|e| e.to_string())?;
        lock.try_lock()
            .map_err(|e| format!("another lunar process owns this package build directory: {e}"))?;
        let path = target.join("lunar");
        no_link(&path)?;
        if path.exists() {
            if fs::read_to_string(path.join(".lunar-owned"))
                .ok()
                .as_deref()
                != Some(MARKER)
            {
                return Err(format!(
                    "refusing unowned build directory {}; move it aside first",
                    path.display()
                ));
            }
        } else {
            fs::create_dir(&path).map_err(|e| e.to_string())?;
            fs::write(path.join(".lunar-owned"), MARKER).map_err(|e| e.to_string())?;
        }
        Ok(Self { path, _lock: lock })
    }
    pub fn clean(&self) -> Result<()> {
        remove_tree(&self.path)
    }
    fn stage(&self) -> Result<PathBuf> {
        let stage = self.path.join("stage");
        remove_tree(&stage)?;
        fs::create_dir(&stage).map_err(|e| e.to_string())?;
        Ok(stage)
    }
    fn publish(&self, stage: &Path) -> Result<PathBuf> {
        let output = self.path.join("build");
        remove_tree(&output)?;
        fs::rename(stage, &output).map_err(|e| e.to_string())?;
        Ok(output)
    }
}

// Compiler config values remain relative, including providers outside a project.
fn relative_path(base: &Path, destination: &Path) -> Result<String> {
    let from: Vec<_> = base.components().collect();
    let to: Vec<_> = destination.components().collect();
    if from.first() != to.first() {
        return Err("provider and build directory must be on the same filesystem volume".into());
    }
    let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut path = PathBuf::new();
    for component in &from[shared..] {
        if matches!(component, Component::Normal(_)) {
            path.push("..");
        }
    }
    for component in &to[shared..] {
        path.push(component.as_os_str());
    }
    let value = path
        .to_str()
        .ok_or("provider paths must be UTF-8")?
        .replace('\\', "/");
    manifest::relative(&value)?;
    Ok(value)
}

fn library_output(graph: &Graph, index: usize, dir: &Path) -> Result<PathBuf> {
    let source = graph.library_source(index)?;
    Ok(dir
        .join("providers")
        .join(index.to_string())
        .join(source.file_name().unwrap())
        .with_extension("llib"))
}

fn config(graph: &Graph, dir: &Path, artifacts: bool) -> Result<PathBuf> {
    let mut bindings = BTreeMap::<String, PathBuf>::new();
    let mut insert = |alias: &str, stem: PathBuf| -> Result<()> {
        if let Some(old) = bindings.get(alias) {
            if old != &stem {
                return Err(format!("conflicting provider binding '{alias}': {} and {}; use unique aliases throughout the dependency graph", old.display(), stem.display()));
            }
        }
        bindings.insert(alias.to_string(), stem);
        Ok(())
    };
    let mut libraries = BTreeMap::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        if node.manifest.lib.is_some() {
            let source = graph.library_source(index)?;
            let stem = if artifacts {
                library_output(graph, index, dir)?.with_extension("")
            } else {
                source.with_extension("")
            };
            let name = source
                .file_stem()
                .unwrap()
                .to_str()
                .ok_or("non-UTF-8 provider name")?;
            insert(name, stem.clone())?;
            libraries.insert(index, stem);
        }
        for (alias, stem) in &node.manifest.providers {
            let stem = node.root.join(stem);
            // Canonicalize the existing parent, while unused missing stems remain legal.
            let parent = stem.parent().unwrap();
            let stem = if let Ok(parent) = fs::canonicalize(parent) {
                parent.join(stem.file_name().ok_or("missing provider filename")?)
            } else {
                stem
            };
            insert(alias, stem)?;
        }
    }
    for node in &graph.nodes {
        for (alias, dependency) in &node.deps {
            insert(alias, libraries[dependency].clone())?;
        }
    }
    let mut providers = toml::map::Map::new();
    for (name, stem) in bindings {
        providers.insert(name, toml::Value::String(relative_path(dir, &stem)?));
    }
    let mut config = toml::map::Map::new();
    config.insert("schema".into(), 1.into());
    config.insert("providers".into(), toml::Value::Table(providers));
    let path = dir.join("luna.toml");
    fs::write(&path, toml::to_string(&config).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(path)
}

fn status(status: ExitStatus, context: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{context} failed ({status})"))
    }
}

fn compile(
    compiler: &OsStr,
    cwd: &Path,
    command: &str,
    source: &Path,
    config: &Path,
    output: Option<&Path>,
    library: bool,
) -> Result<()> {
    let mut process = Command::new(compiler);
    process
        .current_dir(cwd)
        .arg(command)
        .arg(source)
        .arg("--config")
        .arg(config)
        .arg("--quiet");
    if let Some(output) = output {
        fs::create_dir_all(output.parent().unwrap()).map_err(|e| e.to_string())?;
        process.arg("-o").arg(output);
    }
    if library && command == "build" {
        process.args(["--lib", "--emit", "llib"]);
    }
    status(
        process.status().map_err(|e| {
            format!(
                "cannot execute compiler '{}': {e}; set --compiler or LUNA",
                compiler.to_string_lossy()
            )
        })?,
        "Luna compilation",
    )?;
    if let Some(output) = output {
        if !output.is_file() {
            return Err(format!(
                "compiler reported success without producing {}",
                output.display()
            ));
        }
    }
    Ok(())
}

pub fn executable(dir: &Path, kind: &str, name: &str) -> PathBuf {
    let path = dir.join(kind).join(name);
    if cfg!(windows) {
        path.with_extension("exe")
    } else {
        path
    }
}

pub fn select<'a>(targets: &'a [Target], selected: Option<&str>) -> Result<Vec<&'a Target>> {
    match selected {
        Some(name) => targets
            .iter()
            .find(|t| t.name == name)
            .map(|t| vec![t])
            .ok_or_else(|| format!("no binary target '{name}'")),
        None => Ok(targets.iter().collect()),
    }
}

pub struct BuildOutput {
    pub path: PathBuf,
    // Keep the output stable while a built binary or test suite is running.
    _owner: BuildRoot,
}

pub fn build(
    graph: &Graph,
    compiler: &OsStr,
    check: bool,
    tests: bool,
    selected: Option<&str>,
) -> Result<BuildOutput> {
    let root = &graph.nodes[graph.root];
    let binaries = select(&root.manifest.bin, selected)?;
    let owner = BuildRoot::acquire(&root.root)?;
    let stage = owner.stage()?;
    let config = config(graph, &stage, !check)?;
    for (index, node) in graph.nodes.iter().enumerate() {
        if node.manifest.lib.is_some() {
            eprintln!(
                "{} library {}",
                if check { "Checking" } else { "Building" },
                node.manifest.package.name
            );
            let output = library_output(graph, index, &stage)?;
            compile(
                compiler,
                &node.root,
                if check { "check" } else { "build" },
                &graph.library_source(index)?,
                &config,
                (!check).then_some(output.as_path()),
                true,
            )?;
        }
    }
    let targets = binaries.into_iter().map(|t| ("bin", t)).chain(
        root.manifest
            .test
            .iter()
            .filter(|_| tests || check)
            .map(|t| ("test", t)),
    );
    for (kind, target) in targets {
        eprintln!(
            "{} {kind} {}",
            if check { "Checking" } else { "Building" },
            target.name
        );
        let output = executable(&stage, kind, &target.name);
        compile(
            compiler,
            &root.root,
            if check { "check" } else { "build" },
            &manifest::source(&root.root, &target.path)?,
            &config,
            (!check).then_some(output.as_path()),
            false,
        )?;
    }
    let path = if check {
        remove_tree(&stage)?;
        owner.path.join("build")
    } else {
        owner.publish(&stage)?
    };
    Ok(BuildOutput {
        path,
        _owner: owner,
    })
}
