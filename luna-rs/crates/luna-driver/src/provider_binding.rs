use std::collections::BTreeMap;
use std::path::PathBuf;

/// Normalized application discovery input. The driver never reads project TOML.
#[derive(Clone, Debug)]
pub struct ProviderBinding {
    pub stem: PathBuf,
    pub origin: Option<BindingOrigin>,
}

#[derive(Clone, Debug)]
pub struct BindingOrigin {
    pub file: PathBuf,
    pub source: String,
    pub key_range: std::ops::Range<usize>,
}

pub type ProviderBindings = BTreeMap<String, ProviderBinding>;

pub fn valid_provider_name(name: &str) -> bool {
    use luna_lexer::{Lexer, TokenKind};
    let tokens: Vec<_> = Lexer::new(name, luna_common::ids::FileId(0)).collect();
    tokens.first().is_some_and(|token| {
        token.kind == TokenKind::Identifier
            && token.span.start == 0
            && token.span.end as usize == name.len()
    }) && tokens
        .iter()
        .skip(1)
        .all(|token| token.kind == TokenKind::Eof)
}
