use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::runtime::{Runtime, Value};

mod value_impls;
pub use value_impls::*;
pub mod typing;

// ------------- Definitions ------------

pub trait OverloadedFunction {
    fn call(&self, rt: &mut Runtime, values: Vec<Value>) -> Value;
    fn with_name(&mut self, name: &'static str);
}

pub struct Overloaded<T> {
    fns: T,
    name: Option<&'static str>,
}

impl<T> Overloaded<T> {
    pub fn new(fns: T) -> Self {
        Self { fns, name: None }
    }
}

impl<T> OverloadedFunction for Overloaded<(T,)>
where
    T: NativeFunction,
{
    fn with_name(&mut self, name: &'static str) {
        self.name = Some(name);
        self.fns.0.with_name(name);
    }

    fn call(&self, rt: &mut Runtime, values: Vec<Value>) -> Value {
        let (f,) = &self.fns;

        f.call(rt, values)
    }
}

impl<T1, T2> OverloadedFunction for Overloaded<(T1, T2)>
where
    T1: NativeFunction,
    T2: NativeFunction,
{
    fn with_name(&mut self, name: &'static str) {
        self.name = Some(name);
        self.fns.0.with_name(name);
        self.fns.1.with_name(name);
    }

    fn call(&self, rt: &mut Runtime, values: Vec<Value>) -> Value {
        let name = self.name.unwrap_or("<anonymous>");
        let (f1, f2) = &self.fns;
        if f1.signature_matches(rt, &values) {
            return f1.call(rt, values);
        }
        if f2.signature_matches(rt, &values) {
            return f2.call(rt, values);
        }
        Value::Error(format!(
            "{name}: Calling with no matching signature: {values:?}"
        ))
    }
}

pub trait IntoOverloadedFunction<Ctx> {
    fn into_overloaded_function(self) -> Box<dyn OverloadedFunction>;
}

impl<T, Ctx> IntoOverloadedFunction<Ctx> for T
where
    T: IntoNativeFunction<Ctx>,
{
    fn into_overloaded_function(self) -> Box<dyn OverloadedFunction> {
        Box::new(Overloaded::new((self.into_native_function(),)))
    }
}

impl<T, Ctx> IntoOverloadedFunction<(Ctx,)> for (T,)
where
    T: IntoNativeFunction<Ctx>,
{
    fn into_overloaded_function(self) -> Box<dyn OverloadedFunction> {
        let (f,) = self;
        let f = f.into_native_function();
        Box::new(Overloaded::new((f,)))
    }
}
impl<T1, T2, Ctx1, Ctx2> IntoOverloadedFunction<(Ctx1, Ctx2)> for (T1, T2)
where
    T1: IntoNativeFunction<Ctx1>,
    T2: IntoNativeFunction<Ctx2>,
{
    fn into_overloaded_function(self) -> Box<dyn OverloadedFunction> {
        let (f1, f2) = self;
        let f1 = f1.into_native_function();
        let f2 = f2.into_native_function();
        Box::new(Overloaded::new((f1, f2)))
    }
}

pub trait IntoValue {
    fn try_into_value(self, rt: &mut Runtime) -> Result<Value, String>;
}

pub trait FromValue: Sized {
    fn try_from_value(rt: &mut Runtime, value: Value) -> Result<Self, String>;
    fn is_matching(rt: &mut Runtime, value: &Value) -> bool;
}

pub trait NativeFunction {
    fn signature_matches(&self, rt: &mut Runtime, values: &[Value]) -> bool;

    fn call(&self, rt: &mut Runtime, values: Vec<Value>) -> Value {
        self.try_call(rt, values).unwrap_or_else(Value::Error)
    }

    fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String>;

    fn with_name(&mut self, name: &'static str);
}

impl NativeFunction for Box<dyn NativeFunction> {
    fn signature_matches(&self, rt: &mut Runtime, values: &[Value]) -> bool {
        self.as_ref().signature_matches(rt, values)
    }

    fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
        self.as_ref().try_call(rt, values)
    }

    fn with_name(&mut self, name: &'static str) {
        self.as_mut().with_name(name)
    }
}

pub trait IntoNativeFunction<Ctx> {
    fn into_native_function(self) -> Box<dyn NativeFunction>;
}

pub struct FnLike<F, Ctx> {
    f: F,
    name: Option<&'static str>,
    marker: PhantomData<Ctx>,
}

impl<F, Ctx> FnLike<F, Ctx> {
    pub fn box_new(f: F) -> Box<Self> {
        Box::new(Self {
            f,
            name: None,
            marker: PhantomData,
        })
    }

    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
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

    pub fn try_from_values(rt: &mut Runtime, values: Rest<Value>) -> Result<Self, String> {
        let values = values
            .values
            .into_iter()
            .map(|v| T::try_from_value(rt, v))
            .collect::<Result<Vec<T>, String>>()?;
        Ok(Self { values })
    }

    pub fn with_arity_and_rest<const N: usize>(mut self) -> Result<([T; N], Rest<T>), String>
    where
        T: std::fmt::Debug,
    {
        assert_at_least_arity(N, &self.values)?;
        let rest = self.values.split_off(N);
        let args = self.values.try_into().unwrap();
        Ok((args, Rest { values: rest }))
    }

    pub fn with_arity<const N: usize>(self) -> Result<[T; N], String>
    where
        T: std::fmt::Debug,
    {
        assert_arity(N, &self.values)?;
        Ok(self.values.try_into().unwrap())
    }

    pub fn signature_matches(rt: &mut Runtime, values: &[Value]) -> bool {
        for value in values {
            if !T::is_matching(rt, value) {
                return false;
            }
        }
        true
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

// AllParams marker for heterogeneous argument packs
pub struct AllParams<T> {
    values: Vec<Value>,
    marker: PhantomData<T>,
}
impl<T> AllParams<T> {
    pub fn new(values: Vec<Value>) -> Self {
        Self {
            values,
            marker: PhantomData,
        }
    }
}
impl<T> From<AllParams<T>> for Vec<Value> {
    fn from(ap: AllParams<T>) -> Self {
        ap.values
    }
}
impl<T> Deref for AllParams<T> {
    type Target = Vec<Value>;
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}
impl<T> DerefMut for AllParams<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}
impl<T> IntoIterator for AllParams<T> {
    type Item = Value;
    type IntoIter = std::vec::IntoIter<Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

//------------- Utils ------------
fn assert_arity<T>(len: usize, values: &[T]) -> Result<(), String> {
    if !has_arity(len, values) {
        return Err(format!("Expected {len} arguments, got {}", values.len()));
    }
    Ok(())
}

fn has_arity<T>(len: usize, values: &[T]) -> bool {
    values.len() == len
}

fn has_at_least_arity<T>(len: usize, values: &[T]) -> bool {
    values.len() >= len
}

fn assert_at_least_arity<T>(len: usize, values: &[T]) -> Result<(), String> {
    if !has_at_least_arity(len, values) {
        return Err(format!(
            "Expected at least {len} arguments, got {}",
            values.len()
        ));
    }
    Ok(())
}

mod macros;
pub use macros::{WithRest, WithRuntime};
pub struct WithParams;

impl crate::runtime::Env {
    pub fn with_fn_mono<F, Ctx>(self, name: &'static str, f: F) -> Self
    where
        F: IntoOverloadedFunction<Ctx> + crate::api::typing::FnSignature<Ctx> + 'static,
    {
        let r#gen = crate::api::typing::TypeGen::new();
        let tb = <F as crate::api::typing::FnSignature<Ctx>>::type_of(&r#gen);
        let tb = crate::types::builder::id_fn(move |env, src| tb.build(env, src));
        #[allow(deprecated)]
        self.with_dynamic_fn(name, f).with_mono_type(name, tb)
    }

    pub fn with_fn_poly<F, Ctx>(self, name: &'static str, f: F) -> Self
    where
        F: IntoOverloadedFunction<Ctx> + crate::api::typing::FnSignature<Ctx> + 'static,
    {
        let r#gen = crate::api::typing::TypeGen::new();
        let tb = <F as crate::api::typing::FnSignature<Ctx>>::type_of(&r#gen);
        let tb = crate::types::builder::id_fn(move |env, src| tb.build(env, src));
        #[allow(deprecated)]
        self.with_dynamic_fn(name, f).with_poly_type(name, tb)
    }
}

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
