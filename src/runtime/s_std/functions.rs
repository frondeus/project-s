use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    api::{FromValue, Rest},
    ast::{SExp, SExpParser},
    builder::ASTBuilder,
    runtime::{
        Function, Runtime, Value,
        value::{Constructor, Ref},
    },
};

use super::macros::obj_put_thunk;

pub fn sub(args: Rest<f64>) -> f64 {
    args.into_iter().reduce(|a, b| a - b).unwrap_or(0.0)
}

#[tracing::instrument(skip_all)]
pub fn add(rt: &mut Runtime, args: Rest<Value>) -> Result<Value, String> {
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
                        insert_to_struct(
                            rt,
                            Rest::new(vec![self_.clone(), Value::String(key), value]),
                        )?;
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
        tracing::debug!("Adding obj: {:?}, {:?}", left, right);
        tracing::debug!("Self: {:?}", self_);
        tracing::debug!("Root: {:?}", root);
        let left = left.call(
            rt,
            self_.clone(),
            Some(root.clone()),
            self_.clone(),
            Some(origin),
        )?;
        tracing::debug!("Left: {:?}", left);
        let super_ = left.deref();
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

pub fn set(key: Ref, value: Value) -> Value {
    tracing::info!("Setting");

    tracing::info!("Setting {key:?} to {value:?}");

    *key.borrow_mut() = value.clone();

    value
}

pub fn new_ref(one: Value) -> Value {
    Value::ref_(one)
}

struct StructKey(String);
impl FromValue for StructKey {
    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        let key = match value {
            Value::String(s) => s,
            Value::SExp(id) => match rt.asts.get(id) {
                SExp::Symbol(s) => s.to_string(),
                SExp::Keyword(s) => s.to_string(),
                SExp::String(s) => s.to_string(),
                _ => return Err("Expected keyword, symbol or string".into()),
            },
            _ => return Err("Expected keyword, symbol or string".into()),
        };
        Ok(Self(key))
    }
}

#[tracing::instrument(skip_all)]
pub fn insert_to_struct(rt: &mut Runtime, args: Rest<Value>) -> Result<Value, String> {
    // println!("Inserting to struct");
    let [_this, key, value] = args.with_arity::<3>()?;

    let key = StructKey::try_from_value(rt, key)?;

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

    tracing::debug!("Inserting to struct({:?}): {} - {:?}", this, key.0, value);

    let old = this.insert(key.0, value);

    tracing::debug!("After insertion: {this:?}");

    Ok(old.unwrap_or_else(|| Value::List(vec![])))
}

pub fn obj_eval(rt: &mut Runtime, to_eval: Value) -> Result<Value, String> {
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

pub fn obj_construct_or(rt: &mut Runtime, value: Value) -> Value {
    match value {
        Value::Constructor(constructor) => rt.constructor_call(constructor, None),
        val => val,
    }
}

pub fn eager(rt: &mut Runtime, value: Value) -> Value {
    value.eager_rec(rt, true)
}

pub fn deep_eager(rt: &mut Runtime, value: Value) -> Value {
    let eager_value = value.clone().eager_rec(rt, true);
    tracing::debug!("Deep eager: {:?}", eager_value);

    if let Value::Object(map) = &eager_value {
        for value in map.values() {
            deep_eager(rt, value.clone());
        }
    }
    value
}

pub fn obj_has(rt: &mut Runtime, obj: Value, key: Value) -> Result<Value, String> {
    let obj = obj.eager_rec(rt, true);
    let key = key.eager_rec(rt, true);

    let Some(obj) = obj.as_object() else {
        return Err("has?: Expected object".into());
    };

    let Some(key) = rt.as_symbol_or_keyword(&key) else {
        return Err("has?: Expected symbol or keyword".into());
    };

    Ok(Value::Bool(obj.contains_key(key)))
}

pub fn obj_con(rt: &mut Runtime, value: Value) -> Result<Value, String> {
    let value = value.eager_rec(rt, false);
    let Value::Function(constructor) = value else {
        return Err("obj/con: Expected function".into());
    };

    Ok(Value::Constructor(Constructor { constructor }))
}

pub fn make_list(args: Rest<Value>) -> Value {
    Value::List(args.into_iter().collect())
}

pub fn import(rt: &mut Runtime, path: String) -> Result<Value, String> {
    let modules = rt.modules();
    let path = PathBuf::from(path);
    let Some(module) = modules.get_module(&path) else {
        return Err(format!("Module not found: {}", path.display()));
    };
    let module = module.to_string();

    let parser = SExpParser::new(&mut rt.asts).map_err(|e| e.to_string())?;
    let ast = parser.parse(&module).map_err(|e| e.to_string())?;
    let root = ast.root_id().ok_or("Import: Expected root")?;
    rt.asts.add_ast(ast);

    Ok(rt.eval(root))
}

pub fn lg(left: f64, right: f64) -> bool {
    left > right
}

pub fn error(s: String) -> Value {
    Value::Error(s)
}
