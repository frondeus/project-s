use crate::ast::SExpId;

use super::{
    Runtime,
    value::{Macro, Value},
};

impl Runtime {
    // CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
    #[allow(clippy::unnecessary_to_owned)]
    pub(crate) fn macro_def(&mut self, items: &[SExpId]) -> Result<Value, String> {
        let signature = items
            .first()
            .ok_or_else(|| "Expected signature".to_string())?;

        let signature = self.asts.get(*signature);
        let Some(signature) = signature.as_list() else {
            return Err("Expected list".to_string());
        };
        let signature = signature
            .to_vec()
            .into_iter()
            .map(|s| self.asts.get(s).as_keyword().unwrap().to_string())
            .collect();
        let body = items.get(1).ok_or_else(|| "Expected body".to_string())?;

        Ok(Value::Macro(Macro::Lisp {
            signature,
            body: *body,
        }))
    }

    pub(crate) fn macro_call(&mut self, macro_: Macro, args: &[SExpId]) -> Result<SExpId, String> {
        let result = match macro_ {
            Macro::Lisp { signature, body } => {
                self.envs.push();

                for (sig, arg) in signature.iter().zip(args) {
                    self.envs.set(sig, Value::SExp(*arg));
                }

                let result = self.eval(body);

                self.envs.pop();

                let result = result
                    .as_sexp()
                    .ok_or_else(|| "Expected SExpression".to_string())?;
                *result
            }
            Macro::Rust { body } => {
                let args = args.to_vec();
                body(self, args)
            }
        };

        // tracing::debug!("Macro call result: {}", self.asts.fmt(result));
        let envs = self.envs.slice();
        let processed = crate::process_ast(&mut self.asts, result, envs);
        tracing::debug!("Expanded: {}", self.asts.fmt(processed));

        Ok(processed)
    }
}
