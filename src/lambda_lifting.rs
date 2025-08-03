use std::collections::BTreeSet;

use crate::{
    ast::{ASTS, SExpId},
    patterns::Pattern,
    source::Spanned,
    visitor::{List, Visitor, VisitorHelper},
};

const SPECIAL_FORMS: &[&str] = &[
    "quasiquote",
    "+",
    "unquote",
    "let",
    "fn",
    "cl",
    "struct",
    "is-type",
    "quote",
    "has?",
];

// New visitor-based implementation
pub struct LambdaLiftingPass<'a> {
    helper: VisitorHelper<'a>,
    envs: Envs,
}

impl<'a> LambdaLiftingPass<'a> {
    pub fn pass(asts: &'a mut ASTS, root: SExpId, envs: &'a [crate::runtime::Env]) -> SExpId {
        let envs: Envs = envs.into();
        let mut pass = Self {
            helper: VisitorHelper::new(asts),
            envs,
        };

        let root = pass.helper.spanned(root);
        let transformed = pass.visit_sexp(root).unwrap_or(root);
        transformed.inner()
    }
}

impl<'a> Visitor<'a> for LambdaLiftingPass<'a> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        &mut self.helper
    }

    fn helper(&self) -> &VisitorHelper<'a> {
        &self.helper
    }

    fn visit_list(&mut self, list: List) -> Option<Spanned<SExpId>> {
        if self.helper.is_special_form(&list, "quote") {
            return None;
        }

        if self.helper.is_special_form(&list, "quasiquote") {
            return None;
        }

        if self
            .helper
            .is_one_of_special_forms(&list.list, &["do", "top-level"])
        {
            return self.visit_do_block(list);
        }

        if self
            .helper
            .is_one_of_special_forms(&list.list, &["let-rec", "let*"])
        {
            return self.visit_let_rec(list);
        }

        if self.helper.is_special_form(&list, "let") {
            return self.visit_let(list);
        }

        if self.helper.is_special_form(&list, "struct") {
            return self.visit_struct(list);
        }

        if self.helper.is_special_form(&list, "thunk") {
            return self.visit_thunk(list);
        }

        if self.helper.is_special_form(&list, "fn") {
            return self.visit_function(list);
        }

        // Default: visit children
        let mut list = list;
        list.visit_children(self);
        list.id()
    }
}

impl<'a> LambdaLiftingPass<'a> {
    fn visit_do_block(&mut self, mut list: List) -> Option<Spanned<SExpId>> {
        self.envs.push(EnvKind::Local);
        list.visit_children(self);
        self.envs.pop();
        list.id()
    }

    fn visit_let_rec(&mut self, mut list: List) -> Option<Spanned<SExpId>> {
        // For let-rec, don't register patterns before analyzing values
        // This allows recursive references to be detected as free variables
        list.visit_children(self);

        // Now register patterns after processing values
        let mut list_iter = list.list.iter().skip(1); // Skip "let-rec"
        while let Some(pat_id) = list_iter.next() {
            if let Ok(pattern) = Pattern::parse(pat_id.inner(), self.helper.asts) {
                self.process_pattern(pattern);
            }
            _ = list_iter.next(); // Skip value
        }

        list.id()
    }

    fn visit_let(&mut self, mut list: List) -> Option<Spanned<SExpId>> {
        // For regular let, pattern is at index 1
        if list.list.len() >= 3 {
            if let Ok(pattern) = Pattern::parse(list.list[1].inner(), self.helper.asts) {
                self.process_pattern(pattern);
            }
        }

        list.visit_children(self);
        list.id()
    }

    fn visit_struct(&mut self, mut list: List) -> Option<Spanned<SExpId>> {
        self.envs.push(EnvKind::Object);

        // Visit struct body, skipping the "struct" keyword
        for i in 1..list.list.len() {
            if self
                .helper
                .get_sexp(list.list[i])
                .as_symbol_or_keyword()
                .is_some()
            {
                // Key value pair
                if i + 1 < list.list.len() {
                    if let Some(new_id) = self.visit_sexp(list.list[i + 1]) {
                        list.list[i + 1] = new_id;
                        list.edited = true;
                    }
                }
            }
        }

        self.envs.pop();
        list.id()
    }

    fn visit_thunk(&mut self, list: List) -> Option<Spanned<SExpId>> {
        if list.list.len() < 3 {
            return None;
        }

        self.envs.push(EnvKind::Function);

        let thunk_token = list.list[0];
        let free_vars_list = list.list[1];
        let body = list.list[2];

        // Extract existing free variables
        let existing_free_vars =
            if let Some(fv_list) = self.helper.get_sexp(free_vars_list).as_list() {
                fv_list
                    .iter()
                    .filter_map(|id| self.helper.get_sexp(*id).as_symbol())
                    .map(|s| s.to_string())
                    .collect::<BTreeSet<String>>()
            } else {
                BTreeSet::new()
            };

        // Analyze body for additional free variables in a separate scope
        let (new_body, new_free_vars) = {
            let mut analyzer = FreeVariableAnalyzer {
                helper: &self.helper,
                envs: &self.envs,
                free_vars: BTreeSet::new(),
            };

            let new_body = analyzer.visit_sexp(body).unwrap_or(body);
            (new_body, analyzer.free_vars)
        };

        // Combine existing and new free variables
        let mut all_free_vars = existing_free_vars;
        all_free_vars.extend(new_free_vars);

        self.envs.pop();

        if !all_free_vars.is_empty() || new_body != body {
            Some(
                self.helper
                    .assemble((thunk_token, all_free_vars, new_body), list.span),
            )
        } else {
            None
        }
    }

    fn visit_function(&mut self, list: List) -> Option<Spanned<SExpId>> {
        if list.list.len() < 3 {
            return None;
        }

        self.envs.push(EnvKind::Function);

        let fn_token = list.list[0];
        let pattern_id = list.list[1];
        let body = list.list[2];

        // Process pattern to register parameters
        if let Ok(pattern) = Pattern::parse(pattern_id.inner(), self.helper.asts) {
            self.process_pattern(pattern);
        }

        // First, recursively process the body to handle nested functions
        let new_body = self.visit_sexp(body).unwrap_or(body);

        // Then analyze the processed body for free variables
        let free_vars = {
            let mut analyzer = FreeVariableAnalyzer {
                helper: &self.helper,
                envs: &self.envs,
                free_vars: BTreeSet::new(),
            };

            analyzer.visit_sexp(new_body);
            analyzer.free_vars
        };

        self.envs.pop();

        if !free_vars.is_empty() {
            // Convert to closure with captured variables
            let cl_token = self.helper.assemble("cl", fn_token.span);
            Some(
                self.helper
                    .assemble((cl_token, pattern_id, free_vars, new_body), list.span),
            )
        } else if new_body != body {
            // Function changed but no free variables
            Some(
                self.helper
                    .assemble((fn_token, pattern_id, new_body), list.span),
            )
        } else {
            None
        }
    }

    fn process_pattern(&mut self, pattern: Pattern) {
        match pattern {
            Pattern::Hole(_, _) => (),
            Pattern::Splice(s, _, _) => {
                self.process_pattern(*s);
            }
            Pattern::Single(key, _, _) => {
                self.envs.set(&key);
            }
            Pattern::List(patterns, _, _) => {
                for pattern in patterns {
                    self.process_pattern(pattern);
                }
            }
            Pattern::Object(patterns, _, _) => {
                for (_key, pattern) in patterns {
                    self.process_pattern(pattern);
                }
            }
        }
    }
}

// Helper visitor for analyzing free variables
struct FreeVariableAnalyzer<'a, 'b> {
    helper: &'b VisitorHelper<'a>,
    envs: &'b Envs,
    free_vars: BTreeSet<String>,
}

impl<'a, 'b> Visitor<'a> for FreeVariableAnalyzer<'a, 'b> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        unreachable!("FreeVariableAnalyzer is read-only")
    }

    fn helper(&self) -> &VisitorHelper<'a> {
        self.helper
    }

    fn visit_atom(&mut self, id: Spanned<SExpId>) -> Option<Spanned<SExpId>> {
        if let Some(symbol) = self.helper.get_sexp(id).as_symbol() {
            if !SPECIAL_FORMS.contains(&symbol) {
                match self.envs.has(symbol) {
                    Some(VariableKind::Free) => {
                        self.free_vars.insert(symbol.to_string());
                    }
                    None | Some(VariableKind::Local) => {}
                }
            }
        }
        None
    }

    fn visit_list(&mut self, mut list: List) -> Option<Spanned<SExpId>> {
        if self.helper.is_special_form(&list, "quote") {
            return None;
        }

        if self.helper.is_special_form(&list, "quasiquote") {
            // For quasiquote, recursively analyze content looking for unquote
            if list.list.len() >= 2 {
                self.visit_quasiquote_content(list.list[1]);
            }
            return None;
        }

        if self.helper.is_special_form(&list, "fn") || self.helper.is_special_form(&list, "cl") {
            // Don't traverse into nested functions
            return None;
        }

        if self.helper.is_special_form(&list, "thunk") {
            // For thunks, only analyze the captured variables list
            if list.list.len() >= 2 {
                if let Some(captured_list) = self.helper.get_sexp(list.list[1]).as_list() {
                    for captured_id in captured_list {
                        if let Some(symbol) = self.helper.get_sexp(*captured_id).as_symbol() {
                            self.free_vars.insert(symbol.to_string());
                        }
                    }
                }
            }
            return None;
        }

        // For other constructs, visit children normally
        list.visit_children(self);
        None
    }
}

impl<'a, 'b> FreeVariableAnalyzer<'a, 'b> {
    fn visit_quasiquote_content(&mut self, content_id: Spanned<SExpId>) {
        let sexp = self.helper.get_sexp(content_id);
        if let Some(list) = sexp.as_list() {
            if !list.is_empty() {
                let first = list[0];
                if self.helper.is_symbol(first, "unquote") && list.len() >= 2 {
                    // Analyze the unquoted expression
                    self.visit_sexp(self.helper.spanned(list[1]));
                } else {
                    // Recursively analyze nested quasiquote content
                    for &item_id in list {
                        self.visit_quasiquote_content(self.helper.spanned(item_id));
                    }
                }
            }
        }
    }
}

// Public interface - now uses visitor pattern
pub struct LambdaPass;

impl LambdaPass {
    pub fn pass(asts: &mut ASTS, root: SExpId, envs: &[crate::runtime::Env]) -> SExpId {
        LambdaLiftingPass::pass(asts, root, envs)
    }
}

impl From<&crate::runtime::Env> for Env {
    fn from(env: &crate::runtime::Env) -> Self {
        Self {
            // Problem: This populates only with latest env.
            vars: env.keys().map(|k| k.to_string()).collect(),
            kind: EnvKind::Global,
        }
    }
}
impl From<&[crate::runtime::Env]> for Envs {
    fn from(envs: &[crate::runtime::Env]) -> Self {
        let mut envs_iter = envs.iter();
        let mut envs = Vec::new();

        if let Some(global) = envs_iter.next() {
            envs.push(Env {
                vars: global.keys().map(|k| k.to_string()).collect(),
                kind: EnvKind::Global,
            });
        };

        for env in envs_iter {
            envs.push(Env {
                vars: env.keys().map(|k| k.to_string()).collect(),
                kind: EnvKind::Local,
            });
        }

        Self { envs }
    }
}

#[derive(Debug)]
struct Env {
    vars: BTreeSet<String>,
    kind: EnvKind,
}

#[derive(Debug)]
struct Envs {
    envs: Vec<Env>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum EnvKind {
    Global,
    Function,
    Object,
    Local,
}

impl Default for Envs {
    fn default() -> Self {
        Self::new()
    }
}

impl Env {
    fn new(kind: EnvKind) -> Self {
        Self {
            vars: BTreeSet::new(),
            kind,
        }
    }

    fn global() -> Self {
        Self::new(EnvKind::Global)
    }
}

#[derive(Debug, Clone, Copy)]
enum VariableKind {
    Local,
    Free,
}

impl Envs {
    pub fn new() -> Self {
        Self {
            envs: vec![Env::global()],
        }
    }

    pub fn push(&mut self, kind: EnvKind) {
        self.envs.push(Env::new(kind));
    }

    pub fn pop(&mut self) {
        self.envs.pop();
    }

    fn last_mut(&mut self) -> &mut Env {
        self.envs.last_mut().expect("No environment")
    }

    pub fn set(&mut self, name: &str) {
        self.last_mut().vars.insert(name.to_string());
    }

    pub fn has(&self, name: &str) -> Option<VariableKind> {
        let mut outcome = VariableKind::Local;
        const OBJECT_RELATED_VARS: &[&str] = &["self", "super", "root"];
        for env in self.envs.iter().rev() {
            if env.kind == EnvKind::Object && OBJECT_RELATED_VARS.contains(&name) {
                return Some(outcome);
            }
            if env.vars.contains(name) && env.kind != EnvKind::Global {
                return Some(outcome);
            }
            if let EnvKind::Function = env.kind {
                outcome = VariableKind::Free;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{s_std::prelude, source::Sources};
    use test_case::test_case;

    use super::*;

    #[test]
    fn lift() -> test_runner::Result {
        test_runner::test_snapshots("docs/", &["", "s"], "lift", |input, _deps, _args| {
            let mut asts = ASTS::new();
            let (sources, source_id) = Sources::single("<input>", input);
            let ast = asts.parse(source_id, sources.get(source_id)).unwrap();
            let root_id = ast.root_id().unwrap();
            let prelude = prelude();
            let new_root = LambdaPass::pass(&mut asts, root_id, &[prelude]);
            let output = asts.fmt(new_root);
            output.to_string()
        })
    }

    fn lambda_pass_test_helper(input: &str) -> String {
        let mut asts = ASTS::new();
        let (sources, source_id) = Sources::single("<input>", input);
        let ast = asts.parse(source_id, sources.get(source_id)).unwrap();
        let root_id = ast.root_id().unwrap();
        let prelude = prelude();
        let new_root = LambdaPass::pass(&mut asts, root_id, &[prelude]);
        asts.fmt(new_root).to_string()
    }

    fn lambda_lifting_pass_test_helper(input: &str) -> String {
        let mut asts = ASTS::new();
        let (sources, source_id) = Sources::single("<input>", input);
        let ast = asts.parse(source_id, sources.get(source_id)).unwrap();
        let root_id = ast.root_id().unwrap();
        let prelude = prelude();
        let new_root = LambdaLiftingPass::pass(&mut asts, root_id, &[prelude]);
        asts.fmt(new_root).to_string()
    }

    #[test_case("(fn (:x) x)" => "(top-level (fn (:x) x))"; "test_1_simple_function_no_free_vars")]
    #[test_case("(do (let :y 5) (fn (:x) y))" => "(top-level (do (let :y 5) (cl (:x) (y) y)))"; "test_2_function_with_free_var")]
    #[test_case("(do (let :y 5) (fn (:x) (+ x y)))" => "(top-level (do (let :y 5) (cl (:x) (y) (+ x y))))"; "test_3_function_with_free_var_in_expression")]
    #[test_case("(let :x 5) (fn (:y) x)" => "(top-level (let :x 5) (cl (:y) (x) x))"; "test_4_function_capturing_let_bound_var")]
    #[test_case("(let :x 5) (fn (:y) (+ x y))" => "(top-level (let :x 5) (cl (:y) (x) (+ x y)))"; "test_5_function_capturing_let_var_in_expression")]
    #[test_case("(fn (:x) (fn (:y) x))" => "(top-level (fn (:x) (cl (:y) (x) x)))"; "test_6_nested_functions_with_capture")]
    #[test_case("(fn (:x) (fn (:y) y))" => "(top-level (fn (:x) (fn (:y) y)))"; "test_7_nested_functions_no_capture_from_outer")]
    #[test_case("(fn (:x) (let :y x) (fn (:z) y))" => "(top-level (fn (:x) (let :y x) (fn (:z) y)))"; "test_8_function_capturing_inner_let_bound_var")]
    #[test_case("(fn (:x) (do (let :y 5) (fn (:z) (+ x y z))))" => "(top-level (fn (:x) (do (let :y 5) (cl (:z) (x y) (+ x y z)))))"; "test_9_function_capturing_from_multiple_scopes")]
    #[test_case("(fn (:self) self)" => "(top-level (fn (:self) self))"; "test_10_self_parameter_no_capture")]
    #[test_case("(struct :field self)" => "(top-level (struct :field self))"; "test_11_struct_field_with_self")]
    fn lambda_pass_unit_tests(input: &str) -> String {
        lambda_pass_test_helper(input)
    }

    #[test_case("(do (let :x 5) (fn (:y) x))" => "(top-level (do (let :x 5) (cl (:y) (x) x)))"; "test_12_function_in_do_block")]
    // TODO: Recursive functions should capture themselves, but current implementation doesn't handle this
    #[test_case("(let-rec :f (fn (:x) (f x)) f)" => ignore "(top-level (let-rec :f (cl (:x) (f) (f x)) f))"; "test_13_recursive_function")]
    #[test_case("(let :x 1) (let :y 2) (fn (:z) (+ x y z))" => "(top-level (let :x 1) (let :y 2) (cl (:z) (x y) (+ x y z)))"; "test_14_nested_lets_with_captures")]
    #[test_case("(fn (:x) (quote y))" => "(top-level (fn (:x) (quote y)))"; "test_15_quoted_symbols_not_captured")]
    // TODO: Quasiquote/unquote analysis needs enhancement for complete free variable detection
    #[test_case("(do (let :y 5) (fn (:x) (quasiquote (unquote y))))" => ignore "(top-level (do (let :y 5) (cl (:x) (y) (quasiquote (unquote y)))))"; "test_16_unquoted_symbols_captured")]
    fn lambda_pass_advanced_tests(input: &str) -> String {
        lambda_pass_test_helper(input)
    }

    #[test_case("(fn (:x) x)" => "(top-level (fn (:x) x))"; "new_test_1_simple_function_no_free_vars")]
    #[test_case("(do (let :y 5) (fn (:x) y))" => "(top-level (do (let :y 5) (cl (:x) (y) y)))"; "new_test_2_function_with_free_var")]
    #[test_case("(do (let :y 5) (fn (:x) (+ x y)))" => "(top-level (do (let :y 5) (cl (:x) (y) (+ x y))))"; "new_test_3_function_with_free_var_in_expression")]
    #[test_case("(let :x 5) (fn (:y) x)" => "(top-level (let :x 5) (cl (:y) (x) x))"; "new_test_4_function_capturing_let_bound_var")]
    #[test_case("(let :x 5) (fn (:y) (+ x y))" => "(top-level (let :x 5) (cl (:y) (x) (+ x y)))"; "new_test_5_function_capturing_let_var_in_expression")]
    #[test_case("(fn (:x) (fn (:y) x))" => "(top-level (fn (:x) (cl (:y) (x) x)))"; "new_test_6_nested_functions_with_capture")]
    #[test_case("(fn (:x) (fn (:y) y))" => "(top-level (fn (:x) (fn (:y) y)))"; "new_test_7_nested_functions_no_capture_from_outer")]
    #[test_case("(fn (:x) (let :y x) (fn (:z) y))" => "(top-level (fn (:x) (let :y x) (fn (:z) y)))"; "new_test_8_function_capturing_inner_let_bound_var")]
    #[test_case("(fn (:x) (do (let :y 5) (fn (:z) (+ x y z))))" => "(top-level (fn (:x) (do (let :y 5) (cl (:z) (x y) (+ x y z)))))"; "new_test_9_function_capturing_from_multiple_scopes")]
    #[test_case("(fn (:self) self)" => "(top-level (fn (:self) self))"; "new_test_10_self_parameter_no_capture")]
    #[test_case("(struct :field self)" => "(top-level (struct :field self))"; "new_test_11_struct_field_with_self")]
    fn lambda_lifting_pass_new_tests(input: &str) -> String {
        lambda_lifting_pass_test_helper(input)
    }

    #[test_case("(do (let :x 5) (fn (:y) x))" => "(top-level (do (let :x 5) (cl (:y) (x) x)))"; "new_test_12_function_in_do_block")]
    #[test_case("(let :x 1) (let :y 2) (fn (:z) (+ x y z))" => "(top-level (let :x 1) (let :y 2) (cl (:z) (x y) (+ x y z)))"; "new_test_14_nested_lets_with_captures")]
    #[test_case("(fn (:x) (quote y))" => "(top-level (fn (:x) (quote y)))"; "new_test_15_quoted_symbols_not_captured")]
    fn lambda_lifting_pass_comprehensive_tests(input: &str) -> String {
        lambda_lifting_pass_test_helper(input)
    }
}
