use std::{collections::BTreeMap, marker::PhantomData};

use crate::{
    ast::{SExp, SExpId},
    runtime::{
        Function, Runtime, Value,
        value::{Enum, Ref},
    },
};

use super::{
    FromValue, IntoValue,
    typing::{Fun, Param, TypeOf},
};

use crate::types::builder::{id_fn, reference};

pub struct RefOf<A>(pub Ref, pub PhantomData<A>);

impl<A> FromValue for RefOf<A> {
    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Ref(r) => Ok(RefOf(r, PhantomData::<A>)),
            other => Err(format!("Expected ref, got {other:?}")),
        }
    }

    fn is_matching(_rt: &mut Runtime, value: &Value) -> bool {
        value.as_ref().is_some()
    }
}

impl<A> IntoValue for RefOf<A> {
    fn try_into_value(self, _rt: &mut Runtime) -> Result<Value, String> {
        Ok(Value::Ref(self.0))
    }
}

impl<A> TypeOf for RefOf<A>
where
    A: TypeOf,
{
    fn ty(g: &crate::api::typing::TypeGen) -> Box<dyn crate::types::builder::TypeBuilder> {
        let a = <A as TypeOf>::ty(g);
        let a = id_fn(move |env: &mut crate::types::TypeEnv, src| a.build(env, src));
        Box::new(reference(a))
    }
}

impl<T: IntoValue> IntoValue for Result<T, String> {
    fn try_into_value(self, rt: &mut Runtime) -> Result<Value, String> {
        match self {
            Ok(v) => v.try_into_value(rt),
            Err(e) => Ok(Value::Error(e)),
        }
    }
}

impl FromValue for Value {
    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        Ok(value)
    }
    fn is_matching(_rt: &mut Runtime, _value: &Value) -> bool {
        true
    }
}
impl IntoValue for Value {
    fn try_into_value(self, _rt: &mut Runtime) -> Result<Value, String> {
        Ok(self)
    }
}

impl FromValue for f64 {
    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Number(n) => Ok(n),
            value => Err(format!("Expected number, got {value:?}")),
        }
    }
    fn is_matching(_rt: &mut Runtime, value: &Value) -> bool {
        value.as_number().is_some()
    }
}

impl IntoValue for f64 {
    fn try_into_value(self, _rt: &mut Runtime) -> Result<Value, String> {
        Ok(Value::Number(self))
    }
}

impl FromValue for i32 {
    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Number(n) => Ok(n as i32),
            value => Err(format!("Expected number, got {value:?}")),
        }
    }

    fn is_matching(_rt: &mut Runtime, value: &Value) -> bool {
        value.as_number().is_some()
    }
}

impl IntoValue for i32 {
    fn try_into_value(self, _rt: &mut Runtime) -> Result<Value, String> {
        Ok(Value::Number(self as f64))
    }
}

impl FromValue for bool {
    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Bool(b) => Ok(b),
            value => Err(format!("Expected bool, got {value:?}")),
        }
    }

    fn is_matching(_rt: &mut Runtime, value: &Value) -> bool {
        value.as_boolean().is_some()
    }
}

impl IntoValue for bool {
    fn try_into_value(self, _rt: &mut Runtime) -> Result<Value, String> {
        Ok(Value::Bool(self))
    }
}

impl FromValue for String {
    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::String(s) => Ok(s),
            value => Err(format!("Expected string, got {value:?}")),
        }
    }

    fn is_matching(_rt: &mut Runtime, value: &Value) -> bool {
        value.as_string().is_some()
    }
}

impl IntoValue for String {
    fn try_into_value(self, _rt: &mut Runtime) -> Result<Value, String> {
        Ok(Value::String(self))
    }
}

impl FromValue for Ref {
    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Ref(r) => Ok(r),
            value => Err(format!("Expected ref, got {value:?}")),
        }
    }

    fn is_matching(_rt: &mut Runtime, value: &Value) -> bool {
        value.as_ref().is_some()
    }
}
impl IntoValue for Ref {
    fn try_into_value(self, _rt: &mut Runtime) -> Result<Value, String> {
        Ok(Value::Ref(self))
    }
}

impl FromValue for BTreeMap<String, (Value, Option<SExpId>)> {
    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Object(map) => Ok(map),
            value => Err(format!("Expected object, got {value:?}")),
        }
    }

    fn is_matching(_rt: &mut Runtime, value: &Value) -> bool {
        value.as_object().is_some()
    }
}

impl IntoValue for BTreeMap<String, (Value, Option<SExpId>)> {
    fn try_into_value(self, _rt: &mut Runtime) -> Result<Value, String> {
        Ok(Value::Object(self))
    }
}

impl<T: FromValue + 'static> FromValue for Vec<T> {
    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::List(list) => list
                .into_iter()
                .map(|v| <T as FromValue>::try_from_value(rt, v))
                .collect(),
            value => Err(format!("Expected list, got {value:?}")),
        }
    }

    fn is_matching(_: &mut Runtime, value: &Value) -> bool {
        value.as_list().is_some()
    }
}

impl<T: IntoValue + 'static> IntoValue for Vec<T> {
    fn try_into_value(self, _rt: &mut Runtime) -> Result<Value, String> {
        let list = self
            .into_iter()
            .map(|item| item.try_into_value(_rt))
            .collect::<Result<Vec<Value>, String>>()?;
        Ok(Value::List(list))
    }
}

impl FromValue for Function {
    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Function(f) => Ok(f),
            value => Err(format!("Expected function, got {value:?}")),
        }
    }

    fn is_matching(_rt: &mut Runtime, value: &Value) -> bool {
        value.as_function().is_some()
    }
}

pub struct CalledConstructor<T>(pub T);

impl<T> FromValue for CalledConstructor<T>
where
    T: FromValue,
{
    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        let value = match value.ok()? {
            Value::Constructor(c) => rt.constructor_call(c, None),
            value => value,
        };
        let value = T::try_from_value(rt, value)?;

        Ok(Self(value))
    }

    fn is_matching(rt: &mut Runtime, value: &Value) -> bool {
        T::is_matching(rt, value)
    }
}

pub struct EagerRec<T, Marker> {
    pub value: T,
    marker: PhantomData<Marker>,
}

pub struct WithConstructor;
pub struct WithoutConstructor;

impl<T> FromValue for EagerRec<T, WithoutConstructor>
where
    T: FromValue,
{
    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        let value = value.eager_rec(rt, false);
        let value = T::try_from_value(rt, value)?;
        Ok(Self {
            value,
            marker: PhantomData,
        })
    }

    fn is_matching(rt: &mut Runtime, value: &Value) -> bool {
        let value = value.clone().eager_rec(rt, false);
        T::is_matching(rt, &value)
    }
}

impl<T> FromValue for EagerRec<T, WithConstructor>
where
    T: FromValue,
{
    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        let value = value.eager_rec(rt, true);
        let value = T::try_from_value(rt, value)?;
        Ok(Self {
            value,
            marker: PhantomData,
        })
    }

    fn is_matching(rt: &mut Runtime, value: &Value) -> bool {
        let value = value.clone().eager_rec(rt, true);
        T::is_matching(rt, &value)
    }
}

impl<T> FromValue for (T,)
where
    T: FromValue + 'static,
{
    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::List(mut list) => {
                if list.len() != 1 {
                    return Err(format!(
                        "Expected 1-element tuple (list), got {}",
                        list.len()
                    ));
                }
                let v = list.remove(0);
                let inner = T::try_from_value(rt, v)?;
                Ok((inner,))
            }
            other => Err(format!("Expected list for tuple, got {other:?}")),
        }
    }

    fn is_matching(rt: &mut Runtime, value: &Value) -> bool {
        if let Value::List(list) = value {
            if list.len() == 1 {
                return T::is_matching(rt, &list[0]);
            }
        }
        false
    }
}

impl<T> IntoValue for (T,)
where
    T: IntoValue + 'static,
{
    fn try_into_value(self, rt: &mut Runtime) -> Result<Value, String> {
        let (t,) = self;
        let inner = t.try_into_value(rt)?;
        Ok(Value::List(vec![inner]))
    }
}

impl<T, U> IntoValue for (T, U)
where
    T: IntoValue,
    U: IntoValue,
{
    fn try_into_value(self, rt: &mut Runtime) -> Result<Value, String> {
        let (first, second) = self;
        let first = first.try_into_value(rt)?;
        let second = second.try_into_value(rt)?;
        Ok(Value::List(vec![first, second]))
    }
}

pub struct Keyword(pub String);

impl FromValue for Keyword {
    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        let sexp = value.as_sexp().ok_or("Expected symbol or keyword")?;
        let sexp = rt.asts().get(*sexp);
        match &**sexp {
            SExp::Keyword(s) => Ok(Self(s.to_string())),
            _ => Err("Expected symbol or keyword".into()),
        }
    }

    fn is_matching(rt: &mut Runtime, value: &Value) -> bool {
        match value {
            Value::SExp(id) => matches!(&**rt.asts().get(*id), SExp::Keyword(_)),
            _ => false,
        }
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue + 'static,
{
    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Enum(e) => match e.variant.as_str() {
                "Some" => {
                    let mut it = e.fields.into_iter();
                    let first = it.next().ok_or("Some: missing field")?;
                    let v = T::try_from_value(rt, first)?;
                    Ok(Some(v))
                }
                "None" => Ok(None),
                other => Err(format!("Expected Option enum, got variant {other}")),
            },
            v => Err(format!("Expected enum for Option, got {v:?}")),
        }
    }

    fn is_matching(rt: &mut Runtime, value: &Value) -> bool {
        if let Value::Enum(e) = value {
            if e.variant == "None" {
                return true;
            }
            if e.variant == "Some" {
                return e
                    .fields
                    .first()
                    .map(|v| T::is_matching(rt, v))
                    .unwrap_or(false);
            }
        }
        false
    }
}

impl<T> IntoValue for Option<T>
where
    T: IntoValue + 'static,
{
    fn try_into_value(self, rt: &mut Runtime) -> Result<Value, String> {
        match self {
            Some(v) => {
                let inner = v.try_into_value(rt)?;
                Ok(Value::Enum(Enum {
                    variant: "Some".into(),
                    fields: vec![inner],
                }))
            }
            None => Ok(Value::Enum(Enum {
                variant: "None".into(),
                fields: vec![],
            })),
        }
    }
}

impl<T> FromValue for Result<T, String>
where
    T: FromValue + 'static,
{
    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value {
            Value::Error(e) => Ok(Err(e)),
            other => {
                let v = T::try_from_value(rt, other)?;
                Ok(Ok(v))
            }
        }
    }

    fn is_matching(rt: &mut Runtime, value: &Value) -> bool {
        matches!(value, Value::Error(_)) || T::is_matching(rt, value)
    }
}

// Typed function wrapper conversions for higher-order args

impl<Args, Ret> FromValue for Fun<Args, Ret> {
    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Function(f) => Ok(Fun(f, PhantomData::<(Args, Ret)>)),
            other => Err(format!("Expected function, got {other:?}")),
        }
    }
    fn is_matching(_rt: &mut Runtime, value: &Value) -> bool {
        value.as_function().is_some()
    }
}

impl<Args, Ret> IntoValue for Fun<Args, Ret> {
    fn try_into_value(self, _rt: &mut Runtime) -> Result<Value, String> {
        Ok(Value::Function(self.0))
    }
}

// Auto-typing adapters for list/enumerate using Param marker.
// Provide runtime conversions so IntoNativeFunction can parse Param values.
impl<T, const ID: usize> FromValue for Param<ID, T>
where
    T: FromValue,
{
    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String> {
        let inner = T::try_from_value(rt, value)?;
        Ok(Param::<ID, T>(inner))
    }

    fn is_matching(rt: &mut Runtime, value: &Value) -> bool {
        T::is_matching(rt, value)
    }
}

impl<T, const ID: usize> IntoValue for Param<ID, T>
where
    T: IntoValue,
{
    fn try_into_value(self, rt: &mut Runtime) -> Result<Value, String> {
        self.0.try_into_value(rt)
    }
}
