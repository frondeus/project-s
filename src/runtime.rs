use std::collections::BTreeMap;

use crate::{
    ast::{AST, ASTS, SExp, SExpId},
    types::{Type, TypeEnv},
};

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Object(BTreeMap<String, Box<Value>>),
    Symbol(String),
    SExp(SExpId),
    /// For error handling
    Error(String),
}

macro_rules! try_err {
    ($val: expr) => {
        if let Value::Error(e) = $val {
            return Value::Error(e);
        };
    };
}

impl Value {
    fn as_sexp(&self) -> Option<&SExpId> {
        match self {
            Value::SExp(id) => Some(id),
            _ => None,
        }
    }

    fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    fn as_symbol(&self) -> Option<&str> {
        match self {
            Value::Symbol(s) => Some(s),
            _ => None,
        }
    }

    fn as_object(&self) -> Option<&BTreeMap<String, Box<Value>>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    fn to_sexp(&self, target: &mut AST) -> SExpId {
        match self {
            Value::Number(n) => target.add_node(SExp::Number(*n)),
            Value::String(s) => target.add_node(SExp::String(s.clone())),
            Value::Bool(b) => target.add_node(SExp::Bool(*b)),
            Value::Symbol(s) => target.add_node(SExp::Symbol(s.clone())),
            Value::Object(_btree_map) => todo!(),
            Value::SExp(sexp_id) => *sexp_id,
            Value::Error(err) => {
                println!("Error: {err}");
                target.add_node(SExp::Error)
            }
        }
    }
}

impl Runtime {
    fn insert_to_struct(
        &mut self,
        key: &str,
        items: &mut impl Iterator<Item = SExpId>,
    ) -> Result<(), String> {
        eprintln!("Processing pair: {key}");
        let Some(value) = items.next() else {
            return Err("Expected value".to_string());
        };
        let value = self.eval(value);
        self.structs
            .mut_self()
            .unwrap()
            .insert(key.to_string(), Box::new(value));
        Ok(())
    }

    fn make_struct_inner(&mut self, mut items: impl Iterator<Item = SExpId>) -> Result<(), String> {
        while let Some(item_id) = items.next() {
            let item = self.asts.get(item_id).clone();
            match item {
                SExp::List(list) => {
                    eprintln!("Processing list: {list:?}");
                    let first = list.first().ok_or_else(|| "Expected list".to_string())?;
                    let first = self
                        .asts
                        .get(*first)
                        .as_symbol()
                        .map(ToOwned::to_owned)
                        .ok_or_else(|| "Expected symbol".to_string())?;
                    match first.as_str() {
                        "let" => {
                            self.object_let(&list[1..])?;
                        }
                        "if" => {
                            self.object_if(&list[1..])?;
                        }
                        _ => {
                            return Err(format!("Unknown symbol: {}", first));
                        }
                    }
                }
                _ => {
                    let key = self.eval(item_id);
                    let key = match key {
                        Value::Symbol(key) => key,
                        Value::String(key) => key,
                        _ => {
                            return Err("Expected symbol or string".to_string());
                        }
                    };
                    self.insert_to_struct(&key, &mut items)?;
                }
            }
        }
        Ok(())
    }

    // CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
    #[allow(clippy::unnecessary_to_owned)]
    fn make_struct(&mut self, items: &[SExpId]) -> Value {
        let Some(sexp) = items.first() else {
            return Value::Error("Expected SExpression. Found None".to_string());
        };
        let evaled = self.eval(*sexp);
        let Some(sexp) = evaled.as_sexp() else {
            return Value::Error(format!("Expected SExpression. Found {evaled:?}",));
        };
        let sexp = self.asts.get(*sexp);
        let Some(items) = sexp.as_list() else {
            return Value::Error("Expected list".to_string());
        };

        // let mut map = BTreeMap::new();
        let items = items.to_vec().into_iter();
        self.structs.push_default();
        self.envs.push();

        if let Err(e) = self.make_struct_inner(items) {
            self.envs.pop();
            self.structs.pop();
            return Value::Error(e);
        }

        self.envs.pop();
        let map = self.structs.pop();
        Value::Object(map)
    }

    fn is_type(&self, items: &[SExpId]) -> Value {
        let Some(sexp) = items.first() else {
            return Value::Error("Expected SExpression".to_string());
        };
        let mut env = TypeEnv::default();
        let infered = env.infer(self.asts.get_ast(*sexp), *sexp);
        let result = env.get(infered);

        let Some(ty) = items.get(1) else {
            return Value::Error("Expected type".to_string());
        };
        let ty = self.asts.get_ast(*sexp).get(*ty);
        let Some(ty) = ty.as_symbol() else {
            return Value::Error("Expected symbol".to_string());
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

    fn quote(&self, id: &SExpId) -> Value {
        let sexp = self.asts.get(*id);
        match sexp {
            SExp::Number(n) => Value::Number(*n),
            SExp::String(s) => Value::String(s.clone()),
            SExp::Symbol(_) => Value::SExp(*id),
            SExp::Bool(b) => Value::Bool(*b),
            SExp::Error => Value::Error("Quote: AST Error".to_string()),
            SExp::List(_) => Value::SExp(*id),
        }
    }

    fn quasiquote(&mut self, id: &SExpId) -> Value {
        let sexp = self.asts.get(*id);
        match sexp {
            SExp::Number(n) => Value::Number(*n),
            SExp::String(s) => Value::String(s.clone()),
            SExp::Symbol(_) => Value::SExp(*id),
            SExp::Bool(b) => Value::Bool(*b),
            SExp::Error => Value::Error("Quasiquote: AST Error".to_string()),
            SExp::List(_) => {
                let mut new_ast = AST::default();
                self.traverse_unquote(&mut new_ast, id);
                let root = new_ast.root_id().unwrap();
                self.asts.add_ast(new_ast);
                Value::SExp(root)
            }
        }
    }

    fn traverse_unquote(&mut self, new_ast: &mut AST, id: &SExpId) -> SExpId {
        match self.asts.get(*id).clone() {
            SExp::List(items) => {
                let Some(first) = items.first() else {
                    return *id;
                };
                if self.is_unquote(first) {
                    let Some(next) = items.get(1) else {
                        todo!("Somehow return an error");
                    };
                    let evaled = self.eval(*next);
                    evaled.to_sexp(new_ast)
                } else {
                    let parent = new_ast.reserve();
                    let mut result = Vec::new();
                    for item in &items {
                        result.push(self.traverse_unquote(new_ast, item));
                    }
                    // if result == items {
                    //     return *id;
                    // }
                    let new_list = SExp::List(result);
                    new_ast.set(parent, new_list);
                    parent
                }
            }
            _ => *id,
        }
    }

    fn is_unquote(&self, id: &SExpId) -> bool {
        let sexp = self.asts.get(*id);
        match sexp {
            SExp::Symbol(s) => s == "unquote",
            _ => false,
        }
    }

    fn add(&mut self, items: &[SExpId]) -> Value {
        if items.is_empty() {
            return Value::Error("Expected at least one argument".to_string());
        }

        let first = items.first().unwrap();

        let mut first = self.eval(*first);
        try_err!(first);

        match &mut first {
            Value::Number(sum) => {
                for item in items.iter().skip(1) {
                    let right = self.eval(*item);
                    try_err!(right);
                    let Some(n) = right.as_number() else {
                        return Value::Error("Expected number".to_string());
                    };
                    *sum += n;
                }
            }
            Value::Object(left) => {
                let _super = left.clone();
                self.supers.push(_super);
                for item in items.iter().skip(1) {
                    let right = self.eval(*item);
                    try_err!(right);
                    let Some(right) = right.as_object() else {
                        return Value::Error("Expected object".to_string());
                    };
                    for (key, value) in right {
                        left.insert(key.clone(), value.clone());
                    }
                }
                self.supers.pop();
            }
            _ => return Value::Error("Expected number".to_string()),
        }
        first
    }

    fn has_obj(&mut self, items: &[SExpId]) -> Value {
        let Some(obj) = items.first() else {
            return Value::Error("Expected object".to_string());
        };
        let obj = self.eval(*obj);
        let Some(obj) = obj.as_object() else {
            return Value::Error("Expected object".to_string());
        };

        let Some(key) = items.get(1) else {
            return Value::Error("Expected key".to_string());
        };

        let key = self.eval(*key);
        let Some(key) = key.as_symbol() else {
            return Value::Error("Expected symbol".to_string());
        };

        Value::Bool(obj.contains_key(key))
    }

    fn object_let(&mut self, items: &[SExpId]) -> Result<(), String> {
        let Some(ident) = items.first() else {
            return Err("Expected SExpression".to_string());
        };
        let ident = self.asts.get(*ident).clone();
        let Some(ident) = ident.as_symbol() else {
            return Err("Expected symbol".to_string());
        };

        let Some(value) = items.get(1) else {
            return Err("Expected value".to_string());
        };
        let value = self.eval(*value);
        eprintln!("Setting {ident} to {value:?}");
        self.envs.set(ident, value);
        Ok(())
    }

    fn _let(&mut self, items: &[SExpId]) -> Value {
        let Some(ident) = items.first() else {
            return Value::Error("Expected SExpression".to_string());
        };
        let ident = self.asts.get(*ident).clone();
        let Some(ident) = ident.as_symbol() else {
            return Value::Error("Expected symbol".to_string());
        };

        let Some(value) = items.get(1) else {
            return Value::Error("Expected value".to_string());
        };
        let Some(body) = items.get(2) else {
            return Value::Error("Expected body".to_string());
        };
        let value = self.eval(*value);

        self.envs.push();
        self.envs.set(ident, value);
        dbg!(&self.envs);
        let result = self.eval(*body);
        self.envs.pop();
        result
    }

    // CLIPPY: It is necessary to use `to_owned` here because `items` is borrowed
    #[allow(clippy::unnecessary_to_owned)]
    fn object_if(&mut self, items: &[SExpId]) -> Result<(), String> {
        let Some(condition) = items.first() else {
            return Err("Expected condition".to_string());
        };
        let condition = self.eval(*condition);
        let Value::Bool(b) = condition else {
            return Err("Expected boolean".to_string());
        };

        let Some(then) = items.get(1) else {
            return Err("Expected then".to_string());
        };
        let else_ = items.get(2);

        let branch = if b { Some(then) } else { else_ };

        let Some(branch) = branch else {
            return Ok(());
        };

        let evaled = self.eval(*branch);

        let Some(sexp) = evaled.as_sexp() else {
            return Err(format!("Expected SExpression. Found {evaled:?}"));
        };
        let sexp = self.asts.get(*sexp);
        let Some(items) = sexp.as_list() else {
            return Err("Expected list".to_string());
        };
        let items = items.to_vec().into_iter();

        self.make_struct_inner(items)?;
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct Env {
    // is_obj: bool,
    vars: BTreeMap<String, Value>,
}
// impl Env {
//     fn obj() -> Self {
//         Self { is_obj: true, ..Default::default()}
//     }
// }

#[derive(Debug)]
pub struct Envs {
    envs: Vec<Env>,
}

impl Default for Envs {
    fn default() -> Self {
        Self::new()
    }
}

impl Envs {
    pub fn new() -> Self {
        Self {
            envs: vec![Env::default()],
        }
    }

    fn last_mut(&mut self) -> &mut Env {
        self.envs.last_mut().expect("No environment")
    }

    pub fn set(&mut self, name: &str, value: Value) {
        self.last_mut().vars.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.envs.iter().rev().find_map(|env| env.vars.get(name))
    }

    pub fn push(&mut self) {
        self.envs.push(Env::default());
    }

    pub fn pop(&mut self) {
        self.envs.pop();
    }

    // pub fn _self(&self) -> Option<&Env> {
    //     self.envs.iter().rev().find(|env| env.is_obj)
    // }
}

#[derive(Default)]
pub struct Runtime {
    envs: Envs,
    structs: Structs,
    supers: Structs,
    asts: ASTS,
}

#[derive(Default)]
struct Structs {
    stack: Vec<BTreeMap<String, Box<Value>>>,
}

impl Structs {
    fn push_default(&mut self) {
        self.stack.push(BTreeMap::new());
    }
    fn push(&mut self, strukt: BTreeMap<String, Box<Value>>) {
        self.stack.push(strukt);
    }

    fn pop(&mut self) -> BTreeMap<String, Box<Value>> {
        self.stack.pop().unwrap()
    }

    fn _self(&self) -> Option<&BTreeMap<String, Box<Value>>> {
        self.stack.last()
    }

    fn mut_self(&mut self) -> Option<&mut BTreeMap<String, Box<Value>>> {
        self.stack.last_mut()
    }

    fn root(&self) -> Option<&BTreeMap<String, Box<Value>>> {
        self.stack.first()
    }
}

impl Runtime {
    pub fn new(ast: AST) -> Self {
        let mut runtime = Self::default();
        runtime.asts.add_ast(ast);
        runtime
    }

    pub fn eval(&mut self, sexp: SExpId) -> Value {
        let sexp = self.asts.get(sexp).clone();
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
            SExp::Symbol(s) if s.starts_with(":") => {
                let s = s.trim_start_matches(':');
                Value::Symbol(s.to_string())
            }
            SExp::Symbol(s) => dbg!(&self.envs)
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
                    SExp::Symbol(tag) if tag == "struct" => self.make_struct(&items[1..]),
                    SExp::Symbol(tag) if tag == "is-type" => self.is_type(&items[1..]),
                    SExp::Symbol(tag) if tag == "quote" => {
                        let Some(item) = items.get(1) else {
                            return Value::Error("Expected item after quote".to_string());
                        };
                        self.quote(item)
                    }
                    SExp::Symbol(tag) if tag == "+" => self.add(&items[1..]),
                    SExp::Symbol(tag) if tag == "quasiquote" => {
                        let Some(item) = items.get(1) else {
                            return Value::Error("Expected item after quasiquote".to_string());
                        };
                        self.quasiquote(item)
                    }
                    SExp::Symbol(tag) if tag == "let" => self._let(&items[1..]),
                    SExp::Symbol(tag) if tag == "has?" => self.has_obj(&items[1..]),
                    _first => {
                        let first = self.eval(first_id);

                        match first {
                            Value::Object(map) => {
                                let Some(key) = items.get(1) else {
                                    return Value::Error("Expected key".to_string());
                                };
                                let key = self.eval(*key);
                                let Some(key) = key.as_symbol() else {
                                    return Value::Error("Expected symbol".to_string());
                                };

                                map.get(key).cloned().map(|v| *v).unwrap_or_else(|| {
                                    Value::Error(format!("Undefined key: {}", key))
                                })
                            }
                            _ => Value::Error(format!("Unknown value: {:?}", first)),
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
            Value::Symbol(s) => serde_json::Value::String(format!("<Symbol: {s}>")),
            Value::Bool(b) => serde_json::Value::Bool(b),
            Value::Object(map) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in map {
                    obj.insert(k, self.to_json(*v));
                }
                serde_json::Value::Object(obj)
            }
            Value::Error(e) => serde_json::Value::String(format!("<Error: {e}>")),
            Value::SExp(id) => {
                let ast = self.asts.get_ast(id);
                let sexp = ast.get(id).fmt(&self.asts).to_string();
                serde_json::Value::String(sexp)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "json", |input, _deps| {
            eprintln!("---");
            let ast = crate::ast::AST::parse(input).unwrap();
            let root_id = ast.root_id().unwrap();
            let mut runtime = Runtime::new(ast);
            let value = runtime.eval(root_id);
            println!("value: {value:?}");
            let value = runtime.to_json(value);
            serde_json::to_string_pretty(&value).unwrap()
        })
    }
}
