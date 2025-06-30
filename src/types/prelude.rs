use super::{builder::canon::SourceBuilder, core};
use std::rc::Rc;

use crate::{
    diagnostics::Diagnostics,
    source::{SourceId, Sources},
    types::builder::{self, TypeBuilder},
};

use super::{TypeEnv, builder::canon::CanonBuilder};

impl TypeEnv {
    pub fn with_prelude(self, sources: &mut Sources) -> Self {
        let mut env = self;
        use builder::canon::*;

        let builtin = sources.add("<builtin>", "");
        let mut source = SourceBuilder::new(builtin);

        env.with_poly("list", move || func(list(any(0)), list(any(0))), builtin);
        env.with_poly("tuple", move || func(any(0), any(0)), builtin);

        env.with_mono("+", func(list(number()), number()), &mut source);
        env.with_mono("-", func(list(number()), number()), &mut source);
        env.with_mono(">", func((number(), number()), bool()), &mut source);
        env.with_poly("print", move || func(list(any(None)), number()), builtin);

        let empty_struct_ref = reference(Some(empty_record()), Some(empty_record()));

        env.with_mono(
            "obj/insert",
            func((empty_struct_ref, keyword(), any(None)), ()),
            &mut source,
        );
        // TODO: that is not fully correct. We want to have type
        // (Con<T0>) -> T0
        //    | (T0) -> T0
        env.with_mono("obj/construct-or", func((any(0),), any(0)), &mut source);

        sources.get_mut(builtin).set(&source.finalize());
        env
    }

    fn with_poly<F, C>(&mut self, name: &str, value: F, source_id: SourceId)
    where
        F: 'static + Fn() -> C,
        C: CanonBuilder,
    {
        use builder::v_canonical;
        self.envs.set(
            name,
            core::Scheme::Polymorphic(Rc::new(move |env, _asts, diagnostics| {
                let mut source = SourceBuilder::new(source_id);
                v_canonical(value(), &mut source).build(env, diagnostics)
            })),
        );
    }
    fn with_mono(&mut self, name: &str, value: impl CanonBuilder, source: &mut SourceBuilder) {
        use builder::v_canonical;
        source.append(&format!("\"{name}\": "));
        let value = v_canonical(value, source).build(self, &mut Diagnostics::default());
        source.new_line();
        self.envs.set(name, core::Scheme::Monomorphic(value));
    }
}
