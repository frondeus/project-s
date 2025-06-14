use std::collections::BTreeMap;

use crate::runtime::value::Value;

use super::{
    Runtime, // value::{Closure, Constructor},
    value::Constructor,
};

impl Runtime {
    // Constructor (self)
    pub(crate) fn constructor_call(
        &mut self,
        constructor: Constructor,
        self_: Option<Value>,
    ) -> Value {
        tracing::debug!("Constructor call: {self_:?}");
        if let Some(body) = constructor.constructor.body() {
            tracing::debug!("{}", self.asts.fmt(body));
        }
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

        let super_ = self
            .envs
            .get("super")
            .cloned()
            .unwrap_or_else(|| self_.clone());

        self.closure_call_inner(
            constructor.constructor,
            vec![self_.clone(), root, super_, origin],
        )
        // let ret = self_;
        // tracing::info!("Created object: {ret:?}");
        // let ret = match ret {
        //     Value::Ref(ret) => match Rc::try_unwrap(ret) {
        //         Ok(ret) => ret.into_inner(),
        //         Err(ret) => Value::Ref(ret),
        //     },
        //     v => v,
        // };
        // tracing::info!("unwrapping object: {ret:?}");
        // ret
    }

    pub fn new_ref_obj(&self, obj: BTreeMap<String, Value>) -> Value {
        let self_ = Value::Object(obj);
        Value::ref_(self_)
    }
}
