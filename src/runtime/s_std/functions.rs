use std::{collections::BTreeMap, path::PathBuf};

use itertools::Itertools;

use crate::{
    api::{
        CalledConstructor, EagerRec, FromValue, IntoNativeFunction, Rest, WithConstructor,
        WithoutConstructor,
    },
    ast::SExp,
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
fn add_numbers(
    first: EagerRec<f64, WithoutConstructor>,
    args: Rest<EagerRec<f64, WithoutConstructor>>,
) -> f64 {
    first.value
        + args
            .into_iter()
            .map(|a| a.value)
            .reduce(|a, b| a + b)
            .unwrap_or(0.0)
}

#[derive(Clone, Debug)]
pub enum ObjectOrConstructor {
    Object(BTreeMap<String, Value>),
    Constructor(Constructor),
}

impl FromValue for ObjectOrConstructor {
    fn is_matching(_rt: &mut Runtime, value: &Value) -> bool {
        matches!(value, Value::Object(_) | Value::Constructor(_))
    }

    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value {
            Value::Object(left) => Ok(ObjectOrConstructor::Object(left)),
            Value::Constructor(left) => Ok(ObjectOrConstructor::Constructor(left)),
            _ => Err(format!("Expected object or constructor, got {:?}", value)),
        }
    }
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
                let ref_self = self_.as_ref().cloned().unwrap();
                for (key, value) in left {
                    insert_to_struct(ref_self.clone(), StructKey(key), CalledConstructor(value))?;
                }
                self_
            }
        })
    }
}

#[tracing::instrument(skip_all)]
fn add_two_objects(left: ObjectOrConstructor, right: ObjectOrConstructor) -> Constructor {
    let constructor = move |rt: &mut Runtime,
                            self_: Value,
                            root: Value,
                            _super: Value,
                            origin: Value|
          -> Result<Value, String> {
        let left = left.clone();
        let right = right.clone();
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
    };
    let constructor = constructor.into_native_function();

    Constructor {
        constructor: Function::from(move |rt: &mut Runtime, args: Vec<Value>| {
            constructor.call(rt, args)
        }),
    }
}

fn add_objects(
    left: EagerRec<ObjectOrConstructor, WithoutConstructor>,
    rights: Rest<EagerRec<ObjectOrConstructor, WithoutConstructor>>,
) -> Value {
    let mut left = left.value;
    for right in rights {
        left = ObjectOrConstructor::Constructor(add_two_objects(left.clone(), right.value));
    }
    match left {
        ObjectOrConstructor::Object(left) => Value::Object(left),
        ObjectOrConstructor::Constructor(left) => Value::Constructor(left),
    }
}

#[allow(non_upper_case_globals, clippy::type_complexity)]
pub const add: (
    fn(EagerRec<f64, WithoutConstructor>, Rest<EagerRec<f64, WithoutConstructor>>) -> f64,
    fn(
        EagerRec<ObjectOrConstructor, WithoutConstructor>,
        Rest<EagerRec<ObjectOrConstructor, WithoutConstructor>>,
    ) -> Value,
) = (add_numbers, add_objects);

pub fn set(key: Ref, value: Value) -> Value {
    tracing::info!("Setting {key:?} to {value:?}");

    *key.borrow_mut() = value.clone();

    value
}

pub fn new_ref(one: Value) -> Value {
    Value::ref_(one)
}

pub struct StructKey(String);
impl FromValue for StructKey {
    fn is_matching(rt: &mut Runtime, value: &Value) -> bool {
        match value {
            Value::String(_) => true,
            Value::SExp(id) => matches!(
                rt.asts.get(*id).item,
                SExp::Symbol(_) | SExp::Keyword(_) | SExp::String(_)
            ),
            _ => false,
        }
    }

    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        let key = match value {
            Value::String(s) => s,
            Value::SExp(id) => match &rt.asts.get(id).item {
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
pub fn insert_to_struct(
    this: Ref,
    key: StructKey,
    value: CalledConstructor<Value>,
) -> Result<Value, String> {
    let value = value.0;
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
            let ast = &rt.asts.get(id).item;
            let Some(list) = ast.as_list() else {
                return Err("Expected list".into());
            };

            let list = list.to_vec();
            let mut iter = list.into_iter();
            let mut last = None;
            while let Some(key) = iter.next() {
                let key = &rt.asts.get(key).item;
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

pub fn obj_construct_or(value: CalledConstructor<Value>) -> Value {
    value.0
}

pub fn eager(value: EagerRec<Value, WithConstructor>) -> Value {
    value.value
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

pub struct SymbolOrKeyword(String);
impl FromValue for SymbolOrKeyword {
    fn is_matching(rt: &mut Runtime, value: &Value) -> bool {
        match value {
            Value::SExp(id) => matches!(&rt.asts.get(*id).item, SExp::Symbol(_) | SExp::Keyword(_)),
            _ => false,
        }
    }

    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        let sexp = value.as_sexp().ok_or("Expected symbol or keyword")?;
        let sexp = rt.asts.get(*sexp);
        match &sexp.item {
            SExp::Symbol(s) | SExp::Keyword(s) => Ok(Self(s.to_string())),
            _ => Err("Expected symbol or keyword".into()),
        }
    }
}

pub fn obj_has(
    obj: EagerRec<BTreeMap<String, Value>, WithConstructor>,
    key: EagerRec<SymbolOrKeyword, WithConstructor>,
) -> Result<Value, String> {
    let obj = obj.value;
    let key = key.value;

    Ok(Value::Bool(obj.contains_key(&key.0)))
}

pub fn obj_plain(rt: &mut Runtime, args: Rest<Value>) -> Result<Value, String> {
    if args.len() % 2 != 0 {
        return Err("Expected even number of arguments".into());
    }
    let mut inner = BTreeMap::new();

    for (key, value) in args.into_iter().tuples() {
        let key = rt.as_keyword(&key).ok_or("Expected keyword")?;
        inner.insert(key.to_string(), value);
    }

    Ok(Value::Object(inner))
}

pub fn obj_extend(
    rt: &mut Runtime,
    obj: EagerRec<BTreeMap<String, Value>, WithConstructor>,
    args: Rest<Value>,
) -> Result<BTreeMap<String, Value>, String> {
    let mut obj = obj.value;

    for (key, value) in args.into_iter().tuples() {
        let key = rt.as_keyword(&key).ok_or("Expected keyword")?;
        obj.insert(key.to_string(), value);
    }

    Ok(obj)
}

pub fn obj_con(constructor: EagerRec<Function, WithoutConstructor>) -> Value {
    Value::Constructor(Constructor {
        constructor: constructor.value,
    })
}

pub fn make_list(args: Rest<Value>) -> Value {
    Value::List(args.into_iter().collect())
}

pub fn make_tuple(args: Rest<Value>) -> Value {
    Value::List(args.into_iter().collect())
}

pub fn import(rt: &mut Runtime, path: String) -> Result<Value, String> {
    let modules = rt.modules();
    let path_buf = PathBuf::from(&path);
    let Some(module) = modules.get_module(&path_buf) else {
        return Err(format!("Module not found: {}", path_buf.display()));
    };
    let module = module.to_string();

    let ast = rt.asts.parse(&module, &path).map_err(|e| e.to_string())?;
    let root = ast.root_id().ok_or("Import: Expected root")?;

    Ok(rt.eval(root))
}

pub fn lg(left: f64, right: f64) -> bool {
    left > right
}

pub fn error(s: String) -> Value {
    Value::Error(s)
}
