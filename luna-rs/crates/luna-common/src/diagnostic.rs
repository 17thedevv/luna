use crate::ids::Span;
use crate::source::SourceManager;
use std::fmt;
use unicode_width::UnicodeWidthStr;

/// Stable, machine-readable compiler diagnostic identity.
///
/// Numerical discriminants are frozen by `docs/diagnostics/diagnostics-v1.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum DiagnosticCode {
    ExpectedToken = 1,
    UnexpectedToken = 2,
    InvalidSyntax = 3,
    UnclosedDelimiter = 4,
    InvalidAnnotation = 5,
    UnresolvedSymbol = 1001,
    DuplicateDefinition = 1002,
    PrivateSymbolAccess = 1003,
    UnresolvedModuleProvider = 1004,
    CyclicModuleDependency = 1005,
    InvalidVisibility = 1006,
    WildcardImportProhibited = 1007,
    AmbiguousSymbol = 1008,
    TypeMismatch = 2001,
    CannotDereference = 2002,
    CannotIndex = 2003,
    CopyDropConflict = 2004,
    ConflictingTraitImpl = 2005,
    OrphanImpl = 2006,
    UnresolvedTraitImpl = 2007,
    AssociatedTypeCycle = 2008,
    InfiniteSizeRecursiveType = 2009,
    NonObjectSafeTrait = 2010,
    InvalidLvalue = 2011,
    InvalidUnaryOp = 2012,
    InvalidBinaryOp = 2013,
    DuplicateField = 2014,
    DuplicateVariant = 2015,
    LifetimeConstraintViolation = 2016,
    MissingField = 2017,
    NoAssociatedType = 2018,
    AmbiguousAssociatedType = 2019,
    CannotCallNonFunction = 2020,
    TraitBoundNotSatisfied = 2021,
    NonExhaustivePattern = 2022,
    CannotMutateImmutable = 2023,
    InvalidMainSignature = 2024,
    UnsafeOperationOutsideUnsafe = 2025,
    InvalidCast = 2026,
    AwaitOutsideAsync = 2027,
    InvalidTryOperator = 2028,
    LoopControlOutsideLoop = 2029,
    NonFfiSafeType = 2030,
    ExplicitDropCall = 2031,
    UseAfterMove = 3001,
    PartialMoveUnderDrop = 3002,
    BorrowConflict = 3003,
    MissingReturnValue = 3004,
    LocalBorrowEscape = 3005,
    UseOfUninitializedValue = 3006,
    UseAfterDrop = 3007,
    RawStorageAnchorMismatch = 3010,
    RawStorageAnchorViolation = 3011,
    AsyncBorrowAcrossAwait = 4001,
    ComptimeStepLimitExceeded = 4002,
    ComptimeUnserializableEscape = 4003,
    ComptimePanic = 4004,
    ComptimeEvaluationFailed = 4005,
    MonomorphizationBarrier = 5001,
    InfiniteMonomorphizationRecursion = 5002,
    BackendInvariantViolation = 6001,
    ObjectEmissionFailure = 6002,
    LinkerFailure = 6003,
    SysrootConfigurationFailure = 6004,
    ProviderReadFailure = 6005,
    OutputWriteFailure = 6006,
    InvalidArtifactOutput = 6007,
    ProviderConfigurationError = 6008,
    ProcessExecutionFailure = 6009,
}

impl DiagnosticCode {
    /// Every diagnostic code, in freezing order. `docs/diagnostics/diagnostics-v1.md`
    /// (DIAG-10) requires exact parity between this list and the specification.
    pub const ALL: &'static [DiagnosticCode] = &[
        DiagnosticCode::ExpectedToken,
        DiagnosticCode::UnexpectedToken,
        DiagnosticCode::InvalidSyntax,
        DiagnosticCode::UnclosedDelimiter,
        DiagnosticCode::InvalidAnnotation,
        DiagnosticCode::UnresolvedSymbol,
        DiagnosticCode::DuplicateDefinition,
        DiagnosticCode::PrivateSymbolAccess,
        DiagnosticCode::UnresolvedModuleProvider,
        DiagnosticCode::CyclicModuleDependency,
        DiagnosticCode::InvalidVisibility,
        DiagnosticCode::WildcardImportProhibited,
        DiagnosticCode::AmbiguousSymbol,
        DiagnosticCode::TypeMismatch,
        DiagnosticCode::CannotDereference,
        DiagnosticCode::CannotIndex,
        DiagnosticCode::CopyDropConflict,
        DiagnosticCode::ConflictingTraitImpl,
        DiagnosticCode::OrphanImpl,
        DiagnosticCode::UnresolvedTraitImpl,
        DiagnosticCode::AssociatedTypeCycle,
        DiagnosticCode::InfiniteSizeRecursiveType,
        DiagnosticCode::NonObjectSafeTrait,
        DiagnosticCode::InvalidLvalue,
        DiagnosticCode::InvalidUnaryOp,
        DiagnosticCode::InvalidBinaryOp,
        DiagnosticCode::DuplicateField,
        DiagnosticCode::DuplicateVariant,
        DiagnosticCode::LifetimeConstraintViolation,
        DiagnosticCode::MissingField,
        DiagnosticCode::NoAssociatedType,
        DiagnosticCode::AmbiguousAssociatedType,
        DiagnosticCode::CannotCallNonFunction,
        DiagnosticCode::TraitBoundNotSatisfied,
        DiagnosticCode::NonExhaustivePattern,
        DiagnosticCode::CannotMutateImmutable,
        DiagnosticCode::InvalidMainSignature,
        DiagnosticCode::UnsafeOperationOutsideUnsafe,
        DiagnosticCode::InvalidCast,
        DiagnosticCode::AwaitOutsideAsync,
        DiagnosticCode::InvalidTryOperator,
        DiagnosticCode::LoopControlOutsideLoop,
        DiagnosticCode::NonFfiSafeType,
        DiagnosticCode::ExplicitDropCall,
        DiagnosticCode::UseAfterMove,
        DiagnosticCode::PartialMoveUnderDrop,
        DiagnosticCode::BorrowConflict,
        DiagnosticCode::MissingReturnValue,
        DiagnosticCode::LocalBorrowEscape,
        DiagnosticCode::UseOfUninitializedValue,
        DiagnosticCode::UseAfterDrop,
        DiagnosticCode::RawStorageAnchorMismatch,
        DiagnosticCode::RawStorageAnchorViolation,
        DiagnosticCode::AsyncBorrowAcrossAwait,
        DiagnosticCode::ComptimeStepLimitExceeded,
        DiagnosticCode::ComptimeUnserializableEscape,
        DiagnosticCode::ComptimePanic,
        DiagnosticCode::ComptimeEvaluationFailed,
        DiagnosticCode::MonomorphizationBarrier,
        DiagnosticCode::InfiniteMonomorphizationRecursion,
        DiagnosticCode::BackendInvariantViolation,
        DiagnosticCode::ObjectEmissionFailure,
        DiagnosticCode::LinkerFailure,
        DiagnosticCode::SysrootConfigurationFailure,
        DiagnosticCode::ProviderReadFailure,
        DiagnosticCode::OutputWriteFailure,
        DiagnosticCode::InvalidArtifactOutput,
        DiagnosticCode::ProviderConfigurationError,
        DiagnosticCode::ProcessExecutionFailure,
    ];

    pub const fn number(self) -> u16 {
        self as u16
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "E{:04}", self.number())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub code: Option<DiagnosticCode>,
    pub span: Option<Span>,
    pub message: String,
    pub related: Vec<DiagnosticLabel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel { pub span: Span, pub message: String }

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            code: None,
            span: None,
            message: message.into(),
            related: Vec::new(),
        }
    }

    pub fn with_code(mut self, code: DiagnosticCode) -> Self {
        self.code = Some(code);
        self
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_related(mut self, span: Span, message: impl Into<String>) -> Self {
        self.related.push(DiagnosticLabel { span, message: message.into() }); self
    }

    pub fn render(&self, source_manager: &SourceManager) -> String {
        let mut out = String::new();

        let level_str = match self.level {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Note => "note",
        };

        if let (Some(span), Some(file)) = (
            self.span,
            self.span.and_then(|s| source_manager.get_file(s.file_id)),
        ) {
            let (line, col) = file.get_line_col(span.start);
            if let Some(code) = self.code {
                out.push_str(&format!("{}[{}]: {}\n", level_str, code, self.message));
            } else {
                out.push_str(&format!("{}: {}\n", level_str, self.message));
            }
            out.push_str(&format!("  --> {}:{}:{}\n", file.name, line, col));

            if let Some(line_str) = file.get_line_str(line) {
                out.push_str("   |\n");
                out.push_str(&format!("{:<3}| {}\n", line, display_text(line_str, 0).0));

                let prefix: String = line_str.chars().take((col - 1) as usize).collect();
                let start = prefix.len();
                let end = start.saturating_add(span.end.saturating_sub(span.start) as usize)
                    .min(line_str.len());
                let mut end = end;
                while !line_str.is_char_boundary(end) { end -= 1; }
                let (_, width) = display_text(&prefix, 0);
                let indent = " ".repeat(width);
                let length = display_text(&line_str[start..end], width).1.saturating_sub(width).max(1);
                let carets = "^".repeat(length);
                out.push_str(&format!("   | {}{}\n", indent, carets));
            }
        } else {
            if let Some(code) = self.code {
                out.push_str(&format!("{}[{}]: {}\n", level_str, code, self.message));
            } else {
                out.push_str(&format!("{}: {}\n", level_str, self.message));
            }
        }

        for label in &self.related {
            out.push_str(&Self { level: DiagnosticLevel::Note, code: None, span: Some(label.span),
                message: label.message.clone(), related: Vec::new() }.render(source_manager));
        }
        out
    }
}

// Expand tabs consistently in both the source snippet and underline. Unicode
// widths handle wide characters and combining marks without changing byte spans.
fn display_text(text: &str, initial_column: usize) -> (String, usize) {
    let mut rendered = String::new();
    let mut column = initial_column;
    for (index, segment) in text.split('\t').enumerate() {
        if index > 0 {
            let width = 4 - column % 4;
            rendered.push_str(&" ".repeat(width));
            column += width;
        }
        rendered.push_str(segment);
        column += segment.width();
    }
    (rendered, column)
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, DiagnosticCode};
    use crate::source::SourceManager;

    #[test]
    fn candidate_labels_remain_structural_across_rendering() {
        let mut manager = SourceManager::new();
        let file = manager.add_file("names.ln".into(), "a b value".into());
        let primary = crate::Span::new(file, 4, 9);
        let origin = crate::Span::new(file, 0, 1);
        let diagnostic = Diagnostic::error("ambiguous name")
            .with_code(DiagnosticCode::AmbiguousSymbol).with_span(primary)
            .with_related(origin, "first candidate");
        assert_eq!(diagnostic.code, Some(DiagnosticCode::AmbiguousSymbol));
        assert_eq!(diagnostic.span, Some(primary));
        assert_eq!(diagnostic.related[0].span, origin);
        assert_eq!(diagnostic.related[0].message, "first candidate");
        let rendered = diagnostic.render(&manager);
        assert!(rendered.contains("error[E1008]"));
        assert!(rendered.contains("names.ln:1:1"));
        assert!(rendered.contains("note: first candidate"));
    }

    #[test]
    fn structured_code_renders_without_becoming_message_identity() {
        let diagnostic = Diagnostic::error("Use of moved value `value`")
            .with_code(DiagnosticCode::UseAfterMove);

        assert_eq!(diagnostic.code, Some(DiagnosticCode::UseAfterMove));
        assert_eq!(diagnostic.message, "Use of moved value `value`");
        assert_eq!(
            diagnostic.render(&SourceManager::new()),
            "error[E3001]: Use of moved value `value`\n"
        );
    }

    #[test]
    fn underline_uses_display_width_and_clips_to_the_primary_line() {
        // Tab: four columns; é: one; Han: two; combining accent: zero.
        let source = "\té漢e\u{301} nope\r\nnext";
        let mut manager = SourceManager::new();
        let id = manager.add_file("unicode.ln".into(), source.into());
        let start = source.find("nope").unwrap() as u32;
        let diagnostic = Diagnostic::error("unknown name")
            .with_code(DiagnosticCode::UnresolvedSymbol)
            .with_span(crate::Span::new(id, start, source.len() as u32));
        let rendered = diagnostic.render(&manager);
        assert!(rendered.contains("unicode.ln:1:7"), "{rendered}");
        assert!(rendered.contains("1  |     é漢e\u{301} nope\n"), "{rendered}");
        assert!(rendered.contains("   |          ^^^^\n"), "{rendered}");
    }

    #[test]
    fn unicode_highlight_length_is_not_utf8_byte_length() {
        let mut manager = SourceManager::new();
        let id = manager.add_file("test.ln".into(), "a é漢".into());
        let diagnostic = Diagnostic::error("bad token")
            .with_code(DiagnosticCode::InvalidSyntax)
            .with_span(crate::Span::new(id, 2, 7));
        assert!(diagnostic.render(&manager).contains("   |   ^^^\n"));
    }

    #[test]
    fn joined_emoji_in_source_prefix_uses_its_rendered_width() {
        let source = "\t👩\u{200d}💻 nope";
        let mut manager = SourceManager::new();
        let id = manager.add_file("emoji.ln".into(), source.into());
        let diagnostic = Diagnostic::error("unknown name")
            .with_code(DiagnosticCode::UnresolvedSymbol)
            .with_span(crate::Span::new(id, source.find("nope").unwrap() as u32, source.len() as u32));
        let rendered = diagnostic.render(&manager);
        assert!(rendered.contains("emoji.ln:1:6"), "{rendered}");
        assert!(rendered.contains("   |        ^^^^\n"), "{rendered}");
    }
}

/// DIAG-10: exact parity between `DiagnosticCode::ALL` and the frozen registry
/// `docs/diagnostics/diagnostics-v1.md`.
#[cfg(test)]
mod registry_parity {
    use super::DiagnosticCode;

    const DOCS: &str = include_str!("../../../../docs/diagnostics/diagnostics-v1.md");

    #[test]
    fn every_code_is_documented_unique_and_named() {
        let mut seen = std::collections::BTreeSet::new();
        for code in DiagnosticCode::ALL {
            let number = code.number();
            assert!(seen.insert(number), "duplicate diagnostic number {number}");
            let needle = format!("E{number:04}");
            assert!(
                DOCS.contains(&format!("| `{needle}` |")),
                "{needle} is missing from the diagnostics registry table"
            );
            assert!(
                DOCS.contains(&format!("{code:?}")),
                "{code:?} name is missing from the diagnostics registry"
            );
        }
    }

    #[test]
    fn documented_codes_all_exist_in_the_enum() {
        let numbers: std::collections::BTreeSet<u16> =
            DiagnosticCode::ALL.iter().map(|code| code.number()).collect();
        for line in DOCS.lines() {
            let Some(rest) = line.strip_prefix("| `E") else {
                continue;
            };
            let Some((digits, tail)) = rest.split_once('`') else {
                continue;
            };
            // Only exact code rows (`| `E1001` | ...`) count; range rows such as
            // `| `E1000`–`E1999` | ...` are excluded.
            if !tail.starts_with(" |") {
                continue;
            }
            let number: u16 = digits.parse().expect("numeric diagnostic id");
            assert!(
                numbers.contains(&number),
                "registry documents E{number:04} but DiagnosticCode has no such code"
            );
        }
    }
}
