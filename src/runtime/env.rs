use std::collections::BTreeMap;

use super::value::Value;

#[derive(Default, Debug)]
pub struct Env {
    // is_obj: bool,
    vars: BTreeMap<String, Value>,
}
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

    fn last_mut(&mut self) -> &mut Env {
        self.envs.last_mut().expect("No environment")
    }

    pub fn set(&mut self, name: &str, value: Value) {
        self.last_mut().vars.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.envs.iter().rev().find_map(|env| env.vars.get(name))
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.envs
            .iter_mut()
            .rev()
            .find_map(|env| env.vars.get_mut(name))
    }

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
