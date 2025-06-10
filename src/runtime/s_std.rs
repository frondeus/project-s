use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use crate::{
    ast::{AST, SExp, SExpId},
    builder::{ASTBuilder, error, quote},
};

use super::{
    Env, Runtime, Value,
    value::{Constructor, Function},
};

fn sub(_rt: &mut Runtime, args: Vec<Value>) -> Result<Value, String> {
    let mut args = args.into_iter();
    let Some(mut a) = args.next() else {
        return Err("Expected at least one argument".into());
    };

    a = a.ok()?;

    match &mut a {
        Value::Number(a) => {
            for arg in args {
                let arg = arg.ok()?;
                let Some(b) = arg.as_number() else {
                    return Err("Expected number".into());
                };
                *a -= b;
            }
        }
        _ => return Err("Expected number".into()),
    }

    Ok(a)
}

#[tracing::instrument(skip_all)]
fn add(rt: &mut Runtime, args: Vec<Value>) -> Result<Value, String> {
    #[derive(Clone, Debug)]
    enum ObjectOrConstructor {
        Object(BTreeMap<String, Value>),
        Constructor(Constructor),
    }

    impl ObjectOrConstructor {
        fn call(
            self,
            rt: &mut Runtime,
            self_: Value,
            root: Option<Value>,
            super_: Value,
            origin: Option<Value>,
        ) -> Result<Value, String> {
            Ok(match self {
                ObjectOrConstructor::Constructor(left) => {
                    rt.envs.push();
                    // rt.envs.set("self", self_.clone());
                    if let Some(root) = root {
                        rt.envs.set("root", root.clone());
                    }
                    rt.envs.set("super", super_.clone());
                    if let Some(origin) = origin {
                        rt.envs.set("origin", origin.clone());
                    }
                    let res = rt.constructor_call(left, Some(self_.clone()));
                    rt.envs.pop();
                    res
                }
                ObjectOrConstructor::Object(left) => {
                    for (key, value) in left {
                        insert_to_struct(rt, vec![self_.clone(), Value::String(key), value])?;
                    }
                    self_
                }
            })
        }
    }

    #[tracing::instrument(skip_all)]
    fn add_obj_impl(
        rt: &mut Runtime,
        self_: Value,
        root: Value,
        origin: Value,
        left: ObjectOrConstructor,
        right: ObjectOrConstructor,
    ) -> Result<Value, String> {
        /*
           let add = (a, b) => create_obj(({self, root}) => {
               let left = a({ root, super_: self, self });
               let super_ = new Map(Object.entries(left));
               b({ root: undefined, super_, self, origin: root });
           });
        */
        tracing::debug!("Adding obj: {:?}, {:?}", left, right);
        tracing::debug!("Self: {:?}", self_);
        tracing::debug!("Root: {:?}", root);
        // let super_ = Value::ref_(Value::Object(BTreeMap::new()));
        let left = left.call(
            rt,
            self_.clone(),
            Some(root.clone()),
            self_.clone(),
            Some(origin),
        )?;
        tracing::debug!("Left: {:?}", left);
        let super_ = left.deref();
        // let super_ = Value::ref_(self_.clone());
        // let self_ = Value::ref_(self_.clone());
        tracing::debug!("Super: {:?}", super_);
        tracing::debug!("Self: {:?}", self_);
        right.call(rt, self_.clone(), Some(self_.clone()), super_, Some(root))?;
        tracing::debug!("Result: {:?}", self_);

        Ok(self_)
    }

    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Err("Expected at least one argument".into());
    };

    let first = first.eager_rec(rt, false).ok()?;

    match first {
        Value::Number(mut first) => {
            for arg in args {
                let Some(b) = arg.eager(rt, false).ok()?.as_number() else {
                    return Err("Expected number".into());
                };
                first += b;
            }
            Ok(Value::Number(first))
        }
        Value::Constructor(left) => {
            let left = ObjectOrConstructor::Constructor(left);
            let Some(right) = args.next() else {
                return Err("Expected at least two arguments".into());
            };

            let right = match right.eager_rec(rt, false).ok()? {
                Value::Object(right) => ObjectOrConstructor::Object(right),
                Value::Constructor(right) => ObjectOrConstructor::Constructor(right),
                right => {
                    return Err(format!(
                        "+: Expected object or object constructor. Found: {:?}",
                        right
                    ));
                }
            };

            Ok(Value::Constructor(Constructor {
                constructor: Function::from(move |rt: &mut Runtime, args: Vec<Value>| {
                    let Ok([self_, root, _super, origin]) = TryInto::<[Value; 4]>::try_into(args)
                    else {
                        return Value::Error("Expected two arguments".into());
                    };

                    add_obj_impl(rt, self_, root, origin, left.clone(), right.clone())
                        .unwrap_or_else(Value::Error)
                }),
            }))
        }
        Value::Object(left) => {
            let left = ObjectOrConstructor::Object(left);
            let Some(right) = args.next() else {
                return Err("Expected at least two arguments".into());
            };

            let right = match right.eager_rec(rt, false).ok()? {
                Value::Object(right) => ObjectOrConstructor::Object(right),
                Value::Constructor(right) => ObjectOrConstructor::Constructor(right),
                right => {
                    return Err(format!(
                        "+: Expected object or object constructor. Found: {:?}",
                        right
                    ));
                }
            };

            Ok(Value::Constructor(Constructor {
                constructor: Function::from(move |rt: &mut Runtime, args: Vec<Value>| {
                    let Ok([self_, root, _super_, origin]) = TryInto::<[Value; 4]>::try_into(args)
                    else {
                        return Value::Error("Expected two arguments".into());
                    };

                    add_obj_impl(rt, self_, root, origin, left.clone(), right.clone())
                        .unwrap_or_else(Value::Error)
                }),
            }))
        }
        _ => Err(format!("+: Expected number or object. Found: {:?}", first)),
    }
}

fn set(_rt: &mut Runtime, args: Vec<Value>) -> Result<Value, String> {
    let Ok([key, value]) = TryInto::<[Value; 2]>::try_into(args) else {
        return Err("Expected two arguments".into());
    };

    let Some(key) = key.as_ref() else {
        return Err("Expected symbol".into());
    };

    *key.borrow_mut() = value.clone();

    Ok(value)
}

fn add_obj(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    match &args[..] {
        [key, value] => {
            let result = (
                "if",
                ("has?", "super", key),
                // quote((key, ("thunk", (), ("+", ("super", key), value)))),
                quote((key, ("+", ("super", key), value))),
                quote((key, value)),
            );
            Ok(result.build(&mut rt.asts))
            // let result = ("thunk", (), result);

            // let result = result.build(&mut rt.asts);
            // Ok(result)
        }
        _ => Err("Expected two arguments".into()),
    }
}

fn new_ref(_rt: &mut Runtime, args: Vec<Value>) -> Result<Value, String> {
    let Ok([one]) = TryInto::<[Value; 1]>::try_into(args) else {
        return Err("Expected one argument".into());
    };

    Ok(Value::Ref(Rc::new(RefCell::new(one))))
}

#[tracing::instrument(skip_all)]
fn insert_to_struct(rt: &mut Runtime, args: Vec<Value>) -> Result<Value, String> {
    // println!("Inserting to struct");
    let Ok([_this, key, value]) = TryInto::<[Value; 3]>::try_into(args) else {
        return Err("Expected three arguments".into());
    };

    let key = match key {
        Value::String(s) => s,
        Value::SExp(id) => match rt.asts.get(id) {
            SExp::Symbol(s) => s.to_string(),
            SExp::Keyword(s) => s.to_string(),
            SExp::String(s) => s.to_string(),
            _ => return Err("Expected keyword, symbol or string".into()),
        },
        _ => return Err("Expected keyword, symbol or string".into()),
    };

    let value = match value {
        Value::Constructor(v) => rt.constructor_call(v, None),
        _ => value,
    };

    let Some(this) = _this.as_ref() else {
        return Err("Expected self".into());
    };
    let mut this = this.borrow_mut();

    let Some(this) = this.as_object_mut() else {
        return Err("Expected object".into());
    };

    tracing::debug!("Inserting to struct({:?}): {} - {:?}", this, key, value);

    let old = this.insert(key, value);

    tracing::debug!("After insertion: {this:?}");

    Ok(old.unwrap_or_else(|| Value::Error("???".into())))
}

fn condef(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    Ok((
        "obj/con",
        (
            "fn",
            (":self", ":root", ":super", ":origin"),
            move |ast: &mut AST| {
                let mut items = args;
                items.insert(0, "do".assemble(ast));
                items.push("self".assemble(ast));
                items.assemble(ast)
            },
        ),
    )
        .build(&mut rt.asts))
}

fn objput(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    let Ok([key, value]) = TryInto::<[SExpId; 2]>::try_into(args) else {
        return Err("Expected two arguments".into());
    };

    Ok(("obj/insert", "self", key, value).build(&mut rt.asts))
}

fn obj_add(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    match &args[..] {
        [key, value] => Ok((
            "if",
            ("has?", "super", key),
            ("obj/put", key, ("+", ("super", key), value)),
            ("obj/put", key, value),
        )
            .build(&mut rt.asts)),
        arg => Err(format!("Expected two arguments. Found: {}", arg.len())),
    }
}

fn obj_put_thunk(key: String, value: impl ASTBuilder) -> impl ASTBuilder {
    let value = ("obj/construct-or", value);
    let value = ("thunk", ("self", "root", "super", "origin"), value);
    ("obj/put", format!(":{key}"), value)
}

fn obj_eval(rt: &mut Runtime, args: Vec<Value>) -> Result<Value, String> {
    let Ok([to_eval]) = TryInto::<[Value; 1]>::try_into(args) else {
        return Err("Expected one argument".into());
    };

    match to_eval {
        Value::SExp(id) => {
            let ast = rt.asts.get(id);
            let Some(list) = ast.as_list() else {
                return Err("Expected list".into());
            };

            let list = list.to_vec();
            let mut iter = list.into_iter();
            let mut last = None;
            while let Some(key) = iter.next() {
                let key = rt.asts.get(key);
                let Some(key) = key.as_keyword() else {
                    return Err("Expected keyword".into());
                };

                let Some(value) = iter.next() else {
                    return Err("Expected value".into());
                };

                let expr = obj_put_thunk(key.to_string(), value).build(&mut rt.asts);

                let expr = expr.build(&mut rt.asts);
                last = Some(rt.eval(expr));
            }
            Ok(last.unwrap_or_else(|| Value::Error("Expected at least one argument".into())))
        }
        rest => Ok(rest),
    }
}

fn obj_struct(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    let mut args = args.into_iter();
    let mut inner = Vec::new();
    let mut ast = rt.asts.new_ast();

    while let Some(arg_id) = args.next() {
        let arg = rt.asts.get(arg_id);
        if let Some(key) = arg.as_keyword() {
            let Some(value) = args.next() else {
                return Err("Expected value".into());
            };
            inner.push(obj_put_thunk(key.to_string(), value).assemble(&mut ast));
        } else {
            inner.push(("obj/eval", arg_id).assemble(&mut ast));
        }
    }

    inner.insert(0, "obj/condef".assemble(&mut ast));
    let result = inner.assemble(&mut ast);
    rt.asts.add_ast(ast);
    // tracing::debug!("obj/struct: {}", rt.asts.fmt(result));
    Ok(result)
}

fn obj_construct_or(rt: &mut Runtime, args: Vec<Value>) -> Result<Value, String> {
    let Ok([value]) = TryInto::<[Value; 1]>::try_into(args) else {
        return Err("Expected one argument".into());
    };

    match value {
        Value::Constructor(constructor) => Ok(rt.constructor_call(constructor, None)),
        val => Ok(val),
    }
}

pub fn prelude() -> Env {
    Env::default()
        .with_try_fn("-", sub)
        .with_try_fn("+", add)
        .with_try_fn("ref", new_ref)
        .with_try_fn("set", set)
        .with_try_fn("obj/insert", insert_to_struct)
        .with_try_macro("obj/condef", condef)
        .with_try_macro("obj/put", objput)
        .with_try_macro("obj/+", obj_add)
        .with_try_macro("obj/struct", obj_struct)
        .with_try_fn("obj/eval", obj_eval)
        .with_try_fn("obj/construct-or", obj_construct_or)
        .with_try_macro("struct", obj_struct)
        .with_try_macro("+obj", add_obj)
        .with_fn("print", |_rt, args| {
            for arg in args.into_iter() {
                tracing::info!("{:?}", arg);
            }

            Value::Number(1.0)
        })
}

impl Runtime {
    pub fn with_try_fn(
        &mut self,
        name: &str,
        body: impl Fn(&mut Runtime, Vec<Value>) -> Result<Value, String> + 'static,
    ) {
        self.with_fn(name, move |rt, args| {
            let result = body(rt, args);
            result.unwrap_or_else(Value::Error)
        });
    }

    pub fn with_try_macro(
        &mut self,
        name: &str,
        body: impl Fn(&mut Runtime, Vec<SExpId>) -> Result<SExpId, String> + 'static,
    ) {
        self.with_macro(name, move |rt, args| {
            let result = body(rt, args);
            match result {
                Ok(id) => id,
                Err(err) => {
                    tracing::error!("Error: {}", err);
                    error().build(&mut rt.asts)
                }
            }
        });
    }

    pub fn with_env(&mut self, env: Env) {
        self.envs.with_env(env);
    }

    pub fn with_prelude(&mut self) {
        self.with_env(prelude());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::ast::ASTS;

    use super::*;

    #[test]
    fn log() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "log", |input, _deps, _args| {
            // eprintln!("---");
            let mut asts = ASTS::new();
            let ast = asts.parse(input).unwrap();
            let root_id = ast.root_id().unwrap();
            let prelude = prelude();
            let envs = [prelude];
            let root_id = crate::process_ast(&mut asts, root_id, &envs);
            let [prelude] = envs;

            let mut runtime = Runtime::new(asts);
            runtime.with_env(prelude);
            let log = Arc::new(Mutex::new(String::new()));
            let log_clone = log.clone();
            runtime.with_fn("print", move |_rt, args| {
                for arg in args.into_iter() {
                    log_clone.lock().unwrap().push_str(&format!("{:?}\n", arg));
                }

                Value::Number(1.0)
            });

            _ = runtime.eval(root_id);
            let log = log.lock().unwrap().clone();
            log
        })
    }
}
