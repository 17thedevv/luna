//! Trait-bound proofs include the premises of a matched blanket implementation.
use super::TypeChecker;
use crate::ty::{SemanticType, SemanticTypeId, Substitution};
use luna_common::ids::SymbolId;

type Goal = (SemanticTypeId, SymbolId, Vec<SemanticTypeId>);

impl<'a> TypeChecker<'a> {
    pub(super) fn prove_trait_bound(
        &mut self,
        receiver: SemanticTypeId,
        trait_id: SymbolId,
        arguments: &[SemanticTypeId],
    ) -> bool {
        self.prove_trait_goal(receiver, trait_id, arguments, &mut Vec::new())
    }

    fn prove_trait_goal(
        &mut self,
        receiver: SemanticTypeId,
        trait_id: SymbolId,
        arguments: &[SemanticTypeId],
        stack: &mut Vec<Goal>,
    ) -> bool {
        let receiver = self.ctx.types.resolve(receiver);
        let arguments: Vec<_> = arguments
            .iter()
            .map(|&ty| self.ctx.types.resolve(ty))
            .collect();
        let goal = (receiver, trait_id, arguments.clone());
        // A recursive assumption is not evidence. Bound expansion must also
        // terminate for goals whose structural type grows on every recursion.
        if stack.contains(&goal) || stack.len() >= 64 {
            return false;
        }
        stack.push(goal);
        let mut alternatives = Vec::new();
        if let SemanticType::GenericParam(parameter) = self.ctx.types.get(receiver) {
            for bound in self
                .ctx
                .tables
                .trait_bounds
                .get(parameter)
                .into_iter()
                .flatten()
            {
                if bound.trait_id == trait_id && bound.trait_args.len() == arguments.len() {
                    alternatives.push((bound.trait_args.clone(), Vec::new(), Substitution::new()));
                }
            }
        }
        for entry in &self.ctx.tables.trait_impl_entries {
            if entry.trait_id != trait_id || entry.trait_args.len() != arguments.len() {
                continue;
            }
            let mut subst = Substitution::new();
            if self.ctx.matches_impl_pattern(
                entry.self_type,
                receiver,
                &entry.generic_params,
                &mut subst,
            ) {
                alternatives.push((
                    entry.trait_args.clone(),
                    entry.generic_params.clone(),
                    subst,
                ));
            }
        }
        for (provided_arguments, parameters, mut subst) in alternatives {
            let saved_bindings = self.ctx.types.inference_bindings.clone();
            let saved_obligations = self.associated_type_obligations.len();
            let saved_diagnostics = self.ctx.diagnostics.len();
            let mut compatible = true;
            for (&required, &provided) in arguments.iter().zip(&provided_arguments) {
                // Trait arguments can constrain an impl binder even when the
                // receiver header does not mention that binder.
                self.ctx
                    .matches_impl_pattern(provided, required, &parameters, &mut subst);
                let provided = self.ctx.types.subst(provided, &subst);
                if !self.unify_bound_argument(required, provided) {
                    compatible = false;
                    break;
                }
            }
            if compatible {
                for parameter in parameters {
                    let Some(&actual) = subst.get(parameter) else {
                        compatible = false;
                        break;
                    };
                    let bounds = self
                        .ctx
                        .tables
                        .trait_bounds
                        .get(&parameter)
                        .cloned()
                        .unwrap_or_default();
                    for bound in bounds {
                        let required_arguments: Vec<_> = bound
                            .trait_args
                            .iter()
                            .map(|&ty| self.ctx.types.subst(ty, &subst))
                            .collect();
                        if !self.prove_trait_goal(
                            actual,
                            bound.trait_id,
                            &required_arguments,
                            stack,
                        ) {
                            compatible = false;
                            break;
                        }
                    }
                    if compatible {
                        let associated = self
                            .ctx
                            .tables
                            .assoc_type_bounds
                            .get(&parameter)
                            .cloned()
                            .unwrap_or_default();
                        for (owner, associated, expected) in associated {
                            let expected = self.ctx.types.subst(expected, &subst);
                            if !self.prove_associated_bound(actual, owner, associated, expected) {
                                compatible = false;
                                break;
                            }
                        }
                    }
                    if !compatible {
                        break;
                    }
                }
            }
            if compatible {
                stack.pop();
                return true;
            }
            self.ctx.types.inference_bindings = saved_bindings;
            self.associated_type_obligations.truncate(saved_obligations);
            self.ctx.diagnostics.truncate(saved_diagnostics);
        }
        stack.pop();
        false
    }

    pub(super) fn prove_associated_bound(
        &mut self,
        receiver: SemanticTypeId,
        owner: SymbolId,
        associated: SymbolId,
        expected: SemanticTypeId,
    ) -> bool {
        let receiver = self.ctx.types.resolve(receiver);
        if let SemanticType::GenericParam(parameter) = self.ctx.types.get(receiver) {
            let available = self
                .ctx
                .tables
                .assoc_type_bounds
                .get(parameter)
                .cloned()
                .unwrap_or_default();
            return available.into_iter().any(|(trait_id, name, actual)| {
                trait_id == owner
                    && name == associated
                    && self.unify_bound_argument(expected, actual)
            });
        }
        let diagnostic_start = self.ctx.diagnostics.len();
        let actual =
            self.normalize_projection(receiver, owner, associated, luna_common::Span::default());
        self.ctx.diagnostics.len() == diagnostic_start
            && !matches!(
                self.ctx.types.get(actual),
                SemanticType::Error | SemanticType::Projection { .. }
            )
            && self.unify_bound_argument(expected, actual)
    }

    // Trait evidence must preserve rigid caller binders. Ordinary inference
    // variables can still be solved, but two unrelated symbolic arguments are
    // not evidence of the same trait instantiation.
    fn unify_bound_argument(&mut self, expected: SemanticTypeId, actual: SemanticTypeId) -> bool {
        let expected = self.ctx.types.resolve(expected);
        let actual = self.ctx.types.resolve(actual);
        let left = self.ctx.types.get(expected).clone();
        let right = self.ctx.types.get(actual).clone();
        match (left, right) {
            (SemanticType::Error, _) | (_, SemanticType::Error) => false,
            (SemanticType::InferenceVar(_), _) | (_, SemanticType::InferenceVar(_)) => {
                self.unify(expected, actual).is_ok()
            }
            (SemanticType::GenericParam(left), SemanticType::GenericParam(right)) => left == right,
            (SemanticType::GenericParam(_), _) | (_, SemanticType::GenericParam(_)) => false,
            (SemanticType::Struct(left, largs, _), SemanticType::Struct(right, rargs, _))
            | (SemanticType::Enum(left, largs, _), SemanticType::Enum(right, rargs, _)) => {
                left == right
                    && largs.len() == rargs.len()
                    && largs
                        .into_iter()
                        .zip(rargs)
                        .all(|(left, right)| self.unify_bound_argument(left, right))
            }
            (SemanticType::Tuple(left), SemanticType::Tuple(right)) => {
                left.len() == right.len()
                    && left
                        .into_iter()
                        .zip(right)
                        .all(|(left, right)| self.unify_bound_argument(left, right))
            }
            (SemanticType::Reference(_, lm, left), SemanticType::Reference(_, rm, right))
            | (SemanticType::Pointer(lm, left), SemanticType::Pointer(rm, right)) => {
                lm == rm && self.unify_bound_argument(left, right)
            }
            (SemanticType::Slice(left), SemanticType::Slice(right)) => {
                self.unify_bound_argument(left, right)
            }
            (SemanticType::Array(left, ln), SemanticType::Array(right, rn)) => {
                ln == rn && self.unify_bound_argument(left, right)
            }
            (
                SemanticType::Function {
                    params: left,
                    return_type: lr,
                },
                SemanticType::Function {
                    params: right,
                    return_type: rr,
                },
            ) => {
                left.len() == right.len()
                    && left
                        .into_iter()
                        .zip(right)
                        .all(|(left, right)| self.unify_bound_argument(left, right))
                    && self.unify_bound_argument(lr, rr)
            }
            _ => self.unify(expected, actual).is_ok(),
        }
    }
}
