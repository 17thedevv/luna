use crate::{SemanticContext, semantic_tables::SemanticTypeId};
use mellis_ast::{AstArena, Item, Stmt, Expr, Decl};
use mellis_lexer::BuiltinKind;

pub struct TypeChecker<'a> {
    ctx: &'a mut SemanticContext,
    arena: &'a AstArena,
}

impl<'a> TypeChecker<'a> {
    pub fn new(ctx: &'a mut SemanticContext, arena: &'a AstArena) -> Self {
        Self { ctx, arena }
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
                // Dummy logic for now
                SemanticTypeId(0) 
            }
            Expr::Identifier { .. } => {
                if let Some(sym_id) = self.ctx.tables.expr_symbols.get(expr_id) {
                    // Get type of symbol from symbol table or decl definitions
                    SemanticTypeId(0)
                } else {
                    SemanticTypeId(0)
                }
            }
            Expr::Binary { left, right, .. } => {
                let l_ty = self.typecheck_expr(left);
                let r_ty = self.typecheck_expr(right);
                l_ty // simplified
            }
            Expr::Call { callee, args, .. } => {
                self.typecheck_expr(callee);
                for arg in args {
                    self.typecheck_expr(&arg.value);
                }
                SemanticTypeId(0) // return type of callee
            }
            Expr::Assign { lvalue, value, .. } => {
                self.typecheck_expr(lvalue);
                self.typecheck_expr(value);
                SemanticTypeId(0) // void/unit
            }
            _ => SemanticTypeId(0)
        };
        
        self.ctx.tables.expr_types.insert(*expr_id, ty_id);
        ty_id
    }
}
