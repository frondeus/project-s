use super::core;
use std::rc::Rc;

use crate::{
    diagnostics::Diagnostics,
    source::{Sources, Span},
    types::{
        builder::{self, TypeBuilder},
        canonical::Canonical,
    },
};

use super::{TypeEnv, builder::canon::CanonBuilder};

impl TypeEnv {
    pub fn with_prelude(self, sources: &mut Sources) -> Self {
        let mut env = self;
        use builder::canon::*;

        let builtin = sources.add("<builtin>", "");
        let builtin = Span::new_empty(builtin);

        env.with_poly(
            "list",
            move || {
                func(
                    list(any(0, builtin), builtin),
                    list(any(0, builtin), builtin),
                    builtin,
                )
            },
            builtin,
        );
        env.with_poly(
            "tuple",
            move || func(any(0, builtin), any(0, builtin), builtin),
            builtin,
        );

        env.with_mono(
            "+",
            func(list(number(builtin), builtin), number(builtin), builtin),
            builtin,
        );
        env.with_mono(
            "-",
            func(list(number(builtin), builtin), number(builtin), builtin),
            builtin,
        );
        env.with_mono(
            ">",
            func((number(builtin), number(builtin)), bool(builtin), builtin),
            builtin,
        );
        env.with_poly(
            "print",
            move || func(list(any(None, builtin), builtin), number(builtin), builtin),
            builtin,
        );

        let empty_struct = Canonical::Record {
            fields: vec![],
            proto: None,
            span: Some(builtin),
        };
        let empty_struct_ref = reference(Some(empty_struct.clone()), Some(empty_struct), builtin);

        env.with_mono(
            "obj/insert",
            func(
                (empty_struct_ref, keyword(builtin), any(None, builtin)),
                (),
                builtin,
            ),
            builtin,
        );
        // TODO: that is not fully correct. We want to have type
        // (Con<T0>) -> T0
        //    | (T0) -> T0
        env.with_mono(
            "obj/construct-or",
            func((any(0, builtin),), any(0, builtin), builtin),
            builtin,
        );
        env
    }

    fn with_poly<F, C>(&mut self, name: &str, value: F, span: Span)
    where
        F: 'static + Fn() -> C,
        C: CanonBuilder,
    {
        use builder::v_canonical;
        self.envs.set(
            name,
            core::Scheme::Polymorphic(Rc::new(move |env, _asts, diagnostics| {
                v_canonical(value(), span).build(env, diagnostics)
            })),
        );
    }
    fn with_mono(&mut self, name: &str, value: impl CanonBuilder, span: Span) {
        use builder::v_canonical;
        let value = v_canonical(value, span).build(self, &mut Diagnostics::default());
        self.envs.set(name, core::Scheme::Monomorphic(value));
    }
}
