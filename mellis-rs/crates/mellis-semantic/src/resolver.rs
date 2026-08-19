use crate::{SemanticContext, ScopeId, SymbolKind};
use mellis_ast::{AstArena, Item, Stmt, Expr, Decl, Pattern, StructField};

pub struct Resolver<'a> {
    ctx: &'a mut SemanticContext,
    arena: &'a AstArena,
    current_scope: ScopeId,
}

impl<'a> Resolver<'a> {
    pub fn new(ctx: &'a mut SemanticContext, arena: &'a AstArena) -> Self {
        // Assume global scope is 0
        let global_scope = ScopeId(0);
        Self {
            ctx,
            arena,
            current_scope: global_scope,
        }
    }
    
    pub fn enter_scope(&mut self, kind: crate::symbol::ScopeKind) -> ScopeId {
        let new_scope = self.ctx.symbol_table.create_scope(kind, Some(self.current_scope));
        self.current_scope = new_scope;
        new_scope
    }
    
    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.ctx.symbol_table.scopes[self.current_scope.0 as usize].parent {
            self.current_scope = parent;
        }
    }

    pub fn resolve_items(&mut self, items: &[Item]) {
        for item in items {
            self.resolve_item(item);
        }
    }

    fn resolve_item(&mut self, item: &Item) {
        match item {
            Item::Decl(decl_id) => {
                let decl = &self.arena.decls[decl_id.0 as usize];
                match decl {
                    Decl::Function { name, params, body, visibility, .. } => {
                        // We do not have name text directly (Span only). 
                        // In a real compiler we get text from SourceManager. 
                        // For placeholder:
                        let name_str = format!("func_{}_{}", name.start, name.end);
                        
                        let sym_id = self.ctx.symbol_table.declare_symbol(
                            name_str,
                            SymbolKind::Function,
                            self.current_scope,
                            *name,
                            Some(*decl_id),
                            *visibility,
                        );
                        self.ctx.tables.decl_symbols.insert(*decl_id, sym_id);
                        
                        self.enter_scope(crate::symbol::ScopeKind::Function);
                        
                        for param_id in params {
                            if let Decl::Param { name: p_name, visibility: p_vis, .. } = &self.arena.decls[param_id.0 as usize] {
                                let p_name_str = format!("param_{}_{}", p_name.start, p_name.end);
                                let p_sym_id = self.ctx.symbol_table.declare_symbol(
                                    p_name_str,
                                    SymbolKind::Variable,
                                    self.current_scope,
                                    *p_name,
                                    Some(*param_id),
                                    *p_vis,
                                );
                                self.ctx.tables.decl_symbols.insert(*param_id, p_sym_id);
                            }
                        }
                        
                        if let Some(body_stmt) = body {
                            self.resolve_stmt(body_stmt);
                        }
                        
                        self.exit_scope();
                    }
                    Decl::Var { name, initializer, visibility, is_mutable, pattern, .. } => {
                        if let Some(init) = initializer {
                            self.resolve_expr(init);
                        }
                        
                        // We need to resolve the pattern or use the fallback name.
                        let name_str = format!("var_{}_{}", name.start, name.end);
                        let sym_id = self.ctx.symbol_table.declare_symbol(
                            name_str,
                            if *is_mutable { SymbolKind::Variable } else { SymbolKind::Constant },
                            self.current_scope,
                            *name,
                            Some(*decl_id),
                            *visibility,
                        );
                        self.ctx.tables.decl_symbols.insert(*decl_id, sym_id);
                    }
                    Decl::Struct { name, fields, visibility, .. } => {
                        let name_str = format!("struct_{}_{}", name.start, name.end);
                        let sym_id = self.ctx.symbol_table.declare_symbol(
                            name_str,
                            SymbolKind::Struct,
                            self.current_scope,
                            *name,
                            Some(*decl_id),
                            *visibility,
                        );
                        self.ctx.tables.decl_symbols.insert(*decl_id, sym_id);
                    }
                    Decl::Extern { func, visibility, .. } => {
                        let item = Item::Decl(*func);
                        self.resolve_item(&item);
                    }
                    _ => {}
                }
            }
            Item::Stmt(stmt_id) => {
                self.resolve_stmt(stmt_id);
            }
        }
    }
    
    fn resolve_stmt(&mut self, stmt_id: &mellis_ast::StmtId) {
        let stmt = &self.arena.stmts[stmt_id.0 as usize];
        match stmt {
            Stmt::Block { body, tail_expr } => {
                self.enter_scope(crate::symbol::ScopeKind::Block);
                self.resolve_items(body);
                if let Some(expr) = tail_expr {
                    self.resolve_expr(expr);
                }
                self.exit_scope();
            }
            Stmt::Expr { expr, .. } => {
                self.resolve_expr(expr);
            }
            Stmt::If { condition, then_branch, else_branch } => {
                self.resolve_expr(condition);
                self.resolve_stmt(then_branch);
                if let Some(else_br) = else_branch {
                    self.resolve_stmt(else_br);
                }
            }
            Stmt::While { condition, body, .. } => {
                self.resolve_expr(condition);
                self.resolve_stmt(body);
            }
            Stmt::For { init, cond, step, body, iterable, .. } => {
                self.enter_scope(crate::symbol::ScopeKind::Block);
                if let Some(item) = init {
                    self.resolve_item(item);
                }
                if let Some(iter) = iterable {
                    self.resolve_expr(iter);
                }
                if let Some(c) = cond {
                    self.resolve_expr(c);
                }
                if let Some(s) = step {
                    self.resolve_expr(s);
                }
                self.resolve_stmt(body);
                self.exit_scope();
            }
            Stmt::Return { value } => {
                if let Some(val) = value {
                    self.resolve_expr(val);
                }
            }
            _ => {}
        }
    }
    
    fn resolve_expr(&mut self, expr_id: &mellis_ast::ExprId) {
        let expr = &self.arena.exprs[expr_id.0 as usize];
        match expr {
            Expr::Identifier { segments, .. } => {
                if let Some(first_seg) = segments.first() {
                    let name_str = format!("var_{}_{}", first_seg.start, first_seg.end);
                    if let Some(sym_id) = self.ctx.symbol_table.lookup(&name_str, self.current_scope) {
                        self.ctx.tables.expr_symbols.insert(*expr_id, sym_id);
                    } else {
                        // We use dummy names so lookups will fail for non-local standard names unless we use SourceManager
                    }
                }
            }
            Expr::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            Expr::Unary { operand, .. } => {
                self.resolve_expr(operand);
            }
            Expr::Call { callee, args, .. } => {
                self.resolve_expr(callee);
                for arg in args {
                    self.resolve_expr(&arg.value);
                }
            }
            Expr::Assign { lvalue, value, .. } => {
                self.resolve_expr(lvalue);
                self.resolve_expr(value);
            }
            _ => {}
        }
    }
}
