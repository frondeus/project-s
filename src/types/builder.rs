use std::collections::HashMap;

use crate::{diagnostics::Diagnostics, source::Span};

use super::{
    TypeEnv,
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
        let mut builder = crate::types::canonical::CanonicalBuilder::default();
        let canon_root = canon.build(&mut builder);
        let canon = builder.finish();
        let mut vars = HashMap::new();
        canonical_value(env, &canon, &mut vars, canon_root, span)
    }
}

pub fn u_canonical(canon: impl canon::CanonBuilder, span: Span) -> impl TypeBuilder<core::Use> {
    move |env: &mut TypeEnv, _diag: &mut Diagnostics| {
        let mut builder = crate::types::canonical::CanonicalBuilder::default();
        let canon_root = canon.build(&mut builder);
        let canon = builder.finish();
        let mut vars = HashMap::new();
        canonical_use(env, &canon, &mut vars, canon_root, span)
    }
}

fn canonical_value(
    env: &mut TypeEnv,
    // diagnostics: &mut Diagnostics,
    canon: &crate::types::canonical::Canonicalized,
    vars: &mut HashMap<usize, (core::Value, core::Use)>,
    id: crate::types::canonical::CanonId,
    span: Span,
) -> core::Value {
    match canon.get(id) {
        super::canonical::Canonical::Any(i) => {
            if let Some(i) = *i {
                return vars
                    .entry(i)
                    .or_insert_with(|| {
                        let (any_var, _any_bound) = env.engine.var();
                        (any_var, _any_bound)
                    })
                    .0;
            }
            let (any_var, _any_bound) = env.engine.var();
            any_var
        }
        super::canonical::Canonical::Recursive(_) => todo!(),
        super::canonical::Canonical::Or(_) => todo!(),
        super::canonical::Canonical::Bool => env.engine.bool(span),
        super::canonical::Canonical::Number => env.engine.number(span),
        super::canonical::Canonical::String => env.engine.string(span),
        super::canonical::Canonical::Error => env.engine.error(span),
        super::canonical::Canonical::Keyword => env.engine.keyword(span),
        super::canonical::Canonical::Tuple { items } => {
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                values.push(canonical_value(env, canon, vars, *item, span.clone()));
            }
            env.engine.tuple(values, span)
        }
        super::canonical::Canonical::List { item } => {
            let item = canonical_value(env, canon, vars, *item, span.clone());
            env.engine.list(item, span)
        }
        super::canonical::Canonical::Func { pattern, ret } => {
            let pattern_use = canonical_use(env, canon, vars, *pattern, span.clone());
            let ret_value = canonical_value(env, canon, vars, *ret, span.clone());
            env.engine.func(pattern_use, ret_value, span)
        }
        super::canonical::Canonical::Struct { fields } => {
            let mut f = Vec::with_capacity(fields.len());
            for (name, id) in fields {
                let value = canonical_value(env, canon, vars, *id, span.clone());
                f.push((name.clone(), value));
            }
            env.engine.obj(f, span)
        }
        super::canonical::Canonical::Reference { read, write } => {
            let write = write.map(|write| canonical_use(env, canon, vars, write, span.clone()));
            let read = read.map(|read| canonical_value(env, canon, vars, read, span.clone()));
            env.engine.reference(write, read, span)
        }
    }
}

fn canonical_use(
    env: &mut TypeEnv,
    // diagnostics: &mut Diagnostics,
    canon: &crate::types::canonical::Canonicalized,
    vars: &mut HashMap<usize, (core::Value, core::Use)>,
    id: crate::types::canonical::CanonId,
    span: Span,
) -> core::Use {
    match canon.get(id) {
        super::canonical::Canonical::Any(i) => {
            if let Some(i) = *i {
                return vars
                    .entry(i)
                    .or_insert_with(|| {
                        let (any_var, _any_bound) = env.engine.var();
                        (any_var, _any_bound)
                    })
                    .1;
            }
            let (_any_var, any_bound) = env.engine.var();
            any_bound
        }
        super::canonical::Canonical::Recursive(_) => todo!(),
        super::canonical::Canonical::Or(_) => todo!(),
        super::canonical::Canonical::Bool => env.engine.bool_use(span),
        super::canonical::Canonical::Number => env.engine.number_use(span),
        super::canonical::Canonical::String => env.engine.string_use(span),
        super::canonical::Canonical::Error => todo!(),
        super::canonical::Canonical::Keyword => env.engine.keyword_use(span),
        super::canonical::Canonical::Tuple { items } => {
            let mut uses = Vec::with_capacity(items.len());
            for item in items {
                uses.push(canonical_use(env, canon, vars, *item, span.clone()));
            }
            env.engine.tuple_use(uses, span)
        }
        super::canonical::Canonical::List { item } => {
            let item_use = canonical_use(env, canon, vars, *item, span.clone());
            env.engine.list_use(item_use, 0, None, span)
        }
        super::canonical::Canonical::Func { .. } => todo!(),
        super::canonical::Canonical::Struct { fields } => {
            let mut uses = Vec::with_capacity(fields.len());
            for (name, id) in fields {
                let use_ = canonical_use(env, canon, vars, *id, span.clone());
                uses.push((name.clone(), use_));
            }
            env.engine.obj_use(uses, span)
        }
        &super::canonical::Canonical::Reference { read, write } => {
            let read = read.map(|read| canonical_use(env, canon, vars, read, span.clone()));
            let write = write.map(|write| canonical_value(env, canon, vars, write, span.clone()));
            env.engine.reference_use(write, read, span)
        }
    }
}

// -----------------

pub mod canon {
    use crate::types::canonical::{CanonId, Canonical, CanonicalBuilder};

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

    pub fn any(i: impl Into<Option<usize>>) -> impl CanonBuilder {
        Canonical::Any(i.into())
    }

    pub fn recursive(inner: impl CanonBuilder) -> impl CanonBuilder {
        move |canon: &mut CanonicalBuilder| Canonical::Recursive(inner.build(canon))
    }

    pub fn bool() -> impl CanonBuilder {
        Canonical::Bool
    }

    pub fn number() -> impl CanonBuilder {
        Canonical::Number
    }

    pub fn string() -> impl CanonBuilder {
        Canonical::String
    }

    pub fn error() -> impl CanonBuilder {
        Canonical::Error
    }

    pub fn keyword() -> impl CanonBuilder {
        Canonical::Keyword
    }

    pub fn obj() -> impl CanonBuilder {}

    pub fn list(item: impl CanonBuilder) -> impl CanonBuilder {
        move |canon: &mut CanonicalBuilder| Canonical::List {
            item: item.build(canon),
        }
    }
    pub fn reference(
        read: Option<impl CanonBuilder>,
        write: Option<impl CanonBuilder>,
    ) -> impl CanonBuilder {
        move |canon: &mut CanonicalBuilder| Canonical::Reference {
            read: read.map(|read| read.build(canon)),
            write: write.map(|write| write.build(canon)),
        }
    }

    pub fn func(pattern: impl CanonBuilder, ret: impl CanonBuilder) -> impl CanonBuilder {
        move |canon: &mut CanonicalBuilder| {
            let pattern = pattern.build(canon);
            let ret = ret.build(canon);
            Canonical::Func { pattern, ret }
        }
    }

    impl CanonBuilder for () {
        fn build(self, canon: &mut CanonicalBuilder) -> CanonId {
            canon.add(Canonical::Tuple { items: vec![] })
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
                    canon.add(Canonical::Tuple { items: vec![$($item),*] })
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
