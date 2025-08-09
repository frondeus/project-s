use std::{collections::BTreeMap, path::PathBuf};

use itertools::Itertools;

use crate::api::typing::{Fun, Param};

use crate::api::AllParams;
use crate::{
    api::{EagerRec, Rest, WithConstructor, WithoutConstructor},
    runtime::{Runtime, Value, value::Enum},
};

pub fn sub_numbers(args: Rest<EagerRec<f64, WithConstructor>>) -> f64 {
    args.into_iter()
        .map(|a| a.value)
        .reduce(|a, b| a - b)
        .unwrap_or(0.0)
}

pub fn set(key: crate::api::RefOf<Param<0>>, value: Param<0>) -> Param<0> {
    tracing::info!("Setting {:?} to {:?}", &key.0, &value.0);
    *key.0.borrow_mut() = value.0.clone();
    Param(value.0)
}

pub fn get(key: crate::api::RefOf<Param<0>>) -> Param<0> {
    Param(key.0.borrow().clone())
}

pub fn new_ref(a: Param<0>) -> crate::api::RefOf<Param<0>> {
    if let Value::Ref(r) = Value::ref_(a.0) {
        crate::api::RefOf(r, std::marker::PhantomData)
    } else {
        unreachable!()
    }
}

pub fn obj_plain(rt: &mut Runtime, args: Rest<Param<0>>) -> Result<Param<1>, String> {
    let values: Vec<Value> = args.into_iter().map(|p| p.0).collect();
    if values.len() % 2 != 0 {
        return Err("Expected even number of arguments".into());
    }
    let mut inner = BTreeMap::new();

    for (key, value) in values.into_iter().tuples() {
        let key_id = *key.as_sexp().ok_or("Expected keyword")?;
        let key = rt.as_keyword(&key).ok_or("Expected keyword")?;
        inner.insert(key.to_string(), (value, Some(key_id)));
    }

    Ok(Param(Value::Object(inner)))
}

pub fn import(rt: &mut Runtime, path: String) -> Result<Param<0>, String> {
    let modules = rt.modules_mut();
    let path_buf = PathBuf::from(&path);
    let Some(source_id) = modules.get_module(&path_buf) else {
        return Err(format!("Module not found: {}", path_buf.display()));
    };
    let Some(source) = modules.get_source(source_id) else {
        return Err(format!("Module not found: {}", path_buf.display()));
    };
    let source = source.clone();

    let ast = rt
        .asts
        .parse(source_id, &source)
        .map_err(|e| e.to_string())?;
    let root = ast.root_id().ok_or("Import: Expected root")?;

    let save = rt.envs.savepoint();
    let result = rt.eval(root);
    rt.envs.restore(save);

    Ok(Param(result))
}

// Eq
pub fn eq(l: Param<0>, r: Param<1>) -> bool {
    match (l.0, r.0) {
        (Value::Number(l), Value::Number(r)) => l == r,
        _ => false,
    }
}

pub fn eq_any(l: Param<0>, r: Param<1>) -> bool {
    eq(l, r)
}

// Greater than
pub fn gt_numbers(left: f64, right: f64) -> bool {
    left > right
}

// Less than or equal
pub fn lte_numbers(left: f64, right: f64) -> bool {
    left <= right
}

pub fn error(s: String) -> Param<0> {
    Param(Value::Error(s))
}

pub fn list_enumerate(list: Vec<Param<0>>) -> Vec<(i32, Param<0>)> {
    list.into_iter()
        .enumerate()
        .map(|(i, p)| (i as i32, p))
        .collect()
}

pub fn list_map(
    rt: &mut Runtime,
    list: Vec<Param<0>>,
    f: Fun<(Param<0>,), Param<1>>,
) -> Result<Vec<Param<1>>, String> {
    list.into_iter().map(|v| f.clone().call(rt, (v,))).collect()
}

pub fn list_find(
    rt: &mut Runtime,
    list: Vec<Param<0>>,
    f: Fun<(Param<0>,), bool>,
) -> Result<Option<Param<0>>, String> {
    for v in list {
        let res = f.clone().call(rt, (v.clone(),))?;
        if res {
            return Ok(Some(v));
        }
    }
    Ok(None)
}

pub fn some(a: Param<0>) -> Param<1> {
    Param(Value::Enum(Enum {
        variant: "Some".into(),
        fields: vec![a.0],
    }))
}

pub fn none() -> Param<0> {
    Param(Value::Enum(Enum {
        variant: "None".into(),
        fields: vec![],
    }))
}

pub fn add(args: Rest<EagerRec<f64, WithoutConstructor>>) -> f64 {
    args.into_iter().map(|a| a.value).sum()
}

pub fn mul(args: Rest<EagerRec<f64, WithConstructor>>) -> f64 {
    args.into_iter()
        .map(|a| a.value)
        .reduce(|a, b| a * b)
        .unwrap_or(1.0)
}

pub fn print(rt: &mut Runtime, args: Rest<Param<0>>) -> f64 {
    for arg in args.into_iter() {
        let arg = arg.0.eager_rec(rt, true);
        tracing::info!("{:?}", arg);
    }
    1.0
}

pub fn roll(formula: String) -> f64 {
    tracing::info!("Rolling {formula}");
    1.0
}

pub fn make_list(args: Rest<Param<0>>) -> Vec<Param<0>> {
    args.into()
}

pub fn tuple(args: AllParams<Param<0>>) -> Param<0> {
    let values: Vec<Value> = args.into();
    Param(Value::List(values))
}

pub fn construct_enum(rt: &mut Runtime, args: Rest<Param<0>>) -> Result<Param<1>, String> {
    let mut vals: Vec<Param<0>> = args.into();
    if vals.is_empty() {
        return Err("enum: Expected at least one argument".into());
    }
    let first = vals.remove(0).0;
    let name = rt
        .as_keyword(&first)
        .ok_or("enum: Expected keyword as first argument")?;
    let fields: Vec<Value> = vals.into_iter().map(|p| p.0).collect();
    Ok(Param(Value::Enum(Enum {
        variant: name.to_string(),
        fields,
    })))
}

pub fn obj_extend(rt: &mut Runtime, args: Rest<Param<0>>) -> Result<Param<0>, String> {
    use itertools::Itertools;

    let mut vals: Vec<Param<0>> = args.into();
    if vals.is_empty() {
        return Err("obj/extend: Expected at least one argument".into());
    }

    let mut obj = vals.remove(0).0.eager_rec(rt, true);
    let map = obj
        .as_object_mut()
        .ok_or("obj/extend: Expected object as first argument")?;

    for (key_p, val_p) in vals.into_iter().tuples() {
        let key_v = key_p.0;
        let id = *key_v.as_sexp().ok_or("obj/extend: Expected keyword")?;
        let key = rt
            .as_keyword(&key_v)
            .ok_or("obj/extend: Expected keyword")?;
        map.insert(key.to_string(), (val_p.0, Some(id)));
    }

    Ok(Param(obj))
}

pub fn debug(a: Param<0>) -> Param<0> {
    tracing::debug!("{:?}", a.0);
    a
}
