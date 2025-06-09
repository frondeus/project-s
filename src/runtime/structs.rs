use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use crate::{ast::SExpId, runtime::value::Value};

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
                let f = self.eval_eager_rec(*f, false);
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
        println!("Constructor call: {self_:?}");
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

    pub fn new_ref_obj(&self, obj: BTreeMap<String, Value>) -> Value {
        let self_ = Value::Object(obj);
        Value::Ref(Rc::new(RefCell::new(self_)))
    }
}
