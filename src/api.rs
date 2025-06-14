use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::runtime::value::Ref;
use crate::runtime::{Runtime, Value};

//------------- IntoValue and FromValue ------------

impl<T: IntoValue> IntoValue for Result<T, String> {
    fn try_into_value(self) -> Result<Value, String> {
        match self {
            Ok(v) => v.try_into_value(),
            Err(e) => Ok(Value::Error(e)),
        }
    }
}

impl FromValue for Value {
    fn try_from_value(value: Value) -> Result<Self, String> {
        Ok(value)
    }
}
impl IntoValue for Value {
    fn try_into_value(self) -> Result<Value, String> {
        Ok(self)
    }
}

impl FromValue for f64 {
    fn try_from_value(value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Number(n) => Ok(n),
            value => Err(format!("Expected number, got {:?}", value)),
        }
    }
}

impl IntoValue for f64 {
    fn try_into_value(self) -> Result<Value, String> {
        Ok(Value::Number(self))
    }
}

impl FromValue for i32 {
    fn try_from_value(value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Number(n) => Ok(n as i32),
            value => Err(format!("Expected number, got {:?}", value)),
        }
    }
}

impl IntoValue for i32 {
    fn try_into_value(self) -> Result<Value, String> {
        Ok(Value::Number(self as f64))
    }
}

impl FromValue for bool {
    fn try_from_value(value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Bool(b) => Ok(b),
            value => Err(format!("Expected bool, got {:?}", value)),
        }
    }
}

impl IntoValue for bool {
    fn try_into_value(self) -> Result<Value, String> {
        Ok(Value::Bool(self))
    }
}

impl FromValue for String {
    fn try_from_value(value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::String(s) => Ok(s),
            value => Err(format!("Expected string, got {:?}", value)),
        }
    }
}

impl IntoValue for String {
    fn try_into_value(self) -> Result<Value, String> {
        Ok(Value::String(self))
    }
}

impl FromValue for Ref {
    fn try_from_value(value: Value) -> Result<Self, String> {
        match value.ok()? {
            Value::Ref(r) => Ok(r),
            value => Err(format!("Expected ref, got {:?}", value)),
        }
    }
}
impl IntoValue for Ref {
    fn try_into_value(self) -> Result<Value, String> {
        Ok(Value::Ref(self))
    }
}

// ------------- Definitions ------------

pub trait IntoValue {
    fn try_into_value(self) -> Result<Value, String>;
}

pub trait FromValue: Sized {
    fn try_from_value(value: Value) -> Result<Self, String>;
}

pub trait NativeFunction {
    fn call(&self, rt: &mut Runtime, values: Vec<Value>) -> Value {
        self.try_call(rt, values).unwrap_or_else(Value::Error)
    }

    fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String>;
}

pub trait IntoNativeFunction<Ctx> {
    fn into_native_function(self) -> Box<dyn NativeFunction>;
}

pub struct FnLike<F, Ctx> {
    f: F,
    marker: PhantomData<Ctx>,
}

impl<F, Ctx> FnLike<F, Ctx> {
    pub fn box_new(f: F) -> Box<Self> {
        Box::new(Self {
            f,
            marker: PhantomData,
        })
    }
}

pub struct Rest<T> {
    values: Vec<T>,
}
impl<T> Rest<T>
where
    T: FromValue,
{
    pub fn new(values: Vec<T>) -> Self {
        Self { values }
    }

    fn try_from_values(values: Rest<Value>) -> Result<Self, String> {
        let values = values
            .values
            .into_iter()
            .map(T::try_from_value)
            .collect::<Result<Vec<T>, String>>()?;
        Ok(Self { values })
    }

    pub fn with_arity<const N: usize>(self) -> Result<[T; N], String>
    where
        T: std::fmt::Debug,
    {
        assert_arity(N, &self.values)?;
        Ok(self.values.try_into().unwrap())
    }
}
impl<T> From<Rest<T>> for Vec<T> {
    fn from(rest: Rest<T>) -> Self {
        rest.values
    }
}
impl<T> Deref for Rest<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}
impl<T> DerefMut for Rest<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}
impl<T> IntoIterator for Rest<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

fn assert_arity<T>(len: usize, values: &[T]) -> Result<(), String> {
    if values.len() != len {
        return Err(format!("Expected {len} arguments, got {}", values.len()));
    }
    Ok(())
}

//------------- Utils ------------

trait TupleLen {
    const LEN: usize;
}

impl TupleLen for () {
    const LEN: usize = 0;
}
// impl<T> TupleLen for (T,) { const LEN: usize = 1;}
macro_rules! tuple_len {
    () => {};
    ($first: tt  $(,$arg: tt)*) => {

        tuple_len!($($arg),*);
        impl<$first, $($arg),*> TupleLen for ($first, $($arg,)*) { const LEN: usize = 1 + <($($arg,)*) as TupleLen>::LEN; }
    };
}
tuple_len!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

//------------- Implementations ------------

// Possible cases:
// 1. With runtime prefixes
// 2. With arguments in the middle
// 3. With rest at the end

pub struct WithRuntime;
pub struct WithRest;
// struct WithArgs;

macro_rules! fnlike {
    // 1. Without runtime or rest
    (NO,) => {
        impl<F, O> IntoNativeFunction<(O,)> for F
        where F: 'static + Fn() -> O,
            O: IntoValue + 'static,
        {
            fn into_native_function(self) -> Box<dyn NativeFunction> {
                FnLike::<F, (O,)>::box_new(self)
            }
        }

        impl<F, O> NativeFunction for FnLike<F, (O,)>
        where F: 'static + Fn() -> O,
            O: IntoValue,
        {
            fn try_call(&self, _rt: &mut Runtime, _values: Vec<Value>) -> Result<Value, String> {
                let o = (self.f)();
                O::try_into_value(o)
            }
        }
    };

    (NO, $first: tt, $($arg: tt,)*) => {
        fnlike!(NO, $($arg,)*);

        impl<F, O, $first $(,$arg)*> IntoNativeFunction<(O, $first, $($arg),*)> for F
        where
            F: 'static + Fn($first, $($arg),*) -> O,
            O: IntoValue + 'static,
            $first: FromValue + 'static,
            $($arg: FromValue + 'static),*
        {
            fn into_native_function(self) -> Box<dyn NativeFunction> {
                FnLike::<F, (O, $first, $($arg),*)>::box_new(self)
            }
        }

        #[allow(non_snake_case)]
        impl<F, O, $first, $($arg),*> NativeFunction for FnLike<F, (O, $first, $($arg),*)>
        where
            F: 'static + Fn($first, $($arg),*) -> O,
            O: IntoValue,
            $first: FromValue,
            $($arg: FromValue),*
        {
            fn try_call(&self, _rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
                assert_arity(<($first, $($arg),*) as TupleLen>::LEN, &values)?;
                let mut values = values.into_iter();

                let $first = $first::try_from_value(values.next().unwrap())?;
                $(
                    let $arg = $arg::try_from_value(values.next().unwrap())?;
                )*

                let o = (self.f)($first, $($arg,)*);

                O::try_into_value(o)
            }
        }


    };


    // 2. With runtime, no rest
    (RT ,) => {
        impl<F, O> IntoNativeFunction<(O, WithRuntime)> for F
        where F: 'static + Fn(&mut Runtime) -> O,
            O: IntoValue + 'static,
        {
            fn into_native_function(self) -> Box<dyn NativeFunction> {
                FnLike::<F, (O, WithRuntime)>::box_new(self)
            }
        }

        impl<F, O> NativeFunction for FnLike<F, (O, WithRuntime)>
        where F: 'static + Fn(&mut Runtime) -> O,
            O: IntoValue,
        {
            fn try_call(&self, rt: &mut Runtime, _values: Vec<Value>) -> Result<Value, String> {
                let o = (self.f)(rt);
                O::try_into_value(o)
            }
        }

    };

    (RT, $first: tt, $($arg: tt,)*) => {
        fnlike!(RT, $($arg,)*);

        impl<F, O, $first, $($arg),*> IntoNativeFunction<(O, $first, $($arg,)* WithRuntime)> for F
        where
            F: 'static + Fn(&mut Runtime, $first, $($arg),*) -> O,
            O: IntoValue + 'static,
            $first: FromValue + 'static,
            $($arg: FromValue + 'static),*
        {
            fn into_native_function(self) -> Box<dyn NativeFunction> {
                FnLike::<F, (O, $first, $($arg,)* WithRuntime)>::box_new(self)
            }
        }

        #[allow(non_snake_case)]
        impl<F, O, $first, $($arg),*> NativeFunction for FnLike<F, (O, $first, $($arg,)* WithRuntime)>
        where
            F: 'static + Fn(&mut Runtime, $first, $($arg),*) -> O,
            O: IntoValue,
            $first: FromValue,
            $($arg: FromValue),*
        {
            fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
                assert_arity(<($first, $($arg),*) as TupleLen>::LEN, &values)?;
                let mut values = values.into_iter();

                let $first = $first::try_from_value(values.next().unwrap())?;
                $(
                    let $arg = $arg::try_from_value(values.next().unwrap())?;
                )*

                let o = (self.f)(rt, $first, $($arg,)*);

                O::try_into_value(o)
            }
        }

    };

    // 3. With rest no runtime

    (RE ,) => {
        impl<F, O, R> IntoNativeFunction<(O, R, WithRest)> for F
        where
            F: 'static + Fn(Rest<R>) -> O,
            O: IntoValue + 'static,
            R: FromValue + 'static,
        {
            fn into_native_function(self) -> Box<dyn NativeFunction> {
                FnLike::<F, (O, R, WithRest)>::box_new(self)
            }
        }

        impl<F, O, R> NativeFunction for FnLike<F, (O, R, WithRest)>
        where
            F: 'static + Fn(Rest<R>) -> O,
            O: IntoValue,
            R: FromValue,
        {
            fn try_call(&self, _rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
                let rest = Rest { values };
                let rest = Rest::<R>::try_from_values(rest)?;
                let o = (self.f)(rest);
                O::try_into_value(o)
            }
        }
    };

    (RE, $first: tt, $($arg: tt,)*) => {
        fnlike!(RE, $($arg,)*);

        impl<F, O, $first, $($arg,)* R> IntoNativeFunction<(O, $first, $($arg,)* R, WithRest)> for F
        where
            F: 'static + Fn($first, $($arg,)* Rest<R>) -> O,
            O: IntoValue + 'static,
            R: FromValue + 'static,
            $first: FromValue + 'static,
            $($arg: FromValue + 'static),*
        {
            fn into_native_function(self) -> Box<dyn NativeFunction> {
                FnLike::<F, (O, $first, $($arg,)* R, WithRest)>::box_new(self)
            }
        }

        #[allow(non_snake_case)]
        impl<F, O, $first, $($arg,)* R> NativeFunction for FnLike<F, (O, $first, $($arg,)* R, WithRest)>
        where
            F: 'static + Fn($first, $($arg,)* Rest<R>) -> O,
            O: IntoValue,
            R: FromValue,
            $first: FromValue,
            $($arg: FromValue),*
        {
            fn try_call(&self, _rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
                assert_arity(<($first, $($arg),*) as TupleLen>::LEN + 1, &values)?;

                let mut values = values.into_iter();

                let $first = $first::try_from_value(values.next().unwrap())?;
                $(
                    let $arg = $arg::try_from_value(values.next().unwrap())?;
                )*

                let rest = Rest { values: values.collect() };

                let rest = Rest::<R>::try_from_values(rest)?;
                let o = (self.f)($first, $($arg,)* rest);

                O::try_into_value(o)
            }
        }

    };

    // 4. With runtime and rest

    (RTRE ,) => {
        impl<F, O, R> IntoNativeFunction<(O, R, WithRuntime, WithRest)> for F
        where
            F: 'static + Fn(&mut Runtime, Rest<R>) -> O,
            O: IntoValue + 'static,
            R: FromValue + 'static,
        {
            fn into_native_function(self) -> Box<dyn NativeFunction> {
                FnLike::<F, (O, R, WithRuntime, WithRest)>::box_new(self)
            }
        }

        impl<F, O, R> NativeFunction for FnLike<F, (O, R, WithRuntime, WithRest)>
        where
            F: 'static + Fn(&mut Runtime, Rest<R>) -> O,
            O: IntoValue,
            R: FromValue,
        {
            fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
                let rest = Rest { values };
                let rest = Rest::<R>::try_from_values(rest)?;
                let o = (self.f)(rt, rest);
                O::try_into_value(o)
            }
        }
    };

    (RTRE, $first: tt, $($arg: tt,)*) => {
        fnlike!(RTRE, $($arg,)*);

        impl<F, O, $first, $($arg,)* R> IntoNativeFunction<(O, $first, $($arg,)* R, WithRuntime, WithRest)> for F
        where
            F: 'static + Fn(&mut Runtime, $first, $($arg,)* Rest<R>) -> O,
            O: IntoValue + 'static,
            R: FromValue + 'static,
            $first: FromValue + 'static,
            $($arg: FromValue + 'static),*
        {
            fn into_native_function(self) -> Box<dyn NativeFunction> {
                FnLike::<F, (O, $first, $($arg,)* R, WithRuntime, WithRest)>::box_new(self)
            }
        }

        #[allow(non_snake_case)]
        impl<F, O, $first, $($arg,)* R> NativeFunction for FnLike<F, (O, $first, $($arg,)* R, WithRuntime, WithRest)>
        where
            F: 'static + Fn(&mut Runtime, $first, $($arg,)* Rest<R>) -> O,
            O: IntoValue,
            R: FromValue,
            $first: FromValue,
            $($arg: FromValue),*
        {
            fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
                assert_arity(<($first, $($arg),*) as TupleLen>::LEN + 1, &values)?;

                let mut values = values.into_iter();

                let $first = $first::try_from_value(values.next().unwrap())?;
                $(
                    let $arg = $arg::try_from_value(values.next().unwrap())?;
                )*

                let rest = Rest { values: values.collect() };
                let rest = Rest::<R>::try_from_values(rest)?;
                let o = (self.f)(rt, $first, $($arg,)* rest);

                O::try_into_value(o)
            }
        }

    };
}

fnlike!(
    NO, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16,
);
fnlike!(
    RT, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16,
);
fnlike!(
    RE, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16,
);
fnlike!(
    RTRE, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16,
);

#[cfg(test)]
mod tests {
    use super::*;

    fn check<Ctx>(_a: impl IntoNativeFunction<Ctx>) {}

    #[test]
    fn fnlikes() {
        check(|a: i32, b: i32| a + b);
        check(|_rt: &mut Runtime, p1: i32| p1);
        check(|p1: i32, rest: Rest<i32>| p1 + rest.into_iter().sum::<i32>());
        check(|p1: i32, rest: Rest<Value>| {
            p1 + rest
                .into_iter()
                .flat_map(|v| v.as_number())
                .map(|n| n as i32)
                .sum::<i32>()
        });
        check(|_rt: &mut Runtime, p1: i32, rest: Rest<i32>| p1 + rest.into_iter().sum::<i32>());
    }
}
