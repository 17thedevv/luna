use crate::{SemanticContext, ty::{SemanticTypeId, SemanticType, BuiltinType}};
use mellis_ast::{AstArena, Item, Stmt, Expr, Decl};
use mellis_lexer::{BuiltinKind, TokenKind};

pub struct TypeChecker<'a> {
    ctx: &'a mut SemanticContext,
    arena: &'a AstArena,
}

impl<'a> TypeChecker<'a> {
    pub fn new(ctx: &'a mut SemanticContext, arena: &'a AstArena) -> Self {
        Self { ctx, arena }
    }
    
    pub fn unify(&mut self, expected: SemanticTypeId, actual: SemanticTypeId) -> Result<(), String> {
        let t1 = self.ctx.types.get(expected).clone();
        let t2 = self.ctx.types.get(actual).clone();
        
        if expected == actual {
            return Ok(());
        }
        
        match (t1, t2) {
            (SemanticType::InferenceVar(_), _) => {
                // Simplistic unification: bind var to type (not actually updating context yet)
                Ok(())
            }
            (_, SemanticType::InferenceVar(_)) => {
                Ok(())
            }
            _ => Err(format!("Type mismatch")),
        }
    }

    pub fn typecheck_items(&mut self, items: &[Item]) {
        for item in items {
            self.typecheck_item(item);
        }
    }

    fn typecheck_item(&mut self, item: &Item) {
        match item {
            Item::Decl(decl_id) => {
                let decl = &self.arena.decls[decl_id.0 as usize];
                match decl {
                    Decl::Function { body, .. } => {
                        if let Some(body_stmt) = body {
                            self.typecheck_stmt(body_stmt);
                        }
                    }
                    Decl::Var { initializer, .. } => {
                        if let Some(init) = initializer {
                            self.typecheck_expr(init);
                        }
                    }
                    Decl::Extern { func, .. } => {
                        let item = Item::Decl(*func);
                        self.typecheck_item(&item);
                    }
                    _ => {}
                }
            }
            Item::Stmt(stmt_id) => {
                self.typecheck_stmt(stmt_id);
            }
        }
    }

    fn typecheck_stmt(&mut self, stmt_id: &mellis_ast::StmtId) {
        let stmt = &self.arena.stmts[stmt_id.0 as usize];
        match stmt {
            Stmt::Block { body, tail_expr } => {
                self.typecheck_items(body);
                if let Some(expr) = tail_expr {
                    self.typecheck_expr(expr);
                }
            }
            Stmt::Expr { expr, .. } => {
                self.typecheck_expr(expr);
            }
            Stmt::If { condition, then_branch, else_branch } => {
                let cond_ty = self.typecheck_expr(condition);
                self.typecheck_stmt(then_branch);
                if let Some(else_br) = else_branch {
                    self.typecheck_stmt(else_br);
                }
                // Verify cond_ty is bool
            }
            Stmt::While { condition, body, .. } => {
                self.typecheck_expr(condition);
                self.typecheck_stmt(body);
            }
            Stmt::For { init, cond, step, body, iterable, .. } => {
                if let Some(item) = init { self.typecheck_item(item); }
                if let Some(c) = cond { self.typecheck_expr(c); }
                if let Some(s) = step { self.typecheck_expr(s); }
                if let Some(iter) = iterable { self.typecheck_expr(iter); }
                self.typecheck_stmt(body);
            }
            Stmt::Return { value } => {
                if let Some(val) = value {
                    self.typecheck_expr(val);
                }
            }
            _ => {}
        }
    }

    fn typecheck_expr(&mut self, expr_id: &mellis_ast::ExprId) -> SemanticTypeId {
        let expr = &self.arena.exprs[expr_id.0 as usize];
        let ty_id = match expr {
            Expr::Literal(tok) => {
                // Determine type based on literal token type
                let kind = match tok.kind {
                    TokenKind::IntegerLiteral | TokenKind::FloatLiteral => SemanticType::Primitive(BuiltinType::Int), // Simplified
                    TokenKind::StringLiteral => SemanticType::Primitive(BuiltinType::String),
                    TokenKind::KwTrue | TokenKind::KwFalse => SemanticType::Primitive(BuiltinType::Bool),
                    _ => SemanticType::Error,
                };
                self.ctx.types.intern(kind)
            }
            Expr::Identifier { .. } => {
                if let Some(sym_id) = self.ctx.tables.expr_symbols.get(expr_id) {
                    // For now, if we don't have the symbol's type, return inference var
                    self.ctx.types.new_inference_var()
                } else {
                    self.ctx.types.intern(SemanticType::Error)
                }
            }
            Expr::Binary { left, right, .. } => {
                let l_ty = self.typecheck_expr(left);
                let r_ty = self.typecheck_expr(right);
                
                // For simplified logic: require left and right to be same, return left type
                let _ = self.unify(l_ty, r_ty); 
                l_ty 
            }
            Expr::Call { callee, args, .. } => {
                self.typecheck_expr(callee);
                for arg in args {
                    self.typecheck_expr(&arg.value);
                }
                // Return an inference variable as the return type for now
                self.ctx.types.new_inference_var()
            }
            Expr::Assign { lvalue, value, .. } => {
                let l_ty = self.typecheck_expr(lvalue);
                let r_ty = self.typecheck_expr(value);
                let _ = self.unify(l_ty, r_ty);
                self.ctx.types.intern(SemanticType::Void)
            }
            _ => self.ctx.types.intern(SemanticType::Error)
        };
        
        self.ctx.tables.expr_types.insert(*expr_id, ty_id);
        ty_id
    }
}
