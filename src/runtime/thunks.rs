// CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
#![allow(clippy::unnecessary_to_owned)]

use std::{cell::RefCell, rc::Rc};

use crate::ast::SExpId;

use super::{
    Runtime,
    value::{InnerThunk, Thunk, Value},
};

impl Runtime {
    pub(crate) fn thunk_def(&mut self, items: &[SExpId]) -> Result<Value, String> {
        let captured = items
            .first()
            .ok_or_else(|| "Expected captured".to_string())?;
        let captured = self.asts.get(*captured);
        let Some(captured) = captured.as_list() else {
            return Err("Expected list".to_string());
        };
        let captured = captured
            .to_vec()
            .into_iter()
            // .map(|s| self.asts.get(s).as_symbol().unwrap().to_string())
            .map(|s| {
                let name = self.asts.get(s).as_symbol().unwrap().to_string();
                let val = self.eval(s);
                (name, val)
            })
            .collect();
        let body = items.get(1).ok_or_else(|| "Expected body".to_string())?;
        Ok(Value::Thunk(Thunk {
            inner: Rc::new(RefCell::new(InnerThunk::ToEvaluate {
                captured,
                body: *body,
            })),
        }))
    }

    pub(crate) fn thunk_call(&mut self, thunk: Thunk) -> Value {
        let Thunk { inner } = thunk;
        let mut inner = inner.borrow_mut();
        match &mut *inner {
            InnerThunk::Evaluated(val) => val.clone(),
            InnerThunk::ToEvaluate { captured, body } => {
                let captured = std::mem::take(captured);
                self.envs.push();
                for (name, val) in captured.into_iter() {
                    self.envs.set(name.as_str(), val);
                }
                let result = self.eval(*body);
                self.envs.pop();
                *inner = InnerThunk::Evaluated(result.clone());
                result
            }
        }
    }
}
