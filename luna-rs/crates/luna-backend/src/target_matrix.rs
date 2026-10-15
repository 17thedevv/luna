//! Canonical Luna release target selection (R5-FU-TARGET-01).
//!
//! The Luna 0.1 release matrix advertises exactly two targets. An artifact's
//! target identity is always the canonical Luna triple below, never the
//! spelling the host LLVM happens to report. A host triple that denotes the
//! same platform maps onto its canonical Luna triple; every other platform
//! (macOS, 32-bit, freestanding, other environments) is outside the release
//! matrix and is never canonicalized into a supported target.

/// A target advertised by the Luna release matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LunaTarget {
    WindowsX86_64Gnu,
    LinuxX86_64Gnu,
}

impl LunaTarget {
    /// The canonical Luna triple recorded in artifact target contracts.
    pub const fn triple(self) -> &'static str {
        match self {
            LunaTarget::WindowsX86_64Gnu => "x86_64-pc-windows-gnu",
            LunaTarget::LinuxX86_64Gnu => "x86_64-unknown-linux-gnu",
        }
    }
}

/// The supported Luna release targets.
pub const SUPPORTED_TARGETS: &[LunaTarget] =
    &[LunaTarget::WindowsX86_64Gnu, LunaTarget::LinuxX86_64Gnu];

/// Map an LLVM host/target triple onto the canonical Luna release target it
/// denotes, or `None` when the triple is outside the release matrix.
pub fn canonical_supported_target(host_triple: &str) -> Option<LunaTarget> {
    let mut components = host_triple.split('-');
    let arch = components.next()?;
    if arch != "x86_64" {
        return None;
    }
    let _vendor = components.next()?;
    let os = components.next()?;
    let environment = components.next().unwrap_or("");
    if components.next().is_some() {
        // A longer triple carries an ABI/vendor subcomponent Luna does not
        // release for.
        return None;
    }
    match (os, environment) {
        // The Windows release target is the GNU ABI. A Windows host may report
        // the MSVC environment for the same platform.
        ("windows", "gnu") | ("windows", "msvc") => Some(LunaTarget::WindowsX86_64Gnu),
        // The Linux vendor component is not part of Luna's target identity:
        // `pc` and `unknown` denote the same release target.
        ("linux", "gnu") => Some(LunaTarget::LinuxX86_64Gnu),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_host_spellings_map_to_their_canonical_luna_triple() {
        for spelling in ["x86_64-unknown-linux-gnu", "x86_64-pc-linux-gnu"] {
            assert_eq!(
                canonical_supported_target(spelling),
                Some(LunaTarget::LinuxX86_64Gnu),
                "{spelling}"
            );
        }
        for spelling in [
            "x86_64-pc-windows-gnu",
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-windows-gnu",
        ] {
            assert_eq!(
                canonical_supported_target(spelling),
                Some(LunaTarget::WindowsX86_64Gnu),
                "{spelling}"
            );
        }
        assert_eq!(
            LunaTarget::WindowsX86_64Gnu.triple(),
            "x86_64-pc-windows-gnu"
        );
        assert_eq!(
            LunaTarget::LinuxX86_64Gnu.triple(),
            "x86_64-unknown-linux-gnu"
        );
    }

    #[test]
    fn targets_outside_the_release_matrix_are_not_canonicalized() {
        for spelling in [
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
            "aarch64-unknown-linux-gnu",
            "i686-pc-windows-gnu",
            "i686-unknown-linux-gnu",
            "x86_64-unknown-none",
            "x86_64-unknown-freebsd",
            "x86_64-unknown-linux-musl",
            "x86_64-pc-windows-gnullvm",
            "x86_64-unknown-linux-gnuabi64",
            "x86_64-linux-gnu",
            "",
        ] {
            assert_eq!(
                canonical_supported_target(spelling),
                None,
                "{spelling} must not be canonicalized into a Luna release target"
            );
        }
    }
}
