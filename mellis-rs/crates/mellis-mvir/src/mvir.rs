use mellis_common::ids::SymbolId;
use mellis_semantic::semantic_tables::SemanticTypeId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocalId {
    pub name: String,
    pub symbol_id: Option<SymbolId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlobalId {
    pub name: String,
    pub symbol_id: Option<SymbolId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LabelId {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    Local(LocalId),
    Global(GlobalId),
    Number(String),
    Boolean(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    Alloca {
        dest: LocalId,
        ty: SemanticTypeId,
    },
    Store {
        ptr: Operand,
        value: Operand,
    },
    Load {
        dest: LocalId,
        ptr: Operand,
        ty: SemanticTypeId,
    },
    Add {
        dest: LocalId,
        left: Operand,
        right: Operand,
        ty: SemanticTypeId,
    },
    Sub {
        dest: LocalId,
        left: Operand,
        right: Operand,
        ty: SemanticTypeId,
    },
    Mul {
        dest: LocalId,
        left: Operand,
        right: Operand,
        ty: SemanticTypeId,
    },
    Call {
        dest: Option<LocalId>,
        callee: Operand,
        args: Vec<Operand>,
        ret_ty: SemanticTypeId,
    },
    // Other binary, unary ops can be added here
}

#[derive(Clone, Debug, PartialEq)]
pub enum Terminator {
    Ret {
        value: Option<Operand>,
    },
    Br {
        target: LabelId,
    },
    CondBr {
        condition: Operand,
        true_target: LabelId,
        false_target: LabelId,
    },
    Unreachable,
}

#[derive(Clone, Debug)]
pub struct BasicBlock {
    pub label: LabelId,
    pub instructions: Vec<Instruction>,
    pub terminator: Option<Terminator>, // Should always have one in a valid BB
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: GlobalId,
    pub blocks: Vec<BasicBlock>,
    pub ret_ty: SemanticTypeId,
}

#[derive(Clone, Debug)]
pub struct Module {
    pub functions: Vec<Function>,
}

impl Module {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }
}
