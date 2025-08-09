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

    fn with_type(&mut self, source: &mut SourceBuilder, name: &str, ty: impl TypeBuilder) {
        source.append(&format!("type \"{name}\" = "));
        let ty = ty.build(self, source);
        source.new_line();
        self.envs.set_type(name, TypeValue::Type(ty));
    }

    fn with_type_constructor(
        &mut self,
        source: &mut SourceBuilder,
        name: &str,
        args: Vec<Box<dyn TypeBuilder>>,
        ret: impl TypeBuilder,
    ) {
        let from = source.point();
        source.append(&format!("type \"{name}\" ("));
        let args = args
            .into_iter()
            .map(|arg| arg.build(self, source))
            .collect::<Vec<_>>();
        source.append(") = ");
        let ret = ret.build(self, source);
        source.new_line();
        let to = source.point();
        let span = source.span(from, to);
        self.envs.set_type(
            name,
            TypeValue::Constructor {
                args: args.clone(),
                ret,
            },
        );

        let lhs = self.tuple(args, None, span);
        let rhs = ret;
        let runtime_ty = self.function(lhs, rhs, span);

        self.envs.set(
            name,
            InferedTypeScheme::Polymorphic(InferedPolymorphicType {
                level: 0,
                body: runtime_ty,
            }),
        );
    }

    pub fn with_prelude(mut self, sources: &mut Sources) -> Self {
        let builtin = sources.add("<builtin>", "");
        let mut source = SourceBuilder::new(builtin);

        {
            let vars = Vars::default();

            self.with_type_constructor(
                &mut source,
                "Some",
                vec![Box::new(vars.var("'a", 1))],
                enum_({
                    let mut variants: IndexMap<String, Box<dyn TypeBuilder>> = IndexMap::new();

                    variants.insert("Some".to_string(), Box::new(vars.var("'a", 1)));
                    variants
                }),
            );
        }

        {
            self.with_type_constructor(
                &mut source,
                "None",
                vec![],
                enum_({
                    let mut variants: IndexMap<String, Box<dyn TypeBuilder>> = IndexMap::new();

                    variants.insert("None".to_string(), Box::new(()));
                    variants
                }),
            );
        }
        {
            let vars = Vars::default();

            self.with_type_constructor(
                &mut source,
                "Option",
                vec![Box::new(vars.var("'a", 1))],
                enum_({
                    let mut variants: IndexMap<String, Box<dyn TypeBuilder>> = IndexMap::new();

                    variants.insert("Some".to_string(), Box::new(vars.var("'a", 1)));
                    variants.insert("None".to_string(), Box::new(()));
                    variants
                }),
            );
        }

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
        self.with_poly(&mut source, "print", function(var("'a", 1), number()));
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
        self.with_poly(&mut source, "list/find", {
            let vars = Vars::default();

            function(
                (
                    list(vars.var("'a", 1)),
                    function((vars.var("'a", 1),), boolean()),
                ),
                constructor_instance("Option", (vars.var("'a", 1),)),
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
