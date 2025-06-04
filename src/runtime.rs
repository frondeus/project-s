use std::rc::Rc;

use env::Envs;
use structs::Structs;
use value::{Function, Macro, Value};

use crate::{
    ast::{ASTS, SExp, SExpId},
    types::{Type, TypeEnv},
};

mod env;
mod functions;
mod quotes;
mod s_std;
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

    #[allow(dead_code)]
    fn add(&mut self, items: &[SExpId]) -> Value {
        if items.is_empty() {
            return Value::Error("Expected at least one argument".to_string());
        }

        let first = items.first().unwrap();

        let mut first = self.eval_eager(*first);
        try_err!(first);

        match &mut first {
            Value::Number(sum) => {
                for item in items.iter().skip(1) {
                    let right = self.eval_eager(*item);
                    try_err!(right);
                    let Some(n) = right.as_number() else {
                        return Value::Error(format!("Expected number, got: {:?}", right));
                    };
                    *sum += n;
                }
            }
            Value::Object(left) => {
                let _super = left.clone();
                self.supers.push(_super);
                for item in items.iter().skip(1) {
                    let right = self.eval_eager(*item);
                    try_err!(right);
                    let right = match right {
                        Value::Object(right) => right,
                        Value::SExp(id) => {
                            let right = self.eval(id).into_object();
                            let Some(right) = right else {
                                return Value::Error("Expected quoted object".to_string());
                            };
                            right
                        }
                        _ => {
                            return Value::Error("Expected object".to_string());
                        }
                    };
                    for (key, value) in right {
                        left.insert(key, value);
                    }
                }
                self.supers.pop();
            }
            t => return Value::Error(format!("Expected number or object, got: {:?}", t)),
        }
        first
    }

    fn has_obj(&mut self, items: &[SExpId]) -> Value {
        let Some(obj) = items.first() else {
            return Value::Error("Expected object".to_string());
        };
        let obj = self.eval(*obj);
        try_err!(obj);
        let Some(obj) = obj.as_object() else {
            return Value::Error("Expected object".to_string());
        };

        let Some(key) = items.get(1) else {
            return Value::Error("Expected key".to_string());
        };

        let key = self.eval(*key);
        try_err!(key);
        let Some(key) = self.as_symbol_or_keyword(&key) else {
            return Value::Error(format!("Expected symbol or keyword. Found: {:?}", key));
        };

        Value::Bool(obj.contains_key(key))
    }

    fn _let(&mut self, items: &[SExpId]) -> Value {
        match items {
            [ident, value] => {
                let ident = self.asts.get(*ident).clone();
                let Some(ident) = ident.as_keyword() else {
                    return Value::Error("Let: Expected keyword".to_string());
                };

                let value = self.eval(*value);
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
            .map(|s| self.asts.get(s).as_symbol().unwrap().to_string())
            .collect();
        let body = items.get(1).ok_or_else(|| "Expected body".to_string())?;

        Ok(Value::Macro(Macro::Lisp {
            signature,
            body: *body,
        }))
    }

    fn macro_call(&mut self, macro_: Macro, args: &[SExpId]) -> Result<SExpId, String> {
        match macro_ {
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
                Ok(*result)
            }
            Macro::Rust { body } => {
                let args = args.to_vec();
                let result = body(self, args);
                Ok(result)
            }
        }
    }

    fn do_(&mut self, items: &[SExpId]) -> Value {
        let mut result = None;
        self.envs.push();
        for item in items {
            result = Some(self.eval(*item));
        }
        result.unwrap_or_else(|| Value::Error("Expected at least one argument".to_string()))
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

    pub fn eval_eager(&mut self, sexp: SExpId) -> Value {
        let value = self.eval(sexp);
        self.to_eager(value)
    }

    pub fn to_eager(&mut self, value: Value) -> Value {
        match value {
            Value::Thunk(thunk) => self.thunk_call(thunk),
            val => val,
        }
    }

    fn as_symbol_or_keyword_or_string(&self, value: Value) -> Option<&str> {
        let sexp = value.as_sexp()?;
        let sexp = self.asts.get(*sexp);
        match sexp {
            SExp::Symbol(s) => Some(s),
            SExp::Keyword(s) => Some(s),
            SExp::String(s) => Some(s),
            _ => None,
        }
    }

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
            SExp::Symbol(s) if s == "self" => {
                let Some(map) = self.structs._self() else {
                    return Value::Error("self used outside of object".to_string());
                };
                Value::Object(map.clone())
            }
            SExp::Symbol(s) if s == "root" => {
                let Some(map) = self.structs.root() else {
                    return Value::Error("root used outside of object".to_string());
                };
                Value::Object(map.clone())
            }
            SExp::Symbol(s) if s == "super" => {
                let Some(map) = self.supers._self() else {
                    return Value::Error("super used outside of object".to_string());
                };
                Value::Object(map.clone())
            }
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
                    SExp::Symbol(tag) if tag == "struct" => self.make_struct(&items[1..]),
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
                    SExp::Symbol(tag) if tag == "has?" => self.has_obj(&items[1..]),
                    _first => {
                        let first = self.eval_eager(first_id);

                        match first {
                            Value::Error(e) => Value::Error(e),
                            Value::Object(map) => {
                                let Some(key) = items.get(1) else {
                                    return Value::Error("Expected key".to_string());
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
                                    Value::Error(format!("Undefined key: {}", key))
                                })
                            }
                            Value::Closure(closure) => self.closure_call(closure, &items[1..]),
                            Value::Function(function) => self.function_call(function, &items[1..]),
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
    pub fn to_json(&self, value: Value) -> serde_json::Value {
        match value {
            Value::Number(n) => serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap()),
            Value::String(s) => serde_json::Value::String(s),
            Value::Bool(b) => serde_json::Value::Bool(b),
            Value::Object(map) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in map {
                    obj.insert(k, self.to_json(v));
                }
                serde_json::Value::Object(obj)
            }
            Value::Function(function) => {
                serde_json::Value::String(format!("<Function: {:?}>", function))
            }
            Value::Closure(closure) => {
                serde_json::Value::String(format!("<Closure: {:?}>", closure))
            }
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
    structs: Structs,
    supers: Structs,
    asts: ASTS,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "json", |input, _deps| {
            // eprintln!("---");
            let mut asts = ASTS::new();
            let ast = asts.parse(input).unwrap();
            let root_id = ast.root_id().unwrap();
            let root_id = crate::process_ast(&mut asts, root_id);

            let mut runtime = Runtime::new(asts);
            runtime.with_prelude();
            let value = runtime.eval(root_id);
            // println!("value: {value:?}");
            let value = runtime.to_json(value);
            serde_json::to_string_pretty(&value).unwrap()
        })
    }
}
