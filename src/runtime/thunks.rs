// CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
#![allow(clippy::unnecessary_to_owned)]

use crate::ast::SExpId;

use super::{
    Runtime,
    value::{Thunk, Value},
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
            captured,
            body: *body,
        }))
    }
}
