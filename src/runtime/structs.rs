use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

use crate::{
    ast::{SExp, SExpId},
    builder::ASTBuilder,
    runtime::value::Value,
};

use super::{
    Runtime, // value::{Closure, Constructor},
    value::Constructor,
};

// #[derive(Default)]
// pub(crate) struct Structs {
//     stack: Vec<BTreeMap<String, Value>>,
// }

// impl Structs {
//     pub(crate) fn push(&mut self, strukt: BTreeMap<String, Value>) {
//         self.stack.push(strukt);
//     }

//     pub(crate) fn pop(&mut self) -> BTreeMap<String, Value> {
//         self.stack.pop().unwrap()
//     }

//     pub(crate) fn last(&self) -> Option<&BTreeMap<String, Value>> {
//         self.stack.last()
//     }

//     pub(crate) fn super_(&self) -> Option<&BTreeMap<String, Value>> {
//         self.last()
//     }
// }

impl Runtime {
    pub(crate) fn condef(&mut self, items: &[SExpId]) -> Result<Value, String> {
        match items {
            [f] => {
                let f = self.eval_eager_rec(*f);
                let Value::Function(f) = f else {
                    return Err(format!("Expected function found {f:?}"));
                };

                Ok(Value::Constructor(Constructor { constructor: f }))
            }
            _ => Err(format!("Expected 1 argument found {}", items.len())),
        }
    }

    // Constructor (self)
    pub(crate) fn constructor_call(
        &mut self,
        constructor: Constructor,
        self_: Option<Value>,
    ) -> Value {
        // let self_ = self
        //     .envs
        //     .get("self")
        //     .cloned()
        //     .unwrap_or_else(|| self.new_ref_obj(Default::default()));
        let self_ = self_.unwrap_or_else(|| self.new_ref_obj(Default::default()));

        let root = self
            .envs
            .get("root")
            .cloned()
            .unwrap_or_else(|| self_.clone());

        let origin = self
            .envs
            .get("origin")
            .cloned()
            .unwrap_or_else(|| self_.clone());

        self.closure_call_inner(constructor.constructor, vec![self_.clone(), root, origin]);
        let ret = self_;
        println!("Created object: {ret:?}");
        match ret {
            Value::Ref(ret) => match Rc::try_unwrap(ret) {
                Ok(ret) => ret.into_inner(),
                Err(ret) => Value::Ref(ret),
            },
            v => v,
        }
    }

    fn insert_to_struct(&mut self, key: &str, items: &mut VecDeque<SExpId>) -> Result<(), String> {
        let Some(value) = items.pop_front() else {
            return Err("Expected value".to_string());
        };

        let mut env = self.envs.last().keys().collect::<Vec<_>>();
        if !env.contains(&"root") && self.envs.get("root").is_some() {
            env.push("root");
        }
        if !env.contains(&"super") && self.envs.get("super").is_some() {
            env.push("super");
        }

        let thunk = vec![env.build(&mut self.asts), value];
        let value = self.thunk_def(&thunk)?;
        println!("value: {:?}", value);
        let self_ = self.envs.get("self").unwrap().as_ref().unwrap();

        let mut self_ = self_.borrow_mut();

        let self_ = self_.as_object_mut().unwrap();

        self_.insert(key.to_string(), value);
        Ok(())
    }

    fn make_struct_inner(&mut self, items: impl Iterator<Item = SExpId>) -> Result<(), String> {
        let mut items = items.collect::<VecDeque<_>>();

        while let Some(item_id) = items.pop_front() {
            let item = self.asts.get(item_id).clone();
            match item {
                SExp::List(list) => {
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
                    let key = self
                        .as_symbol_or_keyword_or_string(key)
                        .ok_or_else(|| "Expected symbol or string".to_string())?
                        .to_owned();
                    self.insert_to_struct(&key, &mut items)?;
                }
            }
        }
        Ok(())
    }

    pub fn new_ref_obj(&self, obj: BTreeMap<String, Value>) -> Value {
        let self_ = Value::Object(obj);
        Value::Ref(Rc::new(RefCell::new(self_)))
    }

    // CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
    #[allow(clippy::unnecessary_to_owned)]
    pub(crate) fn make_struct(&mut self, items: &[SExpId]) -> Value {
        let items = items.to_vec().into_iter();

        // Value::Constructor(Constructor {
        //     closure: Closure {
        //         signature: vec![],
        //         captured: Default::default(),
        //         body: todo!()
        //     }
        // })
        self.envs.push();
        let self_ = self.new_ref_obj(BTreeMap::new());

        if self.envs.get("root").is_none() {
            self.envs.set("root", self_.clone());
        }

        self.envs.set("self", self_.clone());

        if let Err(e) = self.make_struct_inner(items) {
            self.envs.pop();
            return Value::Error(e);
        }

        let mut env = self.envs.pop().unwrap();
        env.remove("self").unwrap()
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
        let Some(ident) = ident.as_keyword() else {
            return Err("Object let: Expected keyword".to_string());
        };

        let Some(value) = items.get(1) else {
            return Err("Expected value".to_string());
        };
        let value = self.eval(*value);
        self.envs.set(ident, value);
        Ok(())
    }
}
