<!-- luna-doc-role: evidence -->

# C5 report — artifact freshness classification

Branch `c5-artifact-freshness-classification` (from candidate `5994f5a3`).

## Freeze result — one classification counterexample

Format, compiler, MVIR and semantic-metadata version mismatches all surfaced as
the **same** `MlibError::VersionMismatch(u16)`, rendered as
`invalid manifest: VersionMismatch(N)`. The user and tests could not tell which
boundary rejected the artifact.

**Layer:** artifact header/version validation (`luna-llib` reader).
**Fix:** classify the boundary —
`MlibError::VersionMismatch { component, found, expected }` with `component` in
`{ format, compiler, mvir, semantic-metadata }`.

## Classes after the fix

| Class | Diagnostic |
|---|---|
| FORMAT_INCOMPATIBLE | `VersionMismatch { component: "format" }` |
| COMPILER_INCOMPATIBLE | `VersionMismatch { component: "compiler" }` |
| MVIR_INCOMPATIBLE | `VersionMismatch { component: "mvir" }` |
| METADATA_INCOMPATIBLE | `VersionMismatch { component: "semantic-metadata" }` |
| TARGET_INCOMPATIBLE | `TargetMismatch` |
| OBJECT_IDENTITY_MISMATCH | `ObjectIntegrityMismatch` |
| INTERFACE_STALE | `dependency interface fingerprint mismatch ...` |
| EXECUTION_STALE | `dependency execution fingerprint mismatch ...` |
| TAMPERED_SECTION | `SectionChecksumMismatch` |

## Harness (`fresh_artifact_classification_cli.rs`)

Publishes a real provider `.llib` and mutates one boundary per case (header
format / compiler / mvir / target triple, the semantic-metadata section version,
and the object payload), refreshing the section checksum so validation reaches
the boundary under test. Every stale class is rejected deterministically with a
classified reason and no executable is published; a freshly published artifact
builds and runs.

## Related finding

`FIND-ASYNC-PROVIDER-01` (parameterless-async provider `.llib` corruption)
remains independent — it is a deeper serialization/decode issue surfaced as
`CorruptedData`, not one of the freshness classes above, and is not fixed in C5.

## Result

- `fresh_artifact_classification_cli`: 1 passed / 0 failed.
- `luna-llib`: 24 passed / 0 failed.
- `struct_lifetime_contract_acceptance_tests`: 1 passed.
- Full workspace: 214 binaries, 1346 passed / 0 failed / 1 ignored, exit 0.

## Versioning

No bump: diagnostics/classification only — the artifact schema and the
accepted-artifact set are unchanged (compiler22 / metadata9 / MVIR5 / format2).

## Verdict

**C5: CONFORMANT IN TESTED SCOPE** — every freshness/staleness class rejects
deterministically with a classified reason, and fresh artifacts build and run.
