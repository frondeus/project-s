use super::{Env, Runtime};

mod functions;
mod macros;

pub fn prelude() -> Env {
    use functions::*;
    // use macros::*;

    Env::default()
        // Builtin constructs
        .with_fn_poly("import", import)
        // Type constructors
        .with_fn_poly("list", make_list)
        .with_fn_poly("tuple", tuple)
        .with_fn_poly("enum", construct_enum)
        .with_fn_poly("obj/plain", obj_plain)
        .with_fn_poly("ref", new_ref)
        // - Option
        .with_fn_poly("Some", some)
        .with_fn_poly("None", none)
        // Basic operators
        .with_fn_mono("+", add)
        .with_fn_mono("-", sub_numbers)
        .with_fn_mono("*", mul)
        .with_fn_mono(">", gt_numbers)
        .with_fn_mono("<=", lte_numbers)
        .with_fn_poly("=", eq_any)
        // Builtin record operations
        .with_fn_poly("obj/extend", obj_extend)
        // Reference operations
        .with_fn_poly("set", set)
        .with_fn_poly("get", get)
        // List functions
        .with_fn_poly("list/enumerate", list_enumerate)
        .with_fn_poly("list/map", list_map)
        .with_fn_poly("list/find", list_find)
        // Misc
        .with_fn_poly("error", error)
        .with_fn_poly("debug", debug)
        .with_fn_poly("print", print)
        .with_fn_mono("roll", roll)
        .with_try_macro("extend!", macros::extend_macro)
}

impl Runtime {
    pub fn with_env(&mut self, env: Env) {
        self.envs.with_env(env);
    }

    pub fn with_prelude(&mut self) {
        self.with_env(prelude());
    }
}
