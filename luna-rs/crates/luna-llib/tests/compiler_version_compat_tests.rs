use luna_llib::{LlibReader, MlibError};
use std::path::Path;

/// Compatibility: a real artifact produced by compiler22 must be rejected by
/// compiler23 with a typed compiler-version mismatch — not silently reused with
/// the new checked-arithmetic semantics.
#[test]
fn real_compiler22_artifact_is_rejected_by_compiler23() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../../docs/audits/0.1-alpha-completion-2026-10-04/evidence/compat22/num-compiler22.llib",
    );
    let bytes = std::fs::read(&path).expect("archived compiler22 artifact must exist");
    assert_eq!(&bytes[0..4], b"LLIB", "artifact magic");
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 2, "format version is unchanged");
    assert_eq!(
        u16::from_le_bytes([bytes[6], bytes[7]]),
        22,
        "the archived artifact really was produced by compiler22"
    );
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 5, "MVIR wire version is unchanged");

    let mut cursor = std::io::Cursor::new(bytes);
    let err = LlibReader::read_manifest(&mut cursor).expect_err("compiler22 must be rejected");
    match err {
        MlibError::VersionMismatch { component, found, .. } => {
            assert_eq!(component, "compiler", "mismatch must be classified as the compiler component");
            assert_eq!(found, 22);
        }
        other => panic!("expected a compiler VersionMismatch, got {other:?}"),
    }
}
