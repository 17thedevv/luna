use luna_llib::{LlibReader, MlibError};
use std::path::Path;

/// Real compiler22 and pre-DCE compiler23 artifacts must be rejected before any
/// unchecked native body can be reused. Artifact bytes were never header-patched.
#[test]
fn real_older_compiler_artifacts_are_rejected_by_compiler25() {
    for (version, relative) in [
        (22, "compat22/num-compiler22.llib"),
        (23, "compat23/discarded-arithmetic-compiler23.llib"),
        (24, "compat24/enum-alignment-compiler24.llib"),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../../docs/audits/0.1-alpha-completion-2026-10-04/evidence").join(relative);
        let bytes = std::fs::read(&path).expect("genuine archived compiler artifact");
        assert_eq!(&bytes[0..4], b"LLIB");
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 2);
        assert_eq!(u16::from_le_bytes([bytes[6], bytes[7]]), version);
        assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 5);
        let error = LlibReader::read_manifest(&mut std::io::Cursor::new(bytes)).unwrap_err();
        match error {
            MlibError::VersionMismatch { component, found, .. } => {
                assert_eq!(component, "compiler");
                assert_eq!(found, version);
            }
            other => panic!("expected compiler VersionMismatch, got {other:?}"),
        }
    }
}
