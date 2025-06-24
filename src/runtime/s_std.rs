use crate::api::Rest;

use super::{Env, Runtime, Value};

mod functions;
mod macros;

pub fn prelude() -> Env {
    use functions::*;
    use macros::*;

    Env::default()
        .with_fn("-", sub)
        .with_fn("+", add)
        .with_fn(">", lg)
        .with_fn("ref", new_ref)
        .with_fn("set", set)
        .with_fn("list", make_list)
        .with_fn("tuple", make_tuple)
        .with_fn("obj/insert", insert_to_struct)
        .with_fn("obj/con", obj_con)
        .with_fn("obj/plain", obj_plain)
        .with_fn("obj/extend", obj_extend)
        .with_try_macro("obj/condef", condef)
        .with_try_macro("obj/put", objput)
        .with_try_macro("obj/+", obj_add)
        .with_try_macro("obj/struct", obj_struct)
        .with_fn("obj/eval", obj_eval)
        .with_fn("obj/construct-or", obj_construct_or)
        .with_fn("obj/new", obj_construct_or)
        .with_try_macro("struct", obj_struct)
        .with_fn("eager", eager)
        .with_fn("deep-eager", deep_eager)
        .with_fn("has?", obj_has)
        .with_fn("import", import)
        .with_try_macro("let*", let_star)
        .with_fn("error", error)
        .with_fn("debug", |args: Rest<Value>| {
            tracing::info!("Debug: {:#?}", &*args);

            Ok(Value::List(args.into()))
        })
        .with_fn("print", |rt: &mut Runtime, args: Rest<Value>| {
            for arg in args.into_iter() {
                let arg = arg.eager_rec(rt, true);
                tracing::info!("{:?}", arg);
            }

            1.0
        })
}

impl Runtime {
    pub fn with_env(&mut self, env: Env) {
        self.envs.with_env(env);
    }

    pub fn with_prelude(&mut self) {
        self.with_env(prelude());
    }
}
