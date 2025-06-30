use std::collections::HashMap;

use crate::{
    diagnostics::Diagnostics,
    source::Span,
    types::canonical::{CanonId, CanonicalBuilder},
};

use super::{
    TypeEnv,
    canonical::Canonical,
    core::{self},
};

pub trait TypeBuilder<T> {
    fn build(self, engine: &mut TypeEnv, diagnostics: &mut Diagnostics) -> T;
}

impl<F, T> TypeBuilder<T> for F
where
    F: FnOnce(&mut TypeEnv, &mut Diagnostics) -> T,
{
    fn build(self, engine: &mut TypeEnv, diagnostics: &mut Diagnostics) -> T {
        self(engine, diagnostics)
    }
}

pub fn v_canonical(canon: impl canon::CanonBuilder, span: Span) -> impl TypeBuilder<core::Value> {
    move |env: &mut TypeEnv, _diag: &mut Diagnostics| {
        let mut builder = CanonicalBuilder::default();
        let canon_root = canon.build(&mut builder);
        let canon = builder.finish();
        let mut vars = HashMap::new();
        canonical_value(env, &canon, &mut vars, canon_root, span)
    }
}

// pub fn u_canonical(canon: impl canon::CanonBuilder, span: Span) -> impl TypeBuilder<core::Use> {
//     move |env: &mut TypeEnv, _diag: &mut Diagnostics| {
//         let mut builder = CanonicalBuilder::default();
//         let canon_root = canon.build(&mut builder);
//         let canon = builder.finish();
//         let mut vars = HashMap::new();
//         canonical_use(env, &canon, &mut vars, canon_root, span)
//     }
// }

fn canonical_pair_inner(
    env: &mut TypeEnv,
    canon: &crate::types::canonical::Canonicalized,
    vars: &mut HashMap<usize, (core::Value, core::Use)>,
    id: CanonId,
    parent_span: Span,
    diagnostics: &mut Diagnostics,
) -> (core::Value, core::Use) {
    match canon.get(id) {
        Canonical::Wildcard(span) => env.engine.var(span.unwrap_or(parent_span)),
        Canonical::Todo(_, _) => todo!(),
        Canonical::Literal(lit, span) => {
            let u_lit = env
                .engine
                .literal_use(lit.clone(), span.unwrap_or(parent_span));
            let v_lit = env.engine.literal(lit.clone(), span.unwrap_or(parent_span));
            (v_lit, u_lit)
        }
        Canonical::Any(i, span) => {
            if let Some(i) = *i {
                return *vars
                    .entry(i)
                    .or_insert_with(|| env.engine.var(span.unwrap_or(parent_span)));
            }
            env.engine.var(span.unwrap_or(parent_span))
        }
        &Canonical::As(i, inner, span) => {
            let (u_type_value, u_type) = env.engine.var(span.unwrap_or(parent_span));
            let (v_type, v_type_bound) = env.engine.var(span.unwrap_or(parent_span));
            vars.insert(i, (u_type_value, v_type_bound));
            let (inner_v, inner_u) = canonical_pair_inner(
                env,
                canon,
                vars,
                inner,
                span.unwrap_or(parent_span),
                diagnostics,
            );
            env.engine.flow(inner_v, v_type_bound, diagnostics);
            env.engine.flow(u_type_value, inner_u, diagnostics);
            (v_type, u_type)
        }
        Canonical::Or(_canon_ids, _) => todo!(),
        Canonical::And(_canon_ids, _) => todo!(),
        Canonical::Primitive(name, span) => {
            let v_primitive = env
                .engine
                .primitive(name.clone(), span.unwrap_or(parent_span));
            let u_primitive = env
                .engine
                .primitive_use(name.clone(), span.unwrap_or(parent_span));
            (v_primitive, u_primitive)
        }
        Canonical::Applicable {
            args: _,
            ret: _,
            span: _,
        } => {
            todo!()
        }
        Canonical::Error(span) => {
            let v_error = env.engine.error(span.unwrap_or(parent_span));
            let u_error = env.engine.error_use(span.unwrap_or(parent_span));
            (v_error, u_error)
        }
        Canonical::Tuple { items, span } => {
            let mut values = Vec::with_capacity(items.len());
            let mut uses = Vec::with_capacity(items.len());

            for item in items {
                let (value, use_) = canonical_pair_inner(
                    env,
                    canon,
                    vars,
                    *item,
                    span.unwrap_or(parent_span),
                    diagnostics,
                );
                values.push(value);
                uses.push(use_);
            }
            (
                env.engine.tuple(values, span.unwrap_or(parent_span)),
                env.engine.tuple_use(uses, span.unwrap_or(parent_span)),
            )
        }
        Canonical::List { item, span } => {
            let (value, use_) = canonical_pair_inner(
                env,
                canon,
                vars,
                *item,
                span.unwrap_or(parent_span),
                diagnostics,
            );
            (
                env.engine.list(value, span.unwrap_or(parent_span)),
                env.engine
                    .list_use(use_, 0, None, span.unwrap_or(parent_span)),
            )
        }
        Canonical::Func { pattern, ret, span } => {
            let span = span.unwrap_or(parent_span);
            let (pattern_value, pattern_use) =
                canonical_pair_inner(env, canon, vars, *pattern, span, diagnostics);
            let (ret_value, ret_use) =
                canonical_pair_inner(env, canon, vars, *ret, span, diagnostics);
            (
                env.engine.func(pattern_use, ret_value, span),
                env.engine.func_use(pattern_value, ret_use, span),
            )
        }
        Canonical::Record {
            fields,
            proto: _,
            span,
        } => {
            let span = span.unwrap_or(parent_span);
            let mut values = Vec::with_capacity(fields.len());
            let mut uses = Vec::with_capacity(fields.len());
            for (name, id) in fields {
                let (value, use_) = canonical_pair_inner(env, canon, vars, *id, span, diagnostics);
                values.push((name.clone(), value));
                uses.push((name.clone(), use_));
            }
            (
                env.engine.obj(values, None, span),
                env.engine.obj_use(uses, span),
            )
        }
        Canonical::Reference { read, write, span } => {
            let span = span.unwrap_or(parent_span);
            let (read_value, read_use) = read
                .map(|read| canonical_pair_inner(env, canon, vars, read, span, diagnostics))
                .map(|(v, u)| (Some(v), Some(u)))
                .unwrap_or_default();
            let (write_value, write_use) = write
                .map(|write| canonical_pair_inner(env, canon, vars, write, span, diagnostics))
                .map(|(v, u)| (Some(v), Some(u)))
                .unwrap_or_default();
            (
                env.engine.reference(write_use, read_value, span),
                env.engine.reference_use(write_value, read_use, span),
            )
        }
    }
}

pub fn canonical_pair(
    env: &mut TypeEnv,
    canon: CanonicalBuilder,
    id: CanonId,
    span: Span,
    diagnostics: &mut Diagnostics,
) -> (core::Value, core::Use) {
    let canon = canon.finish();
    let mut vars = HashMap::new();
    canonical_pair_inner(env, &canon, &mut vars, id, span, diagnostics)
}

pub fn canonical_value(
    env: &mut TypeEnv,
    // diagnostics: &mut Diagnostics,
    canon: &crate::types::canonical::Canonicalized,
    vars: &mut HashMap<usize, (core::Value, core::Use)>,
    id: CanonId,
    parent_span: Span,
) -> core::Value {
    match canon.get(id) {
        Canonical::Todo(_, span) => {
            let span = span.unwrap_or(parent_span);
            tracing::error!("TODO: {:?}", canon.get(id));
            env.engine.error(span)
        }
        Canonical::Any(i, span) => {
            let span = span.unwrap_or(parent_span);
            if let Some(i) = *i {
                return vars
                    .entry(i)
                    .or_insert_with(|| {
                        let (any_var, _any_bound) = env.engine.var(span);
                        (any_var, _any_bound)
                    })
                    .0;
            }
            let (any_var, _any_bound) = env.engine.var(span);
            any_var
        }
        Canonical::Wildcard(span) => env.engine.var(span.unwrap_or(parent_span)).0,
        Canonical::As(_, _, _) => todo!(),
        Canonical::Or(_, _) => todo!(),
        Canonical::And(_, _) => todo!(),
        Canonical::Literal(lit, span) => {
            env.engine.literal(lit.clone(), span.unwrap_or(parent_span))
        }
        Canonical::Primitive(name, span) => env
            .engine
            .primitive(name.clone(), span.unwrap_or(parent_span)),
        Canonical::Error(span) => env.engine.error(span.unwrap_or(parent_span)),
        Canonical::Tuple { items, span } => {
            let span = span.unwrap_or(parent_span);
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                values.push(canonical_value(env, canon, vars, *item, span));
            }
            env.engine.tuple(values, span)
        }
        Canonical::List { item, span } => {
            let span = span.unwrap_or(parent_span);
            let item = canonical_value(env, canon, vars, *item, span);
            env.engine.list(item, span)
        }
        Canonical::Func { pattern, ret, span } => {
            let span = span.unwrap_or(parent_span);
            let pattern_use = canonical_use(env, canon, vars, *pattern, span);
            let ret_value = canonical_value(env, canon, vars, *ret, span);
            env.engine.func(pattern_use, ret_value, span)
        }
        Canonical::Record {
            fields,
            proto: _,
            span,
        } => {
            let span = span.unwrap_or(parent_span);
            let mut f = Vec::with_capacity(fields.len());
            for (name, id) in fields {
                let value = canonical_value(env, canon, vars, *id, span);
                f.push((name.clone(), value));
            }
            env.engine.obj(f, None, span)
        }
        Canonical::Reference { read, write, span } => {
            let span = span.unwrap_or(parent_span);
            let write = write.map(|write| canonical_use(env, canon, vars, write, span));
            let read = read.map(|read| canonical_value(env, canon, vars, read, span));
            env.engine.reference(write, read, span)
        }
        Canonical::Applicable {
            args: _,
            ret: _,
            span: _,
        } => {
            todo!()
        }
    }
}

pub fn canonical_use(
    env: &mut TypeEnv,
    // diagnostics: &mut Diagnostics,
    canon: &crate::types::canonical::Canonicalized,
    vars: &mut HashMap<usize, (core::Value, core::Use)>,
    id: CanonId,
    parent_span: Span,
) -> core::Use {
    match canon.get(id) {
        Canonical::Todo(_, span) => {
            let span = span.unwrap_or(parent_span);
            tracing::error!("TODO: {:?}", canon.get(id));
            env.engine.error_use(span)
        }
        Canonical::Any(i, span) => {
            let span = span.unwrap_or(parent_span);
            if let Some(i) = *i {
                return vars
                    .entry(i)
                    .or_insert_with(|| {
                        let (any_var, _any_bound) = env.engine.var(span);
                        (any_var, _any_bound)
                    })
                    .1;
            }
            let (_any_var, any_bound) = env.engine.var(span);
            any_bound
        }
        Canonical::As(_, _, _) => todo!(),
        Canonical::Or(_, _) => todo!(),
        Canonical::And(_, _) => todo!(),
        Canonical::Wildcard(span) => env.engine.var(span.unwrap_or(parent_span)).1,
        Canonical::Literal(lit, span) => env
            .engine
            .literal_use(lit.clone(), span.unwrap_or(parent_span)),
        Canonical::Primitive(name, span) => env
            .engine
            .primitive_use(name.clone(), span.unwrap_or(parent_span)),
        Canonical::Error(span) => env.engine.error_use(span.unwrap_or(parent_span)),
        Canonical::Tuple { items, span } => {
            let span = span.unwrap_or(parent_span);
            let mut uses = Vec::with_capacity(items.len());
            for item in items {
                uses.push(canonical_use(env, canon, vars, *item, span));
            }
            env.engine.tuple_use(uses, span)
        }
        Canonical::List { item, span } => {
            let span = span.unwrap_or(parent_span);
            let item_use = canonical_use(env, canon, vars, *item, span);
            env.engine.list_use(item_use, 0, None, span)
        }
        Canonical::Func { pattern, ret, span } => {
            let span = span.unwrap_or(parent_span);
            let pattern_v = canonical_value(env, canon, vars, *pattern, span);
            let ret_u = canonical_use(env, canon, vars, *ret, span);
            env.engine.func_use(pattern_v, ret_u, span)
        }
        Canonical::Record {
            fields,
            proto: _,
            span,
        } => {
            let span = span.unwrap_or(parent_span);
            let mut uses = Vec::with_capacity(fields.len());
            for (name, id) in fields {
                let use_ = canonical_use(env, canon, vars, *id, span);
                uses.push((name.clone(), use_));
            }
            env.engine.obj_use(uses, span)
        }
        &Canonical::Reference { read, write, span } => {
            let span = span.unwrap_or(parent_span);
            let read = read.map(|read| canonical_use(env, canon, vars, read, span));
            let write = write.map(|write| canonical_value(env, canon, vars, write, span));
            env.engine.reference_use(write, read, span)
        }
        Canonical::Applicable { args, ret, span } => {
            let span = span.unwrap_or(parent_span);
            let args = args
                .iter()
                .map(|arg| canonical_value(env, canon, vars, *arg, span))
                .collect();
            // let args = canonical_value(env, canon, vars, *args, span);
            let ret = canonical_use(env, canon, vars, *ret, span);
            env.engine.application_use(args, ret, span, span)
        }
    }
}

// -----------------

pub mod canon {
    use crate::{
        source::Span,
        types::canonical::{CanonId, Canonical, CanonicalBuilder},
    };

    pub trait CanonBuilder {
        fn build(self, canon: &mut CanonicalBuilder) -> CanonId;
    }
    impl<F> CanonBuilder for F
    where
        F: FnOnce(&mut CanonicalBuilder) -> Canonical,
    {
        fn build(self, canon: &mut CanonicalBuilder) -> CanonId {
            let res = self(canon);
            canon.add(res)
        }
    }
    impl CanonBuilder for Canonical {
        fn build(self, canon: &mut CanonicalBuilder) -> CanonId {
            canon.add(self)
        }
    }

    impl CanonBuilder for CanonId {
        fn build(self, _canon: &mut CanonicalBuilder) -> CanonId {
            self
        }
    }

    pub fn any(i: impl Into<Option<usize>>, span: impl Into<Option<Span>>) -> impl CanonBuilder {
        let span = span.into();
        Canonical::Any(i.into(), span)
    }

    // pub fn recursive(inner: impl CanonBuilder) -> impl CanonBuilder {
    //     move |canon: &mut CanonicalBuilder| Canonical::Recursive(inner.build(canon))
    // }

    pub fn primitive(name: impl ToString, span: impl Into<Option<Span>>) -> impl CanonBuilder {
        let span = span.into();
        Canonical::Primitive(name.to_string(), span)
    }

    pub fn bool(span: impl Into<Option<Span>>) -> impl CanonBuilder {
        let span = span.into();
        primitive("bool", span)
    }

    pub fn number(span: impl Into<Option<Span>>) -> impl CanonBuilder {
        let span = span.into();
        primitive("number", span)
    }

    // pub fn string() -> impl CanonBuilder {
    //     Canonical::String
    // }

    // pub fn error() -> impl CanonBuilder {
    //     Canonical::Error
    // }

    pub fn keyword(span: impl Into<Option<Span>>) -> impl CanonBuilder {
        let span = span.into();
        primitive("keyword", span)
    }

    // pub fn obj() -> impl CanonBuilder {}

    pub fn list(item: impl CanonBuilder, span: impl Into<Option<Span>>) -> impl CanonBuilder {
        let span = span.into();
        move |canon: &mut CanonicalBuilder| Canonical::List {
            item: item.build(canon),
            span,
        }
    }
    pub fn reference(
        read: Option<impl CanonBuilder>,
        write: Option<impl CanonBuilder>,
        span: impl Into<Option<Span>>,
    ) -> impl CanonBuilder {
        let span = span.into();
        move |canon: &mut CanonicalBuilder| Canonical::Reference {
            read: read.map(|read| read.build(canon)),
            write: write.map(|write| write.build(canon)),
            span,
        }
    }

    pub fn func(
        pattern: impl CanonBuilder,
        ret: impl CanonBuilder,
        span: impl Into<Option<Span>>,
    ) -> impl CanonBuilder {
        let span = span.into();
        move |canon: &mut CanonicalBuilder| {
            let pattern = pattern.build(canon);
            let ret = ret.build(canon);
            Canonical::Func { pattern, ret, span }
        }
    }

    impl CanonBuilder for () {
        fn build(self, canon: &mut CanonicalBuilder) -> CanonId {
            canon.add(Canonical::Tuple {
                items: vec![],
                span: None,
            })
        }
    }

    macro_rules! canon_tuple {
        ($($item:tt),*) => {
            impl<$($item: CanonBuilder),*> CanonBuilder for ($($item,)*) {
                #[allow(non_snake_case)]
                fn build(self, canon: &mut CanonicalBuilder) -> CanonId {
                    let ($($item,)*) = self;
                    $(
                        let $item = $item.build(canon);
                    )*
                    canon.add(Canonical::Tuple { items: vec![$($item),*], span: None })
                }
            }
        }
    }

    canon_tuple!(T1);
    canon_tuple!(T1, T2);
    canon_tuple!(T1, T2, T3);
    canon_tuple!(T1, T2, T3, T4);
    canon_tuple!(T1, T2, T3, T4, T5);
    canon_tuple!(T1, T2, T3, T4, T5, T6);
    canon_tuple!(T1, T2, T3, T4, T5, T6, T7);
    canon_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
    canon_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
    canon_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
}
