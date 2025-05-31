use std::collections::{BTreeMap, VecDeque};

use crate::{
    ast::{SExp, SExpId},
    runtime::value::Value,
    try_err,
};

use super::Runtime;

#[derive(Default)]
pub(crate) struct Structs {
    stack: Vec<BTreeMap<String, Value>>,
}

impl Structs {
    fn push_default(&mut self) {
        self.stack.push(BTreeMap::new());
    }
    pub(crate) fn push(&mut self, strukt: BTreeMap<String, Value>) {
        self.stack.push(strukt);
    }

    pub(crate) fn pop(&mut self) -> BTreeMap<String, Value> {
        self.stack.pop().unwrap()
    }

    pub(crate) fn _self(&self) -> Option<&BTreeMap<String, Value>> {
        self.stack.last()
    }

    fn mut_self(&mut self) -> Option<&mut BTreeMap<String, Value>> {
        self.stack.last_mut()
    }

    pub(crate) fn root(&self) -> Option<&BTreeMap<String, Value>> {
        self.stack.first()
    }
}

impl Runtime {
    fn insert_to_struct(&mut self, key: &str, items: &mut VecDeque<SExpId>) -> Result<(), String> {
        eprintln!("Processing pair: {key}");
        let Some(value) = items.pop_front() else {
            return Err("Expected value".to_string());
        };
        let value = self.eval(value);
        self.structs
            .mut_self()
            .unwrap()
            .insert(key.to_string(), value);
        Ok(())
    }

    fn make_struct_inner(&mut self, items: impl Iterator<Item = SExpId>) -> Result<(), String> {
        let mut items = items.collect::<VecDeque<_>>();

        while let Some(item_id) = items.pop_front() {
            let item = self.asts.get(item_id).clone();
            match item {
                SExp::List(list) => {
                    eprintln!("Processing list: {list:?}");
                    let first_id = list.first().ok_or_else(|| "Expected list".to_string())?;
                    let first = self
                        .asts
                        .get(*first_id)
                        .as_symbol()
                        .map(ToOwned::to_owned)
                        .ok_or_else(|| "Create struct: Expected symbol".to_string())?;
                    match first.as_str() {
                        "let" => {
                            self.object_let(&list[1..])?;
                        }
                        "if" => {
                            self.object_if(&list[1..])?;
                        }
                        _ => {
                            let first = self.eval(*first_id);
                            match first {
                                Value::Error(e) => return Err(e),
                                Value::Macro(macro_) => {
                                    let result = self.macro_call(macro_, &list[1..])?;
                                    items.push_front(result);
                                    continue;
                                }
                                _ => {
                                    return Err(format!("Invalid struct caller: {:?}", first));
                                }
                            }
                        }
                    }
                }
                _ => {
                    let key = self.eval(item_id);
                    let key = match key {
                        Value::Symbol(key) => key,
                        Value::String(key) => key,
                        _ => {
                            return Err("Expected symbol or string".to_string());
                        }
                    };
                    self.insert_to_struct(&key, &mut items)?;
                }
            }
        }
        Ok(())
    }

    // CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
    #[allow(clippy::unnecessary_to_owned)]
    pub(crate) fn make_struct(&mut self, items: &[SExpId]) -> Value {
        let Some(sexp) = items.first() else {
            return Value::Error("Expected SExpression. Found None".to_string());
        };
        let evaled = self.eval(*sexp);
        try_err!(evaled);
        let Some(sexp) = evaled.as_sexp() else {
            return Value::Error(format!("Expected SExpression. Found {evaled:?}",));
        };
        let sexp = self.asts.get(*sexp);
        let Some(items) = sexp.as_list() else {
            return Value::Error("Expected list".to_string());
        };

        // let mut map = BTreeMap::new();
        let items = items.to_vec().into_iter();
        self.structs.push_default();
        self.envs.push();

        if let Err(e) = self.make_struct_inner(items) {
            self.envs.pop();
            self.structs.pop();
            return Value::Error(e);
        }

        self.envs.pop();
        let map = self.structs.pop();
        Value::Object(map)
    }

    // CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
    #[allow(clippy::unnecessary_to_owned)]
    fn object_if(&mut self, items: &[SExpId]) -> Result<(), String> {
        let Some(condition) = items.first() else {
            return Err("Expected condition".to_string());
        };
        let condition = self.eval(*condition);
        if let Value::Error(e) = condition {
            return Err(e);
        }
        let Value::Bool(b) = condition else {
            return Err("Expected boolean".to_string());
        };

        let Some(then) = items.get(1) else {
            return Err("Expected then".to_string());
        };
        let else_ = items.get(2);

        let branch = if b { Some(then) } else { else_ };

        let Some(branch) = branch else {
            return Ok(());
        };

        let evaled = self.eval(*branch);

        let Some(sexp) = evaled.as_sexp() else {
            return Err(format!("Expected SExpression. Found {evaled:?}"));
        };
        let sexp = self.asts.get(*sexp);
        let Some(items) = sexp.as_list() else {
            return Err("Expected list".to_string());
        };
        let items = items.to_vec().into_iter();

        self.make_struct_inner(items)?;
        Ok(())
    }

    fn object_let(&mut self, items: &[SExpId]) -> Result<(), String> {
        let Some(ident) = items.first() else {
            return Err("Expected SExpression".to_string());
        };
        let ident = self.asts.get(*ident).clone();
        let Some(ident) = ident.as_symbol() else {
            return Err("Object let: Expected symbol".to_string());
        };

        let Some(value) = items.get(1) else {
            return Err("Expected value".to_string());
        };
        let value = self.eval(*value);
        self.envs.set(ident, value);
        Ok(())
    }
}
