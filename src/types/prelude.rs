use super::{
    builder::{SourceBuilder, TypeBuilder, *},
    *,
};

fn bin_op(lhs: impl TypeBuilder, rhs: impl TypeBuilder, ret: impl TypeBuilder) -> impl TypeBuilder {
    function((lhs, rhs), ret)
}

impl TypeEnv {
    fn with_mono(&mut self, source: &mut SourceBuilder, name: &str, ty: impl TypeBuilder) {
        source.append(&format!("\"{name}\": "));
        let ty = ty.build(self, source);
        source.new_line();
        self.envs.set(name, InferedTypeScheme::Monomorphic(ty));
    }

    fn with_poly(&mut self, source: &mut SourceBuilder, name: &str, ty: impl TypeBuilder) {
        source.append(&format!("\"{name}\": forall "));
        let ty = ty.build(self, source);
        source.new_line();
        self.envs.set(
            name,
            InferedTypeScheme::Polymorphic(InferedPolymorphicType { level: 0, body: ty }),
        );
    }

    pub fn with_prelude(mut self, sources: &mut Sources) -> Self {
        let builtin = sources.add("<builtin>", "");
        let mut source = SourceBuilder::new(builtin);

        self.with_mono(&mut source, "+", function(list(number()), number()));
        // self.with_mono(&mut source, "+", bin_op(number(), number(), number()));
        self.with_mono(&mut source, "-", bin_op(number(), number(), number()));
        self.with_mono(&mut source, "*", function(list(number()), number()));
        self.with_mono(&mut source, ">", bin_op(number(), number(), boolean()));
        self.with_mono(&mut source, "<=", bin_op(number(), number(), boolean()));

        self.with_poly(
            &mut source,
            "=",
            bin_op(var("'a", 1), var("'b", 1), boolean()),
        );
        self.with_poly(&mut source, "print", function((var("'a", 1),), number()));
        self.with_poly(&mut source, "debug", {
            let vars = Vars::default();
            let lhs = vars.var("'a", 1);
            let rhs = vars.var("'a", 1);
            function(lhs, rhs)
        });
        self.with_poly(&mut source, "tuple", {
            let vars = Vars::default();
            let lhs = vars.var("'a", 1);
            let rhs = vars.var("'a", 1);
            function(lhs, rhs)
        });
        self.with_poly(&mut source, "list", {
            let vars = Vars::default();
            function(list(vars.var("'a", 1)), list(vars.var("'a", 1)))
        });
        self.with_poly(&mut source, "list/enumerate", {
            let vars = Vars::default();

            function(
                (list(vars.var("'a", 1)),),
                list((number(), vars.var("'a", 1))),
            )
        });
        self.with_poly(&mut source, "list/map", {
            let vars = Vars::default();

            function(
                (
                    list(vars.var("'a", 1)),
                    function((vars.var("'a", 1),), vars.var("'b", 1)),
                ),
                list(vars.var("'b", 1)),
            )
        });
        self.with_poly(&mut source, "get", {
            let vars = Vars::default();

            function((reference(vars.var("'a", 1)),), vars.var("'a", 1))
        });

        sources.get_mut(builtin).set(&source.finalize());
        self
    }
}
