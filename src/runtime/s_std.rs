use crate::{
    ast::SExpId,
    builder::{ASTBuilder, error, quote},
};

use super::{Runtime, Value};

fn sub(_rt: &mut Runtime, args: Vec<Value>) -> Result<Value, String> {
    let mut args = args.into_iter();
    let Some(mut a) = args.next() else {
        return Err("Expected at least one argument".into());
    };

    a = a.ok()?;

    match &mut a {
        Value::Number(a) => {
            for arg in args {
                let arg = arg.ok()?;
                let Some(b) = arg.as_number() else {
                    return Err("Expected number".into());
                };
                *a -= b;
            }
        }
        _ => return Err("Expected number".into()),
    }

    Ok(a)
}

fn add(rt: &mut Runtime, args: Vec<Value>) -> Result<Value, String> {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Err("Expected at least one argument".into());
    };

    let first = first.eager_rec(rt).ok()?;

    match first {
        Value::Number(mut first) => {
            for arg in args {
                let Some(b) = arg.eager(rt).ok()?.as_number() else {
                    return Err("Expected number".into());
                };
                first += b;
            }
            Ok(Value::Number(first))
        }
        Value::Object(mut left) => {
            for right in args {
                rt.envs.push();
                let _super = rt.new_ref_obj(left.clone());
                rt.envs.set("super", _super.clone());

                let Some(right) = right.eager_rec(rt).ok()?.into_object() else {
                    return Err("+: Expected object ".into());
                };

                for (key, value) in right {
                    left.insert(key, value);
                }
                rt.envs.pop();
            }
            Ok(Value::Object(left))
        }
        _ => Err("Expected number or object".into()),
    }
}

fn add_obj(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    match &args[..] {
        [key, value] => {
            let result = (
                "if",
                ("has?", "super", key),
                // quote((key, ("thunk", (), ("+", ("super", key), value)))),
                quote((key, ("+", ("super", key), value))),
                quote((key, value)),
            );
            Ok(result.build(&mut rt.asts))
            // let result = ("thunk", (), result);

            // let result = result.build(&mut rt.asts);
            // Ok(result)
        }
        _ => Err("Expected two arguments".into()),
    }
}

impl Runtime {
    pub fn with_try_fn(
        &mut self,
        name: &str,
        body: impl Fn(&mut Runtime, Vec<Value>) -> Result<Value, String> + 'static,
    ) {
        self.with_fn(name, move |rt, args| {
            let result = body(rt, args);
            result.unwrap_or_else(Value::Error)
        });
    }

    pub fn with_try_macro(
        &mut self,
        name: &str,
        body: impl Fn(&mut Runtime, Vec<SExpId>) -> Result<SExpId, String> + 'static,
    ) {
        self.with_macro(name, move |rt, args| {
            let result = body(rt, args);
            match result {
                Ok(id) => id,
                Err(err) => {
                    eprintln!("Error: {}", err);
                    error().build(&mut rt.asts)
                }
            }
        });
    }

    pub fn with_prelude(&mut self) {
        self.with_try_fn("-", sub);
        self.with_try_fn("+", add);
        self.with_try_macro("+obj", add_obj);
        self.with_fn("print", |_rt, args| {
            for arg in args.into_iter() {
                eprintln!("{:?}", arg);
            }

            Value::Number(1.0)
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::ast::ASTS;

    use super::*;

    #[test]
    fn integration() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "log", |input, _deps| {
            // eprintln!("---");
            let mut asts = ASTS::new();
            let ast = asts.parse(input).unwrap();
            let root_id = ast.root_id().unwrap();
            let root_id = crate::process_ast(&mut asts, root_id);

            let mut runtime = Runtime::new(asts);
            runtime.with_prelude();
            let log = Arc::new(Mutex::new(String::new()));
            let log_clone = log.clone();
            runtime.with_fn("print", move |_rt, args| {
                for arg in args.into_iter() {
                    log_clone.lock().unwrap().push_str(&format!("{:?}\n", arg));
                }

                Value::Number(1.0)
            });

            _ = runtime.eval(root_id);
            let log = log.lock().unwrap().clone();
            log
        })
    }
}
