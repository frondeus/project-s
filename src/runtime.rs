use std::{collections::BTreeMap, rc::Rc};

pub use env::Env;
use env::Envs;
// use structs::Structs;
pub use value::{Function, InnerThunk, Macro, Thunk, Value};

use crate::{
    ast::{ASTS, SExp, SExpId},
    modules::ModuleProvider,
    patterns::Pattern,
    // types::{Type, TypeEnv},
};

mod env;
mod functions;
mod json;
mod macros;
mod quotes;
pub mod s_std;
mod structs;
mod thunks;
pub mod value;

#[macro_export]
macro_rules! try_err {
    ($val: expr) => {
        if let $crate::runtime::value::Value::Error(e) = $val {
            return $crate::runtime::value::Value::Error(e);
        };
    };
}

impl Runtime {
    // fn is_type(&self, items: &[SExpId]) -> Value {
    //     let Some(sexp) = items.first() else {
    //         return Value::Error("Expected SExpression".to_string());
    //     };
    //     // let mut env = TypeEnv::default();
    //     // let infered = env.infer(self.asts.get_ast(*sexp), *sexp);
    //     // let result = env.get(infered);

    //     let Some(ty_id) = items.get(1) else {
    //         return Value::Error("Expected type".to_string());
    //     };
    //     let ty = self.asts.get_ast(*sexp).get(*ty_id);
    //     let Some(ty) = ty.as_symbol() else {
    //         return Value::Error(format!(
    //             "Expected symbol. Found: {:?}",
    //             self.asts.fmt(*ty_id)
    //         ));
    //     };
    //     let ty = match ty {
    //         "Number" => Type::Number,
    //         "String" => Type::String,
    //         "Bool" => Type::Bool,
    //         "Symbol" => Type::Symbol,
    //         "Error" => Type::Error,
    //         ty => return Value::Error(format!("Unknown type: {}", ty)),
    //     };
    //     Value::Bool(*result == ty)
    // }

    pub(crate) fn destruct_with(
        &mut self,
        pattern: Pattern,
        value: Value,
        with: &impl Fn(&mut Self, &str, Value),
    ) -> Result<Value, String> {
        match pattern {
            Pattern::Single(key) => {
                with(self, &key, value.clone());
                Ok(value)
            }
            Pattern::List(patterns) => match value.eager_rec(self, true) {
                Value::List(items) => {
                    let mut result = vec![];
                    for (pattern, item) in patterns.into_iter().zip(items.into_iter()) {
                        let value = self.destruct_with(pattern, item, with)?;
                        result.push(value);
                    }
                    Ok(Value::List(result))
                }
                value => Err(format!("Destructing. Expected list, found: {:?}", value)),
            },
            Pattern::Object(patterns) => match value.eager_rec(self, true) {
                Value::Object(mut map) => {
                    let mut result = BTreeMap::new();
                    for (key, pattern) in patterns {
                        let value = map.remove(&key).unwrap_or_else(|| {
                            Value::Error(format!("Field :{key} not found in {map:?} "))
                        });

                        let value = self.destruct_with(pattern, value, with)?;
                        result.insert(key, value);
                    }

                    Ok(Value::Object(result))
                }
                value => Err(format!("Destructing. Expected object, found: {:?}", value)),
            },
        }
    }

    pub(crate) fn destruct_(&mut self, pattern: Pattern, value: Value) -> Result<Value, String> {
        self.destruct_with(pattern, value, &|this, key, value| {
            tracing::debug!("Adding to env: {:?}", this.envs.last());
            this.envs.set(key, value);
        })
    }

    fn _let(&mut self, items: &[SExpId]) -> Result<Value, String> {
        match items {
            [ident, value] => {
                let pattern = Pattern::parse(*ident, &self.asts)?;
                let value = self.eval(*value);

                self.destruct_(pattern, value)
            }
            _ => Err(format!("Expected 2 arguments, found: {}", items.len())),
        }
    }

    fn _let_rec_pre_destruct(&mut self, pattern: Pattern) {
        match pattern {
            Pattern::Single(key) => self.envs.set(&key, Value::Thunk(Thunk::new_for_let())),
            Pattern::List(patterns) => {
                for pattern in patterns {
                    self._let_rec_pre_destruct(pattern);
                }
            }
            Pattern::Object(hash_map) => {
                for (_key, pattern) in hash_map {
                    self._let_rec_pre_destruct(pattern);
                }
            }
        }
    }

    pub(crate) fn _let_rec_destruct(
        &mut self,
        pattern: Pattern,
        value: Value,
    ) -> Result<Value, String> {
        self.destruct_with(pattern, value, &|this, key, value| {
            let thunk = this.envs.get(key).unwrap().as_thunk().unwrap();

            *thunk.inner.borrow_mut() = InnerThunk::Evaluated(value);
        })
    }

    fn _let_rec(&mut self, items: &[SExpId]) -> Result<Value, String> {
        match items {
            [ident, value] => {
                let pattern = Pattern::parse(*ident, &self.asts)?;
                self._let_rec_pre_destruct(pattern.clone());
                let thunk = self.eval(*value);
                self._let_rec_destruct(pattern, thunk)
            }
            _ => Err(format!("Expected 2 arguments, found: {}", items.len())),
        }
    }

    fn do_(&mut self, items: &[SExpId]) -> Value {
        let mut result = None;
        self.envs.push();
        for item in items {
            let mut value = self.eval(*item);

            if self.eager_do_error {
                value = match value.ok() {
                    Err(e) => {
                        self.envs.pop();
                        return Value::Error(e);
                    }
                    Ok(value) => value,
                }
            }
            result = Some(value);
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
                    Ok(Value::List(vec![]))
                }
            }
            _ => Err(format!("Expected 2 or 3 arguments, found: {}", items.len())),
        }
    }

    pub fn new(asts: ASTS, modules: Box<dyn ModuleProvider>) -> Self {
        Self {
            asts,
            modules,
            eager_do_error: true,
            envs: Default::default(),
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
        match &sexp.item {
            SExp::Symbol(s) => Some(s),
            SExp::Keyword(s) => Some(s),
            _ => None,
        }
    }

    pub fn eval(&mut self, id: SExpId) -> Value {
        let sexp = self.asts.get(id).clone();
        match sexp.item {
            SExp::Error => Value::Error("AST Error".to_string()),
            SExp::Number(n) => Value::Number(n),
            SExp::String(s) => Value::String(s.clone()),
            SExp::Bool(b) => Value::Bool(b),
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
                match &first.item {
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
                    // SExp::Symbol(tag) if tag == "is-type" => self.is_type(&items[1..]),
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
                    SExp::Symbol(tag) if tag == "let" => {
                        self._let(&items[1..]).unwrap_or_else(Value::Error)
                    }
                    SExp::Symbol(tag) if tag == "let-rec" => {
                        self._let_rec(&items[1..]).unwrap_or_else(Value::Error)
                    }
                    SExp::Symbol(tag) if tag == "if" => {
                        self.if_(&items[1..]).unwrap_or_else(Value::Error)
                    }
                    _first => {
                        let first = self.eval_eager_rec(first_id, true);

                        match first {
                            Value::Error(e) => Value::Error(e),
                            Value::Object(map) => {
                                let Some(key) = items.get(1) else {
                                    return Value::Object(map);
                                };
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

    pub fn modules(&self) -> &dyn ModuleProvider {
        &*self.modules
    }
}

pub struct Runtime {
    envs: Envs,
    asts: ASTS,
    eager_do_error: bool,
    modules: Box<dyn ModuleProvider>,
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        io::Read,
        path::PathBuf,
    };

    use test_runner::CowStr;
    use tracing_subscriber::{Layer, layer::SubscriberExt};

    use crate::modules::MemoryModules;

    use super::{s_std::prelude, *};

    fn eval_to_value(input: &str, modules: MemoryModules) -> (Runtime, Value) {
        let mut asts = ASTS::new();
        let ast = asts.parse(input, "<input>").unwrap();
        let root_id = ast.root_id().unwrap();
        tracing::trace!("Before process");
        let prelude = prelude();
        let envs = [prelude];
        let root_id = crate::process_ast(&mut asts, root_id, &envs);
        let [prelude] = envs;

        let modules = Box::new(modules);
        let mut runtime = Runtime::new(asts, modules);
        runtime.with_env(prelude);
        tracing::trace!("Before eval");
        let value = runtime.eval(root_id);
        (runtime, value)
    }

    fn eval_to_json(input: &str, modules: MemoryModules, eager: bool) -> String {
        let (mut runtime, value) = eval_to_value(input, modules);
        tracing::trace!("Value: {value:?}");
        let value = runtime.to_json(value, eager);
        serde_json::to_string_pretty(&value).unwrap()
    }

    fn deps_to_modules(deps: &HashMap<CowStr<'_>, &str>) -> MemoryModules {
        let modules = deps
            .iter()
            .filter(|(name, _value)| name.ends_with(".s"))
            .map(|(name, value)| (PathBuf::from(name.to_string()), value.to_string()))
            .collect::<HashMap<_, _>>();
        // tracing::info!("Modules: {:#?}", modules);
        MemoryModules { modules }
    }

    #[test]
    fn json() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "json", |input, deps, args| {
            let lazy = args.contains("lazy");
            tracing::subscriber::with_default(tracing_subscriber::fmt().finish(), || {
                let deps = deps_to_modules(deps);
                eval_to_json(input, deps, !lazy)
            })
        })
    }

    fn level_from_args(args: &HashSet<&str>) -> tracing::level_filters::LevelFilter {
        const LEVELS: &[(&str, tracing::level_filters::LevelFilter)] = &[
            ("trace", tracing::level_filters::LevelFilter::TRACE),
            ("debug", tracing::level_filters::LevelFilter::DEBUG),
            ("info", tracing::level_filters::LevelFilter::INFO),
            ("warn", tracing::level_filters::LevelFilter::WARN),
            ("error", tracing::level_filters::LevelFilter::ERROR),
        ];

        for (name, level) in LEVELS {
            if args.contains(name) {
                return *level;
            }
        }
        tracing::level_filters::LevelFilter::INFO
    }

    #[test]
    fn traces() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "traces", |input, _deps, args| {
            let mut reader = tempfile::NamedTempFile::new().unwrap();

            let writer = reader.reopen().unwrap();

            {
                let level = level_from_args(args);
                let (writer, _guard) = tracing_appender::non_blocking(writer);

                let file_layer = tracing_subscriber::fmt::Layer::new()
                    // .compact()
                    .with_file(args.contains("file"))
                    .with_line_number(args.contains("line"))
                    .with_writer(writer)
                    .without_time()
                    .with_ansi(false);

                let console_layer = tracing_subscriber::fmt::Layer::new()
                    // .compact()
                    .with_file(args.contains("file"))
                    .with_line_number(args.contains("line"))
                    .with_ansi(true);

                let subscriber = tracing_subscriber::registry()
                    .with(console_layer.with_filter(level))
                    .with(file_layer.with_filter(level));

                tracing::subscriber::with_default(subscriber, move || {
                    let modules = MemoryModules::default();
                    let (mut runtime, value) = eval_to_value(input, modules);
                    runtime.to_json(value, true);
                });
            }

            let mut buf = String::new();
            reader.read_to_string(&mut buf).unwrap();
            buf
        })
    }

    #[test]
    fn processed() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "processed", |input, _deps, _args| {
            // eprintln!("---");
            let mut asts = ASTS::new();
            let ast = asts.parse(input, "<input>").unwrap();
            let root_id = ast.root_id().unwrap();
            let prelude = prelude();
            let root_id = crate::process_ast(&mut asts, root_id, &[prelude]);

            asts.fmt(root_id).to_string()
        })
    }
}
