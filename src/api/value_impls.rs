use std::{collections::BTreeMap, marker::PhantomData};

use crate::{
    ast::SExpId,
    runtime::{Function, Runtime, Value, value::Ref},
};

use super::{FromValue, IntoValue};

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

impl FromValue for Vec<Value> {
    fn try_from_value(_rt: &mut Runtime, value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::List(list) => Ok(list.into_iter().collect()),
            value => Err(format!("Expected list, got {value:?}")),
        }
    }

    fn is_matching(_: &mut Runtime, value: &Value) -> bool {
        value.as_list().is_some()
    }
}

impl<T: IntoValue> IntoValue for Vec<T> {
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
