use crate::ast::{AST, SExp, SExpId};

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

// For now `add` must stay a special form
// because we want to evaluate arguments AFTER some work is done.
fn add(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Err("Expected at least one argument".into());
    };

    let mut first = rt.eval(first).ok()?;

    match &mut first {
        Value::Number(first) => {
            for arg in args {
                let arg = rt.eval(arg).ok()?;
                let Some(b) = arg.as_number() else {
                    return Err("Expected number".into());
                };
                *first += b;
            }
        }
        Value::Object(left) => {
            for right in args {
                let _super = left.clone();
                rt.supers.push(_super);
                let right = rt.eval(right).ok()?;

                let right = match right {
                    Value::Object(right) => right,
                    Value::SExp(id) => {
                        let right = rt.eval(id).into_object();
                        let Some(right) = right else {
                            return Err("Expected quoted object".into());
                        };
                        right
                    }
                    _ => {
                        return Err("Expected object or quoted object".into());
                    }
                };

                for (key, value) in right {
                    left.insert(key, value);
                }
                rt.supers.pop();
            }
        }
        _ => return Err("Expected number or object".into()),
    }

    let mut ast = AST::default();
    first.to_sexp(&mut ast);
    let root = ast.root_id().unwrap();
    rt.asts.add_ast(ast);

    Ok(root)
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
                    let mut ast = AST::default();
                    ast.add_node(SExp::Error);
                    let root = ast.root_id().unwrap();
                    rt.asts.add_ast(ast);
                    root
                }
            }
        });
    }

    pub fn with_prelude(&mut self) {
        self.with_try_fn("-", sub);
        self.with_try_macro("+", add);
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
            let ast = crate::ast::AST::parse(input).unwrap();
            let root_id = ast.root_id().unwrap();

            let mut asts = ASTS::default();
            asts.add_ast(ast);

            let root_id = crate::lambda_lifting::lift_lambdas(&mut asts, root_id);

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
