// CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
#![allow(clippy::unnecessary_to_owned)]

use crate::{ast::SExpId, lambda_lifting::CLOSURE_SYMBOL, try_err};

use super::{
    Runtime,
    value::{Closure, Function, Value},
};

impl Runtime {
    pub(crate) fn function_def(&mut self, items: &[SExpId]) -> Result<Value, String> {
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

        Ok(Value::Function(Function::Lisp {
            signature,
            body: *body,
        }))
    }

    pub(crate) fn closure_def(&mut self, items: &[SExpId]) -> Result<Value, String> {
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

        let captured = items
            .get(1)
            .ok_or_else(|| "Expected captured".to_string())?;
        let captured = self.asts.get(*captured);
        let Some(captured) = captured.as_list() else {
            return Err("Expected list".to_string());
        };
        let captured = captured
            .to_vec()
            .into_iter()
            .map(|s| {
                let name = self.asts.get(s).as_symbol().unwrap().to_string();
                let val = self.eval(s);
                (name, val)
            })
            .collect();

        let body = items.get(2).ok_or_else(|| "Expected body".to_string())?;

        Ok(Value::Closure(Closure {
            signature,
            captured,
            body: *body,
        }))
    }

    pub(crate) fn function_call(&mut self, function: Function, args: &[SExpId]) -> Value {
        match function {
            Function::Lisp { signature, body } => {
                self.envs.push();
                for (sig, arg) in signature.iter().zip(args) {
                    let arg = self.eval(*arg);
                    try_err!(arg);
                    self.envs.set(sig, arg);
                }

                let result = self.eval(body);
                self.envs.pop();
                result
            }
            Function::Rust { body } => {
                let args = args.iter().map(|arg| self.eval(*arg)).collect::<Vec<_>>();
                body(self, args)
            }
        }
    }

    pub(crate) fn closure_call(&mut self, closure: Closure, args: &[SExpId]) -> Value {
        let Closure {
            signature,
            captured,
            body,
        } = closure;
        self.envs.push();
        for (sig, arg) in signature.into_iter().zip(args) {
            let arg: Value = self.eval(*arg);
            try_err!(arg);
            self.envs.set(sig.as_str(), arg);
        }
        self.envs.set(CLOSURE_SYMBOL, Value::Object(captured));

        eprintln!("Calling closure: {}", self.asts.fmt(body));

        let result = self.eval(body);
        self.envs.pop();
        result
    }
}
