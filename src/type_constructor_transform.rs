use std::collections::HashMap;

use crate::{
    ast::{ASTS, SExpId},
    builder::symbol,
    source::Spanned,
    visitor::{Quote, Visitor, VisitorHelper},
};

pub struct TypeConstructorTransformPass<'a> {
    helper: VisitorHelper<'a>,
}

impl<'a> TypeConstructorTransformPass<'a> {
    pub fn pass(asts: &'a mut ASTS, root: SExpId) -> SExpId {
        let mut pass = Self {
            helper: VisitorHelper::new(asts),
        };

        let root = pass.helper.spanned(root);
        let transformed = pass.visit_sexp(root).unwrap_or(root);
        transformed.inner()
    }
}

impl<'a> Visitor<'a> for TypeConstructorTransformPass<'a> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        &mut self.helper
    }

    fn helper(&self) -> &VisitorHelper<'a> {
        &self.helper
    }

    fn visit_list(&mut self, mut list: crate::visitor::List) -> Option<Spanned<SExpId>> {
        if self.helper.is_special_form(&list, "type") {
            return self.transform_type_constructor(list);
        }

        // For do blocks, we need to transform each statement
        if self.helper.is_special_form(&list, "do") {
            list.visit_children(self);
            return list.id();
        }

        list.visit_children(self);
        list.id()
    }
}

// Helper visitor to extract parameter information from quotes
struct TypeParamExtractor<'a, 'b> {
    helper: &'b VisitorHelper<'a>,
    params: Vec<(String, Spanned<SExpId>)>,
}

impl<'a, 'b> Visitor<'a> for TypeParamExtractor<'a, 'b> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        unreachable!("TypeParamExtractor is read-only")
    }

    fn helper(&self) -> &VisitorHelper<'a> {
        self.helper
    }

    fn visit_quote(&mut self, quote: Quote) -> Option<Spanned<SExpId>> {
        if let Some(param_name) = self.helper.get_symbol(quote.quoted.inner()) {
            self.params.push((param_name.to_string(), quote.id));
        }
        Some(quote.id)
    }
}

// Helper visitor to substitute type variables
struct TypeVarSubstitutor<'a, 'b> {
    helper: &'b mut VisitorHelper<'a>,
    mapping: &'b HashMap<String, String>,
}

impl<'a, 'b> Visitor<'a> for TypeVarSubstitutor<'a, 'b> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        self.helper
    }

    fn helper(&self) -> &VisitorHelper<'a> {
        self.helper
    }

    fn visit_quote(&mut self, quote: Quote) -> Option<Spanned<SExpId>> {
        if let Some(quoted_name) = self.helper.get_symbol(quote.quoted.inner()) {
            if let Some(replacement) = self.mapping.get(quoted_name) {
                return self.helper.then_assemble(symbol(replacement), quote.span);
            }
        }
        Some(quote.id)
    }

    fn visit_atom(&mut self, id: Spanned<SExpId>) -> Option<Spanned<SExpId>> {
        Some(id)
    }

    fn visit_list(&mut self, mut list: crate::visitor::List) -> Option<Spanned<SExpId>> {
        list.visit_children(self);
        list.id()
    }
}

impl<'a> TypeConstructorTransformPass<'a> {
    fn transform_type_constructor(
        &mut self,
        list: crate::visitor::List,
    ) -> Option<Spanned<SExpId>> {
        // Expected format: (type :constructor_name type_param1 type_param2 ... type_definition)
        // We want to transform to: (let :constructor_name (fn (param1 param2 ...) type_definition_with_substitution))

        if list.list.len() < 3 {
            // Invalid type definition, return as-is
            return list.id();
        }

        let constructor_name = list.list[1];
        let type_params = &list.list[2..list.list.len() - 1];
        let type_definition = list.list[list.list.len() - 1];

        // Extract type parameters using visitor
        let mut param_extractor = TypeParamExtractor {
            helper: &self.helper,
            params: Vec::new(),
        };

        for &param in type_params {
            param_extractor.visit_sexp(param);
        }

        // Create function parameters and mapping
        let mut fn_params = Vec::new();
        let mut mapping = HashMap::new();

        for (param_name, quote_id) in param_extractor.params {
            let fn_param_name = format!(":{param_name}");
            let fn_param = self
                .helper
                .then_assemble(symbol(&fn_param_name), quote_id.span)?;
            fn_params.push(fn_param);
            mapping.insert(param_name.clone(), param_name);
        }

        // Transform the type definition by substituting type variables
        let mut substitutor = TypeVarSubstitutor {
            helper: &mut self.helper,
            mapping: &mapping,
        };
        let transformed_definition = substitutor.visit_sexp(type_definition)?;

        // Build the function: (fn (params...) transformed_definition)
        let fn_expr = if fn_params.is_empty() {
            // No parameters, create a function that takes unit: (fn () transformed_definition)
            let empty_params = self.helper.then_assemble((), list.span)?;
            self.helper.then_assemble(
                (symbol("fn"), empty_params, transformed_definition),
                list.span,
            )?
        } else {
            let params_list = self.helper.then_assemble(fn_params, list.span)?;
            self.helper.then_assemble(
                (symbol("fn"), params_list, transformed_definition),
                list.span,
            )?
        };

        // Build the let expression: (let :constructor_name fn_expr)
        self.helper
            .then_assemble((symbol("let"), constructor_name, fn_expr), list.span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast::ASTS, source::Sources};

    #[test]
    fn type_constructor_transform() -> test_runner::Result {
        test_runner::test_snapshots("docs/", &["s"], "type-transform", |input, _deps, _args| {
            let mut asts = ASTS::new();
            let (sources, source_id) = Sources::single("<input>", input);
            let ast = asts.parse(source_id, sources.get(source_id)).unwrap();
            let root_id = ast.root_id().unwrap();

            let transformed_root = TypeConstructorTransformPass::pass(&mut asts, root_id);
            let output = asts.fmt(transformed_root);
            format!("{output:#}")
        })
    }
}
