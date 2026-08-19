use crate::mvir::*;
use mellis_ast::{AstArena, Item, Expr, Stmt, Decl};
use mellis_semantic::SemanticContext;

pub struct MvirGenerator<'a> {
    arena: &'a AstArena,
    ctx: &'a SemanticContext,
    module: Module,
    
    // State during function generation
    current_function: Option<Function>,
    current_block: Option<BasicBlock>,
    next_local_id: u32,
    next_label_id: u32,
}

impl<'a> MvirGenerator<'a> {
    pub fn new(arena: &'a AstArena, ctx: &'a SemanticContext) -> Self {
        Self {
            arena,
            ctx,
            module: Module::new(),
            current_function: None,
            current_block: None,
            next_local_id: 0,
            next_label_id: 0,
        }
    }

    pub fn generate(mut self, items: &[Item]) -> Module {
        for item in items {
            self.generate_item(item);
        }
        self.module
    }

    fn generate_item(&mut self, item: &Item) {
        match item {
            Item::Decl(decl_id) => {
                let decl = &self.arena.decls[decl_id.0 as usize];
                if let Decl::Function { name, body, .. } = decl {
                    let name_str = format!("func_{}_{}", name.start, name.end); // Dummy name
                    let global_id = GlobalId {
                        name: name_str,
                        symbol_id: self.ctx.tables.decl_symbols.get(decl_id).copied(),
                    };
                    
                    self.current_function = Some(Function {
                        name: global_id,
                        blocks: Vec::new(),
                        ret_ty: mellis_semantic::SemanticTypeId(0),
                    });
                    
                    let entry_label = self.new_label("entry");
                    self.start_block(entry_label);
                    
                    if let Some(body_stmt) = body {
                        self.generate_stmt(body_stmt);
                    }
                    
                    // Add implicit return if block is not terminated
                    if let Some(mut block) = self.current_block.take() {
                        if block.terminator.is_none() {
                            block.terminator = Some(Terminator::Ret { value: None });
                        }
                        self.current_function.as_mut().unwrap().blocks.push(block);
                    }
                    
                    if let Some(func) = self.current_function.take() {
                        self.module.functions.push(func);
                    }
                }
            }
            Item::Stmt(stmt_id) => {
                self.generate_stmt(stmt_id);
            }
        }
    }

    fn generate_stmt(&mut self, stmt_id: &mellis_ast::StmtId) {
        let stmt = &self.arena.stmts[stmt_id.0 as usize];
        match stmt {
            Stmt::Expr { expr, .. } => {
                self.generate_expr(expr);
            }
            Stmt::Block { body, tail_expr } => {
                for item in body {
                    self.generate_item(item);
                }
                if let Some(expr) = tail_expr {
                    self.generate_expr(expr);
                }
            }
            Stmt::Return { value } => {
                let val_operand = if let Some(expr) = value {
                    Some(self.generate_expr(expr))
                } else {
                    None
                };
                self.terminate_block(Terminator::Ret { value: val_operand });
            }
            // Add if, while, for loops here later
            _ => {}
        }
    }

    fn generate_expr(&mut self, expr_id: &mellis_ast::ExprId) -> Operand {
        let expr = &self.arena.exprs[expr_id.0 as usize];
        let ty_id = self.ctx.tables.expr_types.get(expr_id).copied().unwrap_or(mellis_semantic::SemanticTypeId(0));
        
        match expr {
            Expr::Literal(_tok) => {
                // In real compiler, parse token text.
                Operand::Number("null".to_string())
            }
            Expr::Call { callee, args, .. } => {
                let callee_op = self.generate_expr(callee);
                let mut arg_ops = Vec::new();
                for arg in args {
                    arg_ops.push(self.generate_expr(&arg.value));
                }
                let dest = self.new_local("call_res");
                self.push_inst(Instruction::Call {
                    dest: Some(dest.clone()),
                    callee: callee_op,
                    args: arg_ops,
                    ret_ty: ty_id,
                });
                Operand::Local(dest)
            }
            Expr::Identifier { segments, .. } => {
                // Resolve to GlobalId or LocalId based on SemanticContext
                // Dummy global for now to satisfy print("...")
                let name = format!("global_{}", segments[0].start);
                Operand::Global(GlobalId {
                    name,
                    symbol_id: self.ctx.tables.expr_symbols.get(expr_id).copied(),
                })
            }
            _ => Operand::Number("0".to_string())
        }
    }
    
    // --- Helpers ---
    
    fn new_local(&mut self, prefix: &str) -> LocalId {
        let id = self.next_local_id;
        self.next_local_id += 1;
        LocalId {
            name: format!("%{}{}", prefix, id),
            symbol_id: None,
        }
    }
    
    fn new_label(&mut self, prefix: &str) -> LabelId {
        let id = self.next_label_id;
        self.next_label_id += 1;
        LabelId {
            name: format!("{}{}", prefix, id),
        }
    }
    
    fn start_block(&mut self, label: LabelId) {
        if let Some(mut block) = self.current_block.take() {
            if block.terminator.is_none() {
                block.terminator = Some(Terminator::Br { target: label.clone() });
            }
            self.current_function.as_mut().unwrap().blocks.push(block);
        }
        self.current_block = Some(BasicBlock {
            label,
            instructions: Vec::new(),
            terminator: None,
        });
    }
    
    fn push_inst(&mut self, inst: Instruction) {
        if let Some(block) = &mut self.current_block {
            block.instructions.push(inst);
        }
    }
    
    fn terminate_block(&mut self, term: Terminator) {
        if let Some(block) = &mut self.current_block {
            if block.terminator.is_none() {
                block.terminator = Some(term);
            }
        }
        // Force start of a new block if code follows
        let next_label = self.new_label("unreachable");
        self.start_block(next_label);
    }
}
