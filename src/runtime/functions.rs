// CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
#![allow(clippy::unnecessary_to_owned)]

use crate::{ast::SExpId, patterns::Pattern};

use super::{
    Runtime,
    value::{Function, Value},
};

impl Runtime {
    pub(crate) fn function_def(&mut self, items: &[SExpId]) -> Result<Value, String> {
        let pattern = items
            .first()
            .ok_or_else(|| "Expected pattern".to_string())?;

        let pattern = Pattern::parse(*pattern, &self.asts)?;
        let body = items.get(1).ok_or_else(|| "Expected body".to_string())?;

        Ok(Value::Function(Function::Lisp {
            pattern,
            captured: Default::default(),
            body: *body,
        }))
    }

    pub(crate) fn closure_def(&mut self, items: &[SExpId]) -> Result<Value, String> {
        let pattern = items
            .first()
            .ok_or_else(|| "Expected pattern".to_string())?;

        let pattern = Pattern::parse(*pattern, &self.asts)?;

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

        Ok(Value::Function(Function::Lisp {
            pattern,
            captured,
            body: *body,
        }))
    }

    pub(crate) fn closure_call_inner(&mut self, function: Function, args: Vec<Value>) -> Value {
        match function {
            Function::Lisp {
                pattern,
                body,
                captured,
            } => {
                self.envs.push();
                if let Err(e) = self.destruct_(pattern, Value::List(args)) {
                    return Value::Error(e);
                }
                for (name, val) in captured {
                    self.envs.set(&name, val);
                }
                // self.envs.set(CLOSURE_SYMBOL, Value::Object(captured));

                let result = self.eval(body);
                self.envs.pop();
                result
            }
            Function::Rust { body } => body(self, args),
        }
    }

    pub(crate) fn handle_splice(&mut self, args: &[SExpId]) -> Vec<Value> {
        args.to_vec()
            .into_iter()
            .flat_map(|arg| {
                if let Some(list) = self.as_special_form(arg, "splice") {
                    if let Some(first) = list.get(1) {
                        let value = self.eval(*first);
                        return match value {
                            Value::List(l) => l,
                            got => vec![Value::Error(format!(
                                "Splice: Expected a list, got {got:?}"
                            ))],
                        };
                    } else {
                        return vec![Value::Error(
                            "Splice: expected at least one argument".into(),
                        )];
                    }
                }
                let value = self.eval(arg);
                vec![value]
            })
            .collect()
    }

    pub(crate) fn closure_call(&mut self, function: Function, args: &[SExpId]) -> Value {
        let args = self.handle_splice(args);
        // .collect::<Vec<_>>()
        // .into_iter()
        // .map(|arg| self.eval(arg))
        // .collect::<Vec<_>>();

        self.closure_call_inner(function, args)
    }

    fn as_special_form(&self, list_id: SExpId, name: &str) -> Option<&[SExpId]> {
        let list = self.asts.get(list_id);
        let list = list.as_list()?;
        let first = list.first()?;
        let first = self.asts.get(*first);
        let first = first.as_symbol()?;
        if first == name { Some(list) } else { None }
    }
}
