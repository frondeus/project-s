use std::{collections::BTreeMap, rc::Rc};

use crate::{
    ast::SExpId,
    builder::{ASTBuilder, error},
};

use super::{
    Runtime,
    value::{Function, Macro, Value},
};

#[derive(Default, Debug)]
pub struct Env {
    // is_obj: bool,
    vars: BTreeMap<String, Value>,
}

impl Env {
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.vars.keys().map(|k| k.as_str())
    }

    pub fn with_fn(
        mut self,
        name: impl ToString,
        body: impl Fn(&mut Runtime, Vec<Value>) -> Value + 'static,
    ) -> Self {
        self.vars.insert(
            name.to_string(),
            Value::Function(Function::Rust {
                body: Rc::new(body),
            }),
        );
        self
    }

    pub fn with_macro(
        mut self,
        name: impl ToString,
        body: impl Fn(&mut Runtime, Vec<SExpId>) -> SExpId + 'static,
    ) -> Self {
        self.vars.insert(
            name.to_string(),
            Value::Macro(Macro::Rust {
                body: Rc::new(body),
            }),
        );
        self
    }

    pub fn with_try_fn(
        self,
        name: &str,
        body: impl Fn(&mut Runtime, Vec<Value>) -> Result<Value, String> + 'static,
    ) -> Self {
        self.with_fn(name, move |rt, args| {
            let result = body(rt, args);
            result.unwrap_or_else(Value::Error)
        })
    }

    pub fn with_try_macro(
        self,
        name: &str,
        body: impl Fn(&mut Runtime, Vec<SExpId>) -> Result<SExpId, String> + 'static,
    ) -> Self {
        self.with_macro(name, move |rt, args| {
            let result = body(rt, args);
            match result {
                Ok(id) => id,
                Err(err) => {
                    eprintln!("Error: {}", err);
                    error().build(&mut rt.asts)
                }
            }
        })
    }
}

// impl Env {
// pub fn keys(&self) -> impl Iterator<Item = &str> {
//     self.vars.keys().map(|k| k.as_str())
// }
// }
// impl Env {
//     fn obj() -> Self {
//         Self { is_obj: true, ..Default::default()}
//     }
// }

#[derive(Debug)]
pub struct Envs {
    envs: Vec<Env>,
}

impl Default for Envs {
    fn default() -> Self {
        Self::new()
    }
}

impl Envs {
    pub fn new() -> Self {
        Self {
            envs: vec![Env::default()],
        }
    }

    pub fn with_env(&mut self, env: Env) {
        self.envs.clear();
        self.envs.push(env);
    }

    fn last_mut(&mut self) -> &mut Env {
        self.envs.last_mut().expect("No environment")
    }

    pub fn slice(&self) -> &[Env] {
        &self.envs
    }

    pub fn last(&self) -> &Env {
        self.envs.last().expect("No environment")
    }

    pub fn set(&mut self, name: &str, value: Value) {
        self.last_mut().vars.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.envs.iter().rev().find_map(|env| env.vars.get(name))
    }

    // pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
    //     self.envs
    //         .iter_mut()
    //         .rev()
    //         .find_map(|env| env.vars.get_mut(name))
    // }

    pub fn push(&mut self) {
        self.envs.push(Env::default());
    }

    pub fn pop(&mut self) -> Option<BTreeMap<String, Value>> {
        self.envs.pop().map(|env| env.vars)
    }

    // pub fn _self(&self) -> Option<&Env> {
    //     self.envs.iter().rev().find(|env| env.is_obj)
    // }
}
