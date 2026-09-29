pub struct LangContractManifest {
    pub contracts: Vec<LangContractEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityKind {
    /// Items implicitly required by the compiler (e.g., Iterator, Option, Drop)
    CompilerContract,
    /// Ergonomic foundational types/traits added for API parity, not intrinsic to the compiler
    StandardPrelude,
}

pub struct LangContractEntry {
    pub contract_id: &'static str,
    pub provider_id: &'static str,
    pub visibility_kind: VisibilityKind,
    /// Fully-qualified logical paths selectively exposed under global `std`.
    /// These paths never create unqualified root bindings.
    pub auto_visible_paths: &'static [&'static str],
    /// Compiler language items resolved by exact logical path after provider
    /// acquisition. Provider names select where to load from, not what symbol
    /// a language item means.
    pub language_items: &'static [LanguageItemPath],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageItemPath {
    pub lang_item: &'static str,
    pub canonical_path: &'static str,
}

impl LangContractManifest {
    pub fn canonical() -> Self {
        Self {
            contracts: vec![
                // --- Language Contracts ---
                // These are auto-visible because the compiler natively knows about their semantics
                // and expects them in certain language constructs (e.g., `for` loops, `drop`).
                LangContractEntry {
                    contract_id: "drop",
                    provider_id: "__lang_drop",
                    visibility_kind: VisibilityKind::CompilerContract,
                    auto_visible_paths: &["std::Drop"],
                    language_items: &[
                        LanguageItemPath { lang_item: "drop", canonical_path: "std::Drop" },
                        LanguageItemPath { lang_item: "drop_fn", canonical_path: "std::Drop::drop" },
                    ],
                },
                LangContractEntry {
                    contract_id: "option",
                    provider_id: "__lang_option",
                    visibility_kind: VisibilityKind::CompilerContract,
                    auto_visible_paths: &[
                        "std::Option",
                        "std::option_flatten",
                    ],
                    language_items: &[
                        LanguageItemPath { lang_item: "option", canonical_path: "std::Option" },
                        LanguageItemPath { lang_item: "some", canonical_path: "std::Option::Some" },
                        LanguageItemPath { lang_item: "none", canonical_path: "std::Option::None" },
                    ],
                },
                LangContractEntry {
                    contract_id: "iterator",
                    provider_id: "__lang_iterator",
                    visibility_kind: VisibilityKind::CompilerContract,
                    auto_visible_paths: &["std::Iterator"],
                    language_items: &[
                        LanguageItemPath { lang_item: "iterator", canonical_path: "std::Iterator" },
                    ],
                },
                LangContractEntry {
                    contract_id: "into_iterator",
                    provider_id: "__lang_into_iterator",
                    visibility_kind: VisibilityKind::CompilerContract,
                    auto_visible_paths: &["std::IntoIterator"],
                    language_items: &[
                        LanguageItemPath { lang_item: "into_iterator", canonical_path: "std::IntoIterator" },
                    ],
                },
                LangContractEntry {
                    contract_id: "copy",
                    provider_id: "copy",
                    visibility_kind: VisibilityKind::CompilerContract,
                    auto_visible_paths: &["std::Copy"],
                    language_items: &[
                        LanguageItemPath { lang_item: "copy", canonical_path: "std::Copy" },
                    ],
                },
                LangContractEntry {
                    contract_id: "try",
                    provider_id: "try",
                    visibility_kind: VisibilityKind::CompilerContract,
                    auto_visible_paths: &[
                        "std::Try",
                        "std::FromResidual",
                        "std::ControlFlow",
                    ],
                    language_items: &[
                        LanguageItemPath { lang_item: "try", canonical_path: "std::Try" },
                        LanguageItemPath { lang_item: "try_from_output", canonical_path: "std::Try::from_output" },
                        LanguageItemPath { lang_item: "try_branch", canonical_path: "std::Try::branch" },
                        LanguageItemPath { lang_item: "from_residual", canonical_path: "std::FromResidual" },
                        LanguageItemPath { lang_item: "from_residual_fn", canonical_path: "std::FromResidual::from_residual" },
                        LanguageItemPath { lang_item: "control_flow", canonical_path: "std::ControlFlow" },
                        LanguageItemPath { lang_item: "continue", canonical_path: "std::ControlFlow::Continue" },
                        LanguageItemPath { lang_item: "break", canonical_path: "std::ControlFlow::Break" },
                    ],
                },

                // --- Controlled Standard Prelude ---
                // OptionExt is the only controlled standard-prelude item. Result remains
                // an ordinary explicitly imported core API even though both declarations
                // are supplied by the same logical provider.
                LangContractEntry {
                    contract_id: "result",
                    provider_id: "result",
                    visibility_kind: VisibilityKind::StandardPrelude,
                    auto_visible_paths: &["std::OptionExt"],
                    language_items: &[],
                },
            ],
        }
    }
}
