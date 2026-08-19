use mellis_ast::{ExprId, StmtId, DeclId, PatId, TypeId as AstTypeId};
use mellis_common::ids::SymbolId;
use std::collections::HashMap;

// Placeholder for now, later we'll map to actual Type definitions
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SemanticTypeId(pub u32);

pub struct SemanticTables {
    pub expr_types: HashMap<ExprId, SemanticTypeId>,
    pub expr_symbols: HashMap<ExprId, SymbolId>,
    
    pub pat_symbols: HashMap<PatId, SymbolId>,
    pub pat_types: HashMap<PatId, SemanticTypeId>,
    
    pub decl_symbols: HashMap<DeclId, SymbolId>,
    
    pub ast_type_to_semantic: HashMap<AstTypeId, SemanticTypeId>,
}

impl SemanticTables {
    pub fn new() -> Self {
        Self {
            expr_types: HashMap::new(),
            expr_symbols: HashMap::new(),
            pat_symbols: HashMap::new(),
            pat_types: HashMap::new(),
            decl_symbols: HashMap::new(),
            ast_type_to_semantic: HashMap::new(),
        }
    }
}
