use crate::mvir::*;
use luna_ast::{AstArena, Item, Expr, Stmt, Decl, AssignOp};
use luna_semantic::SemanticContext;
use std::collections::HashMap;

pub struct MvirGenerator<'a> {
    arena: &'a AstArena,
    ctx: &'a SemanticContext,
    source_manager: &'a luna_common::source::SourceManager,
    module: Module,
    
    // State during function generation
    current_function: Option<Function>,
    current_block: Option<BasicBlock>,
    next_label_id: u32,
    
    // Track local variables to their Alloca ValueId
    locals: HashMap<luna_common::ids::SymbolId, ValueId>,
    known_function_values: HashMap<luna_common::ids::SymbolId, GlobalId>,
    lexical_scopes: Vec<Vec<luna_common::ids::SymbolId>>,
    temporary_drop_scopes: Vec<Vec<(ValueId, luna_semantic::SemanticTypeId)>>,
    loop_scopes: Vec<usize>,
    loop_break_targets: Vec<LabelId>,
    loop_continue_targets: Vec<LabelId>,
    current_span: Option<luna_common::Span>,
    current_async_future: Option<ValueId>,
    unsafe_depth: usize,
    pub diagnostics: Vec<luna_common::Diagnostic>,
    pub is_comptime: bool,
    pub root_file_id: Option<luna_common::ids::FileId>,
    
    // The current monomorphic instance being generated
    current_instance: Option<*const luna_semantic::mono::InstantiatedFunction>,
}

impl<'a> MvirGenerator<'a> {

    pub fn get_span_text(&self, span: luna_common::ids::Span) -> &'a str {
        &self.source_manager.get_file(span.file_id).unwrap().source[span.start as usize..span.end as usize]
    }

    pub fn new(arena: &'a AstArena, ctx: &'a SemanticContext, source_manager: &'a luna_common::source::SourceManager) -> Self {
        Self {
            arena,
            ctx,
            source_manager,
            module: Module::new(),
            current_function: None,
            current_block: None,
            next_label_id: 0,
            locals: HashMap::new(),
            known_function_values: HashMap::new(),
            lexical_scopes: Vec::new(),
            temporary_drop_scopes: Vec::new(),
            loop_scopes: Vec::new(),
            loop_break_targets: Vec::new(),
            loop_continue_targets: Vec::new(),
            current_span: None,
            current_async_future: None,
            unsafe_depth: 0,
            diagnostics: Vec::new(),
            is_comptime: false,
            root_file_id: None,
            current_instance: None,
        }
    }

    pub fn generate(mut self, items: &[Item]) -> (Module, Vec<luna_common::Diagnostic>) {
        if self.root_file_id.is_none() {
            for item in items {
                if let Item::Decl(decl_id) = item {
                    if let Some(decl) = self.arena.decls.get(decl_id.0 as usize) {
                        if let Decl::Function { name, .. } = decl {
                            self.root_file_id = Some(name.file_id);
                            break;
                        }
                    }
                }
            }
        }
        for instance in &self.ctx.instantiated_functions {
            self.generate_mono_instance(instance);
        }
        for glue_identity in &self.ctx.drop_glue_instances {
            self.generate_drop_glue(glue_identity);
        }
        (self.module, self.diagnostics)
    }

    pub fn generate_function_by_decl_id(&mut self, decl_id: luna_ast::DeclId) {
        let instance = luna_semantic::mono::InstantiatedFunction {
            instance: luna_semantic::mono::MonoInstance {
                decl_id,
                subst: Vec::new(),
                closure_id: None,
                defaults: None,
            },
            expr_types: std::collections::HashMap::new(),
            ast_types: std::collections::HashMap::new(),
            symbol_types: std::collections::HashMap::new(),
            pat_types: std::collections::HashMap::new(),
            mono_calls: std::collections::HashMap::new(),
            try_calls: std::collections::HashMap::new(),
            mono_for_loops: std::collections::HashMap::new(),
            closure_capture_bindings: Vec::new(),
            closure_env_type: None,
            closure_env_ptr_type: None,
        };
        self.generate_mono_instance(&instance);
    }
    
    fn get_symbol_type(&self, sym_id: &luna_common::ids::SymbolId) -> Option<luna_semantic::SemanticTypeId> {
        if let Some(inst_ptr) = self.current_instance {
            let inst = unsafe { &*inst_ptr };
            if let Some(&ty) = inst.symbol_types.get(sym_id) {
                return Some(ty);
            }
        }
        self.ctx.comptime_root.as_ref().and_then(|root| root.symbol_types.get(sym_id)).copied()
            .or_else(|| self.ctx.tables.symbol_types.get(sym_id).copied())
    }
    
    fn get_pat_type(&self, pat_id: &luna_ast::PatId) -> Option<luna_semantic::SemanticTypeId> {
        if let Some(inst_ptr) = self.current_instance {
            let inst = unsafe { &*inst_ptr };
            if let Some(&ty) = inst.pat_types.get(pat_id) {
                return Some(ty);
            }
        }
        self.ctx.comptime_root.as_ref().and_then(|root| root.pat_types.get(pat_id)).copied()
            .or_else(|| self.ctx.tables.pat_types.get(pat_id).copied())
    }

    fn get_expr_type(&self, expr_id: &luna_ast::ExprId) -> luna_semantic::SemanticTypeId {
        if let Some(inst_ptr) = self.current_instance {
            let inst = unsafe { &*inst_ptr };
            if let Some(&mono_ty) = inst.expr_types.get(expr_id) {
                return mono_ty;
            }
        }
        self.ctx.comptime_root.as_ref().and_then(|root| root.expr_types.get(expr_id)).copied()
            .or_else(|| self.ctx.tables.expr_types.get(expr_id).copied())
            .unwrap_or(luna_semantic::SemanticTypeId(0))
    }

    fn get_try_calls(&self, expr_id: &luna_ast::ExprId) -> Option<&luna_semantic::mono::MonoTryCalls> {
        self.current_instance.and_then(|inst_ptr| {
            // SAFETY: current_instance points into the caller-owned instantiated_functions
            // vector for the duration of this function's lowering.
            unsafe { (&*inst_ptr).try_calls.get(expr_id) }
        }).or_else(|| self.ctx.comptime_root.as_ref().and_then(|root| root.try_calls.get(expr_id)))
    }

    fn get_mono_call(&self, expr_id: &luna_ast::ExprId) -> Option<&luna_semantic::mono::MonoInstance> {
        if let Some(inst_ptr) = self.current_instance {
            // SAFETY: this function is called only while the caller-owned unit
            // remains live during its lowering.
            return unsafe { (&*inst_ptr).mono_calls.get(expr_id) };
        }
        if let Some(root) = self.ctx.comptime_root.as_ref() {
            return root.mono_calls.get(expr_id);
        }
        self.ctx.instantiated_functions.iter().find_map(|function| function.mono_calls.get(expr_id))
    }

    /// Resolve the canonical monomorphized name for a function call at `expr_id`.
    /// Returns the properly mangled name if the call has a mono substitution, otherwise None.
    fn resolve_mono_call_name(&self, expr_id: &luna_ast::ExprId, base_name: &str) -> Option<String> {
        let mono_instance = self.get_mono_call(expr_id);
        if let Some(mi) = mono_instance {
            let canonical_id = mi.canonical_identity();
            return Some(canonical_id.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, base_name));
        }
        None
    }

    fn mono_instance_global(
        &self,
        method_sym: luna_common::ids::SymbolId,
        instance: &luna_semantic::mono::MonoInstance,
    ) -> GlobalId {
        let base_name = self.ctx.symbol_table.get_symbol(method_sym).name.clone();
        let canonical_id = instance.canonical_identity();
        GlobalId {
            name: canonical_id.symbol_name_with_tables(
                &self.ctx.types,
                &self.ctx.symbol_table,
                &self.ctx.tables,
                &base_name,
            ),
            symbol_id: if instance.defaults.is_some() { None } else { Some(method_sym) },
        }
    }

    fn call_symbol_id(&self, expression: &luna_ast::ExprId, symbol: luna_common::ids::SymbolId) -> Option<luna_common::ids::SymbolId> {
        // A synthesized entry has different input ordinals. Its ordinary
        // body-derived effects must not inherit the full-arity declaration's facts.
        if self.get_mono_call(expression).is_some_and(|instance| instance.defaults.is_some()) { None }
        else { Some(symbol) }
    }

    fn default_contract(&self, instance: &luna_semantic::mono::MonoInstance)
        -> Option<(luna_semantic::CanonicalLifetimeContract, bool, Vec<usize>)> {
        let defaults = instance.defaults.as_ref()?;
        let Decl::Function { params, .. } = &self.arena.decls[defaults.declaration.0 as usize] else { return None; };
        let symbol = self.ctx.tables.decl_symbols[&defaults.declaration];
        let contract = self.ctx.tables.fn_lifetime_contracts.get(&symbol).cloned().unwrap_or_default();
        let receiver = params.first().is_some_and(|p| matches!(self.arena.decls[p.0 as usize], Decl::Param { is_self: true, .. }));
        let supplied = (0..params.len()).filter(|p| !defaults.omitted.contains(&(*p as u32))).collect();
        Some((contract, receiver, supplied))
    }

    fn contract_ordinal(subject: luna_semantic::CanonicalContractSubject, receiver: bool) -> usize {
        match subject {
            luna_semantic::CanonicalContractSubject::SelfVal => 0,
            luna_semantic::CanonicalContractSubject::Param(index) => index as usize + usize::from(receiver),
        }
    }

    fn annotate_default_call(&mut self, expression: luna_ast::ExprId, value: ValueId) {
        let Some(instance) = self.get_mono_call(&expression) else { return; };
        let Some((contract, receiver, supplied)) = self.default_contract(instance) else { return; };
        let function = self.current_function.as_mut().expect("call in function");
        let Instruction::CallDirect { args, .. } = &function.values[value.0 as usize].inst else { return; };
        let mut checks = Vec::new();
        for constraint in contract.outlives_constraints {
            let longer = Self::contract_ordinal(constraint.longer, receiver);
            let shorter = Self::contract_ordinal(constraint.shorter, receiver);
            if let (Some(longer), Some(shorter)) = (supplied.iter().position(|p| *p == longer), supplied.iter().position(|p| *p == shorter)) {
                checks.push(OutlivesCheck { longer_subject: constraint.longer, shorter_subject: constraint.shorter,
                    longer: args[longer].clone(), shorter: args[shorter].clone() });
            }
        }
        if !checks.is_empty() { function.lifetime_info.checks.insert(value, checks); }
    }

    fn annotate_default_entry(&mut self, instance: &luna_semantic::mono::MonoInstance, parameters: &[luna_ast::DeclId]) {
        let Some((contract, receiver, supplied)) = self.default_contract(instance) else { return; };
        self.current_function.as_mut().unwrap().lifetime_info.infer_input_requirements = true;
        let mut checks = Vec::new();
        let mut last = None;
        for constraint in contract.outlives_constraints {
            let longer = Self::contract_ordinal(constraint.longer, receiver);
            let shorter = Self::contract_ordinal(constraint.shorter, receiver);
            if let (Some(a), Some(b)) = (supplied.iter().position(|p| *p == longer), supplied.iter().position(|p| *p == shorter)) {
                self.current_function.as_mut().unwrap().lifetime_info.input_assumptions.push((a as u16, b as u16));
                continue;
            }
            let mut operands = Vec::new();
            for ordinal in [longer, shorter] {
                let symbol = self.ctx.tables.decl_symbols[&parameters[ordinal]];
                let ty = self.get_symbol_type(&symbol).expect("checked contract parameter type");
                let loaded = self.push_inst(Instruction::Load { ptr: Operand::Value(self.locals[&symbol]) }, ty);
                last = Some(loaded);
                operands.push(Operand::Value(loaded));
            }
            checks.push(OutlivesCheck { longer_subject: constraint.longer, shorter_subject: constraint.shorter,
                longer: operands[0].clone(), shorter: operands[1].clone() });
        }
        if let Some(last) = last { self.current_function.as_mut().unwrap().lifetime_info.checks.insert(last, checks); }
    }

    fn resolve_ast_type(&self, type_id: &luna_ast::TypeId) -> luna_semantic::SemanticTypeId {
        if let Some(instance) = self.current_instance {
            if let Some(&ty) = unsafe { &*instance }.ast_types.get(type_id) { return ty; }
        }
        if let Some(&ty) = self.ctx.comptime_root.as_ref().and_then(|root| root.ast_types.get(type_id)) { return ty; }
        if let Some(&ty) = self.ctx.tables.ast_type_to_semantic.get(type_id) {
            if ty != luna_semantic::SemanticTypeId(0) {
                if let Some(inst_ptr) = self.current_instance {
                    let inst = unsafe { &*inst_ptr };
                    if let luna_semantic::SemanticType::GenericParam(sym) = self.ctx.types.get(ty) {
                        if let Some(&new_ty) = inst.instance.subst.iter().find(|(symbol, _)| symbol == sym).map(|(_, ty)| ty)
                            .or_else(|| inst.instance.defaults.as_ref()
                                .and_then(|defaults| defaults.subst.iter().find(|(symbol, _)| symbol == sym).map(|(_, ty)| ty))) {
                            return new_ty;
                        }
                    }
                }
                return ty;
            }
        }
        
        if (type_id.0 as usize) < self.arena.types.len() {
            match &self.arena.types[type_id.0 as usize] {
                luna_ast::Type::Builtin(bk) => {
                    use luna_lexer::BuiltinKind;
                    use luna_semantic::ty::BuiltinType;
                    let b_ty = match bk {
                        BuiltinKind::I8 => BuiltinType::I8,
                        BuiltinKind::U8 => BuiltinType::U8,
                        BuiltinKind::I16 => BuiltinType::I16,
                        BuiltinKind::U16 => BuiltinType::U16,
                        BuiltinKind::I32 => BuiltinType::I32,
                        BuiltinKind::U32 => BuiltinType::U32,
                        BuiltinKind::I64 => BuiltinType::I64,
                        BuiltinKind::U64 => BuiltinType::U64,
                        BuiltinKind::I128 => BuiltinType::I128,
                        BuiltinKind::U128 => BuiltinType::U128,
                        BuiltinKind::Isize => BuiltinType::Isize,
                        BuiltinKind::Usize => return self.ctx.types.usize_id(),
                        BuiltinKind::F32 => BuiltinType::F32,
                        BuiltinKind::F64 => BuiltinType::F64,
                        BuiltinKind::Bool => return self.ctx.types.bool_id(),
                        BuiltinKind::Char => BuiltinType::Char,
                        BuiltinKind::Str => BuiltinType::String,
                        BuiltinKind::Void => return luna_semantic::SemanticTypeId(0),
                    };
                    for id in self.ctx.types.get_all_types() {
                        if let luna_semantic::SemanticType::Primitive(p) = self.ctx.types.get(id) {
                            if *p == b_ty {
                                return id;
                            }
                        }
                    }
                }
                luna_ast::Type::Pointer { is_mutable, inner } => {
                    let inner_sem = self.resolve_ast_type(inner);
                    let is_rw = if *is_mutable { luna_semantic::ty::Mutability::Mutable } else { luna_semantic::ty::Mutability::Immutable };
                    for id in self.ctx.types.get_all_types() {
                        if let luna_semantic::SemanticType::Pointer(m, elem) = self.ctx.types.get(id) {
                            if *m == is_rw && *elem == inner_sem {
                                return id;
                            }
                        }
                    }
                }
                luna_ast::Type::Reference { is_mutable, inner, .. } => {
                    let inner_sem = self.resolve_ast_type(inner);
                    let is_rw = if *is_mutable { luna_semantic::ty::Mutability::Mutable } else { luna_semantic::ty::Mutability::Immutable };
                    for id in self.ctx.types.get_all_types() {
                        if let luna_semantic::SemanticType::Reference(_, m, elem) = self.ctx.types.get(id) {
                            if *m == is_rw && *elem == inner_sem {
                                return id;
                            }
                        }
                    }
                }
                luna_ast::Type::Named { .. } => {
                    if let Some(symbol) = self.ctx.tables.type_symbols.get(type_id) {
                        if let Some(inst_ptr) = self.current_instance {
                            let inst = unsafe { &*inst_ptr };
                            if let Some(&ty) = inst.instance.subst.iter().find(|(binder, _)| binder == symbol).map(|(_, ty)| ty)
                                .or_else(|| inst.instance.defaults.as_ref().and_then(|defaults|
                                    defaults.subst.iter().find(|(binder, _)| binder == symbol).map(|(_, ty)| ty))) {
                                return ty;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        luna_semantic::SemanticTypeId(0)
    }

    fn find_pointer_type(&self, elem_ty: luna_semantic::SemanticTypeId) -> luna_semantic::SemanticTypeId {
        for id in self.ctx.types.get_all_types() {
            if let luna_semantic::SemanticType::Pointer(_, elem) = self.ctx.types.get(id) {
                if *elem == elem_ty {
                    return id;
                }
            }
        }
        for id in self.ctx.types.get_all_types() {
            if let luna_semantic::SemanticType::Reference(_, _, elem) = self.ctx.types.get(id) {
                if *elem == elem_ty {
                    return id;
                }
            }
        }
        luna_semantic::SemanticTypeId(0)
    }

    fn find_u64_type(&self) -> luna_semantic::SemanticTypeId {
        for id in self.ctx.types.get_all_types() {
            if let luna_semantic::SemanticType::Primitive(luna_semantic::ty::BuiltinType::U64) = self.ctx.types.get(id) {
                return id;
            }
        }
        self.ctx.types.usize_id()
    }

    pub fn generate_all_known_functions(&mut self) {
        for instance in &self.ctx.instantiated_functions {
            self.generate_mono_instance(instance);
        }
        for (decl_id_idx, decl) in self.arena.decls.iter().enumerate() {
            if let luna_ast::Decl::Function { generic_params, body: Some(_), .. } = decl {
                if generic_params.is_empty() {
                    let decl_id = luna_ast::DeclId(decl_id_idx as u32);
                    let sym_id_opt = self.ctx.tables.decl_symbols.get(&decl_id).copied();
                    if let Some(s) = sym_id_opt {
                        if (s.0 as usize) < self.ctx.symbol_table.symbols.len() {
                            let sym = &self.ctx.symbol_table.symbols[s.0 as usize];
                            // External non-generic functions are already compiled into their own .obj sidecar.
                            if sym.provider_id.is_some() && sym.provider_id != self.ctx.current_provider {
                                continue;
                            }
                        }
                    }
                    let fn_name = sym_id_opt.map(|s| {
                        if (s.0 as usize) < self.ctx.symbol_table.symbols.len() {
                            self.ctx.symbol_table.symbols[s.0 as usize].name.clone()
                        } else {
                            String::new()
                        }
                    }).unwrap_or_default();
                    if !fn_name.is_empty() && !self.module.functions.iter().any(|f| f.name.name == fn_name) {
                        self.generate_function_by_decl_id(decl_id);
                    }
                }
            }
        }
        for glue_identity in &self.ctx.drop_glue_instances {
            self.generate_drop_glue(glue_identity);
        }
    }

    /// Comptime may run before unrelated bodies are typechecked. Lower only
    /// the call graph reachable from this evaluation, not the whole session.
    pub fn generate_comptime_dependencies(&mut self, root: &Function) {
        let mut pending = vec![root.clone()];
        let mut visited = std::collections::HashSet::new();
        while let Some(function) = pending.pop() {
            for value in &function.values {
                let mut callees = Vec::new();
                match &value.inst {
                    Instruction::CallDirect { callee, args } => {
                        callees.push(callee.clone());
                        for arg in args {
                            if let Operand::Global(global) = arg { callees.push(global.clone()); }
                        }
                    }
                    Instruction::Assign(Operand::Global(global))
                    | Instruction::CallIndirect { callee: Operand::Global(global), .. } => {
                        callees.push(global.clone());
                    }
                    Instruction::Drop { callee: Some(global), ty, .. } => {
                        // Early evaluation runs before the ordinary whole-program
                        // drop discovery pass. Materialize only reachable glue.
                        if crate::drop_glue_global_id(self.ctx, *ty).is_some_and(|expected| expected.name == global.name) {
                            let concrete_ty = self.ctx.types.resolve(*ty);
                            let struct_sym = match self.ctx.types.get(concrete_ty) {
                                luna_semantic::SemanticType::Struct(symbol, ..) | luna_semantic::SemanticType::Enum(symbol, ..) => *symbol,
                                _ => luna_common::ids::SymbolId(0),
                            };
                            self.generate_drop_glue(&luna_semantic::CanonicalInstanceIdentity {
                                kind: luna_semantic::CanonicalInstanceKind::DropGlue { struct_sym, concrete_ty },
                                subst: Vec::new(),
                            });
                        }
                        callees.push(global.clone());
                    }
                    Instruction::MakeTraitObject { trait_sym, concrete_sym, .. } => {
                        let key = luna_semantic::semantic_tables::ImplKey {
                            trait_id: Some(*trait_sym),
                            self_type_def: (*concrete_sym).into(),
                        };
                        let methods = self.ctx.tables.impl_methods.get(&key).into_iter().flatten()
                            .copied().chain(self.ctx.tables.drop_impls.get(concrete_sym).copied());
                        for symbol in methods {
                            if let Some(&decl) = self.ctx.tables.symbol_decls.get(&symbol) {
                                let identity = luna_semantic::CanonicalInstanceIdentity {
                                    kind: luna_semantic::CanonicalInstanceKind::Decl(decl),
                                    subst: Vec::new(),
                                };
                                callees.push(GlobalId {
                                    name: identity.symbol_name_with_tables(&self.ctx.types,
                                        &self.ctx.symbol_table, &self.ctx.tables,
                                        &self.ctx.symbol_table.get_symbol(symbol).name),
                                    symbol_id: Some(symbol),
                                });
                            }
                        }
                    }
                    _ => {}
                }
                for callee in callees {
                    if !visited.insert(callee.name.clone()) { continue; }
                    if let Some(existing) = self.module.functions.iter().find(|f| f.name.name == callee.name).cloned() {
                        pending.push(existing);
                        continue;
                    }
                    let instance = self.ctx.instantiated_functions.iter().find(|instance| {
                        let symbol = self.ctx.tables.decl_symbols.get(&instance.instance.decl_id);
                        symbol.is_some_and(|symbol| {
                            let name = &self.ctx.symbol_table.get_symbol(*symbol).name;
                            let identity = instance.instance.canonical_identity();
                            identity.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, name) == callee.name
                        })
                    }).cloned();
                    let before = self.module.functions.len();
                    if let Some(instance) = instance {
                        self.generate_mono_instance(&instance);
                    } else if let Some(symbol) = callee.symbol_id {
                        if let Some(decl) = self.ctx.symbol_table.get_symbol(symbol).decl_id {
                            if matches!(&self.arena.decls[decl.0 as usize], Decl::Function { generic_params, body: Some(_), .. } if generic_params.is_empty()) {
                                self.generate_function_by_decl_id(decl);
                            }
                        }
                    }
                    pending.extend(self.module.functions[before..].iter().cloned());
                }
            }
        }
    }

    fn get_drop_glue_global_id(&self, ty_id: luna_semantic::SemanticTypeId) -> Option<GlobalId> {
        crate::drop_glue_global_id(self.ctx, ty_id)
    }

    fn generate_drop_glue(&mut self, identity: &luna_semantic::CanonicalInstanceIdentity) {
        if let luna_semantic::CanonicalInstanceKind::ClosureDropGlue { env_ty } = identity.kind {
            self.generate_closure_drop_glue(identity, env_ty);
            return;
        }
        let (struct_sym, concrete_ty) = match identity.kind {
            luna_semantic::CanonicalInstanceKind::DropGlue { struct_sym, concrete_ty } => (struct_sym, concrete_ty),
            _ => return,
        };
        
        let nominal_name = if struct_sym.0 == 0 {
            if matches!(self.ctx.types.get(concrete_ty), luna_semantic::SemanticType::Array(_, _)) { "array" } else { "tuple" }.to_string()
        } else if (struct_sym.0 as usize) < self.ctx.symbol_table.symbols.len() {
            self.ctx.symbol_table.symbols[struct_sym.0 as usize].name.clone()
        } else {
            format!("type{}", struct_sym.0)
        };
        let glue_fn_name = identity.symbol_name(&self.ctx.types, &self.ctx.symbol_table, &nominal_name);
        
        if self.module.functions.iter().any(|f| f.name.name == glue_fn_name) {
            return;
        }
        
        let ptr_ty = self.find_pointer_type(concrete_ty);
        
        let global_id = GlobalId {
            name: glue_fn_name.clone(),
            symbol_id: if struct_sym.0 == 0 { None } else { Some(struct_sym) },
        };
        
        let func = Function {
            name: global_id,
            is_extern: false,
            is_async: false,
            arg_count: 1,
            link_name: None,
            param_types: vec![ptr_ty],
            ret_ty: luna_semantic::SemanticTypeId(0),
            lifetime_info: Default::default(),
            blocks: Vec::new(),
            values: Vec::new(),
        };
        
        self.current_function = Some(func);
        self.current_block = None;
        self.locals.clear();
        self.known_function_values.clear();
        self.lexical_scopes.clear();
        self.temporary_drop_scopes.clear();
        self.loop_scopes.clear();
        self.loop_break_targets.clear();
        self.loop_continue_targets.clear();
        self.push_scope();
        self.next_label_id = 0;
        
        let entry_label = self.new_label("entry");
        self.start_block(entry_label);
        
        let alloc_val = self.push_inst(Instruction::Alloca, ptr_ty);
        let obj_ptr = self.push_inst(Instruction::Load { ptr: Operand::Value(alloc_val) }, ptr_ty);
        
        // Step 1: Call user Drop::drop if present
        let drop_meth_sym_opt = self.ctx.tables.drop_impls.get(&struct_sym).copied().or_else(|| {
            let drop_sym = self.ctx.lang_items.get(luna_semantic::lang_item::LangItem::Drop).or_else(|| {
                self.ctx.symbol_table.symbols.iter().position(|s| s.name == "Drop").map(|idx| luna_common::ids::SymbolId(idx as u32))
            })?;
            let key = luna_semantic::semantic_tables::ImplKey {
                trait_id: Some(drop_sym),
                self_type_def: struct_sym.into(),
            };
            self.ctx.tables.impl_methods.get(&key).and_then(|m| m.first().copied())
        });
        if let Some(drop_meth_sym) = drop_meth_sym_opt {
            let mut user_drop_fn_name = None;
            for inst_fn in &self.ctx.instantiated_functions {
                if let Some(&m_sym) = self.ctx.tables.decl_symbols.get(&inst_fn.instance.decl_id) {
                    if m_sym == drop_meth_sym && inst_fn.instance.subst == identity.subst {
                        let fn_base_name = self.ctx.symbol_table.symbols[m_sym.0 as usize].name.clone();
                        let canonical_id = inst_fn.instance.canonical_identity();
                        user_drop_fn_name = Some(canonical_id.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, &fn_base_name));
                        break;
                    }
                }
            }
            if user_drop_fn_name.is_none() {
                let fn_base_name = self.ctx.symbol_table.symbols[drop_meth_sym.0 as usize].name.clone();
                let canonical_id = luna_semantic::CanonicalInstanceIdentity {
                    kind: luna_semantic::CanonicalInstanceKind::Decl(self.ctx.tables.symbol_decls.get(&drop_meth_sym).copied().unwrap_or(luna_ast::DeclId(0))),
                    subst: identity.subst.clone(),
                };
                user_drop_fn_name = Some(canonical_id.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, &fn_base_name));
            }
            
            if let Some(fn_name) = user_drop_fn_name {
                self.push_inst(Instruction::CallDirect {
                    callee: GlobalId { name: fn_name, symbol_id: Some(drop_meth_sym) },
                    args: vec![Operand::Value(obj_ptr)],
                }, luna_semantic::SemanticTypeId(0));
            }
        }
        
        // Step 2: Drop owned fields in reverse declaration order (LIFO)
        if let luna_semantic::SemanticType::Struct(_, _, ref fields) = self.ctx.types.get(concrete_ty).clone() {
            for (idx, &field_ty) in fields.iter().enumerate().rev() {
                if self.ctx.needs_drop(field_ty) {
                    let field_ptr_ty = self.find_pointer_type(field_ty);
                    let field_ptr = self.push_inst(Instruction::FieldPtr {
                        base: Operand::Value(obj_ptr),
                        field_idx: idx as u32,
                        field_name: None,
                    }, field_ptr_ty);
                    
                    let field_callee = self.get_drop_glue_global_id(field_ty);
                    self.push_inst(Instruction::Drop {
                        value: Operand::Value(field_ptr),
                        ty: field_ty,
                        callee: field_callee,
                    }, field_ty);
                }
            }
        } else if let luna_semantic::SemanticType::Tuple(ref elems) = self.ctx.types.get(concrete_ty).clone() {
            for (idx, &elem_ty) in elems.iter().enumerate().rev() {
                if self.ctx.needs_drop(elem_ty) {
                    let elem_ptr_ty = self.find_pointer_type(elem_ty);
                    let elem_ptr = self.push_inst(Instruction::FieldPtr {
                        base: Operand::Value(obj_ptr),
                        field_idx: idx as u32,
                        field_name: None,
                    }, elem_ptr_ty);
                    
                    let elem_callee = self.get_drop_glue_global_id(elem_ty);
                    self.push_inst(Instruction::Drop {
                        value: Operand::Value(elem_ptr),
                        ty: elem_ty,
                        callee: elem_callee,
                    }, elem_ty);
                }
            }
        } else if let luna_semantic::SemanticType::Array(elem_ty, len) = self.ctx.types.get(concrete_ty).clone() {
            let usize_ty = self.ctx.types.usize_id();
            let remaining = self.push_inst(Instruction::Alloca, usize_ty);
            let length = self.push_inst(Instruction::Assign(Operand::Number(len.to_string())), usize_ty);
            let zero = self.push_inst(Instruction::Assign(Operand::Number("0".into())), usize_ty);
            let one = self.push_inst(Instruction::Assign(Operand::Number("1".into())), usize_ty);
            self.push_inst(Instruction::Store {
                ptr: Operand::Value(remaining), value: Operand::Value(length),
            }, usize_ty);
            let elem_ptr_ty = self.find_pointer_type(elem_ty);
            let elements = self.push_inst(Instruction::Cast { value: Operand::Value(obj_ptr), target_ty: elem_ptr_ty }, elem_ptr_ty);
            let condition = self.new_label("array_drop_condition");
            let body = self.new_label("array_drop_element");
            let done = self.new_label("array_drop_done");
            self.terminate_block(Terminator::Br { target: condition.clone() });
            self.start_block(condition.clone());
            let count = self.push_inst(Instruction::Load { ptr: Operand::Value(remaining) }, usize_ty);
            let empty = self.push_inst(Instruction::Eq {
                left: Operand::Value(count), right: Operand::Value(zero),
            }, self.ctx.types.bool_id());
            self.terminate_block(Terminator::CondBr {
                condition: Operand::Value(empty), true_target: done.clone(), false_target: body.clone(),
            });
            self.start_block(body);
            let index = self.push_inst(Instruction::Sub {
                left: Operand::Value(count), right: Operand::Value(one),
            }, usize_ty);
            self.push_inst(Instruction::Store {
                ptr: Operand::Value(remaining), value: Operand::Value(index),
            }, usize_ty);
            let element = self.push_inst(Instruction::Add {
                left: Operand::Value(elements), right: Operand::Value(index),
            }, elem_ptr_ty);
            let callee = self.get_drop_glue_global_id(elem_ty);
            self.push_inst(Instruction::Drop { value: Operand::Value(element), callee, ty: elem_ty }, elem_ty);
            self.terminate_block(Terminator::Br { target: condition });
            self.start_block(done);
        } else if let luna_semantic::SemanticType::Enum(_, _, variants) = self.ctx.types.get(concrete_ty).clone() {
            let tag_ty = self.ctx.types.u32_id();
            let tag_ptr = self.push_inst(Instruction::FieldPtr {
                base: Operand::Value(obj_ptr), field_idx: 0, field_name: None,
            }, tag_ty);
            let tag = self.push_inst(Instruction::Load { ptr: Operand::Value(tag_ptr) }, tag_ty);
            let done = self.new_label("enum_drop_done");
            for (index, payload_ty) in variants.into_iter().enumerate() {
                if !self.ctx.needs_drop(payload_ty) { continue; }
                let selected = self.new_label("enum_drop_payload");
                let next = self.new_label("enum_drop_next");
                let is_variant = self.push_inst(Instruction::Eq {
                    left: Operand::Value(tag), right: Operand::Number(index.to_string()),
                }, self.ctx.types.bool_id());
                self.terminate_block(Terminator::CondBr {
                    condition: Operand::Value(is_variant), true_target: selected.clone(), false_target: next.clone(),
                });
                self.start_block(selected);
                let payload = self.push_inst(Instruction::FieldPtr {
                    base: Operand::Value(obj_ptr), field_idx: 1, field_name: None,
                }, self.find_pointer_type(payload_ty));
                let callee = self.get_drop_glue_global_id(payload_ty);
                self.push_inst(Instruction::Drop { value: Operand::Value(payload), callee, ty: payload_ty }, payload_ty);
                self.terminate_block(Terminator::Br { target: done.clone() });
                self.start_block(next);
            }
            self.terminate_block(Terminator::Br { target: done.clone() });
            self.start_block(done);
        }

        self.terminate_block(Terminator::Ret { value: None });
        if let Some(block) = self.current_block.take() {
            self.current_function.as_mut().unwrap().blocks.push(block);
        }
        if let Some(func) = self.current_function.take() {
            self.module.functions.push(func);
        }
    }

    /// Generate `<closure-drop glue>(closure_ptr)`: load the environment pointer
    /// from the `{ code, env }` closure record, drop the owned captures the
    /// environment still holds, then release the environment allocation.
    fn generate_closure_drop_glue(&mut self, identity: &luna_semantic::CanonicalInstanceIdentity, env_ty: luna_semantic::SemanticTypeId) {
        let glue_fn_name = identity.symbol_name(&self.ctx.types, &self.ctx.symbol_table, "closure");
        if self.module.functions.iter().any(|f| f.name.name == glue_fn_name) {
            return;
        }
        let Some(ptr_ty) = self.ctx.closure_value_ptr_ty else { return; };
        let func = Function {
            name: GlobalId { name: glue_fn_name.clone(), symbol_id: None },
            is_extern: false,
            is_async: false,
            arg_count: 1,
            link_name: None,
            param_types: vec![ptr_ty],
            ret_ty: luna_semantic::SemanticTypeId(0),
            lifetime_info: Default::default(),
            blocks: Vec::new(),
            values: Vec::new(),
        };
        self.current_function = Some(func);
        self.current_block = None;
        self.locals.clear();
        self.known_function_values.clear();
        self.lexical_scopes.clear();
        self.temporary_drop_scopes.clear();
        self.loop_scopes.clear();
        self.loop_break_targets.clear();
        self.loop_continue_targets.clear();
        self.push_scope();
        self.next_label_id = 0;
        let entry_label = self.new_label("entry");
        self.start_block(entry_label);

        let alloc_val = self.push_inst(Instruction::Alloca, ptr_ty);
        let obj_ptr = self.push_inst(Instruction::Load { ptr: Operand::Value(alloc_val) }, ptr_ty);
        // The environment pointer is field 1 of the { code, env } closure record.
        let env_field_ty = self.find_pointer_type(env_ty);
        let env_field = self.push_inst(Instruction::FieldPtr {
            base: Operand::Value(obj_ptr),
            field_idx: 1,
            field_name: None,
        }, env_field_ty);
        let env_ptr = self.push_inst(Instruction::Load { ptr: Operand::Value(env_field) }, env_field_ty);

        if let luna_semantic::SemanticType::Tuple(fields) = self.ctx.types.get(env_ty).clone() {
            for (idx, field_ty) in fields.iter().enumerate().rev() {
                if self.ctx.needs_drop(*field_ty) {
                    let field_ptr_ty = self.find_pointer_type(*field_ty);
                    let field_ptr = self.push_inst(Instruction::FieldPtr {
                        base: Operand::Value(env_ptr),
                        field_idx: idx as u32,
                        field_name: None,
                    }, field_ptr_ty);
                    let callee = self.get_drop_glue_global_id(*field_ty);
                    self.push_inst(Instruction::Drop { value: Operand::Value(field_ptr), ty: *field_ty, callee }, *field_ty);
                }
            }
        }
        self.push_inst(Instruction::HeapFree { value: Operand::Value(env_ptr) }, luna_semantic::SemanticTypeId(0));

        self.terminate_block(Terminator::Ret { value: None });
        if let Some(block) = self.current_block.take() {
            self.current_function.as_mut().unwrap().blocks.push(block);
        }
        if let Some(func) = self.current_function.take() {
            self.module.functions.push(func);
        }
    }

    pub fn current_module(&self) -> &Module {
        &self.module
    }
    pub fn generate_expr_as_function(&mut self, expr_id: &luna_ast::ExprId, ret_ty: luna_semantic::SemanticTypeId) -> Function {
        let global_id = GlobalId {
            name: format!("__comptime_eval_{}", expr_id.0),
            symbol_id: None,
        };
        self.current_function = Some(Function {
            name: global_id,
            is_extern: false,
            is_async: false,
            arg_count: 0,
            link_name: None,
            param_types: Vec::new(),
            ret_ty,
            lifetime_info: Default::default(),
            blocks: Vec::new(),
            values: Vec::new(),
        });
        self.start_block(LabelId { name: "entry".to_string() });
        self.push_scope();
        let val_op = self.transfer_return_value(expr_id);
        self.pop_scope_and_drop(None);
        self.terminate_block(Terminator::Ret { value: Some(val_op) });
        if let Some(block) = self.current_block.take() {
            self.current_function.as_mut().unwrap().blocks.push(block);
        }
        self.current_function.take().unwrap()
    }

    pub fn generate_stmt_as_function(&mut self, stmt_id: &luna_ast::StmtId, ret_ty: luna_semantic::SemanticTypeId) -> Function {
        let global_id = GlobalId {
            name: format!("__comptime_eval_stmt_{}", stmt_id.0),
            symbol_id: None,
        };
        self.current_function = Some(Function {
            name: global_id,
            is_extern: false,
            is_async: false,
            arg_count: 0,
            link_name: None,
            param_types: Vec::new(),
            ret_ty,
            lifetime_info: Default::default(),
            blocks: Vec::new(),
            values: Vec::new(),
        });
        self.start_block(LabelId { name: "entry".to_string() });
        self.push_scope();
        let ret_val = self.generate_block_expr(stmt_id);
        let ret_val = if matches!(self.ctx.types.get(ret_ty), luna_semantic::SemanticType::Void | luna_semantic::SemanticType::Never) {
            ret_val
        } else {
            let span = match &ret_val {
                Operand::Value(value) => self.current_function.as_ref().unwrap().values[value.0 as usize].span,
                _ => None,
            };
            let transferred = self.push_inst_span(Instruction::Assign(ret_val), ret_ty, span);
            Operand::Value(transferred)
        };
        self.pop_scope_and_drop(None);
        if let Some(mut block) = self.current_block.take() {
            if block.terminator.is_none() {
                block.terminator = Some(Terminator::Ret { value: Some(ret_val) });
            }
            self.current_function.as_mut().unwrap().blocks.push(block);
        }
        self.current_function.take().unwrap()
    }

    fn push_scope(&mut self) {
        self.lexical_scopes.push(Vec::new());
        self.temporary_drop_scopes.push(Vec::new());
    }

    fn pop_scope_and_drop(&mut self, keep: Option<luna_common::ids::SymbolId>) {
        if let Some(temporaries) = self.temporary_drop_scopes.pop() {
            self.emit_temporary_drops(&temporaries);
        }
        if let Some(scope) = self.lexical_scopes.pop() {
            self.emit_drops_for_scope(&scope, keep);
        }
    }

    fn emit_temporary_drops(
        &mut self,
        temporaries: &[(ValueId, luna_semantic::SemanticTypeId)],
    ) {
        for &(value, ty) in temporaries.iter().rev() {
            if self.ctx.needs_drop(ty) {
                self.emit_place_cleanup(Operand::Value(value), ty, None);
            }
        }
    }

    fn emit_drops_for_scope(&mut self, scope: &[luna_common::ids::SymbolId], keep: Option<luna_common::ids::SymbolId>) {
        for &sym in scope.iter().rev() {
            if Some(sym) == keep { continue; }
            if let Some(&val_id) = self.locals.get(&sym) {
                let mut ty_id = self.get_symbol_type(&sym).unwrap_or(luna_semantic::SemanticTypeId(0));
                if ty_id == luna_semantic::SemanticTypeId(0) {
                    if let Some(func) = &self.current_function {
                        ty_id = func.values[val_id.0 as usize].ty;
                    }
                }
                if self.ctx.needs_drop(ty_id) {
                    self.emit_place_cleanup(Operand::Value(val_id), ty_id, None);
                }
            }
        }
    }

    // Aggregates without a user destructor can be partially moved. Keep their
    // remaining fields visible to move analysis instead of deleting the entire
    // aggregate destructor when one field has transferred ownership.
    fn emit_place_cleanup(&mut self, place: Operand, ty: luna_semantic::SemanticTypeId, span: Option<luna_common::Span>) {
        let ty = self.ctx.types.resolve(ty);
        if !self.ctx.needs_drop(ty) { return; }
        if let luna_semantic::SemanticType::Closure(..) = self.ctx.types.get(ty) {
            // Closure destruction is an ordinary whole-place Drop backed by a
            // closure-environment drop glue. Never read the closure place here:
            // it may have been consumed (moved) by a call.
            let callee = self.get_drop_glue_global_id(ty);
            self.push_inst_span(Instruction::Drop { value: place, callee, ty }, ty, span);
            return;
        }
        if let luna_semantic::SemanticType::Array(elem_ty, len) = self.ctx.types.get(ty).clone() {
            // Descend per element so a partially moved array drops only its
            // remaining initialized elements: each element is a distinct place
            // that move analysis can eliminate when moved out. The whole-array
            // drop glue is retained for dropping an entire array value.
            if self.ctx.needs_drop(elem_ty) {
                let elem_ptr_ty = self.find_pointer_type(elem_ty);
                for index in 0..len {
                    let elem_ptr = self.push_inst_span(Instruction::FieldPtr {
                        base: place.clone(),
                        field_idx: index as u32,
                        field_name: None,
                    }, elem_ptr_ty, span);
                    self.emit_place_cleanup(Operand::Value(elem_ptr), elem_ty, span);
                }
            }
            return;
        }
        let fields = match self.ctx.types.get(ty).clone() {
            luna_semantic::SemanticType::Struct(symbol, _, fields)
                if !self.ctx.tables.drop_impls.contains_key(&symbol) => Some(fields),
            luna_semantic::SemanticType::Tuple(fields) => Some(fields),
            _ => None,
        };
        if let Some(fields) = fields {
            for (index, field_ty) in fields.into_iter().enumerate().rev() {
                if !self.ctx.needs_drop(field_ty) { continue; }
                let pointer_ty = self.find_pointer_type(field_ty);
                let field = self.push_inst_span(Instruction::FieldPtr {
                    base: place.clone(), field_idx: index as u32, field_name: None,
                }, pointer_ty, span);
                self.emit_place_cleanup(Operand::Value(field), field_ty, span);
            }
        } else {
            let callee = self.get_drop_glue_global_id(ty);
            self.push_inst_span(Instruction::Drop { value: place, callee, ty }, ty, span);
        }
    }

    fn emit_drops_up_to(&mut self, target_depth: usize, keep: Option<luna_common::ids::SymbolId>) {
        for i in (target_depth..self.lexical_scopes.len()).rev() {
            let temporaries = self.temporary_drop_scopes[i].clone();
            self.emit_temporary_drops(&temporaries);
            let scope = self.lexical_scopes[i].clone();
            self.emit_drops_for_scope(&scope, keep);
        }
    }

    fn generate_item(&mut self, item: &Item) {
        match item {
            Item::Decl(decl_id) => {
                let decl = &self.arena.decls[decl_id.0 as usize];
                match decl {
                    Decl::Var { name, initializer, pattern, is_mutable, .. } => {
                        let init_op = initializer.as_ref().map(|init_expr| self.generate_expr(init_expr));
                        if !*is_mutable {
                            if let (Some(pattern), Some(Operand::Global(function))) = (pattern, &init_op) {
                                if matches!(self.arena.pats.get(pattern.0 as usize), Some(luna_ast::Pattern::Identifier { .. })) {
                                    if let Some(&symbol) = self.ctx.tables.pat_symbols.get(pattern) {
                                        if self.get_symbol_type(&symbol).is_some_and(|ty|
                                            matches!(self.ctx.types.get(ty), luna_semantic::SemanticType::Function { .. })) {
                                            self.known_function_values.insert(symbol, function.clone());
                                        }
                                    }
                                }
                            }
                        }
                        
                        let prev_span = self.current_span.clone();
                        self.current_span = Some(*name);
                        
                        if let Some(pat_id) = pattern {
                            self.bind_pattern(pat_id, init_op);
                        }
                        
                        self.current_span = prev_span;
                    }
                    _ => unreachable!("ICE: Unhandled variant, should be impossible after semantic invariants") // Functions are handled by generate_mono_instance
                }
            }
            Item::Stmt(stmt_id) => {
                self.generate_stmt(stmt_id);
            }
        }
    }

    fn bind_pattern(&mut self, pat_id: &luna_ast::PatId, val_op: Option<Operand>) {
        match &self.arena.pats[pat_id.0 as usize] {
            luna_ast::Pattern::Identifier { .. } => {
                if let Some(sym_id) = self.ctx.tables.pat_symbols.get(pat_id).copied() {
                    let mut ty_id = self.get_symbol_type(&sym_id).unwrap_or(luna_semantic::SemanticTypeId(0));
                    if ty_id == luna_semantic::SemanticTypeId(0) {
                        if let Some(Operand::Value(v)) = &val_op {
                            ty_id = self.current_function.as_ref().unwrap().values[v.0 as usize].ty;
                        }
                    }
                    let alloca_val = self.push_inst(Instruction::Alloca, ty_id);
                    self.locals.insert(sym_id, alloca_val);
                    if let Some(scope) = self.lexical_scopes.last_mut() {
                        scope.push(sym_id);
                    }
                    
                    if let Some(val) = val_op {
                        self.push_inst(Instruction::Store {
                            ptr: Operand::Value(alloca_val),
                            value: val,
                        }, ty_id);
                    }
                }
            }
            luna_ast::Pattern::Struct { fields, .. } => {
                let ty_id = self.ctx.tables.pat_types.get(pat_id).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                let resolved_ty = self.ctx.types.get(ty_id).clone();
                if let luna_semantic::SemanticType::Struct(sym_id, _, _) = resolved_ty {
                    if let Some(decl_id) = self.ctx.symbol_table.get_symbol(sym_id).decl_id {
                        let decl = self.arena.decls[decl_id.0 as usize].clone();
                        if let luna_ast::Decl::Struct { fields: struct_fields, .. } = &decl {
                            for field in fields {
                                if let Some(field_pat) = field.pattern {
                                    let field_name_str = self.get_span_text(field.name);
                                    let mut field_idx = 0;
                                    for (idx, struct_field) in struct_fields.iter().enumerate() {
                                        let struct_field_name = self.get_span_text(struct_field.name);
                                        if field_name_str == struct_field_name {
                                            field_idx = idx;
                                            break;
                                        }
                                    }
                                    let field_ty = self.ctx.tables.pat_types.get(&field_pat).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                                    let extract_op = val_op.as_ref().map(|v| {
                                        let extract_val = self.push_inst(Instruction::Extract {
                                            value: v.clone(),
                                            variant_idx: 0,
                                            field_idx: field_idx as u32,
                                        }, field_ty);
                                        Operand::Value(extract_val)
                                    });
                                    self.bind_pattern(&field_pat, extract_op);
                                }
                            }
                        }
                    }
                }
            }
            luna_ast::Pattern::Tuple { elements, .. } => {
                for (idx, elem) in elements.iter().enumerate() {
                    let field_ty = self.ctx.tables.pat_types.get(elem).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                    let extract_op = val_op.as_ref().map(|v| {
                        let extract_val = self.push_inst(Instruction::Extract {
                            value: v.clone(),
                            variant_idx: 0,
                            field_idx: idx as u32,
                        }, field_ty);
                        Operand::Value(extract_val)
                    });
                    self.bind_pattern(elem, extract_op);
                }
            }
            luna_ast::Pattern::Enum { fields: elements, .. } => {
                let variant_idx = self.resolved_pattern_variant_index(pat_id)
                    .unwrap_or_else(|| panic!("MVIR invariant: enum pattern has no resolved enum variant symbol"));
                
                for (field_idx, elem) in elements.iter().enumerate() {
                    let field_ty = self.get_pat_type(elem).unwrap_or(luna_semantic::SemanticTypeId(0));
                    let extract_op = val_op.as_ref().map(|v| {
                        let extract_val = self.push_inst(Instruction::Extract {
                            value: v.clone(),
                            variant_idx,
                            field_idx: field_idx as u32,
                        }, field_ty);
                        Operand::Value(extract_val)
                    });
                    self.bind_pattern(elem, extract_op);
                }
            }
            _ => unreachable!("ICE: Unhandled variant, should be impossible after semantic invariants")
        }
    }

    // Unmanaged raw storage writes initialize a cell without invoking its
    // previous destructor (PTR-MEM-3). Safe/local places use overwrite cleanup.
    fn is_raw_storage_lvalue(&self, expr_id: luna_ast::ExprId) -> bool {
        match &self.arena.exprs[expr_id.0 as usize] {
            Expr::Unary { op: luna_ast::expr::UnaryOp::Deref | luna_ast::expr::UnaryOp::DerefMut, operand } => {
                matches!(self.ctx.types.get(self.ctx.types.resolve(self.get_expr_type(operand))), luna_semantic::SemanticType::Pointer(..))
            }
            Expr::Member { object, .. } | Expr::TupleIndex { object, .. } => {
                self.is_raw_storage_lvalue(*object)
                    || matches!(self.ctx.types.get(self.ctx.types.resolve(self.get_expr_type(object))), luna_semantic::SemanticType::Pointer(..))
            }
            Expr::Index { base, .. } => {
                self.is_raw_storage_lvalue(*base)
                    || matches!(self.ctx.types.get(self.ctx.types.resolve(self.get_expr_type(base))), luna_semantic::SemanticType::Pointer(..))
            }
            _ => false,
        }
    }

    fn canonical_constant_symbol(&self, sym: luna_common::ids::SymbolId) -> luna_common::ids::SymbolId {
        // Comptime reconstruction can register a declaration in a new symbol
        // context. References in an already checked body still identify the
        // same declaration; do not resolve that identity by its short name.
        self.ctx.symbol_table.get_symbol(sym).decl_id
            .and_then(|decl| self.ctx.tables.decl_symbols.get(&decl).copied())
            .filter(|canonical| self.ctx.const_values.contains_key(canonical))
            .unwrap_or(sym)
    }

    fn generate_lvalue(&mut self, expr_id: &luna_ast::ExprId) -> Operand {
        let expr = &self.arena.exprs[expr_id.0 as usize];
        
        match expr {
            Expr::Identifier { .. } => {
                if let Some(sym_id) = self.ctx.tables.expr_symbols.get(expr_id).copied() {
                    if let Some(&val_id) = self.locals.get(&sym_id) {
                        return Operand::Value(val_id);
                    }
                    let sym_id = self.canonical_constant_symbol(sym_id);
                    if let Some(value) = self.ctx.const_values.get(&sym_id).cloned() {
                        // Array-size comptime evaluation can lower a constant
                        // identifier before that expression has a type-table
                        // entry. Storage has the checked declaration's type.
                        let ty = self.get_symbol_type(&sym_id)
                            .unwrap_or_else(|| self.get_expr_type(expr_id));
                        let symbol = self.ctx.symbol_table.get_symbol(sym_id);
                        let scope = &self.ctx.symbol_table.scopes[symbol.scope.0 as usize];
                        if matches!(scope.kind, luna_semantic::symbol::ScopeKind::Global | luna_semantic::symbol::ScopeKind::Module) {
                            let data = (|| {
                                use crate::static_data::{StaticData, StaticType, StaticValue};
                                let provider = if let Some(pid) = symbol.provider_id {
                                    self.ctx.provider_lookup.iter().filter(|(_, id)| **id == pid)
                                        .map(|(name, _)| name.clone()).min()
                                        .ok_or("module constant lacks canonical provider identity")?
                                } else {
                                    self.ctx.current_provider_name.clone().unwrap_or_else(|| "<entry>".into())
                                };
                                let mut name = "__luna_const_".to_string();
                                for part in std::iter::once(provider).chain(self.ctx.symbol_table.get_full_logical_path(sym_id)) {
                                    name.push_str(&format!("{}_{}", part.len(), part));
                                }
                                Ok::<_, String>(StaticData {
                                    name,
                                    ty: StaticType::from_semantic(self.ctx, ty)?,
                                    value: StaticValue::from_comptime(self.ctx, ty, &value)?,
                                })
                            })();
                            return match data {
                                Ok(data) => Operand::Value(self.push_inst(Instruction::StaticAddress(data), ty)),
                                Err(message) => {
                                    self.diagnostics.push(luna_common::Diagnostic::error(message)
                                        .with_code(luna_common::DiagnosticCode::BackendInvariantViolation)
                                        .with_span(symbol.span));
                                    Operand::Value(self.push_inst(Instruction::Null { ty }, ty))
                                }
                            };
                        }
                        let value = self.materialize_comptime_value(&value, ty);
                        let storage = self.push_inst(Instruction::Alloca, ty);
                        self.push_inst(Instruction::Store { ptr: Operand::Value(storage), value }, ty);
                        return Operand::Value(storage);
                    }
                    let sym_name = if (sym_id.0 as usize) < self.ctx.symbol_table.symbols.len() {
                        self.ctx.symbol_table.symbols[sym_id.0 as usize].name.clone()
                    } else {
                        format!("global_{}", sym_id.0)
                    };
                    return Operand::Global(GlobalId {
                        name: sym_name,
                        symbol_id: Some(sym_id),
                    });
                }
                Operand::Number("0".to_string())
            }
            Expr::Member { object, member } => {
                let mut base_op = self.generate_lvalue(object);
                let mut obj_ty_id = self.get_expr_type(object);
                if obj_ty_id == luna_semantic::SemanticTypeId(0) {
                    if let Some(sym) = self.ctx.tables.expr_symbols.get(object) {
                        if let Some(ty) = self.get_symbol_type(sym) {
                            obj_ty_id = ty;
                        }
                    }
                }
                let mut struct_sym_opt = None;
                let mut struct_field_tys_opt = None;
                let mut is_slice = false;
                match self.ctx.types.get(obj_ty_id) {
                    luna_semantic::SemanticType::Struct(sym_id, _, field_tys) => {
                        struct_sym_opt = Some(*sym_id);
                        struct_field_tys_opt = Some(field_tys.clone());
                    }
                    luna_semantic::SemanticType::Reference(_, _, inner) | luna_semantic::SemanticType::Pointer(_, inner) => {
                        if let luna_semantic::SemanticType::Slice(_) = self.ctx.types.get(*inner) {
                            // Slices are fat pointers represented as structs stored directly in the local alloca.
                            // Do not load, so base_op remains a pointer to the fat pointer struct!
                            is_slice = true;
                        } else {
                            let load_val = self.push_inst(Instruction::Load { ptr: base_op }, obj_ty_id);
                            base_op = Operand::Value(load_val);
                            if let luna_semantic::SemanticType::Struct(sym_id, _, field_tys) = self.ctx.types.get(*inner) {
                                struct_sym_opt = Some(*sym_id);
                                struct_field_tys_opt = Some(field_tys.clone());
                            }
                        }
                    }
                    luna_semantic::SemanticType::Slice(_) => {
                        is_slice = true;
                    }
                    _ => {}
                }
                let mut field_idx = 0;
                
                if let Some(sym_id) = struct_sym_opt {
                    let sym = self.ctx.symbol_table.get_symbol(sym_id);
                    if let Some(decl_id) = sym.decl_id {
                        if let luna_ast::Decl::Struct { fields, .. } = &self.arena.decls[decl_id.0 as usize] {
                            let member_name = self.get_span_text(*member).to_string();
                            for (i, f) in fields.iter().enumerate() {
                                let f_name = self.get_span_text(f.name).to_string();
                                if f_name == member_name {
                                    field_idx = i as u32;
                                    break;
                                }
                            }
                        }
                    }
                } else if is_slice {
                    let member_name = self.get_span_text(*member);
                    if member_name == "length" || member_name == "len" {
                        field_idx = 1;
                    } else if member_name == "ptr" || member_name == "data" {
                        field_idx = 0;
                    }
                } else if let Some(&idx) = self.ctx.tables.expr_member_indices.get(expr_id) {
                    field_idx = idx;
                }
                
                let mut field_ty = self.get_expr_type(expr_id);
                if field_ty == luna_semantic::SemanticTypeId(0) {
                    if let Some(field_tys) = &struct_field_tys_opt {
                        if let Some(&fty) = field_tys.get(field_idx as usize) {
                            field_ty = fty;
                        }
                    }
                }
                let field_ptr_val = self.push_inst(Instruction::FieldPtr {
                    base: base_op,
                    field_idx,
                    field_name: struct_sym_opt.map(|_| self.get_span_text(*member).to_string()),
                }, field_ty);
                Operand::Value(field_ptr_val)
            }
            Expr::TupleIndex { object, index } => {
                let mut base_op = self.generate_lvalue(object);
                let obj_ty_id = self.get_expr_type(object);
                let mut tuple_field_tys_opt = None;
                match self.ctx.types.get(obj_ty_id) {
                    luna_semantic::SemanticType::Tuple(types) => {
                        tuple_field_tys_opt = Some(types.clone());
                    }
                    luna_semantic::SemanticType::Reference(_, _, inner) | luna_semantic::SemanticType::Pointer(_, inner) => {
                        let load_val = self.push_inst(Instruction::Load { ptr: base_op }, obj_ty_id);
                        base_op = Operand::Value(load_val);
                        if let luna_semantic::SemanticType::Tuple(types) = self.ctx.types.get(*inner) {
                            tuple_field_tys_opt = Some(types.clone());
                        }
                    }
                    _ => {}
                }
                let mut field_ty = self.get_expr_type(expr_id);
                if field_ty == luna_semantic::SemanticTypeId(0) {
                    if let Some(types) = &tuple_field_tys_opt {
                        if let Some(&fty) = types.get(*index as usize) {
                            field_ty = fty;
                        }
                    }
                }
                let field_ptr_val = self.push_inst(Instruction::FieldPtr {
                    base: base_op,
                    field_idx: *index,
                    field_name: None,
                }, field_ty);
                Operand::Value(field_ptr_val)
            }
            Expr::Unary { op: luna_ast::expr::UnaryOp::Deref, operand } |
            Expr::Unary { op: luna_ast::expr::UnaryOp::DerefMut, operand } => {
                self.generate_expr(operand)
            }
            Expr::Index { base, index } => {
                let index_op = self.generate_expr(index);
                let base_ty_id = self.get_expr_type(base);
                let base_ty = self.ctx.types.get(base_ty_id).clone();

                let mut peeled_base_ty = base_ty.clone();
                while let luna_semantic::SemanticType::Reference(_, _, inner) | luna_semantic::SemanticType::Pointer(_, inner) = peeled_base_ty {
                    peeled_base_ty = self.ctx.types.get(inner).clone();
                }

                match peeled_base_ty {
                    luna_semantic::SemanticType::Slice(elem_ty_id) => {
                        let base_ptr = match &self.arena.exprs[base.0 as usize] {
                            Expr::Identifier { .. } | Expr::Member { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                                self.generate_lvalue(base)
                            }
                            _ => {
                                let base_val = self.generate_expr(base);
                                let alloca = self.push_inst(Instruction::Alloca, base_ty_id);
                                self.push_inst(Instruction::Store {
                                    ptr: Operand::Value(alloca),
                                    value: base_val,
                                }, self.ctx.types.bool_id());
                                Operand::Value(alloca)
                            }
                        };

                        let ptr_ptr_ty = self.find_pointer_type(elem_ty_id);
                        let ptr_field = self.push_inst(Instruction::FieldPtr {
                            base: base_ptr.clone(),
                            field_idx: 0,
                            field_name: None,
                        }, ptr_ptr_ty);
                        let data_ptr = self.push_inst(Instruction::Load {
                            ptr: Operand::Value(ptr_field),
                        }, ptr_ptr_ty);

                        let u64_ty = self.find_u64_type();
                        let len_field = self.push_inst(Instruction::FieldPtr {
                            base: base_ptr,
                            field_idx: 1,
                            field_name: None,
                        }, u64_ty);
                        let len_val = self.push_inst(Instruction::Load {
                            ptr: Operand::Value(len_field),
                        }, u64_ty);

                        self.push_inst(Instruction::BoundsCheck {
                            index: index_op.clone(),
                            len: Operand::Value(len_val),
                        }, self.ctx.types.bool_id());

                        let elem_ptr = self.push_inst(Instruction::PtrOffset {
                            ptr: Operand::Value(data_ptr),
                            offset: index_op,
                        }, elem_ty_id);

                        Operand::Value(elem_ptr)
                    }
                    luna_semantic::SemanticType::Array(elem_ty_id, len) => {
                        self.push_inst(Instruction::BoundsCheck {
                            index: index_op.clone(),
                            len: Operand::Number(len.to_string()),
                        }, self.ctx.types.bool_id());

                        let mut base_lval = self.generate_lvalue(base);
                        // A reference binding stores an address to the array,
                        // not the array elements. Load each reference layer
                        // before applying an element offset to that storage.
                        let mut address_ty = base_ty_id;
                        while let luna_semantic::SemanticType::Reference(_, _, inner)
                            | luna_semantic::SemanticType::Pointer(_, inner) = self.ctx.types.get(address_ty) {
                            let inner = *inner;
                            base_lval = Operand::Value(self.push_inst(Instruction::Load { ptr: base_lval }, address_ty));
                            address_ty = inner;
                        }
                        // A constant index is emitted as a literal offset so
                        // move analysis can map it to the element subplace; a
                        // dynamic index stays an opaque operand. Both remain a
                        // raw pointer offset, leaving raw-slice provenance
                        // analysis unaffected.
                        let offset_operand = match &self.arena.exprs[index.0 as usize] {
                            Expr::Literal(tok, text) if tok.kind == luna_lexer::TokenKind::IntegerLiteral => {
                                match text.parse::<u32>() {
                                    Ok(const_index) => Operand::Number(const_index.to_string()),
                                    Err(_) => index_op.clone(),
                                }
                            }
                            _ => index_op.clone(),
                        };
                        let elem_ptr = self.push_inst(Instruction::PtrOffset {
                            ptr: base_lval,
                            offset: offset_operand,
                        }, elem_ty_id);
                        Operand::Value(elem_ptr)
                    }
                    _ => Operand::Number("0".to_string()),
                }
            }
            _ => {
                let mut ty_id = self.ctx.tables.expr_types.get(expr_id).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                if let Some(inst_ptr) = self.current_instance {
                    let inst = unsafe { &*inst_ptr };
                    if let Some(&mono_ty) = inst.expr_types.get(expr_id) {
                        ty_id = mono_ty;
                    }
                }
                let val_op = self.generate_expr(expr_id);
                let alloca = self.push_inst(Instruction::Alloca, ty_id);
                self.push_inst(Instruction::Store {
                    ptr: Operand::Value(alloca),
                    value: val_op,
                }, ty_id);
                Operand::Value(alloca)
            }
        }
    }

    fn generate_closure_mono_instance(&mut self, instance: &luna_semantic::mono::InstantiatedFunction, expr_id: luna_ast::ExprId) {
        let expr = &self.arena.exprs[expr_id.0 as usize];
        if let Expr::Lambda { body, params, return_type: _, is_move: _ } = expr {
            let mut fn_name = format!("closure_{}", expr_id.0);
            if !instance.instance.subst.is_empty() {
                let canonical_id = instance.instance.canonical_identity();
                fn_name = canonical_id.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, &fn_name);
            }
            let global_id = GlobalId {
                name: fn_name,
                symbol_id: None, // No symbol for anonymous closure
            };
            
            // The environment is passed as the first parameter to the closure (the environment pointer)
            let env_ty_id = instance.closure_env_ptr_type
                .or_else(|| self.ctx.tables.closure_env_ptr_types.get(&expr_id).copied())
                .unwrap_or(luna_semantic::SemanticTypeId(0));
            
            let mut ret_ty_id = luna_semantic::SemanticTypeId(0);
            if let Some(luna_semantic::SemanticType::Closure(_, _, ret)) = instance.expr_types.get(&expr_id).map(|id| self.ctx.types.get(*id)) {
                ret_ty_id = *ret;
            } else if let Some(luna_semantic::SemanticType::Closure(_, _, ret)) = self.ctx.tables.expr_types.get(&expr_id).map(|id| self.ctx.types.get(*id)) {
                ret_ty_id = *ret;
            }

            let mut param_types = Vec::new();
            if let Some(env_ptr_ty) = instance.closure_env_ptr_type {
                param_types.push(env_ptr_ty);
            } else {
                param_types.push(luna_semantic::SemanticTypeId(0));
            }
            for param_id in params {
                let mut param_ty = luna_semantic::SemanticTypeId(0);
                if let Some(sym_id) = self.ctx.tables.decl_symbols.get(param_id).copied() {
                    param_ty = self.get_symbol_type(&sym_id).unwrap_or(luna_semantic::SemanticTypeId(0));
                }
                param_types.push(param_ty);
            }
            
            self.current_function = Some(Function {
                name: global_id,
                is_extern: false,
                is_async: false,
                ret_ty: ret_ty_id,
                lifetime_info: Default::default(),
                arg_count: params.len() + 1, // environment is the extra argument (first)
                link_name: None,
                param_types,
                blocks: Vec::new(),
                values: Vec::new(),
            });
            self.locals.clear();
            self.known_function_values.clear();
            self.lexical_scopes.clear();
            self.temporary_drop_scopes.clear();
            self.loop_scopes.clear();
            self.loop_break_targets.clear();
            self.loop_continue_targets.clear();
            self.push_scope(); // Function root scope
            self.next_label_id = 0;
            
            let entry_label = self.new_label("entry");
            self.start_block(entry_label);
            
            // The first values must be allocas for the hidden environment and explicit parameters.
            let env_param = self.push_inst(Instruction::Alloca, env_ty_id);
            self.current_function.as_mut().unwrap().values[env_param.0 as usize].origin = ValueOrigin::Parameter(0);
            for (idx, param_id) in params.iter().enumerate() {
                if let Decl::Param { .. } = &self.arena.decls[param_id.0 as usize] {
                    if let Some(sym_id) = self.ctx.tables.decl_symbols.get(&param_id).copied() {
                        let ty_id = instance.symbol_types.get(&sym_id).copied()
                            .or_else(|| self.ctx.tables.symbol_types.get(&sym_id).copied())
                            .unwrap_or(luna_semantic::SemanticTypeId(0));
                        let alloc_val = self.push_inst(Instruction::Alloca, ty_id);
                        self.current_function.as_mut().unwrap().values[alloc_val.0 as usize].origin = ValueOrigin::Parameter((idx + 1) as u32);
                        self.locals.insert(sym_id, alloc_val);
                        self.lexical_scopes.last_mut().unwrap().push(sym_id);
                    }
                }
            }

            // Bind captured variables to environment fields after parameter allocas exist.
            let environment = self.push_inst(Instruction::Load { ptr: Operand::Value(env_param) }, env_ty_id);
            let closure_bindings = if instance.closure_capture_bindings.is_empty() {
                self.ctx.tables.expect_closure_capture_bindings(expr_id)
            } else {
                instance.closure_capture_bindings.clone()
            };
            if !closure_bindings.is_empty() {
                for binding in &closure_bindings {
                    let field_ptr = self.push_inst(Instruction::FieldPtr {
                        base: Operand::Value(environment),
                        field_idx: binding.env_field,
                        field_name: None,
                    }, binding.env_ty);
                    let val = match binding.mode {
                        luna_semantic::semantic_tables::CaptureMode::SharedBorrow |
                        luna_semantic::semantic_tables::CaptureMode::MutableBorrow => {
                            self.push_inst(Instruction::Load { ptr: Operand::Value(field_ptr) }, binding.env_ty)
                        }
                        luna_semantic::semantic_tables::CaptureMode::Move => {
                            // A consumed invocation owns its environment. Move each
                            // owned capture into an ordinary body local so the
                            // existing path-dependent whole-local cleanup destroys
                            // whatever is still initialized on every exit path.
                            let loaded = self.push_inst(Instruction::Load { ptr: Operand::Value(field_ptr) }, binding.ty);
                            let local = self.push_inst(Instruction::Alloca, binding.ty);
                            self.push_inst(Instruction::Store { ptr: Operand::Value(local), value: Operand::Value(loaded) }, binding.ty);
                            self.push_inst(Instruction::MarkInit { value: Operand::Value(local) }, binding.ty);
                            self.lexical_scopes.last_mut().unwrap().push(binding.symbol);
                            local
                        }
                    };
                    self.locals.insert(binding.symbol, val);
                }
            }
                        
            self.generate_fn_body(body, ret_ty_id);

            // A consuming invocation owns its environment: release the heap
            // environment allocation on every exit path, after the capture
            // cleanup above. Exactly one HeapFree runs per executed path.
            let consuming = closure_bindings.iter().any(|binding| {
                binding.mode == luna_semantic::semantic_tables::CaptureMode::Move && self.ctx.needs_drop(binding.ty)
            });
            if consuming {
                if let Some(mut func) = self.current_function.take() {
                    for block in func.blocks.iter_mut() {
                        if matches!(block.terminator, Some(Terminator::Ret { .. })) {
                            let id = crate::ValueId(func.values.len() as u32);
                            func.values.push(crate::ValueData {
                                inst: Instruction::HeapFree { value: Operand::Value(environment) },
                                ty: luna_semantic::SemanticTypeId(0),
                                span: None,
                                origin: ValueOrigin::Temporary,
                            });
                            block.insts.push(id);
                        }
                    }
                    self.current_function = Some(func);
                }
            }

        if let Some(func) = self.current_function.take() {
            self.module.functions.push(func);
        }
    }
}

    fn generate_mono_instance(&mut self, instance: &luna_semantic::mono::InstantiatedFunction) {
        // Monomorphization Barrier (P0-C / Rule 8):
        // Assert that all types inside the instantiated function are strictly monomorphic.
        // No GenericParam, InferenceVar, Error, or Unresolved Projection may enter MVIR lowering.
        for (&sym_id, &ty) in &instance.symbol_types {
            if !self.ctx.types.is_monomorphic(ty) {
                if matches!(self.ctx.types.get(ty), luna_semantic::SemanticType::Function { .. }) {
                    continue;
                }
                panic!("MVIR Monomorphization Barrier Violation: Symbol {:?} has non-monomorphic type {:?}", sym_id, self.ctx.types.get(ty));
            }
        }
        for (&expr_id, &ty) in &instance.expr_types {
            if !self.ctx.types.is_monomorphic(ty) {
                // Expressions referencing uncalled generic functions or enum types are exempted
                if !matches!(self.ctx.types.get(ty), luna_semantic::SemanticType::Function { .. } | luna_semantic::SemanticType::Enum(..)) {
                    panic!("MVIR Monomorphization Barrier Violation: Expr {:?} has non-monomorphic type {:?}", expr_id, self.ctx.types.get(ty));
                }
            }
        }

        self.current_instance = Some(instance as *const _);
        
        if let Some(closure_id) = instance.instance.closure_id {
            self.generate_closure_mono_instance(instance, closure_id);
            self.current_instance = None;
            return;
        }
        if (instance.instance.decl_id.0 as usize) >= self.arena.decls.len() {
            let sym_id_opt = self.ctx.tables.decl_symbols.get(&instance.instance.decl_id).copied();
            let mut fn_name = "func".to_string();
            let mut ret_ty_id = luna_semantic::SemanticTypeId(0);
            let mut param_types = Vec::new();
            if let Some(sym_id) = sym_id_opt {
                if (sym_id.0 as usize) < self.ctx.symbol_table.symbols.len() {
                    fn_name = self.ctx.symbol_table.symbols[sym_id.0 as usize].name.clone();
                }
                if let Some(fn_ty_id) = self.get_symbol_type(&sym_id) {
                    if let luna_semantic::SemanticType::Function { return_type, params, .. } = self.ctx.types.get(fn_ty_id) {
                        ret_ty_id = *return_type;
                        param_types = params.clone();
                    }
                }
            }
            let global_id = GlobalId {
                name: fn_name,
                symbol_id: sym_id_opt,
            };
            let func = Function {
                name: global_id,
                is_extern: true,
                is_async: false,
                arg_count: param_types.len(),
                link_name: None,
                param_types,
                ret_ty: ret_ty_id,
                lifetime_info: Default::default(),
                blocks: Vec::new(),
                values: Vec::new(),
            };
            self.module.functions.push(func);
            self.current_instance = None;
            return;
        }
        let decl = &self.arena.decls[instance.instance.decl_id.0 as usize];
        if let Decl::Function { body, .. } = decl {
            let sym_id_opt = self.ctx.tables.decl_symbols.get(&instance.instance.decl_id).copied();
            let mut fn_name = "func".to_string();
            let mut ret_ty_id = luna_semantic::SemanticTypeId(0);
            
            let mut resolved_param_types = None;
            let mut link_name = None;

            if let Decl::Function { annotations, .. } = decl {
                for annot in annotations {
                    let annot_name_str = if (annot.name.end as usize) <= self.source_manager.get_file(annot.name.file_id).unwrap().source.len() && annot.name.start <= annot.name.end {
                        self.get_span_text(annot.name)
                    } else { "" };

                    if annot_name_str == "link" {
                        for arg in &annot.args {
                            if let Some(key_span) = arg.key {
                                let key_str = if (key_span.end as usize) <= self.source_manager.get_file(key_span.file_id).unwrap().source.len() && key_span.start <= key_span.end {
                                    self.get_span_text(key_span)
                                } else { "" };

                                if key_str == "name" {
                                    if let luna_ast::Expr::Literal(_, val) = &self.arena.exprs[arg.value.0 as usize] {
                                        link_name = Some(val.trim_matches('"').to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(sym_id) = sym_id_opt {
                if (sym_id.0 as usize) < self.ctx.symbol_table.symbols.len() {
                    fn_name = self.ctx.symbol_table.symbols[sym_id.0 as usize].name.clone();
                }
                if let Some(fn_ty_id) = self.get_symbol_type(&sym_id) {
                    if let luna_semantic::SemanticType::Function { return_type, params, .. } = self.ctx.types.get(fn_ty_id) {
                        ret_ty_id = *return_type;
                        resolved_param_types = Some(params.clone());
                    }
                }
            }
            
            let canonical_id = instance.instance.canonical_identity();
            let name_str = canonical_id.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, &fn_name);
            let global_id = GlobalId {
                name: name_str,
                symbol_id: if instance.instance.defaults.is_some() { None } else { sym_id_opt },
            };
            
            let supplied_ordinals: Vec<_> = if let Decl::Function { params, .. } = decl {
                (0..params.len()).filter(|ordinal| !instance.instance.defaults.as_ref()
                    .is_some_and(|defaults| defaults.omitted.contains(&(*ordinal as u32)))).collect()
            } else { Vec::new() };
            let arg_count = supplied_ordinals.len();
            let param_types = resolved_param_types.unwrap_or_else(|| {
                let mut p = Vec::new();
                if let Decl::Function { params, .. } = decl {
                    for param_id in params {
                        let mut param_ty = luna_semantic::SemanticTypeId(0);
                        if let Some(sym_id) = self.ctx.tables.decl_symbols.get(param_id).copied() {
                            param_ty = self.get_symbol_type(&sym_id).unwrap_or(luna_semantic::SemanticTypeId(0));
                        }
                        if param_ty == luna_semantic::SemanticTypeId(0) {
                            if let Decl::Param { ty: Some(ty_id), .. } = &self.arena.decls[param_id.0 as usize] {
                                param_ty = self.ctx.tables.ast_type_to_semantic.get(ty_id).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                            }
                        }
                        p.push(param_ty);
                    }
                }
                p
            });

            let param_types = supplied_ordinals.iter().map(|&ordinal| param_types[ordinal]).collect();
            let is_async = if let Decl::Function { is_async, .. } = decl { *is_async } else { false };

            let memory_intrinsic = sym_id_opt.and_then(|symbol| self.ctx.lang_items.from_symbol(symbol))
                .filter(|item| item.is_memory_intrinsic());
            let is_extern = instance.instance.defaults.is_none() && ((body.is_none() && memory_intrinsic.is_none()) || {
                instance.instance.subst.is_empty() && !self.is_comptime
                    && sym_id_opt.is_some_and(|symbol| self.ctx.tables.object_backed_functions.contains(&symbol))
            });
            self.current_function = Some(Function {
                name: global_id,
                is_extern,
                is_async,
                arg_count,
                link_name: if instance.instance.defaults.is_some() { None } else { link_name },
                param_types,
                ret_ty: ret_ty_id,
                lifetime_info: Default::default(),
                blocks: Vec::new(),
                values: Vec::new(),
            });
            self.current_block = None;

            if is_extern {
                if let Some(func) = self.current_function.take() {
                    self.module.functions.push(func);
                }
                self.current_instance = None;
                return;
            }

            self.locals.clear();
            self.known_function_values.clear();
            self.lexical_scopes.clear();
            self.temporary_drop_scopes.clear();
            self.loop_scopes.clear();
            self.loop_break_targets.clear();
            self.loop_continue_targets.clear();
            self.push_scope(); // Function root scope
            self.next_label_id = 0;
            
            let entry_label = self.new_label("entry");
            self.start_block(entry_label);
            
            if let Decl::Function { params, .. } = decl {
                // The actual ABI parameters occupy the first arg_count values.
                // Defaults are ordinary local storage in the same callee frame.
                for (idx, &ordinal) in supplied_ordinals.iter().enumerate() {
                    let param_id = &params[ordinal];
                    if let Decl::Param { .. } = &self.arena.decls[param_id.0 as usize] {
                        if let Some(sym_id) = self.ctx.tables.decl_symbols.get(&param_id).copied() {
                            let ty_id = self.get_symbol_type(&sym_id).unwrap_or(luna_semantic::SemanticTypeId(0));
                            let alloc_val = self.push_inst(Instruction::Alloca, ty_id);
                            self.current_function.as_mut().unwrap().values[alloc_val.0 as usize].origin = ValueOrigin::Parameter(idx as u32);
                            self.locals.insert(sym_id, alloc_val);
                            self.lexical_scopes.last_mut().unwrap().push(sym_id);
                        }
                    }
                }
                if let Some(defaults) = &instance.instance.defaults {
                    let Decl::Function { params: source_parameters, .. } = &self.arena.decls[defaults.declaration.0 as usize] else {
                        panic!("ICE: checked default source is not a function");
                    };
                    for &ordinal in &defaults.omitted {
                        let symbol = self.ctx.tables.decl_symbols[&params[ordinal as usize]];
                        let ty = self.get_symbol_type(&symbol).expect("checked default parameter type");
                        let storage = self.push_inst(Instruction::Alloca, ty);
                        self.locals.insert(symbol, storage);
                        self.lexical_scopes.last_mut().unwrap().push(symbol);
                    }
                    for (&source, &target) in source_parameters.iter().zip(params) {
                        let source = self.ctx.tables.decl_symbols[&source];
                        let target = self.ctx.tables.decl_symbols[&target];
                        self.locals.insert(source, self.locals[&target]);
                    }
                    for &ordinal in &defaults.omitted {
                        let Decl::Param { default: Some(default), .. } = &self.arena.decls[source_parameters[ordinal as usize].0 as usize] else {
                            panic!("ICE: omitted argument has no checked default");
                        };
                        let value = self.generate_expr(&default.value);
                        let symbol = self.ctx.tables.decl_symbols[&params[ordinal as usize]];
                        let ty = self.get_symbol_type(&symbol).expect("checked default type");
                        self.push_inst_span(Instruction::Store { ptr: Operand::Value(self.locals[&symbol]), value }, ty, Some(default.span));
                    }
                    self.annotate_default_entry(&instance.instance, params);
                }
            }

            if is_async {
                let future_alloca = self.push_inst(Instruction::Alloca, ret_ty_id);
                self.push_inst(Instruction::MarkInit { value: Operand::Value(future_alloca) }, ret_ty_id);
                self.current_async_future = Some(future_alloca);
                let i32_ty = luna_semantic::SemanticTypeId(3);
                let state_ptr = self.push_inst(Instruction::FieldPtr {
                    base: Operand::Value(future_alloca),
                    field_idx: 0,
                    field_name: None,
                }, i32_ty);
                self.push_inst(Instruction::Store {
                    ptr: Operand::Value(state_ptr),
                    value: Operand::Number("0".to_string()),
                }, i32_ty);
            } else {
                self.current_async_future = None;
            }
            
            if let Some(item) = memory_intrinsic {
                self.generate_memory_intrinsic_body(item, decl, ret_ty_id);
            } else if let Some(body_stmt) = body {
                self.generate_fn_body(&body_stmt, ret_ty_id);
            } else if instance.instance.defaults.is_some() {
                let Decl::Function { params, .. } = decl else { unreachable!() };
                let mut arguments = Vec::new();
                for parameter in params {
                    let symbol = self.ctx.tables.decl_symbols[parameter];
                    let ty = self.get_symbol_type(&symbol).expect("checked extern parameter");
                    arguments.push(Operand::Value(self.push_inst(Instruction::Load { ptr: Operand::Value(self.locals[&symbol]) }, ty)));
                }
                let identity = luna_semantic::CanonicalInstanceIdentity { kind: luna_semantic::CanonicalInstanceKind::Decl(instance.instance.decl_id), subst: instance.instance.subst.clone() };
                let result = self.push_inst(Instruction::CallDirect { callee: GlobalId {
                    name: identity.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, &fn_name), symbol_id: sym_id_opt }, args: arguments }, ret_ty_id);
                self.terminate_block(Terminator::Ret { value: Some(Operand::Value(result)) });
            }
            
            if let Some(func) = self.current_function.take() {
                self.module.functions.push(func);
            }
        }
        self.current_instance = None;
    }

    fn generate_memory_intrinsic_body(&mut self, item: luna_semantic::lang_item::LangItem,
        decl: &Decl, return_type: luna_semantic::SemanticTypeId) {
        use luna_semantic::{SemanticType, lang_item::LangItem};
        let result = (|| {
            let Decl::Function { params, .. } = decl else { return Err("missing declaration"); };
            let mut operands = Vec::new();
            let mut types = Vec::new();
            for parameter in params {
                let symbol = self.ctx.tables.decl_symbols.get(parameter).ok_or("missing parameter symbol")?;
                let ty = self.get_symbol_type(symbol).ok_or("missing parameter type")?;
                let storage = self.locals.get(symbol).copied().ok_or("missing parameter storage")?;
                let value = self.push_inst(Instruction::Load { ptr: Operand::Value(storage) }, ty);
                operands.push(Operand::Value(value));
                types.push(ty);
            }
            match item {
                LangItem::DropInPlace if operands.len() == 1 => {
                    let SemanticType::Pointer(_, element) = self.ctx.types.get(types[0]) else {
                        return Err("drop operand is not a pointer");
                    };
                    let element = *element;
                    if self.ctx.needs_drop(element) {
                        let callee = self.get_drop_glue_global_id(element);
                        self.push_inst(Instruction::Drop { value: operands[0].clone(), callee, ty: element }, element);
                    }
                    Ok(None)
                }
                LangItem::SliceFromRawParts | LangItem::SliceFromRawPartsMut if operands.len() == 2 => {
                    let value = self.push_inst(Instruction::MakeSlice {
                        data_ptr: operands[0].clone(), len: operands[1].clone(),
                    }, return_type);
                    Ok(Some(Operand::Value(value)))
                }
                _ => Err("incorrect intrinsic arity or kind"),
            }
        })();
        match result {
            Ok(value) => self.terminate_block(Terminator::Ret { value }),
            Err(message) => {
                self.diagnostics.push(luna_common::Diagnostic::error(format!(
                    "memory intrinsic lowering invariant: {message}"))
                    .with_code(luna_common::DiagnosticCode::BackendInvariantViolation));
                self.terminate_block(Terminator::Unreachable);
            }
        }
    }

    // Transfer the return place before scope cleanup. In particular, returning
    // a field must not exempt its entire containing aggregate from destruction.
    fn transfer_return_value(&mut self, expr: &luna_ast::ExprId) -> Operand {
        let value = self.generate_expr(expr);
        let ty = self.get_expr_type(expr);
        if matches!(self.ctx.types.get(ty), luna_semantic::SemanticType::Void) {
            return value;
        }
        let transferred = self.push_inst_span(
            Instruction::Assign(value), ty, self.extract_expr_span(expr),
        );
        Operand::Value(transferred)
    }

    fn generate_fn_body(&mut self, body_stmt_id: &luna_ast::StmtId, ret_ty_id: luna_semantic::SemanticTypeId) {
        let stmt = &self.arena.stmts[body_stmt_id.0 as usize];
        match stmt {
            Stmt::Block { body, tail_expr } => {
                self.push_scope();
                for item in body {
                    self.generate_item(item);
                }
                if let Some(tail_expr_id) = tail_expr {
                    let tail_op = self.transfer_return_value(tail_expr_id);
                    self.pop_scope_and_drop(None);
                    if let Some(mut block) = self.current_block.take() {
                        if block.terminator.is_none() {
                            self.current_block = Some(block);
                            self.emit_drops_up_to(0, None);
                            block = self.current_block.take().unwrap();
                            let ret_val = if ret_ty_id == luna_semantic::SemanticTypeId(0) {
                                None
                            } else {
                                Some(tail_op)
                            };
                            block.terminator = Some(Terminator::Ret { value: ret_val });
                        }
                        self.current_function.as_mut().unwrap().blocks.push(block);
                    }
                    return;
                }
                self.pop_scope_and_drop(None);
            }
            Stmt::Unsafe { body } => {
                self.unsafe_depth += 1;
                self.generate_fn_body(body, ret_ty_id);
                self.unsafe_depth -= 1;
                return;
            }
            Stmt::Expr { expr, .. } => {
                let tail_op = self.transfer_return_value(expr);
                if let Some(mut block) = self.current_block.take() {
                    if block.terminator.is_none() {
                        self.current_block = Some(block);
                        self.emit_drops_up_to(0, None);
                        block = self.current_block.take().unwrap();
                        let ret_val = if ret_ty_id == luna_semantic::SemanticTypeId(0) {
                            None
                        } else {
                            Some(tail_op)
                        };
                        block.terminator = Some(Terminator::Ret { value: ret_val });
                    }
                    self.current_function.as_mut().unwrap().blocks.push(block);
                }
                return;
            }
            _ => {
                self.generate_stmt(body_stmt_id);
            }
        }

        if let Some(mut block) = self.current_block.take() {
            if block.terminator.is_none() {
                self.current_block = Some(block);
                self.emit_drops_up_to(0, None);

                let ret_val = if let Some(fut_alloca) = self.current_async_future {
                    let i32_ty = luna_semantic::SemanticTypeId(3);
                    let state_ptr = self.push_inst(Instruction::FieldPtr {
                        base: Operand::Value(fut_alloca),
                        field_idx: 0,
                        field_name: None,
                    }, i32_ty);
                    self.push_inst(Instruction::Store {
                        ptr: Operand::Value(state_ptr),
                        value: Operand::Number("-1".to_string()),
                    }, i32_ty);
                    let load_fut = self.push_inst(Instruction::Load {
                        ptr: Operand::Value(fut_alloca),
                    }, ret_ty_id);
                    Some(Operand::Value(load_fut))
                } else if ret_ty_id == luna_semantic::SemanticTypeId(0) {
                    None
                } else {
                    Some(Operand::Number("0".to_string()))
                };
                block = self.current_block.take().unwrap();
                block.terminator = Some(Terminator::Ret { value: ret_val });
            }
            self.current_function.as_mut().unwrap().blocks.push(block);
        }
    }

    fn generate_stmt(&mut self, stmt_id: &luna_ast::StmtId) {
        let stmt = &self.arena.stmts[stmt_id.0 as usize];
        match stmt {
            Stmt::Expr { expr, .. } => {
                self.generate_expr(expr);
            }
            Stmt::Block { body, tail_expr } => {
                self.push_scope();
                for item in body {
                    self.generate_item(item);
                }
                if let Some(expr) = tail_expr {
                    self.generate_expr(expr);
                }
                self.pop_scope_and_drop(None);
            }
            Stmt::Return { value } => {
                if let Some(fut_alloca) = self.current_async_future {
                    let i32_ty = luna_semantic::SemanticTypeId(3);
                    if let Some(expr_id) = value {
                        let ret_ty = self.ctx.tables.expr_types.get(expr_id).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                        let val_op = self.generate_expr(expr_id);
                        let val_ptr = self.push_inst(Instruction::FieldPtr {
                            base: Operand::Value(fut_alloca),
                            field_idx: 1,
                            field_name: None,
                        }, ret_ty);
                        self.push_inst(Instruction::Store {
                            ptr: Operand::Value(val_ptr),
                            value: val_op,
                        }, ret_ty);
                    }
                    let state_ptr = self.push_inst(Instruction::FieldPtr {
                        base: Operand::Value(fut_alloca),
                        field_idx: 0,
                        field_name: None,
                    }, i32_ty);
                    self.push_inst(Instruction::Store {
                        ptr: Operand::Value(state_ptr),
                        value: Operand::Number("-1".to_string()),
                    }, i32_ty);
                    
                    self.emit_drops_up_to(0, None);
                    
                    let ret_ty_id = self.current_function.as_ref().unwrap().ret_ty;
                    let load_fut = self.push_inst(Instruction::Load {
                        ptr: Operand::Value(fut_alloca),
                    }, ret_ty_id);
                    self.terminate_block(Terminator::Ret { value: Some(Operand::Value(load_fut)) });
                    return;
                }

                let val_operand = if let Some(expr_id) = value {
                    Some(self.transfer_return_value(expr_id))
                } else {
                    None
                };
                
                self.emit_drops_up_to(0, None);
                self.terminate_block(Terminator::Ret { value: val_operand });
            }
            Stmt::While { condition, body, .. } => {
                let cond_label = self.new_label("while_cond");
                let body_label = self.new_label("while_body");
                let end_label = self.new_label("while_end");

                self.terminate_block(Terminator::Br { target: cond_label.clone() });
                self.start_block(cond_label.clone());

                let cond_op = self.generate_expr(condition);
                self.terminate_block(Terminator::CondBr {
                    condition: cond_op,
                    true_target: body_label.clone(),
                    false_target: end_label.clone(),
                });

                self.start_block(body_label.clone());
                
                self.loop_scopes.push(self.lexical_scopes.len());
                self.loop_break_targets.push(end_label.clone());
                self.loop_continue_targets.push(cond_label.clone());
                
                self.generate_stmt(body);
                self.terminate_block(Terminator::Br { target: cond_label.clone() });

                self.loop_scopes.pop();
                self.loop_break_targets.pop();
                self.loop_continue_targets.pop();

                self.start_block(end_label.clone());
            }
            Stmt::Break { .. } => {
                if let (Some(&target_depth), Some(end_label)) = (self.loop_scopes.last(), self.loop_break_targets.last().cloned()) {
                    self.emit_drops_up_to(target_depth, None);
                    self.terminate_block(Terminator::Br { target: end_label });
                }
            }
            Stmt::Continue { .. } => {
                if let (Some(&target_depth), Some(cond_label)) = (self.loop_scopes.last(), self.loop_continue_targets.last().cloned()) {
                    self.emit_drops_up_to(target_depth, None);
                    self.terminate_block(Terminator::Br { target: cond_label });
                }
            }
            Stmt::If { condition, then_branch, else_branch } => {
                let then_label = self.new_label("if_then");
                let else_label = self.new_label("if_else");
                let end_label = self.new_label("if_end");

                let cond_op = self.generate_expr(condition);
                self.terminate_block(Terminator::CondBr {
                    condition: cond_op,
                    true_target: then_label.clone(),
                    false_target: if else_branch.is_some() { else_label.clone() } else { end_label.clone() },
                });

                self.start_block(then_label.clone());
                self.generate_stmt(then_branch);
                self.terminate_block(Terminator::Br { target: end_label.clone() });

                if let Some(else_branch) = else_branch {
                    self.start_block(else_label.clone());
                    self.generate_stmt(else_branch);
                    self.terminate_block(Terminator::Br { target: end_label.clone() });
                }

                self.start_block(end_label.clone());
            }
            Stmt::Unsafe { body } => {
                self.unsafe_depth += 1;
                self.generate_stmt(body);
                self.unsafe_depth -= 1;
            }
            Stmt::For { kind, init, cond, step, body, pattern, iterable, .. } => {
                use luna_ast::stmt::ForKind;
                match kind {
                    ForKind::CStyle => {
                        // Scope: init lives in loop's outer scope
                        self.push_scope();
                        
                        // Generate init (e.g., `dec rw i: i32 = 0`)
                        if let Some(item) = init {
                            self.generate_item(item);
                        }
                        
                        let cond_label = self.new_label("for_cond");
                        let body_label = self.new_label("for_body");
                        let step_label = self.new_label("for_step");
                        let end_label = self.new_label("for_end");
                        
                        // Jump to condition check
                        self.terminate_block(Terminator::Br { target: cond_label.clone() });
                        self.start_block(cond_label.clone());
                        
                        // Evaluate condition
                        if let Some(c) = cond {
                            let cond_op = self.generate_expr(c);
                            self.terminate_block(Terminator::CondBr {
                                condition: cond_op,
                                true_target: body_label.clone(),
                                false_target: end_label.clone(),
                            });
                        } else {
                            // No condition -> infinite loop (always true)
                            self.terminate_block(Terminator::Br { target: body_label.clone() });
                        }
                        
                        // Body
                        self.start_block(body_label.clone());
                        
                        // Push loop targets: break -> end, continue -> step
                        self.loop_scopes.push(self.lexical_scopes.len());
                        self.loop_break_targets.push(end_label.clone());
                        self.loop_continue_targets.push(step_label.clone());
                        
                        self.generate_stmt(body);
                        
                        // Fall through to step
                        self.terminate_block(Terminator::Br { target: step_label.clone() });
                        
                        // Step
                        self.start_block(step_label.clone());
                        if let Some(s) = step {
                            self.generate_expr(s);
                        }
                        self.terminate_block(Terminator::Br { target: cond_label.clone() });
                        
                        // Pop loop targets
                        self.loop_scopes.pop();
                        self.loop_break_targets.pop();
                        self.loop_continue_targets.pop();
                        
                        // End block + drop init scope variables
                        self.start_block(end_label.clone());
                        self.pop_scope_and_drop(None);
                    }
                    ForKind::ForEach => {
                        let pat = pattern.as_ref().unwrap();
                        let iter_expr = iterable.as_ref().unwrap();
                        let iter = &self.arena.exprs[iter_expr.0 as usize];
                        
                        if let luna_ast::Expr::Binary { op: luna_ast::expr::BinaryOp::Range, left, right, .. } = iter {
                            let left_op = self.generate_expr(left);
                            let right_op = self.generate_expr(right);
                            let left_ty_id = self.ctx.tables.expr_types.get(left).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                            
                            let iter_cur = self.push_inst(Instruction::Alloca, left_ty_id);
                            self.push_inst(Instruction::Store { ptr: Operand::Value(iter_cur), value: left_op }, left_ty_id);
                            
                            let cond_label = self.new_label("for_cond");
                            let body_label = self.new_label("for_body");
                            let step_label = self.new_label("for_step");
                            let end_label = self.new_label("for_end");
                            
                            self.terminate_block(Terminator::Br { target: cond_label.clone() });
                            self.start_block(cond_label.clone());
                            
                            let cur_val = self.push_inst(Instruction::Load { ptr: Operand::Value(iter_cur) }, left_ty_id);
                            let cond_val = self.push_inst(Instruction::LessThan { left: Operand::Value(cur_val), right: right_op.clone() }, self.ctx.types.bool_id());
                            self.terminate_block(Terminator::CondBr {
                                condition: Operand::Value(cond_val),
                                true_target: body_label.clone(),
                                false_target: end_label.clone(),
                            });
                            
                            self.start_block(body_label.clone());
                            self.push_scope();
                            self.loop_scopes.push(self.lexical_scopes.len());
                            self.loop_break_targets.push(end_label.clone());
                            self.loop_continue_targets.push(step_label.clone());
                            
                            if let Some(pat_sym) = self.ctx.tables.pat_symbols.get(pat) {
                                let pat_ty_id = self.get_symbol_type(pat_sym).unwrap_or(left_ty_id);
                                let pat_var = self.push_inst(Instruction::Alloca, pat_ty_id);
                                self.locals.insert(*pat_sym, pat_var);
                                self.push_inst(Instruction::Store { ptr: Operand::Value(pat_var), value: Operand::Value(cur_val) }, pat_ty_id);
                                if let Some(scope) = self.lexical_scopes.last_mut() {
                                    scope.push(*pat_sym);
                                }
                            }
                            
                            self.generate_stmt(body);
                            
                            self.terminate_block(Terminator::Br { target: step_label.clone() });
                            
                            self.start_block(step_label.clone());
                            let cur_val_for_add = self.push_inst(Instruction::Load { ptr: Operand::Value(iter_cur) }, left_ty_id);
                            let inc_val = self.push_inst(Instruction::Add { left: Operand::Value(cur_val_for_add), right: Operand::Number("1".to_string()) }, left_ty_id);
                            self.push_inst(Instruction::Store { ptr: Operand::Value(iter_cur), value: Operand::Value(inc_val) }, left_ty_id);
                            self.terminate_block(Terminator::Br { target: cond_label.clone() });
                            
                            self.loop_scopes.pop();
                            self.loop_break_targets.pop();
                            self.loop_continue_targets.pop();
                            
                            self.start_block(end_label.clone());
                            self.pop_scope_and_drop(None);
                        } else {
                            let mono_loop = self.current_instance.and_then(|inst_ptr| {
                                let inst = unsafe { &*inst_ptr };
                                inst.mono_for_loops.get(stmt_id).cloned()
                            }).or_else(|| self.ctx.comptime_root.as_ref()
                                .and_then(|root| root.mono_for_loops.get(stmt_id).cloned()));
                            let semantic_loop = self
                                .ctx
                                .tables
                                .for_loop_resolutions
                                .get(stmt_id)
                                .cloned();
                            let (Some(mono_loop), Some(semantic_loop)) =
                                (mono_loop, semantic_loop)
                            else {
                                self.diagnostics.push(luna_common::Diagnostic::error(
                                    "MVIR invariant violated: resolved for-in protocol plan is missing"
                                        .to_string(),
                                ).with_code(luna_common::DiagnosticCode::BackendInvariantViolation));
                                return;
                            };

                            let Some(some_sym) = self
                                .ctx
                                .lang_items
                                .get(luna_semantic::lang_item::LangItem::OptionSome)
                            else {
                                self.diagnostics.push(luna_common::Diagnostic::error(
                                    "MVIR invariant violated: Option::Some language item is missing"
                                        .to_string(),
                                ).with_code(luna_common::DiagnosticCode::BackendInvariantViolation));
                                return;
                            };
                            let some_variant = match self.ctx.symbol_table.get_symbol(some_sym).kind {
                                luna_semantic::SymbolKind::EnumVariant(index) => index,
                                _ => {
                                    self.diagnostics.push(luna_common::Diagnostic::error(
                                        "MVIR invariant violated: Option::Some is not an enum variant"
                                            .to_string(),
                                    ).with_code(luna_common::DiagnosticCode::BackendInvariantViolation));
                                    return;
                                }
                            };

                            // The iterator temporary owns the result of the
                            // protocol conversion for the entire loop.
                            self.push_scope();
                            let source = self.generate_expr(iter_expr);
                            let into_iter = self.push_inst(
                                Instruction::CallDirect {
                                    callee: self.mono_instance_global(
                                        semantic_loop.into_iter_method,
                                        &mono_loop.into_iter,
                                    ),
                                    args: vec![source],
                                },
                                mono_loop.iterator_type,
                            );
                            let iterator_slot =
                                self.push_inst(Instruction::Alloca, mono_loop.iterator_type);
                            self.push_inst(
                                Instruction::Store {
                                    ptr: Operand::Value(iterator_slot),
                                    value: Operand::Value(into_iter),
                                },
                                mono_loop.iterator_type,
                            );
                            if let Some(scope) = self.temporary_drop_scopes.last_mut() {
                                scope.push((iterator_slot, mono_loop.iterator_type));
                            }

                            let next_label = self.new_label("for_next");
                            let body_label = self.new_label("for_body");
                            let end_label = self.new_label("for_end");
                            self.terminate_block(Terminator::Br {
                                target: next_label.clone(),
                            });
                            self.start_block(next_label.clone());

                            let next_is_rw = matches!(
                                self.ctx.types.get(mono_loop.next_receiver_type),
                                luna_semantic::SemanticType::Reference(
                                    _,
                                    luna_semantic::ty::Mutability::Mutable,
                                    _
                                )
                            );
                            let receiver = self.push_inst(
                                Instruction::Borrow {
                                    is_rw: next_is_rw,
                                    base: Operand::Value(iterator_slot),
                                },
                                mono_loop.next_receiver_type,
                            );
                            let next_value = self.push_inst(
                                Instruction::CallDirect {
                                    callee: self.mono_instance_global(
                                        semantic_loop.next_method,
                                        &mono_loop.next,
                                    ),
                                    args: vec![Operand::Value(receiver)],
                                },
                                mono_loop.option_type,
                            );
                            let tag_ty = self.ctx.types.u32_id();
                            let tag = self.push_inst(
                                Instruction::Tag {
                                    value: Operand::Value(next_value),
                                },
                                tag_ty,
                            );
                            let is_some = self.push_inst(
                                Instruction::Eq {
                                    left: Operand::Value(tag),
                                    right: Operand::Number(some_variant.to_string()),
                                },
                                self.ctx.types.bool_id(),
                            );
                            self.terminate_block(Terminator::CondBr {
                                condition: Operand::Value(is_some),
                                true_target: body_label.clone(),
                                false_target: end_label.clone(),
                            });

                            self.start_block(body_label);
                            self.push_scope();
                            let item = self.push_inst(
                                Instruction::Extract {
                                    value: Operand::Value(next_value),
                                    variant_idx: some_variant,
                                    field_idx: 0,
                                },
                                mono_loop.item_type,
                            );
                            self.bind_pattern(pat, Some(Operand::Value(item)));

                            // A break/continue leaves the per-iteration pattern
                            // scope, while the iterator itself remains alive
                            // until the common loop exit.
                            self.loop_scopes.push(self.lexical_scopes.len() - 1);
                            self.loop_break_targets.push(end_label.clone());
                            self.loop_continue_targets.push(next_label.clone());
                            self.generate_stmt(body);
                            self.pop_scope_and_drop(None);
                            self.terminate_block(Terminator::Br {
                                target: next_label.clone(),
                            });
                            self.loop_scopes.pop();
                            self.loop_break_targets.pop();
                            self.loop_continue_targets.pop();

                            self.start_block(end_label);
                            self.pop_scope_and_drop(None);
                        }
                    }
                }
            }
            Stmt::Comptime { .. } => {
                unreachable!("ICE: Stmt::Comptime reached MVIR generator, should have been blocked by Typechecker");
            }
            _ => unreachable!("ICE: Unhandled variant, should be impossible after semantic invariants")
        }
    }

    fn generate_block_expr(&mut self, stmt_id: &luna_ast::StmtId) -> Operand {
        let stmt = &self.arena.stmts[stmt_id.0 as usize];
        if let Stmt::Block { body, tail_expr } = stmt {
            let mut last_ret = None;
            for (i, item) in body.iter().enumerate() {
                if i == body.len() - 1 && tail_expr.is_none() {
                    if let Item::Stmt(s) = item {
                        last_ret = Some(self.generate_block_expr(s));
                        continue;
                    }
                }
                self.generate_item(item);
            }
            if let Some(expr) = tail_expr {
                return self.generate_expr(expr);
            }
            if let Some(ret) = last_ret {
                return ret;
            }
        } else if let Stmt::Expr { expr, .. } = stmt {
            return self.generate_expr(expr);
        } else if let Stmt::Unsafe { body } = stmt {
            return self.generate_block_expr(body);
        } else {
            self.generate_stmt(stmt_id);
        }
        Operand::Number("0".to_string())
    }

    fn get_expr_span(&self, expr: &luna_ast::Expr) -> Option<luna_common::Span> {
        use luna_ast::Expr;
        match expr {
            Expr::Literal(tok, _) => Some(tok.span),
            Expr::Identifier { segments, .. } => segments.first().copied(),
            Expr::Member { member, .. } => Some(*member),
            Expr::Call { callee, .. } => self.get_expr_span(&self.arena.exprs[callee.0 as usize]),
            Expr::MethodCall { method_name, .. } => Some(*method_name),
            Expr::StructInit { path, .. } => path.first().copied(),
            Expr::Match { match_span, .. } => Some(*match_span),
            Expr::Try { try_span, .. } => Some(*try_span),
            Expr::Lambda { .. } => None,
            Expr::Assign { lvalue, .. } => self.get_expr_span(&self.arena.exprs[lvalue.0 as usize]),
            Expr::Binary { left, .. } => self.get_expr_span(&self.arena.exprs[left.0 as usize]),
            Expr::Unary { operand, .. } => self.get_expr_span(&self.arena.exprs[operand.0 as usize]),
            Expr::Cast { expr, .. } => self.get_expr_span(&self.arena.exprs[expr.0 as usize]),
            Expr::Index { base, .. } => self.get_expr_span(&self.arena.exprs[base.0 as usize]),
            Expr::TupleIndex { object, .. } => self.get_expr_span(&self.arena.exprs[object.0 as usize]),
            Expr::Await { expr } => self.get_expr_span(&self.arena.exprs[expr.0 as usize]),
            Expr::Sizeof { .. } | Expr::Alignof { .. } | Expr::Comptime { .. } => None,
            Expr::MacroCall { span, .. } => Some(*span),
            Expr::ArrayLiteral { elements } | Expr::TupleLiteral { elements } => {
                elements.first().and_then(|e| self.get_expr_span(&self.arena.exprs[e.0 as usize]))
            }
        }
    }

    fn generate_call_arguments(&mut self, expression: luna_ast::ExprId, arguments: &[luna_ast::CallArg]) -> Vec<Operand> {
        // Evaluate once in source order; only the resulting operands are reordered.
        let mut values = Vec::new();
        for argument in arguments {
            let value = self.generate_expr(&argument.value);
            let ty = match &value {
                Operand::Value(value) => self.current_function.as_ref().expect("Must be in a function").values[value.0 as usize].ty,
                _ => self.get_expr_type(&argument.value),
            };
            if matches!(self.ctx.types.get(ty), luna_semantic::SemanticType::Void | luna_semantic::SemanticType::Never) {
                values.push(value);
            } else if self.ctx.needs_drop(ty) {
                // Transfer now, before evaluating the next source argument.
                // Ordinary temporary cleanup also covers a later argument's return.
                let slot = self.push_inst_span(Instruction::Alloca, ty, self.extract_expr_span(&argument.value));
                self.push_inst_span(Instruction::Store { ptr: Operand::Value(slot), value }, ty, self.extract_expr_span(&argument.value));
                self.temporary_drop_scopes.last_mut().expect("ICE: call argument has no cleanup scope").push((slot, ty));
                let value = self.push_inst_span(Instruction::Load { ptr: Operand::Value(slot) }, ty, self.extract_expr_span(&argument.value));
                values.push(Operand::Value(value));
            } else {
                // An SSA snapshot severs delayed place loads; Assign performs
                // the same generic ownership transfer as an ordinary return.
                let transferred = self.push_inst_span(Instruction::Assign(value), ty, self.extract_expr_span(&argument.value));
                values.push(Operand::Value(transferred));
            }
        }
        if let Some(binding) = self.ctx.tables.call_argument_bindings.get(&expression) {
            binding.in_parameter_order(&values).expect("ICE: checked call binding is not a permutation")
        } else {
            assert!(arguments.iter().all(|argument| argument.label.is_none()),
                "ICE: named call reached lowering without a checked binding");
            values
        }
    }

    fn generate_expr(&mut self, expr_id: &luna_ast::ExprId) -> Operand {
        let expr = &self.arena.exprs[expr_id.0 as usize];
        let span = self.get_expr_span(expr);
        let prev_span = self.current_span.clone();
        if span.is_some() {
            self.current_span = span;
        }

        let mut result = self.generate_expr_inner(expr_id);
        
        if let Some(coercion) = self.ctx.tables.coercions.get(expr_id) {
            match coercion {
                luna_semantic::CoercionKind::ConcreteToDyn { trait_sym, concrete_sym } => {
                    let trait_name = self.ctx.symbol_table.get_symbol(*trait_sym).name.clone();
                    let concrete_name = self.ctx.symbol_table.get_symbol(*concrete_sym).name.clone();
                    let vtable_global = GlobalId {
                        name: format!("vtable_{}_{}", trait_name, concrete_name),
                        symbol_id: Some(*trait_sym),
                    };
                    let ty_id = self.ctx.tables.expr_types.get(expr_id).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                    let trait_obj_val = self.push_inst(Instruction::MakeTraitObject {
                        data_ptr: result,
                        vtable: vtable_global,
                        trait_sym: *trait_sym,
                        concrete_sym: *concrete_sym,
                    }, ty_id);
                    result = Operand::Value(trait_obj_val);
                }
                luna_semantic::CoercionKind::ArrayToSlice { element_type: _, length } => {
                    let ty_id = self.ctx.tables.expr_types.get(expr_id).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                    let len_operand = Operand::Number(length.to_string());
                    let slice_val = self.push_inst(Instruction::MakeSlice {
                        data_ptr: result,
                        len: len_operand,
                    }, ty_id);
                    result = Operand::Value(slice_val);
                }
                luna_semantic::CoercionKind::RefMutToShared => {
                    // Representation is unchanged
                }
            }
        }
        
        let ty_id = self.ctx.tables.expr_types.get(expr_id).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
        let ty = self.ctx.types.get(ty_id);
        if let luna_semantic::SemanticType::Never = ty {
            self.terminate_block(Terminator::Unreachable);
        }
        
        self.current_span = prev_span;
        result
    }

    fn generate_expr_inner(&mut self, expr_id: &luna_ast::ExprId) -> Operand {
        let expr = &self.arena.exprs[expr_id.0 as usize];
        let ty_id = self.get_expr_type(expr_id);
        
        if let Some(ct_val) = self.ctx.comptime_values.get(expr_id).cloned() {
            return self.materialize_comptime_value(&ct_val, ty_id);
        }
        
        match expr {
            Expr::Literal(tok, text) => {
                match tok.kind {
                    luna_lexer::TokenKind::IntegerLiteral => {
                        let literal = luna_lexer::literal::decode_integer(text)
                            .expect("semantic analysis validated the integer literal");
                        let constant = self.push_inst(Instruction::Assign(Operand::Number(literal.magnitude.to_string())), ty_id);
                        Operand::Value(constant)
                    }
                    luna_lexer::TokenKind::ByteLiteral | luna_lexer::TokenKind::ByteStringLiteral => {
                        let bytes = luna_lexer::literal::decode_bytes(text, tok.kind == luna_lexer::TokenKind::ByteLiteral)
                            .expect("semantic analysis validated the byte literal");
                        if tok.kind == luna_lexer::TokenKind::ByteLiteral {
                            let constant = self.push_inst(Instruction::Assign(Operand::Number(bytes[0].to_string())), ty_id);
                            Operand::Value(constant)
                        } else {
                            let byte_ty = match self.ctx.types.get(ty_id) {
                                luna_semantic::SemanticType::Array(elem, _) => *elem,
                                _ => {
                                    self.diagnostics.push(luna_common::Diagnostic::error("byte array reached MVIR without its semantic array type").with_code(luna_common::DiagnosticCode::BackendInvariantViolation).with_span(tok.span));
                                    return Operand::Number("0".to_string());
                                }
                            };
                            let value = luna_semantic::ComptimeValue::Array {
                                elements: bytes.into_iter().map(|byte| luna_semantic::ComptimeValue::Int { val: byte as i128, width: luna_semantic::IntWidth::U8 }).collect(),
                                elem_ty: Some(byte_ty),
                            };
                            self.materialize_comptime_value(&value, ty_id)
                        }
                    }
                    luna_lexer::TokenKind::FloatLiteral => {
                        let ty = self.ctx.types.get(ty_id);
                        let float_ty = match ty {
                            luna_semantic::SemanticType::Primitive(luna_semantic::BuiltinType::F32) => FloatType::F32,
                            _ => FloatType::F64,
                        };
                        let clean_text = text.trim_end_matches("f32").trim_end_matches("f64").to_string();
                        Operand::Float { text: clean_text, ty: float_ty }
                    }
                    luna_lexer::TokenKind::StringLiteral => {
                        
                        Operand::StringRef(text.clone())
                    }
                    luna_lexer::TokenKind::CharLiteral => {
                        
                        Operand::Char(text.clone())
                    }
                    luna_lexer::TokenKind::KwTrue => Operand::Boolean(true),
                    luna_lexer::TokenKind::KwFalse => Operand::Boolean(false),
                    _ => Operand::Number("null".to_string()),
                }
            }
            Expr::Binary { op, left, right, .. } => {
                if matches!(op, luna_ast::expr::BinaryOp::LogicAnd | luna_ast::expr::BinaryOp::LogicOr) {
                    let left_op = self.generate_expr(left);
                    let result_alloca = self.push_inst(Instruction::Alloca, ty_id);
                    self.push_inst(Instruction::Store {
                        ptr: Operand::Value(result_alloca),
                        value: left_op.clone(),
                    }, ty_id);
                    
                    let right_label = self.new_label("logical_right");
                    let end_label = self.new_label("logical_end");
                    
                    if *op == luna_ast::expr::BinaryOp::LogicAnd {
                        self.terminate_block(Terminator::CondBr {
                            condition: left_op,
                            true_target: right_label.clone(),
                            false_target: end_label.clone(),
                        });
                    } else {
                        self.terminate_block(Terminator::CondBr {
                            condition: left_op,
                            true_target: end_label.clone(),
                            false_target: right_label.clone(),
                        });
                    }
                    
                    self.start_block(right_label);
                    let right_op = self.generate_expr(right);
                    self.push_inst(Instruction::Store {
                        ptr: Operand::Value(result_alloca),
                        value: right_op,
                    }, ty_id);
                    self.terminate_block(Terminator::Br { target: end_label.clone() });
                    
                    self.start_block(end_label);
                    let load_val = self.push_inst(Instruction::Load {
                        ptr: Operand::Value(result_alloca),
                    }, ty_id);
                    Operand::Value(load_val)
                } else {
                    let left_op = self.generate_expr(left);
                    let right_op = self.generate_expr(right);
                    let mut bin_ty = ty_id;
                    if bin_ty == luna_semantic::SemanticTypeId(0) {
                        match op {
                            luna_ast::expr::BinaryOp::Eq | luna_ast::expr::BinaryOp::Ne |
                            luna_ast::expr::BinaryOp::Lt | luna_ast::expr::BinaryOp::Le |
                            luna_ast::expr::BinaryOp::Gt | luna_ast::expr::BinaryOp::Ge => {
                                bin_ty = self.ctx.types.bool_id();
                            }
                            _ => {
                                if let Operand::Value(v) = &left_op {
                                    bin_ty = self.current_function.as_ref().unwrap().values[v.0 as usize].ty;
                                } else if let Operand::Value(v) = &right_op {
                                    bin_ty = self.current_function.as_ref().unwrap().values[v.0 as usize].ty;
                                }
                            }
                        }
                    }
                    let inst = match op {
                        luna_ast::expr::BinaryOp::Add => Instruction::Add { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::Sub => Instruction::Sub { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::Mul => Instruction::Mul { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::Div => Instruction::Div { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::Mod => Instruction::Rem { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::Eq => Instruction::Eq { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::Ne => Instruction::NotEq { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::Lt => Instruction::LessThan { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::Le => Instruction::LessOrEq { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::Gt => Instruction::GreaterThan { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::Ge => Instruction::GreaterOrEq { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::BitAnd => Instruction::BitAnd { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::BitOr => Instruction::BitOr { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::BitXor => Instruction::BitXor { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::LShift => Instruction::Shl { left: left_op, right: right_op },
                        luna_ast::expr::BinaryOp::RShift => Instruction::Shr { left: left_op, right: right_op },
                        _ => panic!("Unsupported binary operator in MVIR generation: {:?}", op),
                    };
                    let val_id = self.push_inst(inst, bin_ty);
                    Operand::Value(val_id)
                }
            }
            Expr::Call { callee, args, .. } => {
                if let Some(&method_idx) = self.ctx.tables.dyn_method_indices.get(callee) {
                    let callee_expr = &self.arena.exprs[callee.0 as usize];
                    if let luna_ast::Expr::Member { object, .. } = callee_expr {
                        let obj_op = self.generate_expr(object);
                        let arg_ops = self.generate_call_arguments(*expr_id, args);
                        let call_val = self.push_inst(Instruction::CallVirt {
                            obj: obj_op,
                            method_idx,
                            args: arg_ops,
                        }, ty_id);
                        self.annotate_default_call(*expr_id, call_val);
                        return Operand::Value(call_val);
                    }
                }

                // Check if callee is a struct method call: obj.method(args...)
                let callee_expr = &self.arena.exprs[callee.0 as usize];
                if let luna_ast::Expr::Member { object, .. } = callee_expr {
                    if let Some(&m_sym) = self.ctx.tables.expr_symbols.get(callee) {
                        let mut m_name = self.ctx.symbol_table.get_symbol(m_sym).name.clone();
                        if let Some(canonical_name) = self.resolve_mono_call_name(expr_id, &m_name) {
                            m_name = canonical_name;
                        } else if let Some(decl_id) = self.ctx.symbol_table.get_symbol(m_sym).decl_id {
                            let canonical_id = luna_semantic::CanonicalInstanceIdentity {
                                kind: luna_semantic::CanonicalInstanceKind::Decl(decl_id),
                                subst: Vec::new(),
                            };
                            m_name = canonical_id.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, &m_name);
                        }
                        let obj_op = self.generate_expr(object);
                        let mut arg_ops = vec![obj_op];
                        arg_ops.extend(self.generate_call_arguments(*expr_id, args));
                        let call_val = self.push_inst(Instruction::CallDirect {
                            callee: GlobalId {
                                name: m_name,
                                symbol_id: self.call_symbol_id(expr_id, m_sym),
                            },
                            args: arg_ops,
                        }, ty_id);
                        self.annotate_default_call(*expr_id, call_val);
                        return Operand::Value(call_val);
                    }
                }
                
                let callee_op = self.generate_expr(callee);
                let arg_ops = self.generate_call_arguments(*expr_id, args);
                
                // Check if callee is an enum variant
                let is_variant = if let Some(sym_id) = self.ctx.tables.expr_symbols.get(callee) {
                    let symbol = self.ctx.symbol_table.get_symbol(*sym_id);
                    if let luna_semantic::SymbolKind::EnumVariant(idx) = symbol.kind {
                        Some((idx, symbol.decl_id.unwrap()))
                    } else { None }
                } else { None };

                if let Some((variant_idx, decl_id)) = is_variant {
                    let mut enum_ty = ty_id;
                    if enum_ty == luna_semantic::SemanticTypeId(0) || self.ctx.types.contains_inference_var(enum_ty) {
                        if let Some(func) = &self.current_function {
                            if matches!(self.ctx.types.get(func.ret_ty), luna_semantic::SemanticType::Enum(..)) && !self.ctx.types.contains_inference_var(func.ret_ty) {
                                enum_ty = func.ret_ty;
                            }
                        }
                    }
                    if enum_ty == luna_semantic::SemanticTypeId(0) || self.ctx.types.contains_inference_var(enum_ty) {
                        if let Some(enum_sym_id) = self.ctx.tables.decl_symbols.get(&decl_id) {
                            if let Some(ety) = self.get_symbol_type(enum_sym_id) {
                                enum_ty = ety;
                            }
                        }
                    }
                    let call_val = self.push_inst(Instruction::Variant {
                        enum_ty,
                        variant_idx,
                        args: arg_ops,
                    }, enum_ty);
                    self.annotate_default_call(*expr_id, call_val);
                    return Operand::Value(call_val);
                } else {
                    let callee_ty_id = self.ctx.tables.expr_types.get(callee).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                    let is_closure = matches!(self.ctx.types.get(callee_ty_id), luna_semantic::SemanticType::Closure(..));
                    
                    let intrinsic = match &callee_op {
                        Operand::Global(function) => function.symbol_id.and_then(|symbol| self.ctx.lang_items.from_symbol(symbol)),
                        _ => None,
                    };
                    use luna_semantic::lang_item::LangItem;
                    match intrinsic {
                        Some(LangItem::SliceFromRawParts | LangItem::SliceFromRawPartsMut) if arg_ops.len() == 2 => {
                            let value = self.push_inst(Instruction::MakeSlice {
                                data_ptr: arg_ops[0].clone(), len: arg_ops[1].clone(),
                            }, ty_id);
                            return Operand::Value(value);
                        }
                        Some(LangItem::DropInPlace) if arg_ops.len() == 1 => {
                            let argument_type = self.get_expr_type(&args[0].value);
                            if let luna_semantic::SemanticType::Pointer(_, element) = self.ctx.types.get(argument_type) {
                                let element = *element;
                                if self.ctx.needs_drop(element) {
                                    let callee = self.get_drop_glue_global_id(element);
                                    self.push_inst(Instruction::Drop { value: arg_ops[0].clone(), callee, ty: element }, element);
                                }
                                return Operand::Number("0".to_string());
                            }
                            self.diagnostics.push(luna_common::Diagnostic::error("drop intrinsic operand is not a checked pointer")
                                .with_code(luna_common::DiagnosticCode::BackendInvariantViolation));
                        }
                        Some(item) if item.is_memory_intrinsic() => {
                            self.diagnostics.push(luna_common::Diagnostic::error("memory intrinsic call has incorrect checked arity")
                                .with_code(luna_common::DiagnosticCode::BackendInvariantViolation));
                        }
                        _ => {}
                    }

                    let call_val = self.push_inst(if is_closure {
                        Instruction::CallClosure { closure: callee_op, args: arg_ops }
                    } else {
                        match callee_op {
                            Operand::Global(mut id) => {
                                if let Some(canonical_name) = self.resolve_mono_call_name(expr_id, &id.name) {
                                    id.name = canonical_name;
                                    id.symbol_id = id.symbol_id.and_then(|symbol| self.call_symbol_id(expr_id, symbol));
                                }
                                // Global operands already carry their canonical instance name.
                                // An immutable function value may select a generic instance at
                                // its initializer, without a new substitution at this call.
                                Instruction::CallDirect { callee: id, args: arg_ops }
                            }
                            _ => Instruction::CallIndirect { callee: callee_op, args: arg_ops },
                        }
                    }, ty_id);
                    self.annotate_default_call(*expr_id, call_val);
                    Operand::Value(call_val)
                }
            }
            Expr::Identifier { segments, .. } => {
                if let Some(sym_id) = self.ctx.tables.expr_symbols.get(expr_id).copied() {
                    if let Some(function) = self.known_function_values.get(&sym_id) {
                        return Operand::Global(function.clone());
                    }
                    if let Some(&val_id) = self.locals.get(&sym_id) {
                        // Let's check if the local is a pointer. Wait, locals is a map to Alloca.
                        // Wait, what if sym_id is EnumVariant? Locals won't have it.
                    }
                    
                    let symbol = self.ctx.symbol_table.get_symbol(sym_id);
                    if let luna_semantic::SymbolKind::EnumVariant(idx) = symbol.kind {
                        let mut concrete_enum_ty = ty_id;
                        if concrete_enum_ty == luna_semantic::SemanticTypeId(0) || self.ctx.types.contains_inference_var(concrete_enum_ty) {
                            if let Some(func) = &self.current_function {
                                if matches!(self.ctx.types.get(func.ret_ty), luna_semantic::SemanticType::Enum(..)) && !self.ctx.types.contains_inference_var(func.ret_ty) {
                                    concrete_enum_ty = func.ret_ty;
                                }
                            }
                        }
                        if concrete_enum_ty == luna_semantic::SemanticTypeId(0) || self.ctx.types.contains_inference_var(concrete_enum_ty) {
                            if let Some(decl_id) = symbol.decl_id {
                                if let Some(enum_sym_id) = self.ctx.tables.decl_symbols.get(&decl_id) {
                                    if let Some(enum_ty) = self.get_symbol_type(enum_sym_id) {
                                        concrete_enum_ty = enum_ty;
                                    }
                                }
                            }
                        }
                        let variant_val = self.push_inst(Instruction::Variant {
                            enum_ty: concrete_enum_ty,
                            variant_idx: idx,
                            args: Vec::new(),
                        }, concrete_enum_ty);
                        return Operand::Value(variant_val);
                    }
                    
                    let constant_sym = self.canonical_constant_symbol(sym_id);
                    if let Some(ct_val) = self.ctx.const_values.get(&constant_sym).cloned() {
                        let scope = self.ctx.symbol_table.get_symbol(constant_sym).scope;
                        if matches!(self.ctx.symbol_table.scopes[scope.0 as usize].kind,
                            luna_semantic::symbol::ScopeKind::Global | luna_semantic::symbol::ScopeKind::Module) {
                            let ptr = self.generate_lvalue(expr_id);
                            let constant_ty = self.get_symbol_type(&constant_sym).unwrap_or(ty_id);
                            return Operand::Value(self.push_inst(Instruction::Load { ptr }, constant_ty));
                        }
                        return self.materialize_comptime_value(&ct_val, ty_id);
                    }

                    if let Some(&val_id) = self.locals.get(&sym_id) {
                        let mut sym_ty = if ty_id != luna_semantic::SemanticTypeId(0) {
                            ty_id
                        } else {
                            self.get_symbol_type(&sym_id).unwrap_or(ty_id)
                        };
                        if sym_ty == luna_semantic::SemanticTypeId(0) {
                            sym_ty = self.current_function.as_ref().unwrap().values[val_id.0 as usize].ty;
                        }
                        let load_val = self.push_inst(Instruction::Load {
                            ptr: Operand::Value(val_id),
                        }, sym_ty);
                        return Operand::Value(load_val);
                    }
                    
                    let mut sym_name = if (sym_id.0 as usize) < self.ctx.symbol_table.symbols.len() {
                        self.ctx.symbol_table.symbols[sym_id.0 as usize].name.clone()
                    } else {
                        format!("global_{}", segments[0].start)
                    };
                    if let Some(decl_id) = symbol.decl_id {
                        if matches!(symbol.kind, luna_semantic::SymbolKind::Function) {
                            let subst = self.get_mono_call(expr_id).map(|instance| instance.subst.clone()).unwrap_or_default();
                            let canonical_id = luna_semantic::CanonicalInstanceIdentity {
                                kind: luna_semantic::CanonicalInstanceKind::Decl(decl_id),
                                subst,
                            };
                            sym_name = canonical_id.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, &sym_name);
                        }
                    }
                    return Operand::Global(GlobalId {
                        name: sym_name,
                        symbol_id: Some(sym_id),
                    });
                }
                Operand::Number("0".to_string())
            }
            Expr::Assign { op, lvalue, value } => {
                let ptr_op = self.generate_lvalue(lvalue);
                let val_op = self.generate_expr(value);
                let val_ty_id = self.ctx.tables.expr_types.get(value).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                let lvalue_ty_id = self.get_expr_type(lvalue);
                let store_ty_id = if lvalue_ty_id != luna_semantic::SemanticTypeId(0) {
                    lvalue_ty_id
                } else {
                    val_ty_id
                };

                let mut final_val = match op {
                    AssignOp::Assign => val_op,
                    _ => {
                        let current_val = self.push_inst(Instruction::Load {
                            ptr: ptr_op.clone(),
                        }, store_ty_id);
                        let inst = match op {
                            AssignOp::AddAssign => Instruction::Add { left: Operand::Value(current_val), right: val_op },
                            AssignOp::SubAssign => Instruction::Sub { left: Operand::Value(current_val), right: val_op },
                            AssignOp::MulAssign => Instruction::Mul { left: Operand::Value(current_val), right: val_op },
                            AssignOp::DivAssign => Instruction::Div { left: Operand::Value(current_val), right: val_op },
                            AssignOp::ModAssign => Instruction::Rem { left: Operand::Value(current_val), right: val_op },
                            AssignOp::BitAndAssign => Instruction::BitAnd { left: Operand::Value(current_val), right: val_op },
                            AssignOp::BitOrAssign => Instruction::BitOr { left: Operand::Value(current_val), right: val_op },
                            AssignOp::BitXorAssign => Instruction::BitXor { left: Operand::Value(current_val), right: val_op },
                            AssignOp::LShiftAssign => Instruction::Shl { left: Operand::Value(current_val), right: val_op },
                            AssignOp::RShiftAssign => Instruction::Shr { left: Operand::Value(current_val), right: val_op },
                            AssignOp::Assign => unreachable!(),
                        };
                        let res_val = self.push_inst(inst, store_ty_id);
                        Operand::Value(res_val)
                    }
                };

                if *op == AssignOp::Assign && self.ctx.needs_drop(store_ty_id)
                    && !self.is_raw_storage_lvalue(*lvalue)
                {
                    // Transfer the RHS before destroying the destination. This
                    // also lets move analysis suppress the old drop when the
                    // RHS moved from this same place (e.g. x = identity(x)).
                    let replacement = self.push_inst_span(
                        Instruction::Assign(final_val), store_ty_id, self.extract_expr_span(expr_id),
                    );
                    final_val = Operand::Value(replacement);
                    self.emit_place_cleanup(ptr_op.clone(), store_ty_id, self.extract_expr_span(expr_id));
                }
                self.push_inst_span(Instruction::Store {
                    ptr: ptr_op,
                    value: final_val,
                }, store_ty_id, self.extract_expr_span(expr_id));
                Operand::Number("0".to_string())
            }
            Expr::StructInit { fields, .. } => {
                let struct_alloca = self.push_inst(Instruction::Alloca, ty_id);
                // We use MarkInit if empty struct so it's considered initialized
                if fields.is_empty() {
                    self.push_inst(Instruction::MarkInit { value: Operand::Value(struct_alloca) }, ty_id);
                } else {
                    let indices_opt = self.ctx.tables.expr_struct_init_indices.get(expr_id).cloned();
                    let computed_indices: Vec<u32> = if let Some(indices) = indices_opt {
                        indices
                    } else {
                        let mut resolved_indices = Vec::with_capacity(fields.len());
                        let declared_fields_opt = match self.ctx.types.get(ty_id) {
                            luna_semantic::SemanticType::Struct(sym_id, _, _) => {
                                self.ctx.symbol_table.get_symbol(*sym_id).decl_id
                                    .and_then(|did| {
                                        if (did.0 as usize) < self.arena.decls.len() {
                                            if let Decl::Struct { fields: df, .. } = &self.arena.decls[did.0 as usize] {
                                                Some(df.clone())
                                            } else { None }
                                        } else { None }
                                    })
                            }
                            _ => None,
                        };
                        for (i, field) in fields.iter().enumerate() {
                            let idx = if let Some(ref df) = declared_fields_opt {
                                let f_name = self.get_span_text(field.name);
                                df.iter().position(|d| self.get_span_text(d.name) == f_name)
                                    .map(|pos| pos as u32)
                                    .unwrap_or(i as u32)
                            } else {
                                i as u32
                            };
                            resolved_indices.push(idx);
                        }
                        resolved_indices
                    };

                    for (i, field) in fields.iter().enumerate() {
                        let field_idx = computed_indices[i];
                        if field_idx != u32::MAX {
                            let val_op = self.generate_expr(&field.value);
                            // Ensure proper field ptr type
                            let ptr = self.push_inst(Instruction::FieldPtr {
                                base: Operand::Value(struct_alloca),
                                field_idx,
                                field_name: Some(self.get_span_text(field.name).to_string()),
                            }, ty_id);
                            
                            self.push_inst(Instruction::Store {
                                ptr: Operand::Value(ptr),
                                value: val_op,
                            }, self.ctx.types.bool_id());
                        }
                    }
                }
                let load_val = self.push_inst(Instruction::Load {
                    ptr: Operand::Value(struct_alloca),
                }, ty_id);
                Operand::Value(load_val)
            }
            Expr::TupleLiteral { elements } => {
                let tuple_alloca = self.push_inst(Instruction::Alloca, ty_id);
                if elements.is_empty() {
                    self.push_inst(Instruction::MarkInit { value: Operand::Value(tuple_alloca) }, ty_id);
                } else {
                    for (i, elem) in elements.iter().enumerate() {
                        let val_op = self.generate_expr(elem);
                        let ptr = self.push_inst(Instruction::FieldPtr {
                            base: Operand::Value(tuple_alloca),
                            field_idx: i as u32,
                            field_name: None,
                        }, ty_id);
                        
                        self.push_inst(Instruction::Store {
                            ptr: Operand::Value(ptr),
                            value: val_op,
                        }, self.ctx.types.bool_id());
                    }
                }
                let load_val = self.push_inst(Instruction::Load {
                    ptr: Operand::Value(tuple_alloca),
                }, ty_id);
                Operand::Value(load_val)
            }
            Expr::ArrayLiteral { elements } => {
                let array_alloca = self.push_inst(Instruction::Alloca, ty_id);
                if elements.is_empty() {
                    self.push_inst(Instruction::MarkInit { value: Operand::Value(array_alloca) }, ty_id);
                } else {
                    for (i, elem) in elements.iter().enumerate() {
                        let val_op = self.generate_expr(elem);
                        let ptr = self.push_inst(Instruction::FieldPtr {
                            base: Operand::Value(array_alloca),
                            field_idx: i as u32,
                            field_name: None,
                        }, ty_id);
                        
                        self.push_inst(Instruction::Store {
                            ptr: Operand::Value(ptr),
                            value: val_op,
                        }, self.ctx.types.bool_id());
                    }
                }
                let load_val = self.push_inst(Instruction::Load {
                    ptr: Operand::Value(array_alloca),
                }, ty_id);
                Operand::Value(load_val)
            }
            Expr::Member { .. } | Expr::TupleIndex { .. } => {
                let ptr_op = self.generate_lvalue(expr_id);
                let mut load_ty = ty_id;
                if load_ty == luna_semantic::SemanticTypeId(0) {
                    if let Operand::Value(v) = ptr_op {
                        let ptr_ty = self.current_function.as_ref().unwrap().values[v.0 as usize].ty;
                        load_ty = ptr_ty;
                    }
                }
                let load_val = self.push_inst(Instruction::Load {
                    ptr: ptr_op,
                }, load_ty);
                Operand::Value(load_val)
            }
            Expr::MethodCall { object, args, method_name, .. } => {
                if let Some(&idx) = self.ctx.tables.expr_member_indices.get(expr_id) {
                    let obj_ty_id = self.ctx.tables.expr_types.get(object).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                    let obj_ty = self.ctx.types.get(obj_ty_id);
                    let is_slice = match obj_ty {
                        luna_semantic::SemanticType::Reference(_, _, inner) | luna_semantic::SemanticType::Pointer(_, inner) => {
                            matches!(self.ctx.types.get(*inner), luna_semantic::SemanticType::Slice(_))
                        }
                        luna_semantic::SemanticType::Slice(_) => true,
                        _ => false,
                    };
                    if is_slice {
                        let lval = self.generate_lvalue(object);
                        let usize_ty = self.ctx.types.usize_id();
                        let len_ptr = self.push_inst(Instruction::FieldPtr {
                            base: lval.clone(),
                            field_idx: 1,
                            field_name: None,
                        }, usize_ty);
                        let len_val = self.push_inst(Instruction::Load {
                            ptr: Operand::Value(len_ptr),
                        }, usize_ty);
                        if idx == 1 {
                            return Operand::Value(len_val);
                        } else if idx == 2 {
                            let zero = Operand::Number("0".to_string());
                            let eq_val = self.push_inst(Instruction::Eq {
                                left: Operand::Value(len_val),
                                right: zero,
                            }, self.ctx.types.bool_id());
                            return Operand::Value(eq_val);
                        }
                    }
                }

                if let Some(&method_idx) = self.ctx.tables.dyn_method_indices.get(expr_id) {
                    let obj_op = self.generate_expr(object);
                    let arg_ops = self.generate_call_arguments(*expr_id, args);
                    let call_val = self.push_inst(Instruction::CallVirt {
                        obj: obj_op,
                        method_idx,
                        args: arg_ops,
                    }, ty_id);
                    self.annotate_default_call(*expr_id, call_val);
                    return Operand::Value(call_val);
                }

                let m_sym_opt = self.ctx.tables.expr_symbols.get(expr_id).copied().or_else(|| {
                    let m_name_str = self.get_span_text(*method_name);
                    let obj_ty_id = if let Some(inst_ptr) = self.current_instance {
                        let inst = unsafe { &*inst_ptr };
                        inst.expr_types.get(object).copied()
                    } else {
                        self.ctx.tables.expr_types.get(object).copied()
                    }.unwrap_or(luna_semantic::SemanticTypeId(0));
                    
                    let peeled_obj = match self.ctx.types.get(obj_ty_id) {
                        luna_semantic::SemanticType::Reference(_, _, inner) | luna_semantic::SemanticType::Pointer(_, inner) => *inner,
                        _ => obj_ty_id,
                    };
                    let peeled_ty = self.ctx.types.get(peeled_obj);
                    let target_key = match peeled_ty {
                        luna_semantic::SemanticType::Struct(s, ..) | luna_semantic::SemanticType::Enum(s, ..) => {
                            Some(luna_semantic::semantic_tables::ImplSelfTypeKey::Nominal(*s))
                        }
                        luna_semantic::SemanticType::Primitive(b) => {
                            Some(luna_semantic::semantic_tables::ImplSelfTypeKey::Primitive(*b))
                        }
                        luna_semantic::SemanticType::Slice(_) => {
                            Some(luna_semantic::semantic_tables::ImplSelfTypeKey::Slice)
                        }
                        _ => None,
                    };
                    if let Some(self_key) = target_key {
                        return self.ctx.tables
                            .find_method_prefer_inherent(self_key, &m_name_str, &self.ctx.symbol_table)
                            .map(|(method, _)| method);
                    }
                    self.ctx.symbol_table.symbols.iter().find(|s| s.name == m_name_str && matches!(s.kind, luna_semantic::symbol::SymbolKind::Function | luna_semantic::symbol::SymbolKind::TraitMethod)).map(|s| s.id)
                });

                if let Some(m_sym) = m_sym_opt {
                    let mut m_name = self.ctx.symbol_table.get_symbol(m_sym).name.clone();
                    if let Some(canonical_name) = self.resolve_mono_call_name(expr_id, &m_name) {
                        m_name = canonical_name;
                    } else if let Some(decl_id) = self.ctx.symbol_table.get_symbol(m_sym).decl_id {
                        let canonical_id = luna_semantic::CanonicalInstanceIdentity {
                            kind: luna_semantic::CanonicalInstanceKind::Decl(decl_id),
                            subst: Vec::new(),
                        };
                        m_name = canonical_id.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, &m_name);
                    }
                    let m_ty_opt = self.get_symbol_type(&m_sym);
                    let mut receiver_ty = luna_semantic::SemanticTypeId(0);
                    let (is_ref_self, is_rw_self) = if let Some(m_ty) = m_ty_opt {
                        if let luna_semantic::SemanticType::Function { params, .. } = self.ctx.types.get(m_ty) {
                            if let Some(&first_param) = params.first() {
                                receiver_ty = first_param;
                                match self.ctx.types.get(first_param) {
                                    luna_semantic::SemanticType::Reference(_, mutability, _) => (
                                        true,
                                        matches!(mutability, luna_semantic::ty::Mutability::Mutable),
                                    ),
                                    _ => (false, false),
                                }
                            } else { (false, false) }
                        } else { (false, false) }
                    } else { (false, false) };

                    let is_already_ref = matches!(
                        self.ctx.types.get(
                            self.ctx.tables.expr_types.get(object).copied().unwrap_or(luna_semantic::SemanticTypeId(0))
                        ),
                        luna_semantic::SemanticType::Reference(..) | luna_semantic::SemanticType::Pointer(..)
                    );

                    enum PreparedReceiver {
                        Place(Operand),
                        Value(Operand),
                    }

                    let prepared_receiver = if is_ref_self && !is_already_ref {
                        PreparedReceiver::Place(self.generate_lvalue(object))
                    } else {
                        PreparedReceiver::Value(self.generate_expr(object))
                    };

                    let arg_ops = self.generate_call_arguments(*expr_id, args);

                    let obj_op = match prepared_receiver {
                        PreparedReceiver::Place(place) => {
                            let borrow_val = self.push_inst(Instruction::Borrow { is_rw: is_rw_self, base: place }, receiver_ty);
                            Operand::Value(borrow_val)
                        }
                        PreparedReceiver::Value(value) => value,
                    };

                    let mut final_args = vec![obj_op];
                    final_args.extend(arg_ops);

                    let call_val = self.push_inst(Instruction::CallDirect {
                        callee: GlobalId {
                            name: m_name,
                            symbol_id: self.call_symbol_id(expr_id, m_sym),
                        },
                        args: final_args,
                    }, ty_id);
                    self.annotate_default_call(*expr_id, call_val);
                    return Operand::Value(call_val);
                }
                let method_text = self.get_span_text(*method_name);
                self.diagnostics.push(
                    luna_common::Diagnostic::error(format!(
                        "MVIR invariant violated: unresolved method `{}` reached lowering",
                        method_text
                    ))
                    .with_code(luna_common::DiagnosticCode::BackendInvariantViolation)
                    .with_span(*method_name),
                );
                Operand::Number("0".to_string())
            }
            Expr::Index { .. } => {
                let ptr_op = self.generate_lvalue(expr_id);
                let mut load_ty = ty_id;
                if load_ty == luna_semantic::SemanticTypeId(0) {
                    if let Operand::Value(v) = &ptr_op {
                        let ptr_ty = self.current_function.as_ref().unwrap().values[v.0 as usize].ty;
                        if let luna_semantic::SemanticType::Pointer(_, inner) | luna_semantic::SemanticType::Reference(_, _, inner) = self.ctx.types.get(ptr_ty) {
                            load_ty = *inner;
                        } else {
                            load_ty = ptr_ty;
                        }
                    }
                }
                let load_val = self.push_inst(Instruction::Load {
                    ptr: ptr_op,
                }, load_ty);
                Operand::Value(load_val)
            }
            Expr::Match { subject, arms, match_span: _ } => {
                let subject_op = self.generate_expr(subject);
                let match_ty_id = ty_id;
                let result_alloca = if match_ty_id == self.ctx.types.void_id() {
                    None
                } else {
                    Some(self.push_inst(Instruction::Alloca, match_ty_id))
                };
                let end_label = self.new_label("match_end");
                
                let mut next_arm_label = self.new_label("match_arm");
                let mut any_arm_reached = false;
                
                for (i, arm) in arms.iter().enumerate() {
                    self.terminate_block(Terminator::Br { target: next_arm_label.clone() });
                    self.start_block(next_arm_label.clone());
                    
                    next_arm_label = if i == arms.len() - 1 {
                        self.new_label("match_unreachable") // We assume exhaustion
                    } else {
                        self.new_label("match_arm")
                    };
                    
                    let is_match = self.generate_pat_match(&arm.pattern, &subject_op);
                    
                    let body_label = self.new_label("match_body");
                    self.terminate_block(Terminator::CondBr {
                        condition: is_match,
                        true_target: body_label.clone(),
                        false_target: next_arm_label.clone(),
                    });
                    
                    self.start_block(body_label);
                    
                    self.push_scope();
                    self.bind_pat_vars(&arm.pattern, &subject_op);
                    let body_op = self.generate_block_expr(&arm.body);
                    if self.current_block.is_some() {
                        any_arm_reached = true;
                        if let Some(result_alloca) = result_alloca {
                            self.push_inst(Instruction::Store {
                                ptr: Operand::Value(result_alloca),
                                value: body_op,
                            }, match_ty_id);
                        }
                        
                        self.pop_scope_and_drop(None);
                        self.terminate_block(Terminator::Br { target: end_label.clone() });
                    } else {
                        self.lexical_scopes.pop();
                        self.temporary_drop_scopes.pop();
                    }
                }
                
                self.start_block(next_arm_label);
                self.terminate_block(Terminator::Unreachable);
                
                if any_arm_reached {
                    self.start_block(end_label);

                    if let Some(result_alloca) = result_alloca {
                        let load_val = self.push_inst(Instruction::Load {
                            ptr: Operand::Value(result_alloca),
                        }, match_ty_id);
                        Operand::Value(load_val)
                    } else {
                        Operand::Number("0".to_string())
                    }
                } else {
                    Operand::Number("0".to_string())
                }
            }

            Expr::Lambda { .. } => {
                let env_ty_id = self.ctx.tables.closure_env_types.get(expr_id).copied().unwrap_or_else(|| panic!("ICE: Missing closure_env_types for closure ID {:?}", expr_id));
                let env_ptr = self.push_inst(Instruction::HeapAlloc, env_ty_id);
                
                let mut captures_info = Vec::new();
                if let Some(bindings) = self.ctx.tables.closure_capture_bindings.get(expr_id) {
                    for binding in bindings {
                        let local_val = self.locals.get(&binding.symbol).copied().unwrap_or(crate::ValueId(0));
                        captures_info.push(crate::CaptureInfo {
                            symbol: binding.symbol,
                            mode: binding.mode,
                            source: local_val,
                            env_field: binding.env_field,
                            ty: binding.ty,
                            env_ty: binding.env_ty,
                        });
                    }
                }
                
                let mut fn_name = format!("closure_{}", expr_id.0);
                if let Some(inst_ptr) = self.current_instance {
                    let inst = unsafe { &*inst_ptr };
                    if !inst.instance.subst.is_empty() {
                        let canonical_id = inst.instance.canonical_identity();
                        fn_name = canonical_id.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, &fn_name);
                    }
                }
                let func_id = GlobalId {
                    name: fn_name,
                    symbol_id: None,
                };
                
                let make_closure_val = self.push_inst(Instruction::MakeClosure {
                    func: func_id,
                    env_ptr: Operand::Value(env_ptr),
                    captures: captures_info,
                }, ty_id);
                Operand::Value(make_closure_val)
            }
            Expr::Await { expr } => {
                let fut_op = self.generate_expr(expr);
                let out_ty = self.ctx.tables.expr_types.get(expr_id).copied().unwrap_or(luna_semantic::SemanticTypeId(0));
                
                let await_val = self.push_inst(Instruction::Await { future: fut_op }, out_ty);
                Operand::Value(await_val)
            }
            Expr::Try { expr: inner, .. } => {
                let inner_op = self.generate_expr(inner);
                
                // 1. Get the branch method
                let branch_instance = self.get_try_calls(expr_id).and_then(|calls| calls.branch.clone());
                let branch_decl = branch_instance.as_ref().map(|call| call.decl_id)
                    .or_else(|| self.ctx.tables.try_branch_methods.get(expr_id).copied()).unwrap();
                let branch_sym = self.ctx.tables.decl_symbols.get(&branch_decl).copied().unwrap();
                let branch_ret_ty = self.get_try_calls(expr_id).and_then(|calls| calls.branch_return_type)
                    .or_else(|| self.ctx.tables.try_branch_return_types.get(expr_id).copied()).or_else(|| {
                    self.ctx.tables.symbol_types.get(&branch_sym).and_then(|&ty| {
                        match self.ctx.types.get(ty) {
                            luna_semantic::SemanticType::Function { return_type, .. } => Some(*return_type),
                            _ => None,
                        }
                    })
                }).unwrap_or(luna_semantic::SemanticTypeId(0));
                
                // Call `branch`
                let branch_base_name = self.ctx.symbol_table.get_symbol(branch_sym).name.clone();
                let canonical_branch_id = luna_semantic::CanonicalInstanceIdentity {
                    kind: luna_semantic::CanonicalInstanceKind::Decl(branch_decl),
                    subst: branch_instance.map(|call| call.subst).unwrap_or_else(|| {
                        self.ctx.tables.try_branch_substs.get(expr_id)
                            .map(|subst| subst.map.iter().map(|(&sym, &ty)| (sym, ty)).collect())
                            .unwrap_or_default()
                    }),
                };
                let branch_fn_name = canonical_branch_id.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, &branch_base_name);
                let flow_val = self.push_inst(Instruction::CallDirect {
                    callee: GlobalId {
                        name: branch_fn_name,
                        symbol_id: Some(branch_sym),
                    },
                    args: vec![inner_op.clone()],
                    
                }, branch_ret_ty);
                
                // Get ControlFlow's Break and Continue variants from LangItem!
                let break_sym = self.ctx.lang_items.get(luna_semantic::lang_item::LangItem::ControlFlowBreak).unwrap();
                let continue_sym = self.ctx.lang_items.get(luna_semantic::lang_item::LangItem::ControlFlowContinue).unwrap();
                
                let break_idx = match self.ctx.symbol_table.get_symbol(break_sym).kind {
                    luna_semantic::symbol::SymbolKind::EnumVariant(idx) => idx,
                    _ => panic!("Expected EnumVariant for ControlFlowBreak"),
                };
                let continue_idx = match self.ctx.symbol_table.get_symbol(continue_sym).kind {
                    luna_semantic::symbol::SymbolKind::EnumVariant(idx) => idx,
                    _ => panic!("Expected EnumVariant for ControlFlowContinue"),
                };
                
                let success_label = self.new_label("try_success");
                let failure_label = self.new_label("try_failure");
                
                // Tag of the flow_val
                let tag_ty = self.ctx.types.u32_id();
                let tag_val = self.push_inst(Instruction::Tag {
                    value: Operand::Value(flow_val),
                }, tag_ty);
                
                let is_failure = self.push_inst(Instruction::Eq {
                    left: Operand::Value(tag_val),
                    right: Operand::Number(break_idx.to_string()),
                }, self.ctx.types.bool_id());
                
                self.terminate_block(Terminator::CondBr {
                    condition: Operand::Value(is_failure),
                    true_target: failure_label.clone(),
                    false_target: success_label.clone(),
                });
                
                // -- Failure Path --
                self.start_block(failure_label);
                
                let flow_sem_ty = self.ctx.types.get(branch_ret_ty).clone();
                let residual_ty = if let luna_semantic::SemanticType::Enum(_, _, variant_tys) = &flow_sem_ty {
                    variant_tys[break_idx as usize]
                } else {
                    luna_semantic::SemanticTypeId(0)
                };
                
                let residual_val = self.push_inst(Instruction::Extract {
                    value: Operand::Value(flow_val),
                    variant_idx: break_idx,
                    field_idx: 0,
                }, residual_ty);
                
                // Call `FromResidual::from_residual`
                let from_residual_instance = self.get_try_calls(expr_id).and_then(|calls| calls.from_residual.clone());
                let from_residual_decl = from_residual_instance.as_ref().map(|call| call.decl_id)
                    .or_else(|| self.ctx.tables.try_from_residual_methods.get(expr_id).copied()).unwrap();
                let from_residual_sym = self.ctx.tables.decl_symbols.get(&from_residual_decl).copied().unwrap();
                let from_residual_base_name = self.ctx.symbol_table.get_symbol(from_residual_sym).name.clone();
                let canonical_fr_id = luna_semantic::CanonicalInstanceIdentity {
                    kind: luna_semantic::CanonicalInstanceKind::Decl(from_residual_decl),
                    subst: from_residual_instance.map(|call| call.subst).unwrap_or_else(|| {
                        self.ctx.tables.try_from_residual_substs.get(expr_id)
                            .map(|subst| subst.map.iter().map(|(&sym, &ty)| (sym, ty)).collect())
                            .unwrap_or_default()
                    }),
                };
                let from_residual_fn_name = canonical_fr_id.symbol_name_with_tables(&self.ctx.types, &self.ctx.symbol_table, &self.ctx.tables, &from_residual_base_name);
                let expected_ret_ty = self.current_function.as_ref().unwrap().ret_ty;
                
                let ret_val = self.push_inst(Instruction::CallDirect {
                    callee: GlobalId {
                        name: from_residual_fn_name,
                        symbol_id: Some(from_residual_sym),
                    },
                    args: vec![Operand::Value(residual_val)],
                    
                }, expected_ret_ty);
                
                self.emit_drops_up_to(0, None);
                self.terminate_block(Terminator::Ret { value: Some(Operand::Value(ret_val)) });
                
                // -- Success Path --
                self.start_block(success_label);
                
                let output_ty = if let luna_semantic::SemanticType::Enum(_, _, variant_tys) = &flow_sem_ty {
                    variant_tys[continue_idx as usize]
                } else {
                    luna_semantic::SemanticTypeId(0)
                };
                
                let output_val = self.push_inst(Instruction::Extract {
                    value: Operand::Value(flow_val),
                    variant_idx: continue_idx,
                    field_idx: 0,
                }, output_ty);
                
                Operand::Value(output_val)
            }
            Expr::Unary { op, operand } => {
                if *op == luna_ast::expr::UnaryOp::Neg {
                    if let Expr::Literal(token, text) = &self.arena.exprs[operand.0 as usize] {
                        if token.kind == luna_lexer::TokenKind::IntegerLiteral {
                            let literal = luna_lexer::literal::decode_integer(text).expect("validated integer literal");
                            let negative = (literal.magnitude as i128).wrapping_neg();
                            let constant = self.push_inst(Instruction::Assign(Operand::Number(negative.to_string())), ty_id);
                            return Operand::Value(constant);
                        }
                    }
                }
                use luna_ast::expr::UnaryOp;
                match op {
                    UnaryOp::Ref => {
                        let lval = self.generate_lvalue(operand);
                        let val = self.push_inst(Instruction::Borrow { is_rw: false, base: lval }, ty_id);
                        Operand::Value(val)
                    }
                    UnaryOp::RefMut => {
                        let lval = self.generate_lvalue(operand);
                        let val = self.push_inst(Instruction::Borrow { is_rw: true, base: lval }, ty_id);
                        Operand::Value(val)
                    }
                    UnaryOp::Deref | UnaryOp::DerefMut => {
                        let ptr_op = self.generate_expr(operand);
                        let mut load_ty = ty_id;
                        if load_ty == luna_semantic::SemanticTypeId(0) {
                            if let Operand::Value(v) = &ptr_op {
                                let ptr_ty = self.current_function.as_ref().unwrap().values[v.0 as usize].ty;
                                match self.ctx.types.get(ptr_ty) {
                                    luna_semantic::SemanticType::Pointer(_, inner) |
                                    luna_semantic::SemanticType::Reference(_, _, inner) => {
                                        load_ty = *inner;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        let val = self.push_inst(Instruction::Load { ptr: ptr_op }, load_ty);
                        Operand::Value(val)
                    }
                    UnaryOp::Neg => {
                        let val_op = self.generate_expr(operand);
                        let val = self.push_inst(Instruction::Neg { value: val_op }, ty_id);
                        Operand::Value(val)
                    }
                    _ => self.generate_expr(operand),
                }
            }
            Expr::Sizeof { target_type } => {
                let target_ty_id = self.resolve_ast_type(target_type);
                let usize_ty = self.ctx.types.usize_id();
                let res_ty = if ty_id != luna_semantic::SemanticTypeId(0) { ty_id } else { usize_ty };
                Operand::Value(self.push_inst(Instruction::SizeOf { ty: target_ty_id }, res_ty))
            }
            Expr::Alignof { target_type } => {
                let target_ty_id = self.resolve_ast_type(target_type);
                let usize_ty = self.ctx.types.usize_id();
                let res_ty = if ty_id != luna_semantic::SemanticTypeId(0) { ty_id } else { usize_ty };
                Operand::Value(self.push_inst(Instruction::AlignOf { ty: target_ty_id }, res_ty))
            }
            Expr::Comptime { .. } => {
                if let Some(ct_val) = self.ctx.comptime_values.get(expr_id).cloned() {
                    self.materialize_comptime_value(&ct_val, ty_id)
                } else {
                    Operand::Number("0".to_string())
                }
            }
            Expr::Cast { expr, target_type } => {
                let val_op = self.generate_expr(expr);
                
                let mut target_ty_id = if ty_id != luna_semantic::SemanticTypeId(0) {
                    ty_id
                } else {
                    self.resolve_ast_type(target_type)
                };
                if target_ty_id == luna_semantic::SemanticTypeId(0) {
                    target_ty_id = self.resolve_ast_type(target_type);
                }

                // A true semantic identity cast preserves the operand's place
                // and ownership. Introducing a detached Cast value here would
                // hide the source place from normal move/drop accounting.
                let source_ty = self.get_expr_type(expr);
                if self.ctx.types.resolve(source_ty) == self.ctx.types.resolve(target_ty_id) {
                    return val_op;
                }
                
                let val = self.push_inst(Instruction::Cast {
                    value: val_op,
                    target_ty: target_ty_id
                }, target_ty_id);
                Operand::Value(val)
            }
            _ => Operand::Number("0".to_string())
        }
    }

    pub fn materialize_comptime_value(&mut self, val: &luna_semantic::ComptimeValue, ty_id: luna_semantic::SemanticTypeId) -> Operand {
        match val {
            luna_semantic::ComptimeValue::Unit => Operand::Number("0".to_string()),
            luna_semantic::ComptimeValue::Bool(b) => Operand::Boolean(*b),
            luna_semantic::ComptimeValue::Int { val, .. } => {
                let constant = self.push_inst(Instruction::Assign(Operand::Number(val.to_string())), ty_id);
                Operand::Value(constant)
            },
            luna_semantic::ComptimeValue::Float { val, .. } => Operand::Number(val.to_string()),
            luna_semantic::ComptimeValue::Char(c) => Operand::Number((*c as u32).to_string()),
            luna_semantic::ComptimeValue::Str(s) => Operand::StringRef(luna_lexer::literal::quote_string(s)),
            luna_semantic::ComptimeValue::Tuple(elements) => {
                let alloca_val = self.push_inst(Instruction::Alloca, ty_id);
                for (i, elem) in elements.iter().enumerate() {
                    let elem_ty = match self.ctx.types.get(ty_id) {
                        luna_semantic::SemanticType::Tuple(tys) => tys.get(i).copied().unwrap_or(luna_semantic::SemanticTypeId(0)),
                        _ => luna_semantic::SemanticTypeId(0),
                    };
                    let elem_op = self.materialize_comptime_value(elem, elem_ty);
                    let ptr = self.push_inst(Instruction::FieldPtr {
                        base: Operand::Value(alloca_val),
                        field_idx: i as u32,
                        field_name: None,
                    }, elem_ty);
                    self.push_inst(Instruction::Store {
                        ptr: Operand::Value(ptr),
                        value: elem_op,
                    }, elem_ty);
                }
                let load_val = self.push_inst(Instruction::Load {
                    ptr: Operand::Value(alloca_val),
                }, ty_id);
                Operand::Value(load_val)
            }
            luna_semantic::ComptimeValue::Array { elements, elem_ty } => {
                let alloca_val = self.push_inst(Instruction::Alloca, ty_id);
                if elements.is_empty() {
                    self.push_inst(Instruction::MarkInit { value: Operand::Value(alloca_val) }, ty_id);
                }
                let e_ty = elem_ty.unwrap_or(luna_semantic::SemanticTypeId(0));
                for (i, elem) in elements.iter().enumerate() {
                    let elem_op = self.materialize_comptime_value(elem, e_ty);
                    let ptr = self.push_inst(Instruction::FieldPtr {
                        base: Operand::Value(alloca_val),
                        field_idx: i as u32,
                        field_name: None,
                    }, e_ty);
                    self.push_inst(Instruction::Store {
                        ptr: Operand::Value(ptr),
                        value: elem_op,
                    }, e_ty);
                }
                let load_val = self.push_inst(Instruction::Load {
                    ptr: Operand::Value(alloca_val),
                }, ty_id);
                Operand::Value(load_val)
            }
            luna_semantic::ComptimeValue::Struct { fields, .. } => {
                let alloca_val = self.push_inst(Instruction::Alloca, ty_id);
                for (i, (_, f_val)) in fields.iter().enumerate() {
                    let f_ty = luna_semantic::SemanticTypeId(0);
                    let elem_op = self.materialize_comptime_value(f_val, f_ty);
                    let ptr = self.push_inst(Instruction::FieldPtr {
                        base: Operand::Value(alloca_val),
                        field_idx: i as u32,
                        field_name: None,
                    }, f_ty);
                    self.push_inst(Instruction::Store {
                        ptr: Operand::Value(ptr),
                        value: elem_op,
                    }, f_ty);
                }
                let load_val = self.push_inst(Instruction::Load {
                    ptr: Operand::Value(alloca_val),
                }, ty_id);
                Operand::Value(load_val)
            }
            luna_semantic::ComptimeValue::Enum { variant_index, payload, .. } => {
                let mut payload_ops = Vec::new();
                for p in payload {
                    payload_ops.push(self.materialize_comptime_value(p, luna_semantic::SemanticTypeId(0)));
                }
                let var_val = self.push_inst(Instruction::Variant {
                    enum_ty: ty_id,
                    variant_idx: *variant_index,
                    args: payload_ops,
                }, ty_id);
                Operand::Value(var_val)
            }
            _ => Operand::Number("0".to_string()),
        }
    }
    
    // --- Helpers ---
    
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
            insts: Vec::new(),
            terminator: None,
        });
    }
    
    fn extract_expr_span(&self, expr_id: &luna_ast::ExprId) -> Option<luna_common::Span> {
        match &self.arena.exprs[expr_id.0 as usize] {
            luna_ast::Expr::Literal(tok, _) => Some(tok.span.clone()),
            luna_ast::Expr::Identifier { segments, .. } => segments.last().copied(),
            luna_ast::Expr::MethodCall { method_name, .. } => Some(method_name.clone()),
            luna_ast::Expr::Member { member, .. } => Some(member.clone()),
            luna_ast::Expr::Match { match_span, .. } => Some(match_span.clone()),
            luna_ast::Expr::Try { try_span, .. } => Some(try_span.clone()),
            luna_ast::Expr::StructInit { path, .. } => path.last().copied(),
            luna_ast::Expr::Binary { left, .. } => self.extract_expr_span(left),
            luna_ast::Expr::Unary { operand, .. } => self.extract_expr_span(operand),
            luna_ast::Expr::Assign { lvalue, .. } => self.extract_expr_span(lvalue),
            luna_ast::Expr::Call { callee, .. } => self.extract_expr_span(callee),
            luna_ast::Expr::Index { base, .. } => self.extract_expr_span(base),
            luna_ast::Expr::TupleIndex { object, .. } => self.extract_expr_span(object),
            luna_ast::Expr::Cast { expr, .. } => self.extract_expr_span(expr),
            _ => None,
        }
    }

    fn push_inst_span(&mut self, inst: Instruction, ty: luna_semantic::SemanticTypeId, span: Option<luna_common::Span>) -> ValueId {
        let inst = match (self.unsafe_depth > 0, inst) {
            (true, Instruction::Store { ptr, value }) => Instruction::StoreAnchored { ptr, value },
            (_, other) => other,
        };
        let func = self.current_function.as_mut().expect("Must be in a function");
        let val_id = ValueId(func.values.len() as u32);
        let origin = match &inst {
            Instruction::Alloca => ValueOrigin::Local,
            Instruction::StaticAddress(_) => ValueOrigin::Global,
            _ => ValueOrigin::Temporary,
        };
        func.values.push(ValueData { inst, ty, span, origin });
        
        if let Some(block) = &mut self.current_block {
            block.insts.push(val_id);
        }
        val_id
    }

    fn push_inst(&mut self, inst: Instruction, ty: luna_semantic::SemanticTypeId) -> ValueId {
        let span = self.current_span.clone();
        self.push_inst_span(inst, ty, span)
    }

    
    fn terminate_block(&mut self, term: Terminator) {
        if let Some(mut block) = self.current_block.take() {
            if block.terminator.is_none() {
                block.terminator = Some(term);
            }
            self.current_function.as_mut().unwrap().blocks.push(block);
        }
    }
    
    /// Return the tag of a pattern only when semantic resolution has identified
    /// it as an enum variant. Never infer variant identity from its source name:
    /// distinct enums may legitimately declare variants with the same name.
    fn resolved_pattern_variant_index(&self, pat: &luna_ast::PatId) -> Option<u32> {
        let symbol = self.ctx.tables.pat_symbols.get(pat).copied()?;
        match self.ctx.symbol_table.get_symbol(symbol).kind {
            luna_semantic::SymbolKind::EnumVariant(index) => Some(index),
            _ => None,
        }
    }

    fn generate_pat_match(&mut self, pat: &luna_ast::PatId, subject: &Operand) -> Operand {
        use luna_ast::Pattern;
        let pattern = &self.arena.pats[pat.0 as usize];
        match pattern {
            Pattern::Wildcard => Operand::Boolean(true),
            Pattern::Identifier { .. } => {
                if let Some(variant_idx) = self.resolved_pattern_variant_index(pat) {
                    let tag_ty = self.ctx.types.u32_id();
                    let tag_val = self.push_inst(
                        Instruction::Tag {
                            value: subject.clone(),
                        },
                        tag_ty,
                    );
                    let eq_val = self.push_inst(
                        Instruction::Eq {
                            left: Operand::Value(tag_val),
                            right: Operand::Number(variant_idx.to_string()),
                        },
                        self.ctx.types.bool_id(),
                    );
                    return Operand::Value(eq_val);
                }
                Operand::Boolean(true)
            }
            Pattern::Enum { fields, .. } => {
                let variant_idx = self.resolved_pattern_variant_index(pat)
                    .unwrap_or_else(|| panic!("MVIR invariant: enum pattern has no resolved enum variant symbol"));
                
                let tag_ty = self.ctx.types.u32_id();
                let tag_val = self.push_inst(Instruction::Tag {
                    value: subject.clone(),
                }, tag_ty);
                
                let expected_tag = Operand::Number(variant_idx.to_string());
                
                let eq_val = self.push_inst(Instruction::Eq {
                    left: Operand::Value(tag_val),
                    right: expected_tag,
                }, self.ctx.types.bool_id());
                
                let mut current_res = Operand::Value(eq_val);
                
                for (field_idx, field_pat) in fields.iter().enumerate() {
                    let child = &self.arena.pats[field_pat.0 as usize];
                    let is_binding = matches!(child, Pattern::Identifier { .. })
                        && self.resolved_pattern_variant_index(field_pat).is_none();
                    if matches!(child, Pattern::Wildcard) || is_binding {
                        continue;
                    }
                    let field_ty = self.get_pat_type(field_pat).unwrap_or(self.ctx.types.bool_id());
                    let extracted = self.push_inst(Instruction::Extract {
                        value: subject.clone(),
                        variant_idx,
                        field_idx: field_idx as u32,
                    }, field_ty);
                    
                    let field_match = self.generate_pat_match(field_pat, &Operand::Value(extracted));
                    let and_val = self.push_inst(Instruction::BitAnd {
                        left: current_res.clone(),
                        right: field_match,
                    }, luna_semantic::SemanticTypeId(0));
                    current_res = Operand::Value(and_val);
                }
                
                current_res
            }
            Pattern::Literal(token) => {
                let text = self.get_span_text(token.span).to_string();
                let expected = match token.kind {
                    luna_lexer::TokenKind::KwTrue => Operand::Boolean(true),
                    luna_lexer::TokenKind::KwFalse => Operand::Boolean(false),
                    luna_lexer::TokenKind::StringLiteral => Operand::StringRef(text),
                    luna_lexer::TokenKind::CharLiteral => Operand::Char(text),
                    luna_lexer::TokenKind::IntegerLiteral => {
                        let number = luna_lexer::literal::decode_integer(&text).expect("validated integer pattern");
                        let ty = self.get_pat_type(pat).expect("typed literal pattern");
                        let constant = self.push_inst(Instruction::Assign(Operand::Number(number.magnitude.to_string())), ty);
                        Operand::Value(constant)
                    }
                    _ => Operand::Number(text),
                };
                
                let eq_val = self.push_inst(Instruction::Eq {
                    left: subject.clone(),
                    right: expected,
                }, self.ctx.types.bool_id());
                
                Operand::Value(eq_val)
            }
            Pattern::Tuple { elements, .. } => {
                let mut current_res = Operand::Boolean(true);
                for (i, elem) in elements.iter().enumerate() {
                    let extracted = self.push_inst(Instruction::Extract {
                        value: subject.clone(),
                        variant_idx: 0,
                        field_idx: i as u32,
                    }, luna_semantic::SemanticTypeId(0));
                    
                    let field_match = self.generate_pat_match(elem, &Operand::Value(extracted));
                    let and_val = self.push_inst(Instruction::BitAnd {
                        left: current_res.clone(),
                        right: field_match,
                    }, luna_semantic::SemanticTypeId(0));
                    current_res = Operand::Value(and_val);
                }
                current_res
            }
            Pattern::Struct { fields, .. } => {
                let mut current_res = Operand::Boolean(true);
                if let Some(&pat_ty_id) = self.ctx.tables.pat_types.get(pat) {
                    let sem_ty = self.ctx.types.get(pat_ty_id);
                    if let luna_semantic::SemanticType::Struct(sym_id, _, field_tys) = sem_ty {
                        let struct_sym = self.ctx.symbol_table.get_symbol(*sym_id);
                        if let Some(decl_id) = struct_sym.decl_id {
                            if let luna_ast::Decl::Struct { fields: decl_fields, .. } = &self.arena.decls[decl_id.0 as usize] {
                                for struct_field in fields {
                                    let field_name = self.get_span_text(struct_field.name);
                                    let mut found_idx = None;
                                    for (i, f) in decl_fields.iter().enumerate() {
                                        let f_name = self.get_span_text(f.name);
                                        if f_name == field_name {
                                            found_idx = Some(i);
                                            break;
                                        }
                                    }
                                    if let Some(field_idx) = found_idx {
                                        if let Some(ref field_pat) = struct_field.pattern {
                                            let extracted = self.push_inst(Instruction::Extract {
                                                value: subject.clone(),
                                                variant_idx: 0,
                                                field_idx: field_idx as u32,
                                            }, field_tys[field_idx]);
                                            
                                            let field_match = self.generate_pat_match(field_pat, &Operand::Value(extracted));
                                            let and_val = self.push_inst(Instruction::BitAnd {
                                                left: current_res.clone(),
                                                right: field_match,
                                            }, luna_semantic::SemanticTypeId(0));
                                            current_res = Operand::Value(and_val);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                current_res
            }
            _ => Operand::Boolean(false),
        }
    }
    
    fn bind_pat_vars(&mut self, pat: &luna_ast::PatId, subject: &Operand) {
        use luna_ast::Pattern;
        let pattern = &self.arena.pats[pat.0 as usize];
        match pattern {
            Pattern::Identifier { .. } => {
                if let Some(sym_id) = self.ctx.tables.pat_symbols.get(pat).copied() {
                    let symbol = self.ctx.symbol_table.get_symbol(sym_id);
                    if let luna_semantic::SymbolKind::EnumVariant(_) = symbol.kind {
                        // Enum variant matches do not bind a local variable.
                    } else if symbol.name == "_" {
                        // Wildcard identifier "_" does not bind a local variable.
                    } else if let Operand::Value(val_id) = subject {
                        let ty_id = self.get_symbol_type(&sym_id)
                            .or_else(|| self.get_pat_type(pat))
                            .unwrap_or(luna_semantic::SemanticTypeId(0));
                        let alloca_val = self.push_inst(Instruction::Alloca, ty_id);
                        self.locals.insert(sym_id, alloca_val);
                        if let Some(scope) = self.lexical_scopes.last_mut() {
                            scope.push(sym_id);
                        }
                        
                        self.push_inst(Instruction::Store {
                            ptr: Operand::Value(alloca_val),
                            value: Operand::Value(*val_id),
                        }, ty_id);
                    }
                }
            }
            Pattern::Enum { fields, .. } => {
                let variant_idx = self.resolved_pattern_variant_index(pat)
                    .unwrap_or_else(|| panic!("MVIR invariant: enum pattern has no resolved enum variant symbol"));
                
                if fields.is_empty() {
                    self.push_inst(Instruction::Assign(subject.clone()), luna_semantic::SemanticTypeId(0));
                }
                for (field_idx, field) in fields.iter().enumerate() {
                    let field_ty = self.get_pat_type(field).unwrap_or(luna_semantic::SemanticTypeId(0));
                    let extracted = self.push_inst(Instruction::Extract {
                        value: subject.clone(),
                        variant_idx,
                        field_idx: field_idx as u32,
                    }, field_ty);
                    
                    self.bind_pat_vars(field, &Operand::Value(extracted));
                }
            }
            Pattern::Tuple { elements, .. } => {
                for (i, elem) in elements.iter().enumerate() {
                    let field_ty = self.get_pat_type(elem).unwrap_or(luna_semantic::SemanticTypeId(0));
                    let extracted = self.push_inst(Instruction::Extract {
                        value: subject.clone(),
                        variant_idx: 0,
                        field_idx: i as u32,
                    }, field_ty);
                    
                    self.bind_pat_vars(elem, &Operand::Value(extracted));
                }
            }
            Pattern::Struct { fields, .. } => {
                if let Some(&pat_ty_id) = self.ctx.tables.pat_types.get(pat) {
                    let sem_ty = self.ctx.types.get(pat_ty_id);
                    if let luna_semantic::SemanticType::Struct(sym_id, _, field_tys) = sem_ty {
                        let struct_sym = self.ctx.symbol_table.get_symbol(*sym_id);
                        if let Some(decl_id) = struct_sym.decl_id {
                            if let luna_ast::Decl::Struct { fields: decl_fields, .. } = &self.arena.decls[decl_id.0 as usize] {
                                for struct_field in fields {
                                    let field_name = self.get_span_text(struct_field.name);
                                    let mut found_idx = None;
                                    for (i, f) in decl_fields.iter().enumerate() {
                                        let f_name = self.get_span_text(f.name);
                                        if f_name == field_name {
                                            found_idx = Some(i);
                                            break;
                                        }
                                    }
                                    if let Some(field_idx) = found_idx {
                                        if let Some(ref field_pat) = struct_field.pattern {
                                            let extracted = self.push_inst(Instruction::Extract {
                                                value: subject.clone(),
                                                variant_idx: 0,
                                                field_idx: field_idx as u32,
                                            }, field_tys[field_idx]);
                                            
                                            self.bind_pat_vars(field_pat, &Operand::Value(extracted));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
