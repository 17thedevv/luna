use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[serde(default = "schema")]
    pub schema: u32,
    pub package: Package,
    pub lib: Option<Library>,
    #[serde(default)]
    pub bin: Vec<Target>,
    #[serde(default)]
    pub test: Vec<Target>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
    #[serde(default)]
    pub providers: BTreeMap<String, String>,
}
fn schema() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Library {
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub path: String,
}

pub fn name(value: &str, identifier: bool) -> Result<()> {
    let mut chars = value.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || (!identifier && c == '-'))
    {
        return Err(format!(
            "invalid {} name '{value}'",
            if identifier {
                "provider"
            } else {
                "target/package"
            }
        ));
    }
    let upper = value.to_ascii_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.as_bytes()[3].is_ascii_digit())
    {
        return Err(format!("reserved filesystem name '{value}'"));
    }
    Ok(())
}

pub fn relative(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    if value.trim().is_empty()
        || value.starts_with(['/', '\\', '~'])
        || value.contains(['\\', '$', '%', ':'])
        || value.chars().any(char::is_control)
        || (bytes.len() > 1 && bytes[1] == b':')
    {
        return Err(format!(
            "expected a relative path with '/' separators, without expansion: '{value}'"
        ));
    }
    Ok(())
}

pub fn source(root: &Path, value: &str) -> Result<PathBuf> {
    relative(value)?;
    if !value.ends_with(".ln") {
        return Err(format!("target source must end in .ln: '{value}'"));
    }
    let path = fs::canonicalize(root.join(value)).map_err(|e| format!("source '{value}': {e}"))?;
    if !path.starts_with(root) || !path.is_file() {
        return Err(format!(
            "target source must be a file inside its package: '{value}'"
        ));
    }
    Ok(path)
}

pub fn discover(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return fs::canonicalize(path).map_err(|e| format!("manifest {}: {e}", path.display()));
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    cwd.ancestors()
        .map(|p| p.join("luna.toml"))
        .find(|p| p.is_file())
        .ok_or_else(|| "no luna.toml found; use 'lunar new NAME' or 'lunar init'".into())
}

pub fn read(path: &Path) -> Result<Manifest> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut manifest: Manifest =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if manifest.schema != 1 {
        return Err(format!(
            "{}: unsupported schema {}; expected 1",
            path.display(),
            manifest.schema
        ));
    }
    name(&manifest.package.name, false)?;
    if manifest
        .package
        .version
        .as_ref()
        .is_some_and(|v| v.trim().is_empty())
    {
        return Err("package.version must be nonempty when supplied".into());
    }
    let root = path.parent().ok_or("manifest has no parent")?;
    if manifest.bin.is_empty() && manifest.lib.is_none() && root.join("src/main.ln").is_file() {
        manifest.bin.push(Target {
            name: manifest.package.name.clone(),
            path: "src/main.ln".into(),
        });
    }
    if manifest.bin.is_empty() && manifest.lib.is_none() {
        return Err(format!("{}: no lib or bin target", path.display()));
    }
    for targets in [&manifest.bin, &manifest.test] {
        let mut names = std::collections::BTreeSet::new();
        for target in targets {
            name(&target.name, false)?;
            if !names.insert(target.name.to_ascii_lowercase()) {
                return Err(format!("duplicate target '{}'", target.name));
            }
            source(root, &target.path)?;
        }
    }
    if let Some(lib) = &manifest.lib {
        let file = source(root, &lib.path)?;
        name(
            file.file_stem()
                .and_then(|s| s.to_str())
                .ok_or("non-UTF-8 library stem")?,
            true,
        )?;
    }
    for (alias, dep) in &manifest.dependencies {
        name(alias, true)?;
        relative(&dep.path)?;
    }
    for (alias, stem) in &manifest.providers {
        name(alias, true)?;
        relative(stem)?;
        if matches!(
            Path::new(stem).extension().and_then(|s| s.to_str()),
            Some("ln" | "llib" | "ms" | "mlib")
        ) {
            return Err(format!("provider '{alias}' requires an extensionless stem"));
        }
    }
    Ok(manifest)
}

pub struct Node {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub deps: Vec<(String, usize)>,
}
pub struct Graph {
    pub nodes: Vec<Node>,
    pub root: usize,
}

impl Graph {
    pub fn load(path: &Path) -> Result<Self> {
        let mut graph = Graph {
            nodes: Vec::new(),
            root: 0,
        };
        graph.root = graph.visit(path, &mut Vec::new(), &mut BTreeMap::new())?;
        Ok(graph)
    }

    fn visit(
        &mut self,
        path: &Path,
        active: &mut Vec<PathBuf>,
        done: &mut BTreeMap<PathBuf, usize>,
    ) -> Result<usize> {
        let path =
            fs::canonicalize(path).map_err(|e| format!("manifest {}: {e}", path.display()))?;
        if active.contains(&path) {
            let chain = active
                .iter()
                .chain(std::iter::once(&path))
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(format!("dependency cycle: {chain}"));
        }
        if let Some(index) = done.get(&path) {
            return Ok(*index);
        }
        active.push(path.clone());
        let manifest = read(&path)?;
        let root = path.parent().unwrap().to_path_buf();
        let mut deps = Vec::new();
        for (alias, dep) in &manifest.dependencies {
            let index = self.visit(&root.join(&dep.path).join("luna.toml"), active, done)?;
            if self.nodes[index].manifest.lib.is_none() {
                return Err(format!("dependency '{alias}' has no [lib] target"));
            }
            deps.push((alias.clone(), index));
        }
        active.pop();
        let index = self.nodes.len();
        self.nodes.push(Node {
            root,
            manifest,
            deps,
        });
        done.insert(path, index);
        Ok(index)
    }

    pub fn library_source(&self, index: usize) -> Result<PathBuf> {
        let node = &self.nodes[index];
        source(
            &node.root,
            &node
                .manifest
                .lib
                .as_ref()
                .ok_or("missing library target")?
                .path,
        )
    }
}
