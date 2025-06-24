#![allow(dead_code)]
use std::collections::BTreeMap;

use crate::{
    ast::{ASTS, SExpId},
    diagnostics::Diagnostics,
    patterns::Pattern,
    runtime::Macro,
    source::{Span, WithSpan},
    visitor::{Visitor, VisitorHelper},
};

pub struct MacroExpansionPass<'a> {
    helper: VisitorHelper<'a>,
    envs: Envs,
}

impl<'a> MacroExpansionPass<'a> {
    pub fn pass(
        asts: &'a mut ASTS,
        root: SExpId,
        diagnostics: &'a mut Diagnostics,
        envs: &'a [crate::runtime::Env],
    ) -> SExpId {
        let envs: Envs = envs.into();
        let mut pass = Self {
            helper: VisitorHelper::new(asts),
            envs,
        };
        let mut visitor = MacroForbidden {
            helper: &mut pass.helper,
            envs: &mut pass.envs,
            diagnostics,
        };
        let expanded = visitor.visit_sexp(root).unwrap_or(root);

        MacroSanitizer {
            helper: &mut pass.helper,
        }
        .visit_sexp(expanded)
        .unwrap_or(expanded)
    }
}

struct MacroForbidden<'a, 'b> {
    helper: &'b mut VisitorHelper<'a>,
    envs: &'b mut Envs,
    diagnostics: &'b mut Diagnostics,
}

impl<'a> Visitor<'a> for MacroForbidden<'a, '_> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        self.helper
    }

    fn helper(&self) -> &VisitorHelper<'a> {
        self.helper
    }

    fn visit_list(&mut self, mut list: crate::visitor::List) -> Option<SExpId> {
        if self.helper.is_special_form(&list, "macro") {
            self.diagnostics
                .add(list.span.clone(), "Macro is forbidden in this context");
            return None;
        }
        if self.helper.is_special_form(&list, "let") {
            if let &[_let, pattern, value] = &list.list[..] {
                let mut let_visitor = LetVisitor {
                    helper: self.helper,
                    envs: self.envs,
                    diagnostics: self.diagnostics,
                    macro_def: None,
                };
                let_visitor.visit_sexp(value);
                if let Some(macro_) = let_visitor.macro_def {
                    let (pattern, p_span) =
                        parse_pattern(pattern, self.helper.asts, self.diagnostics)?;
                    let pattern = if let Pattern::Single(p) = pattern {
                        p
                    } else {
                        self.diagnostics
                            .add(p_span, "Macros can be used only with simplest pattern");
                        return None;
                    };
                    self.envs.set(&pattern, macro_);
                }
                return None;
            }
        }

        if let Some(first) = self.helper.maybe_get_symbol(list.list.first().copied()) {
            if let Some(macro_) = self.envs.get(first).cloned() {
                let result = MacroEvaluator {
                    helper: self.helper,
                    env: Default::default(),
                    diag: self.diagnostics,
                    macro_,
                    is_top: true,
                    args: &list.list[1..],
                }
                .evaluate(list.span.clone())?;

                return Some(self.visit_sexp(result).unwrap_or(result));
            }
        }
        list.visit_children(self);
        list.id()
    }
}

struct LetVisitor<'a, 'b> {
    helper: &'b mut VisitorHelper<'a>,
    envs: &'b mut Envs,
    diagnostics: &'b mut Diagnostics,
    macro_def: Option<Macro>,
}

fn parse_pattern(pattern: SExpId, ast: &ASTS, diag: &mut Diagnostics) -> Option<(Pattern, Span)> {
    let span = ast.get(pattern).span.clone();
    match Pattern::parse(pattern, ast) {
        Ok(p) => Some((p, span)),
        Err(e) => {
            diag.add(span, e);
            None
        }
    }
}

impl<'a> Visitor<'a> for LetVisitor<'a, '_> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        self.helper
    }

    fn helper(&self) -> &VisitorHelper<'a> {
        self.helper
    }

    fn visit_list(&mut self, list: crate::visitor::List) -> Option<SExpId> {
        if self.helper.is_special_form(&list, "macro") {
            if let &[_macro, pattern, body] = &list.list[..] {
                let (pattern, _) = parse_pattern(pattern, self.helper.asts, self.diagnostics)?;

                self.macro_def = Some(Macro::Lisp { body, pattern });
            }
            return None;
        }

        MacroForbidden {
            helper: self.helper,
            envs: self.envs,
            diagnostics: self.diagnostics,
        }
        .visit_list(list)
    }
}

struct MacroEvaluator<'a, 'b> {
    helper: &'b mut VisitorHelper<'a>,
    env: BTreeMap<String, SExpId>,
    diag: &'b mut Diagnostics,
    macro_: Macro,
    args: &'b [SExpId],
    is_top: bool,
}

impl MacroEvaluator<'_, '_> {
    fn pattern_to_list(
        pattern: &Pattern,
        diag: &mut Diagnostics,
        span: Span,
    ) -> Option<Vec<String>> {
        let Pattern::List(list) = pattern else {
            diag.add(span, "Macro is using non-list pattern matching");
            return None;
        };
        let mut args = vec![];

        for el in list {
            let Pattern::Single(el) = el else {
                diag.add(span, "Macro is using nested pattern matching");
                return None;
            };
            args.push(el.clone());
        }

        Some(args)
    }

    pub fn evaluate(mut self, span: Span) -> Option<SExpId> {
        match &self.macro_ {
            Macro::Lisp { pattern, body } => {
                let signature = Self::pattern_to_list(pattern, self.diag, span.clone())?;
                for (sig, &arg) in signature.into_iter().zip(self.args) {
                    self.env.insert(sig, arg);
                }

                self.visit_sexp(*body)
            }
            Macro::Rust { body } => {
                let args = self.args.to_vec();
                let result = body(self.helper.asts, args);
                Some(result)
            }
        }
    }
}

impl<'a> Visitor<'a> for MacroEvaluator<'a, '_> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        self.helper
    }

    fn helper(&self) -> &VisitorHelper<'a> {
        self.helper
    }

    fn visit_quote(&mut self, quote: crate::visitor::Quote) -> Option<SExpId> {
        Some(quote.quoted)
    }

    fn visit_quasiquote(&mut self, mut quasiquote: crate::visitor::Quasiquote) -> Option<SExpId> {
        quasiquote.visit_unquote(self);
        Some(quasiquote.quoted)
    }

    fn visit_unquote(&mut self, unquote: crate::visitor::Unquote) -> Option<SExpId> {
        let new_id = self.visit_sexp(unquote.unquoted)?;
        Some(new_id)
    }

    fn visit_list(&mut self, list: crate::visitor::List) -> Option<SExpId> {
        self.diag.add(list.span, "Expected quote or unquote");
        None
    }

    fn visit_atom(&mut self, id: WithSpan<SExpId>) -> Option<SExpId> {
        let item = self.helper.get_symbol(id.item);

        if let Some(item) = item.and_then(|item| self.env.get(item)) {
            return Some(*item);
        }

        self.diag.add(id.span, "Expected quote or unquote");

        None
    }
}

struct MacroSanitizer<'a, 'b> {
    helper: &'b mut VisitorHelper<'a>,
}

impl<'a> Visitor<'a> for MacroSanitizer<'a, '_> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        self.helper
    }

    fn helper(&self) -> &VisitorHelper<'a> {
        self.helper
    }

    fn visit_list(&mut self, mut list: crate::visitor::List) -> Option<SExpId> {
        if self.helper.is_special_form(&list, "macro") {
            return self.helper.then_assemble(());
        }
        list.visit_children(self);
        list.id()
    }
}

impl From<&crate::runtime::Env> for Env {
    fn from(env: &crate::runtime::Env) -> Self {
        Self {
            // Problem: This populates only with latest env.
            vars: env
                .iter()
                .filter_map(|(k, val)| match val {
                    crate::runtime::Value::Macro(macro_) => Some((k.to_string(), macro_.clone())),
                    _ => None,
                })
                .collect(),
        }
    }
}
impl From<&[crate::runtime::Env]> for Envs {
    fn from(envs: &[crate::runtime::Env]) -> Self {
        let envs = envs.iter().map(|e| e.into()).collect();
        Self { envs }
    }
}

#[derive(Default)]
struct Env {
    vars: BTreeMap<String, Macro>,
}

struct Envs {
    envs: Vec<Env>,
}

impl Default for Envs {
    fn default() -> Self {
        Self::new()
    }
}

impl Envs {
    fn new() -> Self {
        Self {
            envs: vec![Env::default()],
        }
    }

    pub fn push(&mut self) {
        self.envs.push(Env::default());
    }

    pub fn pop(&mut self) {
        self.envs.pop();
    }

    pub fn set(&mut self, name: &str, macro_: Macro) {
        self.envs
            .last_mut()
            .unwrap()
            .vars
            .insert(name.to_string(), macro_);
    }

    pub fn get(&self, name: &str) -> Option<&Macro> {
        self.envs.iter().rev().find_map(|env| env.vars.get(name))
    }
}

#[cfg(test)]
mod tests {
    use crate::s_std::prelude;

    use super::*;

    #[test]
    fn macro_expansion() -> test_runner::Result {
        unsafe { std::env::set_var("NO_COLOR", "1") }
        test_runner::test_snapshots("docs/", "macro", |input, _deps, _args| {
            let mut asts = ASTS::new();
            let ast = asts.parse(input, "<input>").unwrap();
            let root_id = ast.root_id().unwrap();
            let mut diagnostics = Diagnostics::default();
            let prelude = prelude();
            let new_root =
                MacroExpansionPass::pass(&mut asts, root_id, &mut diagnostics, &[prelude]);
            let output = asts.fmt(new_root);
            if diagnostics.has_errors() {
                return diagnostics.pretty_print();
            }
            output.to_string()
        })
    }
}
