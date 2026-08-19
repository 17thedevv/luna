pub mod resolver;
pub mod typechecker;
pub mod symbol;
pub mod semantic_tables;
pub mod ty;

pub use resolver::Resolver;
pub use typechecker::TypeChecker;
pub use symbol::{SymbolTable, ScopeId, SymbolKind};
pub use mellis_common::ids::SymbolId;
pub use semantic_tables::SemanticTables;
pub use ty::{TypeContext, SemanticTypeId, SemanticType, BuiltinType};

use mellis_ast::AstArena;
use mellis_common::Diagnostic;

pub struct SemanticContext {
    pub symbol_table: SymbolTable,
    pub tables: SemanticTables,
    pub types: TypeContext,
    pub diagnostics: Vec<Diagnostic>,
}

impl SemanticContext {
    pub fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            tables: SemanticTables::new(),
            types: TypeContext::new(),
            diagnostics: Vec::new(),
        }
    }
}
