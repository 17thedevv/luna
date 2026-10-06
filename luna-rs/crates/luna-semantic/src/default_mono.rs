use super::*;

impl MonoCollector<'_> {
    pub(super) fn attach_call_defaults(&mut self, expression: ExprId, instance: &mut MonoInstance) {
        let Some(binding) = self
            .ctx
            .tables
            .call_argument_bindings
            .get(&expression)
            .and_then(|binding| binding.defaults.clone())
        else {
            return;
        };
        let mut source_subst = instance.subst.clone();
        if binding.declaration != instance.decl_id {
            let source_symbol = self.ctx.tables.decl_symbols[&binding.declaration];
            let trait_owner = self
                .ctx
                .tables
                .trait_methods
                .iter()
                .find(|(_, methods)| methods.contains(&source_symbol))
                .map(|(&owner, _)| owner);
            let target_symbol = self.ctx.tables.decl_symbols[&instance.decl_id];
            let implementation = self
                .ctx
                .tables
                .method_sym_to_impl_decl
                .get(&target_symbol)
                .copied()
                .or_else(|| {
                    self.ctx
                        .tables
                        .method_to_impl_decl
                        .get(&instance.decl_id)
                        .copied()
                });
            let mut selected = Substitution::new();
            for &(symbol, ty) in &instance.subst {
                selected.insert(symbol, ty);
            }
            source_subst.clear();
            if let (Some(owner), Some(implementation)) = (trait_owner, implementation) {
                if let Some(header) = self
                    .ctx
                    .tables
                    .checked_impl_headers
                    .get(&implementation)
                    .cloned()
                {
                    source_subst.push((owner, self.ctx.types.subst(header.self_type, &selected)));
                }
                if let Some(entry) = self
                    .ctx
                    .tables
                    .trait_impl_entries
                    .iter()
                    .find(|entry| entry.decl_id == Some(implementation) && entry.trait_id == owner)
                    .cloned()
                {
                    let parameters = self
                        .ctx
                        .tables
                        .trait_generic_params
                        .get(&owner)
                        .cloned()
                        .unwrap_or_default();
                    for (symbol, ty) in parameters.into_iter().zip(entry.trait_args) {
                        source_subst.push((symbol, self.ctx.types.subst(ty, &selected)));
                    }
                }
            }
            let mut ordinal = 0;
            while let Some(&source) = self
                .ctx
                .tables
                .generic_param_symbols
                .get(&(binding.declaration, ordinal))
            {
                if let Some(&target) = self
                    .ctx
                    .tables
                    .generic_param_symbols
                    .get(&(instance.decl_id, ordinal))
                {
                    if let Some(&(_, ty)) =
                        instance.subst.iter().find(|(symbol, _)| *symbol == target)
                    {
                        source_subst.push((source, ty));
                    }
                }
                ordinal += 1;
            }
        }
        source_subst.sort_by_key(|&(symbol, _)| symbol);
        instance.defaults = Some(MonoDefaults {
            declaration: binding.declaration,
            omitted: binding.omitted,
            subst: source_subst,
        });
    }

    pub(super) fn visit_parameter_defaults(&mut self, instance: &MonoInstance) {
        let Some(defaults) = &instance.defaults else {
            return;
        };
        let Decl::Function {
            params: source_parameters,
            ..
        } = &self.arena.decls[defaults.declaration.0 as usize]
        else {
            panic!("ICE: default source is not a function declaration");
        };
        let Decl::Function {
            params: target_parameters,
            ..
        } = &self.arena.decls[instance.decl_id.0 as usize]
        else {
            panic!("ICE: omission target is not a function declaration");
        };
        let source_parameters = source_parameters.clone();
        let target_parameters = target_parameters.clone();
        assert_eq!(
            source_parameters.len(),
            target_parameters.len(),
            "ICE: default parameter role mismatch"
        );
        for (&source, &target) in source_parameters.iter().zip(&target_parameters) {
            let source_symbol = self.ctx.tables.decl_symbols[&source];
            let target_symbol = self.ctx.tables.decl_symbols[&target];
            let ty = self.ctx.tables.symbol_types[&target_symbol];
            let ty = self.substitute(ty);
            self.current_symbol_types.insert(source_symbol, ty);
        }
        let body_subst = self.current_subst.clone();
        for &(symbol, ty) in &defaults.subst {
            self.current_subst.insert(symbol, ty);
        }
        for &ordinal in &defaults.omitted {
            let Decl::Param {
                default: Some(default),
                ..
            } = &self.arena.decls[source_parameters[ordinal as usize].0 as usize]
            else {
                panic!("ICE: omitted parameter has no checked default");
            };
            let expression = default.value;
            self.visit_expr(&expression);
        }
        self.current_subst = body_subst;
    }
}
