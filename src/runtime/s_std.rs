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
fn add(rt: &mut Runtime, args: Vec<Value>) -> Result<Value, String> {
    let mut args = args.into_iter();
    let Some(mut first) = args.next() else {
        return Err("Expected at least one argument".into());
    };

    first = first.ok()?;

    match &mut first {
        Value::Number(first) => {
            for arg in args {
                let arg = arg.ok()?;
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
                let right = right.ok()?;
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
                // self.supers.pop();
            }
        }
        _ => return Err("Expected number or object".into()),
    }

    Ok(first)
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

    pub fn with_prelude(&mut self) {
        self.with_try_fn("-", sub);
        self.with_try_fn("+", add);
    }
}
