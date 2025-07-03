use super::{Envs, builder::canon::SourceBuilder, core};
use std::rc::Rc;

use crate::{
    diagnostics::Diagnostics,
    source::SourceId,
    types::builder::{self, TypeBuilder},
};

use super::{TypeEnv, builder::canon::CanonBuilder};

impl Envs {
    fn with_poly<F, C>(&mut self, name: &str, value: F, source_id: SourceId)
    where
        F: 'static + Fn() -> C,
        C: CanonBuilder,
    {
        use builder::v_canonical;
        self.set(
            name,
            core::Scheme::Polymorphic(Rc::new(move |env, _asts, diagnostics| {
                let mut source = SourceBuilder::new(source_id);
                v_canonical(value(), &mut source).build(env, diagnostics)
            })),
        );
    }
    fn with_mono(
        &mut self,
        name: &str,
        env: &mut TypeEnv,
        value: impl CanonBuilder,
        source: &mut SourceBuilder,
    ) {
        use builder::v_canonical;
        source.append(&format!("\"{name}\": "));
        let value = v_canonical(value, source).build(env, &mut Diagnostics::default());
        source.new_line();
        self.set(name, core::Scheme::Monomorphic(value));
    }
}

impl TypeEnv {
    pub fn with_prelude(mut self) -> Self {
        use builder::canon::*;

        let builtin = self.modules.sources_mut().add("<builtin>", "");
        let mut source = SourceBuilder::new(builtin);

        let mut envs = std::mem::take(&mut self.envs);

        envs.with_poly("list", move || func(list(any(0)), list(any(0))), builtin);
        envs.with_poly("tuple", move || func(any(0), any(0)), builtin);

        envs.with_mono("+", &mut self, func(list(number()), number()), &mut source);
        envs.with_mono("*", &mut self, func(list(number()), number()), &mut source);
        envs.with_mono("-", &mut self, func(list(number()), number()), &mut source);
        envs.with_poly("=", move || func((any(0), any(1)), bool()), builtin);
        envs.with_mono(
            ">",
            &mut self,
            func((number(), number()), bool()),
            &mut source,
        );
        envs.with_mono(
            "<=",
            &mut self,
            func((number(), number()), bool()),
            &mut source,
        );
        envs.with_poly("print", move || func(list(any(None)), number()), builtin);
        envs.with_poly("debug", move || func(any(0), any(0)), builtin);

        let empty_struct_ref = reference(Some(empty_record()), Some(empty_record()));

        envs.with_mono(
            "obj/insert",
            &mut self,
            func((empty_struct_ref, keyword(), any(None)), ()),
            &mut source,
        );
        // TODO: that is not fully correct. We want to have type
        // (Con<T0>) -> T0
        //    | (T0) -> T0
        envs.with_mono(
            "obj/construct-or",
            &mut self,
            func((any(0),), any(0)),
            &mut source,
        );

        envs.with_poly(
            "list/enumerate",
            move || func((list(any(0)),), list((number(), any(0)))),
            builtin,
        );

        envs.with_poly(
            "list/map",
            move || func((list(any(0)), func((any(0),), any(1))), list(any(1))),
            builtin,
        );
        envs.with_poly(
            "list/find-or",
            move || func((list(any(0)), func((any(0),), bool()), any(0)), any(0)),
            builtin,
        );

        envs.with_mono("roll", &mut self, func((string(),), number()), &mut source);

        self.envs = envs;
        self.modules
            .sources_mut()
            .get_mut(builtin)
            .set(&source.finalize());
        self
    }
}
