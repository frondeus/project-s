use std::rc::Rc;

pub use env::Env;
use env::Envs;
// use structs::Structs;
use value::{Function, Macro, Value};

use crate::{
    ast::{ASTS, SExp, SExpId},
    types::{Type, TypeEnv},
};

mod env;
mod functions;
mod lists;
mod quotes;
pub mod s_std;
mod structs;
mod thunks;
mod value;

#[macro_export]
macro_rules! try_err {
    ($val: expr) => {
        if let $crate::runtime::value::Value::Error(e) = $val {
            return $crate::runtime::value::Value::Error(e);
        };
    };
}

impl Runtime {
    fn is_type(&self, items: &[SExpId]) -> Value {
        let Some(sexp) = items.first() else {
            return Value::Error("Expected SExpression".to_string());
        };
        let mut env = TypeEnv::default();
        let infered = env.infer(self.asts.get_ast(*sexp), *sexp);
        let result = env.get(infered);

        let Some(ty_id) = items.get(1) else {
            return Value::Error("Expected type".to_string());
        };
        let ty = self.asts.get_ast(*sexp).get(*ty_id);
        let Some(ty) = ty.as_symbol() else {
            return Value::Error(format!(
                "Expected symbol. Found: {:?}",
                self.asts.fmt(*ty_id)
            ));
        };
        let ty = match ty {
            "Number" => Type::Number,
            "String" => Type::String,
            "Bool" => Type::Bool,
            "Symbol" => Type::Symbol,
            "Error" => Type::Error,
            ty => return Value::Error(format!("Unknown type: {}", ty)),
        };
        Value::Bool(*result == ty)
    }

    fn has_obj(&mut self, items: &[SExpId]) -> Value {
        match items {
            [obj, key] => {
                let obj = self.eval_eager_rec(*obj, true);
                try_err!(obj);
                let Some(obj) = obj.as_object() else {
                    return Value::Error(format!("has_obj: Expected object, found {:?}", obj));
                };
                let key = self.eval(*key);
                try_err!(key);
                let Some(key) = self.as_symbol_or_keyword(&key) else {
                    return Value::Error(format!("Expected symbol or keyword. Found: {:?}", key));
                };

                Value::Bool(obj.contains_key(key))
            }
            _ => Value::Error(format!(
                "has_obj: Expected 2 arguments, found: {}",
                items.len()
            )),
        }
    }

    fn _let(&mut self, items: &[SExpId]) -> Value {
        match items {
            [ident, value] => {
                let ident = self.asts.get(*ident).clone();
                let Some(ident) = ident.as_keyword() else {
                    return Value::Error("Let: Expected keyword".to_string());
                };

                let value = self.eval(*value);
                println!("Adding to env: {:?}", self.envs.last());
                self.envs.set(ident, value.clone());
                value
            }
            _ => Value::Error(format!("Expected 2 arguments, found: {}", items.len())),
        }
    }

    // CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
    #[allow(clippy::unnecessary_to_owned)]
    fn macro_def(&mut self, items: &[SExpId]) -> Result<Value, String> {
        let signature = items
            .first()
            .ok_or_else(|| "Expected signature".to_string())?;

        let signature = self.asts.get(*signature);
        let Some(signature) = signature.as_list() else {
            return Err("Expected list".to_string());
        };
        let signature = signature
            .to_vec()
            .into_iter()
            .map(|s| self.asts.get(s).as_keyword().unwrap().to_string())
            .collect();
        let body = items.get(1).ok_or_else(|| "Expected body".to_string())?;

        Ok(Value::Macro(Macro::Lisp {
            signature,
            body: *body,
        }))
    }

    fn macro_call(&mut self, macro_: Macro, args: &[SExpId]) -> Result<SExpId, String> {
        let result = match macro_ {
            Macro::Lisp { signature, body } => {
                self.envs.push();

                for (sig, arg) in signature.iter().zip(args) {
                    self.envs.set(sig, Value::SExp(*arg));
                }

                let result = self.eval(body);

                self.envs.pop();

                let result = result
                    .as_sexp()
                    .ok_or_else(|| "Expected SExpression".to_string())?;
                *result
            }
            Macro::Rust { body } => {
                let args = args.to_vec();
                body(self, args)
            }
        };

        println!("Macro call result: {}", self.asts.fmt(result));
        let envs = self.envs.slice();
        let processed = crate::process_ast(&mut self.asts, result, envs);
        println!("Processed: {}", self.asts.fmt(processed));

        Ok(processed)
    }

    fn do_(&mut self, items: &[SExpId]) -> Value {
        let mut result = None;
        self.envs.push();
        for item in items {
            result = Some(self.eval(*item));
        }
        self.envs.pop();
        result.unwrap_or_else(|| Value::Error("DO: Expected at least one argument".to_string()))
    }

    fn if_(&mut self, items: &[SExpId]) -> Result<Value, String> {
        match items {
            [condition, then, else_] => {
                let condition = self.eval(*condition);
                Ok(if condition.as_boolean().ok_or("Expected boolean")? {
                    self.eval(*then)
                } else {
                    self.eval(*else_)
                })
            }
            [condition, then] => {
                let condition = self.eval(*condition);
                if condition.as_boolean().ok_or("Expected boolean")? {
                    Ok(self.eval(*then))
                } else {
                    Err("No else branch".to_string())
                }
            }
            _ => Err(format!("Expected 2 or 3 arguments, found: {}", items.len())),
        }
    }

    pub fn new(asts: ASTS) -> Self {
        Self {
            asts,
            ..Default::default()
        }
    }

    pub fn with_macro(
        &mut self,
        name: &str,
        body: impl Fn(&mut Runtime, Vec<SExpId>) -> SExpId + 'static,
    ) {
        self.envs.set(
            name,
            Value::Macro(Macro::Rust {
                body: Rc::new(body),
            }),
        );
    }

    pub fn with_fn(
        &mut self,
        name: &str,
        body: impl Fn(&mut Runtime, Vec<Value>) -> Value + 'static,
    ) {
        self.envs.set(
            name,
            Value::Function(Function::Rust {
                body: Rc::new(body),
            }),
        );
    }

    pub fn eval_eager_rec(&mut self, sexp: SExpId, include_constructor: bool) -> Value {
        let mut value = self.eval(sexp);
        while value.is_lazy(include_constructor) {
            value = self.to_eager(value, include_constructor);
        }
        value
    }

    pub fn eval_eager(&mut self, sexp: SExpId, include_constructor: bool) -> Value {
        let value = self.eval(sexp);
        self.to_eager(value, include_constructor)
    }

    pub fn to_eager(&mut self, value: Value, include_constructor: bool) -> Value {
        match value {
            Value::Thunk(thunk) => self.thunk_call(thunk),
            Value::Ref(rc) => {
                let eager = rc.borrow();
                eager.clone()
            }
            Value::Constructor(constructor) if include_constructor => {
                self.constructor_call(constructor, None)
            }
            val => val,
        }
    }

    // fn as_symbol_or_keyword_or_string(&self, value: Value) -> Option<&str> {
    //     let sexp = value.as_sexp()?;
    //     let sexp = self.asts.get(*sexp);
    //     match sexp {
    //         SExp::Symbol(s) => Some(s),
    //         SExp::Keyword(s) => Some(s),
    //         SExp::String(s) => Some(s),
    //         _ => None,
    //     }
    // }

    fn as_symbol_or_keyword(&self, value: &Value) -> Option<&str> {
        let sexp = value.as_sexp()?;
        let sexp = self.asts.get(*sexp);
        match sexp {
            SExp::Symbol(s) => Some(s),
            SExp::Keyword(s) => Some(s),
            _ => None,
        }
    }

    pub fn eval(&mut self, id: SExpId) -> Value {
        let sexp = self.asts.get(id).clone();
        match sexp {
            SExp::Error => Value::Error("AST Error".to_string()),
            SExp::Number(n) => Value::Number(n),
            SExp::String(s) => Value::String(s.clone()),
            SExp::Bool(b) => Value::Bool(b),
            // SExp::Symbol(s) if s == "super" => {
            //     let Some(map) = self.supers.super_() else {
            //         return Value::Error("super used outside of object".to_string());
            //     };
            //     Value::Object(map.clone())
            // }
            SExp::Keyword(_s) => Value::SExp(id),
            SExp::Symbol(s) if s.starts_with(":") => {
                panic!("This should be a keyword: {}", s);
            }
            SExp::Symbol(s) => self
                .envs
                .get(s.as_str())
                .cloned()
                .unwrap_or_else(|| Value::Error(format!("Undefined variable: {}", s))),
            SExp::List(items) => {
                let first_id = items.first().copied();
                let first = self.asts.maybe_get(first_id);
                let Some(first) = first else {
                    todo!("Empty tuple");
                };
                let first_id = first_id.unwrap();
                match first {
                    SExp::Symbol(tag) if tag == "do" => self.do_(&items[1..]),
                    SExp::Symbol(tag) if tag == "thunk" => {
                        self.thunk_def(&items[1..]).unwrap_or_else(Value::Error)
                    }
                    SExp::Symbol(tag) if tag == "macro" => {
                        self.macro_def(&items[1..]).unwrap_or_else(Value::Error)
                    }
                    SExp::Symbol(tag) if tag == "fn" => {
                        self.function_def(&items[1..]).unwrap_or_else(Value::Error)
                    }
                    SExp::Symbol(tag) if tag == "cl" => {
                        self.closure_def(&items[1..]).unwrap_or_else(Value::Error)
                    }
                    SExp::Symbol(tag) if tag == "obj/con" => {
                        self.condef(&items[1..]).unwrap_or_else(Value::Error)
                    }
                    SExp::Symbol(tag) if tag == "list" => self.make_list(&items[1..]),
                    SExp::Symbol(tag) if tag == "is-type" => self.is_type(&items[1..]),
                    SExp::Symbol(tag) if tag == "quote" => {
                        let Some(item) = items.get(1) else {
                            return Value::Error("Expected item after quote".to_string());
                        };
                        self.quote(item)
                    }
                    // SExp::Symbol(tag) if tag == "+" => self.add(&items[1..]),
                    SExp::Symbol(tag) if tag == "quasiquote" => {
                        let Some(item) = items.get(1) else {
                            return Value::Error("Expected item after quasiquote".to_string());
                        };
                        self.quasiquote(item)
                    }
                    SExp::Symbol(tag) if tag == "let" => self._let(&items[1..]),
                    SExp::Symbol(tag) if tag == "if" => {
                        self.if_(&items[1..]).unwrap_or_else(Value::Error)
                    }
                    SExp::Symbol(tag) if tag == "has?" => self.has_obj(&items[1..]),
                    _first => {
                        // eprintln!("Evaling first");
                        let first = self.eval_eager_rec(first_id, true);

                        match first {
                            Value::Error(e) => Value::Error(e),
                            Value::Object(map) => {
                                let Some(key) = items.get(1) else {
                                    return Value::Object(map);
                                };
                                println!("key: {:?}", self.asts.fmt(*key));
                                let key = self.eval(*key);
                                try_err!(key);
                                let Some(key) = self.as_symbol_or_keyword(&key) else {
                                    return Value::Error(format!(
                                        "Access field: Expected symbol or keyword. Found: {:?}",
                                        key
                                    ));
                                };

                                map.get(key).cloned().unwrap_or_else(|| {
                                    Value::Error(format!("Undefined key: {} in {:?}", key, map))
                                })
                            }
                            Value::Constructor(constructor) => {
                                self.constructor_call(constructor, None)
                            }
                            Value::Function(function) => self.closure_call(function, &items[1..]),
                            Value::Macro(macro_) => self
                                .macro_call(macro_, &items[1..])
                                .map(|id| self.eval(id))
                                .unwrap_or_else(Value::Error),
                            _ => Value::Error(format!("Invalid caller: {:?}", first)),
                        }
                    }
                }
                // Otherwise, just return error for now
            }
        }
    }

    pub fn to_json(&mut self, value: Value, eager: bool) -> serde_json::Value {
        self.to_json_inner(value, eager, 5)
    }
    pub fn to_json_inner(
        &mut self,
        mut value: Value,
        eager: bool,
        depth: usize,
    ) -> serde_json::Value {
        println!("To json");
        if depth == 0 {
            return serde_json::Value::String("...".to_string());
        }
        if eager {
            value = value.eager_rec(self, true);
        }
        match value {
            Value::Number(n) => serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap()),
            Value::String(s) => serde_json::Value::String(s),
            Value::Bool(b) => serde_json::Value::Bool(b),
            Value::Object(map) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in map {
                    obj.insert(k, self.to_json_inner(v, eager, depth - 1));
                }
                serde_json::Value::Object(obj)
            }
            Value::List(list) => {
                let mut arr = vec![];
                for value in list {
                    arr.push(self.to_json_inner(value, eager, depth - 1));
                }
                serde_json::Value::Array(arr)
            }
            Value::Function(function) => {
                serde_json::Value::String(format!("<Function: {:?}>", function))
            }
            Value::Constructor(constructor) => {
                serde_json::Value::String(format!("<Constructor: {:?}>", constructor))
            }
            Value::Ref(rc) => serde_json::Value::String(format!("<Ref: {:?}>", rc)),
            Value::Thunk(thunk) => serde_json::Value::String(format!("<Thunk: {:?}>", thunk)),
            Value::Macro(macro_) => serde_json::Value::String(format!("<Macro: {:?}>", macro_)),
            Value::Error(e) => serde_json::Value::String(format!("<Error: {e}>")),
            Value::SExp(id) => {
                let ast = self.asts.get_ast(id);
                let sexp = ast.get(id).fmt(&self.asts).to_string();
                serde_json::Value::String(sexp)
            }
        }
    }
}

#[derive(Default)]
pub struct Runtime {
    envs: Envs,
    asts: ASTS,
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, io::Read, sync::Mutex};

    use super::{s_std::prelude, *};

    #[test]
    fn json_lazy() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "json-lazy", |input, _deps, _args| {
            // eprintln!("---");
            let mut asts = ASTS::new();
            let ast = asts.parse(input).unwrap();
            let root_id = ast.root_id().unwrap();
            let prelude = prelude();
            let envs = [prelude];
            let root_id = crate::process_ast(&mut asts, root_id, &envs);
            let [prelude] = envs;

            let mut runtime = Runtime::new(asts);
            runtime.with_env(prelude);
            let value = runtime.eval(root_id);
            let value = runtime.to_json(value, false);
            serde_json::to_string_pretty(&value).unwrap()
        })
    }

    fn eager_test(input: &str) -> String {
        eprintln!("---");
        let mut asts = ASTS::new();
        let ast = asts.parse(input).unwrap();
        let root_id = ast.root_id().unwrap();
        eprintln!("Before process");
        let prelude = prelude();
        let envs = [prelude];
        let root_id = crate::process_ast(&mut asts, root_id, &envs);
        let [prelude] = envs;

        let mut runtime = Runtime::new(asts);
        runtime.with_env(prelude);
        eprintln!("Before eval");
        let value = runtime.eval(root_id);
        eprintln!("Value: {value:?}");
        let value = runtime.to_json(value, true);
        serde_json::to_string_pretty(&value).unwrap()
    }

    #[test]
    fn json() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "json", |input, _deps, _args| eager_test(input))
    }

    fn level_from_args(args: &HashSet<&str>) -> tracing::Level {
        const LEVELS: &[(&str, tracing::Level)] = &[
            ("trace", tracing::Level::TRACE),
            ("debug", tracing::Level::DEBUG),
            ("info", tracing::Level::INFO),
            ("warn", tracing::Level::WARN),
            ("error", tracing::Level::ERROR),
        ];

        for (name, level) in LEVELS {
            if args.contains(name) {
                return *level;
            }
        }
        tracing::Level::INFO
    }

    #[test]
    fn traces() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "traces", |input, _deps, args| {
            let writer = tempfile::NamedTempFile::new().unwrap();
            let mut reader = writer.reopen().unwrap();
            let path = writer.path().to_owned();

            let level = level_from_args(args);

            let subscriber = tracing_subscriber::fmt()
                .pretty()
                .with_max_level(level)
                .with_writer(Mutex::new(writer))
                .finish();
            tracing::subscriber::with_default(subscriber, move || {
                tracing::info!("Logs ({level:?}) stored in: {path:?}");
                eager_test(input);
            });

            let mut buf = String::new();
            reader.read_to_string(&mut buf).unwrap();
            buf
        })
    }

    #[test]
    fn json_eager() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "json-eager", |input, _deps, _args| {
            eager_test(input)
        })
    }

    #[test]
    fn processed() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "processed", |input, _deps, _args| {
            // eprintln!("---");
            let mut asts = ASTS::new();
            let ast = asts.parse(input).unwrap();
            let root_id = ast.root_id().unwrap();
            let prelude = prelude();
            let root_id = crate::process_ast(&mut asts, root_id, &[prelude]);

            asts.fmt(root_id).to_string()
        })
    }
}
