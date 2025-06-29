use crate::ast::{AST, SExp, SExpId};

use super::{Runtime, value::Value};

impl Runtime {
    pub(crate) fn quote(&self, id: &SExpId) -> Value {
        let sexp = self.asts.get(*id);
        match &**sexp {
            SExp::Number(n) => Value::Number(*n),
            SExp::String(s) => Value::String(s.clone()),
            SExp::Symbol(_) => Value::SExp(*id),
            SExp::Keyword(_) => Value::SExp(*id),
            SExp::Bool(b) => Value::Bool(*b),
            SExp::Error => Value::Error("Quote: AST Error".to_string()),
            SExp::List(_) => Value::SExp(*id),
        }
    }

    pub(crate) fn quasiquote(&mut self, id: &SExpId) -> Value {
        let sexp = self.asts.get(*id);
        match &**sexp {
            SExp::Number(n) => Value::Number(*n),
            SExp::String(s) => Value::String(s.clone()),
            SExp::Symbol(_) => Value::SExp(*id),
            SExp::Keyword(_) => Value::SExp(*id),
            SExp::Bool(b) => Value::Bool(*b),
            SExp::Error => Value::Error("Quasiquote: AST Error".to_string()),
            SExp::List(_) => {
                let mut new_ast = self.asts.new_ast();
                let root = self.traverse_unquote(&mut new_ast, id);
                new_ast.set_root(root);
                self.asts.add_ast(new_ast);
                Value::SExp(root)
            }
        }
    }

    fn traverse_unquote(&mut self, new_ast: &mut AST, id: &SExpId) -> SExpId {
        let original = self.asts.get(*id);
        let span = original.span;

        match (**original).clone() {
            SExp::List(items) => {
                let Some(first) = items.first() else {
                    return *id;
                };
                if self.is_unquote(first) {
                    let Some(next) = items.get(1) else {
                        todo!("Somehow return an error");
                    };
                    let next_span = self.asts.get(*next).span;
                    let evaled = self.eval(*next);
                    evaled.to_sexp(new_ast, next_span)
                } else {
                    let mut result = Vec::new();
                    for item in &items {
                        result.push(self.traverse_unquote(new_ast, item));
                    }
                    // if result == items {
                    //     return *id;
                    // }
                    let new_list = SExp::List(result);
                    new_ast.add_node(new_list, span, None)
                }
            }
            _ => *id,
        }
    }

    fn is_unquote(&self, id: &SExpId) -> bool {
        let sexp = self.asts.get(*id);
        match &**sexp {
            SExp::Symbol(s) => s == "unquote",
            _ => false,
        }
    }
}
