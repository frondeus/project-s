use crate::{ast::SExpId, patterns::Pattern};

use super::{
    Runtime,
    value::{Macro, Value},
};

impl Runtime {
    // CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
    #[allow(clippy::unnecessary_to_owned)]
    pub(crate) fn macro_def(&mut self, items: &[SExpId]) -> Result<Value, String> {
        let pattern = items
            .first()
            .ok_or_else(|| "Expected pattern".to_string())?;

        let pattern = Pattern::parse(*pattern, &self.asts)?;
        let body = items.get(1).ok_or_else(|| "Expected body".to_string())?;

        Ok(Value::Macro(Macro::Lisp {
            pattern,
            body: *body,
        }))
    }

    pub(crate) fn macro_call(&mut self, macro_: Macro, args: &[SExpId]) -> Result<SExpId, String> {
        let result = match macro_ {
            Macro::Lisp { pattern, body } => {
                self.envs.push();

                let args = args.iter().copied().map(Value::SExp).collect();
                self.destruct_(pattern, Value::List(args))?;

                let result = self.eval(body);

                self.envs.pop();

                let result = result.as_sexp().ok_or_else(|| {
                    format!("Macro call: Expected SExpression. Found {:?}", result)
                })?;
                *result
            }
            Macro::Rust { body } => {
                let args = args.to_vec();
                body(&mut self.asts, args)
            }
        };

        // tracing::debug!("Macro call result: {}", self.asts.fmt(result));
        let envs = self.envs.slice();
        let (processed, diag) = crate::process_ast(&mut self.asts, result, envs);
        tracing::debug!("Expanded: {}", self.asts.fmt(processed));

        if diag.has_errors() {
            let p = diag.pretty_print();
            tracing::error!("{}", p);
        }

        Ok(processed)
    }
}
