use std::collections::BTreeMap;

use itertools::Itertools;

use crate::{
    ast::{AST, SExp, SExpId},
    types::{Type, TypeEnv},
};

pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Object(BTreeMap<String, Box<Value>>),
    /// For error handling
    Error(String),
}

fn make_struct(ast: &AST, items: &[SExpId]) -> Value {
    let mut map = BTreeMap::new();
    for (key, value) in items.iter().skip(1).map(|id| ast.get(*id)).tuples() {
        let Some(symbol) = key.as_symbol() else {
            return Value::Error("Expected symbol as struct key".to_string());
        };
        let key = symbol.trim_start_matches(':');
        let value = eval(ast, value);
        map.insert(key.to_string(), Box::new(value));
    }
    Value::Object(map)
}

fn is_type(ast: &AST, items: &[SExpId]) -> Value {
    let Some(sexp) = items.get(1) else {
        return Value::Error("Expected SExpression".to_string());
    };
    let mut env = TypeEnv::default();
    let infered = env.infer(ast, *sexp);
    let result = env.get(infered);

    let Some(ty) = items.get(2) else {
        return Value::Error("Expected type".to_string());
    };
    let ty = ast.get(*ty);
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

pub fn eval(ast: &AST, sexp: &SExp) -> Value {
    match sexp {
        SExp::Error => Value::Error("AST Error".to_string()),
        SExp::Number(n) => Value::Number(*n),
        SExp::String(s) => Value::String(s.clone()),
        SExp::Symbol(s) => Value::String(s.clone()),
        SExp::List(items) => {
            // Check for (struct ...)
            if let Some(SExp::Symbol(tag)) = ast.maybe_get(items.first().copied()) {
                if tag == "struct" {
                    return make_struct(ast, items);
                } else if tag == "is-type" {
                    return is_type(ast, items);
                }
            }
            // Otherwise, just return error for now
            Value::Error("Only (:struct ...) supported for now".to_string())
        }
    }
}

pub fn to_json(value: Value) -> serde_json::Value {
    match value {
        Value::Number(n) => serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap()),
        Value::String(s) => serde_json::Value::String(s),
        Value::Bool(b) => serde_json::Value::Bool(b),
        Value::Object(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k, to_json(*v));
            }
            serde_json::Value::Object(obj)
        }
        Value::Error(e) => serde_json::Value::String(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "json", |input, _deps| {
            let ast = crate::ast::AST::parse(input).unwrap();
            let value = eval(&ast, ast.root().unwrap());
            let value = to_json(value);
            serde_json::to_string_pretty(&value).unwrap()
        })
    }
}
