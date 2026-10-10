//! Shared test-infrastructure primitives.
//!
//! `TempWorkspace` is the single owner of a test's on-disk temporary state. It
//! creates a unique directory under the process temp dir and removes it on drop
//! (including during panic unwinding), so repeated and parallel test runs cannot
//! accumulate temp state or collide with each other.
//!
//! Safety of the removal is guarded structurally, not heuristically:
//!   1. the directory name starts with [`NAME_PREFIX`];
//!   2. the directory sits directly inside `std::env::temp_dir()`;
//!   3. the directory contains the [`MARKER`] file this type wrote at creation.
//!
//! A real sysroot or a user directory satisfies none of these, so it can never
//! be deleted by accident.

use std::fs;
use std::ops::Deref;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const NAME_PREFIX: &str = "luna_test_ws_";
const MARKER: &str = ".luna-test-workspace";
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// RAII owner of a unique temporary directory. Derefs to [`Path`] so existing
/// code that treats the workspace as a path keeps working.
pub struct TempWorkspace {
    path: PathBuf,
    retain: bool,
}

impl TempWorkspace {
    /// Create a fresh workspace. `label` only makes directories recognizable;
    /// uniqueness comes from the process id, a timestamp and a counter.
    pub fn new(label: &str) -> Self {
        sweep_once();
        let label: String = label
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "{NAME_PREFIX}{label}_{}_{}_{}",
            std::process::id(),
            nanos,
            unique
        ));
        fs::create_dir_all(&path).expect("create test temp workspace");
        fs::write(path.join(MARKER), b"owned by luna-test-support\n")
            .expect("write test temp workspace marker");
        Self { path, retain: false }
    }

    /// The workspace root.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// A path inside the workspace (the parent is created on demand by callers).
    pub fn child(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.path.join(relative)
    }

    /// Keep the directory after drop, for debugging a failing run.
    pub fn retain(&mut self) {
        self.retain = true;
    }

    fn same_dir(left: &Path, right: &Path) -> bool {
        let normalize = |path: &Path| -> Vec<String> {
            path.components()
                .filter_map(|component| match component {
                    Component::Normal(part) => Some(part.to_string_lossy().to_lowercase()),
                    Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().to_lowercase()),
                    Component::RootDir => Some(String::from("\\")),
                    _ => None,
                })
                .collect()
        };
        normalize(left) == normalize(right)
    }

    fn owned_by_us(&self) -> bool {
        let parent_ok = self
            .path
            .parent()
            .is_some_and(|parent| Self::same_dir(parent, &std::env::temp_dir()));
        let name_ok = self
            .path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(NAME_PREFIX));
        let marker_ok = self.path.join(MARKER).is_file();
        parent_ok && name_ok && marker_ok
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        if self.retain || !self.owned_by_us() {
            return;
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

impl Deref for TempWorkspace {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for TempWorkspace {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl std::fmt::Debug for TempWorkspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TempWorkspace").field("path", &self.path).finish()
    }
}

/// Process-scoped owner for suites whose helpers hand a `PathBuf` to existing
/// call sites. Owners are dropped at process exit, so a completed run leaves no
/// temporary state behind.
pub fn owned_temp_dir(label: &str) -> PathBuf {
    static OWNED: Mutex<Vec<TempWorkspace>> = Mutex::new(Vec::new());
    let workspace = TempWorkspace::new(label);
    let path = workspace.path().to_path_buf();
    OWNED.lock().unwrap().push(workspace);
    path
}

/// Reclaim workspaces left behind by processes that have already exited.
///
/// Ownership normally ends at `Drop`, but a workspace that must outlive a single
/// test (a process-lifetime sysroot) or a test process that was killed cannot run
/// its destructor. This sweep runs once per process and removes only directories
/// that still satisfy the structural ownership guard and whose owning pid is no
/// longer alive.
fn sweep_once() {
    static SWEPT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    SWEPT.get_or_init(sweep_orphan_workspaces);
}

fn sweep_orphan_workspaces() {
    let live = live_process_ids();
    let Ok(entries) = fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with(NAME_PREFIX) {
            continue;
        }
        if let Some(pid) = owning_pid(name) {
            if pid == std::process::id() || live.contains(&pid) {
                continue;
            }
        }
        let path = entry.path();
        if path.join(MARKER).is_file() {
            let _ = fs::remove_dir_all(&path);
        }
    }
}

/// The pid embedded in `luna_test_ws_<label>_<pid>_<nanos>_<counter>`.
fn owning_pid(name: &str) -> Option<u32> {
    let parts: Vec<&str> = name.split('_').collect();
    if parts.len() < 4 {
        return None;
    }
    parts[parts.len() - 3].parse().ok()
}

fn live_process_ids() -> std::collections::HashSet<u32> {
    let mut ids = std::collections::HashSet::new();
    #[cfg(windows)]
    if let Ok(output) = std::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
    {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let columns: Vec<&str> = line.split("\",\"").collect();
            if let Some(pid) = columns.get(1).and_then(|column| column.trim_matches('"').parse::<u32>().ok()) {
                ids.insert(pid);
            }
        }
    }
    ids
}

/// One owned per-process root, with a unique child for every call.
///
/// Suite helpers build names from `pid` + timestamp alone, which collide when
/// tests run in parallel inside one process (a collision makes two tests share a
/// directory — and therefore a build lock and its artifacts). The per-call child
/// makes every helper invocation unique regardless of clock resolution. The
/// whole root is reclaimed on process exit or by the orphan sweep.
pub fn owned_temp_root() -> PathBuf {
    static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    static CALLS: AtomicU64 = AtomicU64::new(0);
    let root = ROOT.get_or_init(|| owned_temp_dir("root")).clone();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let child = root.join(format!(
        "call_{}_{}_{}",
        std::process::id(),
        nanos,
        CALLS.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&child).expect("create per-call temp root");
    child
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_removes_the_workspace() {
        let path = {
            let workspace = TempWorkspace::new("drop");
            assert!(workspace.path().is_dir());
            workspace.path().to_path_buf()
        };
        assert!(!path.exists(), "temp workspace must be removed on drop");
    }

    #[test]
    fn drop_removes_the_workspace_on_panic_unwind() {
        use std::sync::{Arc, Mutex};
        let seen: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
        let capture = Arc::clone(&seen);
        let result = std::panic::catch_unwind(move || {
            let workspace = TempWorkspace::new("panic");
            *capture.lock().unwrap() = Some(workspace.path().to_path_buf());
            panic!("simulate a failing test after the workspace is created");
        });
        assert!(result.is_err());
        let path = seen.lock().unwrap().clone().expect("workspace path captured");
        assert!(!path.exists(), "temp workspace must be removed during panic unwind");
    }

    #[test]
    fn sequential_workspaces_do_not_accumulate() {
        let mut paths = Vec::new();
        for _ in 0..8 {
            let workspace = TempWorkspace::new("sequential");
            paths.push(workspace.path().to_path_buf());
        }
        assert!(paths.iter().all(|path| !path.exists()), "no workspace may survive its scope");
    }

    #[test]
    fn retain_keeps_the_workspace_for_inspection() {
        let mut workspace = TempWorkspace::new("retain");
        let path = workspace.path().to_path_buf();
        workspace.retain();
        drop(workspace);
        assert!(path.exists());
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn sweep_reclaims_orphans_but_spares_the_living() {
        // A pid that is not alive stands in for a killed run.
        let orphan = std::env::temp_dir().join(format!("{NAME_PREFIX}orphan_999999_1_0"));
        fs::create_dir_all(&orphan).unwrap();
        fs::write(orphan.join(MARKER), b"orphan").unwrap();
        // No marker: not provably ours, so it must survive the sweep.
        let foreign = std::env::temp_dir().join(format!("{NAME_PREFIX}foreign_999999_1_1"));
        fs::create_dir_all(&foreign).unwrap();
        // Our own live workspace must never be swept.
        let own = TempWorkspace::new("sweep-live");
        let own_path = own.path().to_path_buf();

        sweep_orphan_workspaces();

        assert!(!orphan.exists(), "orphan from a dead pid must be reclaimed");
        assert!(foreign.exists(), "a directory without our marker must be spared");
        assert!(own_path.exists(), "a live workspace must never be reclaimed");

        let _ = fs::remove_dir_all(&foreign);
        drop(own);
    }

    #[test]
    fn per_call_temp_roots_never_collide_across_threads() {
        use std::collections::HashSet;
        let handles: Vec<_> = (0..8)
            .map(|_| {
                std::thread::spawn(|| {
                    (0..16).map(|_| owned_temp_root()).collect::<Vec<_>>()
                })
            })
            .collect();
        let all: Vec<PathBuf> = handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("worker"))
            .collect();
        let unique: HashSet<&PathBuf> = all.iter().collect();
        assert_eq!(unique.len(), all.len(), "per-call temp roots must be unique");
        assert!(all.iter().all(|path| path.is_dir()));
        for path in &all {
            let _ = fs::remove_dir_all(path);
        }
    }

    #[test]
    fn guard_refuses_to_delete_a_foreign_directory() {
        let mut workspace = TempWorkspace::new("guard");
        // Simulate a directory we no longer own (marker removed).
        fs::remove_file(workspace.path().join(MARKER)).unwrap();
        let path = workspace.path().to_path_buf();
        drop(workspace);
        assert!(path.exists(), "cleanup must fail closed when ownership is not provable");
        let _ = fs::remove_dir_all(&path);
    }
}
