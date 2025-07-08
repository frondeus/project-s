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
                let name = self.asts.get(s);
                let name = name.as_symbol().map(|s| s.to_string()).unwrap_or_else(|| {
                    panic!("Expected symbol, got {name:?}");
                });
                tracing::trace!("name: {name}");
                let val = self.eval(s);
                tracing::trace!("val: {val:?}");
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
        tracing::trace!("ThunkCall: {}", thunk.inner.as_ptr() as usize);
        let Thunk { inner } = thunk;
        {
            let inner_ref = inner.borrow();
            match &*inner_ref {
                InnerThunk::Evaluated(val) => {
                    tracing::trace!("Trunk is already evaluated");
                    return val.clone();
                }
                InnerThunk::Evaluating => {
                    tracing::error!("Thunk is already evaluating");
                    panic!("Thunk is already evaluating");
                }
                _ => tracing::trace!("To Evaluate"),
            }
        }
        let thunk = { std::mem::replace(&mut *inner.borrow_mut(), InnerThunk::Evaluating) };

        let InnerThunk::ToEvaluate { captured, body } = thunk else {
            panic!("Thunk is not to evaluate");
        };

        self.envs.push();
        for (name, val) in captured.into_iter() {
            self.envs.set(name.as_str(), val);
        }
        let result = self.eval(body);
        self.envs.pop();
        *inner.borrow_mut() = InnerThunk::Evaluated(result.clone());
        result
    }
}
