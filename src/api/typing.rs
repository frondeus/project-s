#![allow(non_snake_case, non_camel_case_types, unused_attributes)]
use std::marker::PhantomData;

use crate::{
    runtime::{Function, Runtime, Value},
    types::{
        InferedTypeId, TypeEnv,
        builder::{
            SourceBuilder, TypeBuilder, boolean, constructor_instance, function, id_fn, keyword,
            list, number, string,
        },
    },
};

use super::{EagerRec, FromValue, IntoValue, Rest};
use crate::types::InferedType;

/// A generator/context used while producing TypeBuilder graphs for a single function signature.
/// It provides stable type variables for Param<ID, _> occurrences, so multiple positions sharing
/// the same ID refer to the very same type variable.
#[derive(Default, Clone)]
pub struct TypeGen {
    // Map: stable param-id -> infered type variable id
    vars: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<usize, InferedTypeId>>>,
}

impl TypeGen {
    pub fn new() -> Self {
        Self::default()
    }

    /// Produce a TypeBuilder for a stable type variable keyed by `id`.
    /// Reuses the same InferedTypeId within this TypeGen session.
    pub fn var(&self, id: usize) -> impl TypeBuilder + 'static {
        let vars = std::rc::Rc::clone(&self.vars);
        id_fn(move |env: &mut TypeEnv, src| {
            // Render some stable textual name for trace/debug; it doesn't affect semantics.
            let letter = crate::types::variable_letters(id);
            let span = src.append(&letter);
            let mut vars = vars.borrow_mut();
            if let Some(var) = vars.get(&id) {
                *var
            } else {
                let fresh = env.fresh_var(span, 1);
                vars.insert(id, fresh);
                fresh
            }
        })
    }
}

// ---------------- Typing wrappers ----------------

/// Marker wrapper to tie multiple positions to the same type variable.
/// Any Param<ID, T> appearing in the signature is mapped to the same type variable for that ID.
/// The inner T exists only to make the wrapper kind-aware at the Rust level; it is ignored by TypeOf.
pub struct Param<const ID: usize, T = Value>(pub T);

impl<T, const ID: usize> Clone for Param<ID, T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Generalized function-typed marker that accepts a tuple of arguments:
/// Fun<Args, Ret> maps to function(Args, Ret). This allows multi-arg functions via tuple Args.
pub struct Fun<Args, Ret>(pub Function, pub PhantomData<(Args, Ret)>);

impl<Args, Ret> Clone for Fun<Args, Ret> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<Args, Ret> Fun<Args, Ret>
where
    Args: IntoValue,
    Ret: FromValue,
{
    pub fn call(self, rt: &mut Runtime, args: Args) -> Result<Ret, String> {
        let arg = Args::try_into_value(args, rt)?;
        let args_vec = match arg {
            Value::List(list) => list,
            other => {
                return Err(format!(
                    "Expected tuple of arguments encoded as list, got {other:?}"
                ));
            }
        };
        let res = rt.closure_call_inner(self.0, args_vec);
        <Ret as FromValue>::try_from_value(rt, res)
    }
}

// ---------------- TypeOf ----------------

/// Maps a Rust type to a TypeBuilder.
pub trait TypeOf {
    fn ty(g: &TypeGen) -> Box<dyn TypeBuilder>;
}

// Ground types
impl TypeOf for f64 {
    fn ty(_gen: &TypeGen) -> Box<dyn TypeBuilder> {
        Box::new(number())
    }
}
impl TypeOf for i32 {
    fn ty(_gen: &TypeGen) -> Box<dyn TypeBuilder> {
        Box::new(number())
    }
}
impl TypeOf for bool {
    fn ty(_gen: &TypeGen) -> Box<dyn TypeBuilder> {
        Box::new(boolean())
    }
}
impl TypeOf for String {
    fn ty(_gen: &TypeGen) -> Box<dyn TypeBuilder> {
        Box::new(string())
    }
}
impl TypeOf for super::Keyword {
    fn ty(_gen: &TypeGen) -> Box<dyn TypeBuilder> {
        Box::new(keyword())
    }
}

// Wrappers erase to inner for typing purposes
impl<T, M> TypeOf for EagerRec<T, M>
where
    T: TypeOf,
{
    fn ty(g: &TypeGen) -> Box<dyn TypeBuilder> {
        T::ty(g)
    }
}

// Rest<T> maps to [T] (list T) at the type level
impl<T> TypeOf for Rest<T>
where
    T: TypeOf,
{
    fn ty(g: &TypeGen) -> Box<dyn TypeBuilder> {
        let inner = T::ty(g);
        // Adapter: Box<dyn TypeBuilder> -> impl TypeBuilder
        let adapted = id_fn(move |env: &mut TypeEnv, src| inner.build(env, src));
        Box::new(list(adapted))
    }
}

// Param<ID, T> maps to a stable type variable for ID
impl<T> TypeOf for Vec<T>
where
    T: TypeOf + 'static,
{
    fn ty(g: &TypeGen) -> Box<dyn TypeBuilder> {
        let inner = T::ty(g);
        let adapted = id_fn(move |env: &mut TypeEnv, src| inner.build(env, src));
        Box::new(list(adapted))
    }
}

impl<T> TypeOf for crate::api::AllParams<T>
where
    T: TypeOf,
{
    fn ty(g: &TypeGen) -> Box<dyn TypeBuilder> {
        T::ty(g)
    }
}

impl<const ID: usize, T> TypeOf for Param<ID, T> {
    fn ty(g: &TypeGen) -> Box<dyn TypeBuilder> {
        Box::new(g.var(ID))
    }
}

impl<T> TypeOf for Option<T>
where
    T: TypeOf,
{
    fn ty(g: &TypeGen) -> Box<dyn TypeBuilder> {
        let a = T::ty(g);
        let a = id_fn(move |env: &mut TypeEnv, src| a.build(env, src));
        Box::new(constructor_instance("Option", (a,)))
    }
}

impl<T> TypeOf for Result<T, String>
where
    T: TypeOf,
{
    fn ty(g: &TypeGen) -> Box<dyn TypeBuilder> {
        // Result<T, String> is invisible at the type level; it erases to T.
        T::ty(g)
    }
}

// Generalized function-typed parameters: Fun<Args, Ret> -> function(Args, Ret)
impl<Args, Ret> TypeOf for Fun<Args, Ret>
where
    Args: TypeOf,
    Ret: TypeOf,
{
    fn ty(g: &TypeGen) -> Box<dyn TypeBuilder> {
        let args = Args::ty(g);
        let ret = Ret::ty(g);

        let args = id_fn(move |env: &mut TypeEnv, src| args.build(env, src));
        let ret = id_fn(move |env: &mut TypeEnv, src| ret.build(env, src));

        Box::new(function(args, ret))
    }
}

// TypeOf for tuple (A, B) -> (TypeOf<A>, TypeOf<B>)
impl<A, B> TypeOf for (A, B)
where
    A: TypeOf,
    B: TypeOf,
{
    fn ty(g: &TypeGen) -> Box<dyn TypeBuilder> {
        let a = A::ty(g);
        let b = B::ty(g);
        Box::new(move |env: &mut TypeEnv, src: &mut SourceBuilder| {
            let from = src.point();
            src.append("(");
            let a_id = a.build(env, src);
            src.append(", ");
            let b_id = b.build(env, src);
            src.append(")");
            let to = src.point();
            let span = src.span(from, to);
            InferedType::Tuple {
                items: vec![a_id, b_id],
                rest: None,
                span,
            }
        })
    }
}

// TypeOf for single-element tuple (A,) -> (TypeOf<A>)
impl<A> TypeOf for (A,)
where
    A: TypeOf,
{
    fn ty(g: &TypeGen) -> Box<dyn TypeBuilder> {
        let a = A::ty(g);
        Box::new(move |env: &mut TypeEnv, src: &mut SourceBuilder| {
            let from = src.point();
            src.append("(");
            let a_id = a.build(env, src);
            src.append(")");
            let to = src.point();
            let span = src.span(from, to);
            InferedType::Tuple {
                items: vec![a_id],
                rest: None,
                span,
            }
        })
    }
}

// ---------------- FnSignature ----------------

/// Builds a TypeBuilder for a Rust function by mapping each parameter/return via TypeOf.
/// Parameterized by the same Ctx tuple used by IntoNativeFunction (e.g., (O,), (O, A), (O, WithRuntime), ...).
/// Variadic Rest<...> is intentionally not covered here; keep explicit overrides for varargs.
pub trait FnSignature<Ctx> {
    fn type_of(g: &TypeGen) -> Box<dyn TypeBuilder>;
}

// Helpers to adapt Box<dyn TypeBuilder> to impl TypeBuilder
fn adapt(tb: Box<dyn TypeBuilder>) -> impl TypeBuilder {
    id_fn(move |env: &mut TypeEnv, src| tb.build(env, src))
}

// ---- NO: without runtime or rest ----
// Implement for any F: Fn(...) with Ctx = (O, [Args...])

macro_rules! fnsig_no {
    () => {
        impl<F, O> FnSignature<(O,)> for F
        where
            F: 'static + Fn() -> O,
            O: TypeOf,
        {
            fn type_of(g: &TypeGen) -> Box<dyn TypeBuilder> {
                let o = adapt(O::ty(g));
                Box::new(function((), o))
            }
        }
    };
    ($first:ident $(, $arg:ident)*) => {
        fnsig_no!($($arg),*);
        impl<F, O, $first, $($arg,)*> FnSignature<(O, $first, $($arg,)*)> for F
        where
            F: 'static + Fn($first, $($arg),*) -> O,
            O: TypeOf,
            $first: TypeOf,
            $($arg: TypeOf),*
        {
            fn type_of(g: &TypeGen) -> Box<dyn TypeBuilder> {
                let o = adapt(O::ty(g));
                let $first = adapt(<$first as TypeOf>::ty(g));
                $(
                    let $arg = adapt(<$arg as TypeOf>::ty(g));
                )*
                Box::new(function(($first, $($arg,)*), o))
            }
        }
    };
}
#[allow(non_snake_case)]
fnsig_no!(t1, t2, t3);

// ---- RT: with &mut Runtime, no rest ----
// Implement for any F: Fn(&mut Runtime, ...) with Ctx = (O, [Args...], WithRuntime)

macro_rules! fnsig_rt {
    () => {
        impl<F, O> FnSignature<(O, crate::api::WithRuntime)> for F
        where
            F: 'static + Fn(&mut Runtime) -> O,
            O: TypeOf,
        {
            fn type_of(g: &TypeGen) -> Box<dyn TypeBuilder> {
                let o = adapt(O::ty(g));
                Box::new(function((), o))
            }
        }
    };
    ($first:ident $(, $arg:ident)*) => {
        fnsig_rt!($($arg),*);
        impl<F, O, $first, $($arg,)*> FnSignature<(O, $first, $($arg,)* crate::api::WithRuntime)> for F
        where
            F: 'static + Fn(&mut Runtime, $first, $($arg),*) -> O,
            O: TypeOf,
            $first: TypeOf,
            $($arg: TypeOf),*
        {
            fn type_of(g: &TypeGen) -> Box<dyn TypeBuilder> {
                let o = adapt(O::ty(g));
                let $first = adapt(<$first as TypeOf>::ty(g));
                $(
                    let $arg = adapt(<$arg as TypeOf>::ty(g));
                )*
                Box::new(function(($first, $($arg,)*), o))
            }
        }
    };
}
#[allow(non_snake_case)]
fnsig_rt!(t1, t2, t3);

// ---- RE: pure Rest<T>, no runtime ----
// Implement for any F: Fn(Rest<R>) -> O with Ctx = (O, R, WithRest)
impl<F, O, R> FnSignature<(O, R, crate::api::WithRest)> for F
where
    F: 'static + Fn(Rest<R>) -> O,
    O: TypeOf,
    R: TypeOf,
{
    fn type_of(g: &TypeGen) -> Box<dyn TypeBuilder> {
        let o = adapt(O::ty(g));
        let r = adapt(<Rest<R> as TypeOf>::ty(g));
        Box::new(function(r, o))
    }
}

// ---- PA: AllParams<R>, no runtime ----
impl<F, O, R> FnSignature<(O, R, crate::api::WithParams)> for F
where
    F: 'static + Fn(crate::api::AllParams<R>) -> O,
    O: TypeOf,
    R: TypeOf,
{
    fn type_of(g: &TypeGen) -> Box<dyn TypeBuilder> {
        let o = adapt(O::ty(g));
        let a = adapt(<crate::api::AllParams<R> as TypeOf>::ty(g));
        Box::new(function(a, o))
    }
}

// ---- RTPA: AllParams<R> with &mut Runtime ----
impl<F, O, R> FnSignature<(O, R, crate::api::WithRuntime, crate::api::WithParams)> for F
where
    F: 'static + Fn(&mut Runtime, crate::api::AllParams<R>) -> O,
    O: TypeOf,
    R: TypeOf,
{
    fn type_of(g: &TypeGen) -> Box<dyn TypeBuilder> {
        let o = adapt(O::ty(g));
        let a = adapt(<crate::api::AllParams<R> as TypeOf>::ty(g));
        Box::new(function(a, o))
    }
}

// ---- RTRE: pure Rest<T> with &mut Runtime ----
// Implement for any F: Fn(&mut Runtime, Rest<R>) -> O with Ctx = (O, R, WithRuntime, WithRest)
impl<F, O, R> FnSignature<(O, R, crate::api::WithRuntime, crate::api::WithRest)> for F
where
    F: 'static + Fn(&mut Runtime, Rest<R>) -> O,
    O: TypeOf,
    R: TypeOf,
{
    fn type_of(g: &TypeGen) -> Box<dyn TypeBuilder> {
        let o = adapt(O::ty(g));
        let r = adapt(<Rest<R> as TypeOf>::ty(g));
        Box::new(function(r, o))
    }
}
