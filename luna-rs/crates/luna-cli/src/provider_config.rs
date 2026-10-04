use luna_driver::provider_binding::{BindingOrigin, ProviderBinding, ProviderBindings};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    schema: Option<toml::Spanned<u32>>,
    #[serde(default)]
    providers: std::collections::BTreeMap<String, toml::Spanned<String>>,
}

fn error(
    file: &Path,
    source: &str,
    range: std::ops::Range<usize>,
    message: impl Into<String>,
) -> String {
    let mut manager = luna_common::source::SourceManager::new();
    let id = manager.add_file(file.display().to_string(), source.to_string());
    luna_common::Diagnostic::error(message)
        .with_code(luna_common::DiagnosticCode::ProviderConfigurationError)
        .with_span(luna_common::Span::new(
            id,
            range.start as u32,
            range.end as u32,
        ))
        .render(&manager)
}

pub fn load(
    entry: &Path,
    explicit: Option<&Path>,
    disabled: bool,
) -> Result<ProviderBindings, String> {
    let selected = if disabled {
        None
    } else if let Some(file) = explicit {
        Some(std::path::absolute(file).map_err(|e| error(file, "", 0..0, e.to_string()))?)
    } else {
        let entry =
            std::path::absolute(entry).map_err(|e| error(entry, "", 0..0, e.to_string()))?;
        entry.parent().and_then(|dir| {
            dir.ancestors()
                .map(|dir| dir.join("luna.toml"))
                .find(|file| file.exists())
        })
    };
    let Some(file) = selected else {
        return Ok(Default::default());
    };
    let source = std::fs::read_to_string(&file).map_err(|e| {
        error(
            &file,
            "",
            0..0,
            format!("Cannot read provider configuration: {}", e),
        )
    })?;
    let config: Configuration = toml::from_str(&source)
        .map_err(|e| error(&file, &source, e.span().unwrap_or(0..0), e.message()))?;
    if let Some(schema) = config.schema {
        if *schema.get_ref() != 1 {
            return Err(error(
                &file,
                &source,
                schema.span(),
                "Unsupported luna.toml schema; expected 1",
            ));
        }
    }
    let mut bindings = ProviderBindings::new();
    for (name, value) in config.providers {
        let range = value.span();
        let prefix = &source[..range.start];
        let key_start = prefix.rfind(['\n', '{', ',']).map_or(0, |i| i + 1);
        let key_end = prefix.rfind('=').unwrap_or(range.start);
        let key_range = key_start..key_end;
        if !luna_driver::provider_binding::valid_provider_name(&name) {
            return Err(error(
                &file,
                &source,
                key_range,
                format!("Invalid provider name '{}'", name),
            ));
        }
        let path = value.into_inner();
        let stem = PathBuf::from(&path);
        if path.trim().is_empty()
            || matches!(
                stem.extension().and_then(|s| s.to_str()),
                Some("ln" | "llib" | "ms" | "mlib")
            )
        {
            return Err(error(
                &file,
                &source,
                range,
                format!(
                    "Provider '{}' requires a nonempty extensionless file stem",
                    name
                ),
            ));
        }
        // Project path policy is independent of the host OS: a Windows drive
        // or UNC/root path must also reject when the loader runs on Unix.
        let bytes = path.as_bytes();
        let drive_qualified =
            bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
        if drive_qualified || path.starts_with(['/', '\\']) || stem.has_root() {
            return Err(error(
                &file, &source, range,
                format!("Provider '{}' requires a relative path; rooted and drive-qualified paths are forbidden", name),
            ));
        }
        if path.starts_with('~') || path.contains(['$', '%']) || path.chars().any(char::is_control)
        {
            return Err(error(
                &file, &source, range,
                format!("Provider '{}' path cannot contain home/environment expansion markers or control characters", name),
            ));
        }
        // Only file contents are relative-only. The driver still receives an
        // absolute discovery stem anchored to the selected configuration.
        let stem = file.parent().unwrap().join(stem);
        bindings.insert(
            name,
            ProviderBinding {
                stem,
                origin: Some(BindingOrigin {
                    file: file.clone(),
                    source: source.clone(),
                    key_range,
                }),
            },
        );
    }
    Ok(bindings)
}
