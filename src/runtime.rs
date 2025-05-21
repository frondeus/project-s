use std::collections::BTreeMap;

use crate::ast::AST;

pub enum Value {
    Number(f64),
    String(String),
    Object(BTreeMap<String, Box<Value>>),
    /// For error handling
    Error(String),
}

pub fn walk(ast: &AST, sexp: &crate::ast::SExp) -> Value {
    use crate::ast::SExp;
    match sexp {
        SExp::Error => Value::Error("AST Error".to_string()),
        SExp::Number(n) => Value::Number(*n),
        SExp::String(s) => Value::String(s.clone()),
        SExp::Symbol(s) => Value::String(s.clone()),
        SExp::List(items) => {
            // Check for (:struct ...)
            if let Some(SExp::Symbol(tag)) = ast.maybe_get(items.first().copied()) {
                if tag == ":struct" {
                    let mut map = BTreeMap::new();
                    let mut i = 1;
                    while i + 1 < items.len() {
                        if let SExp::Symbol(key) = ast.get(items[i]) {
                            let key = key.trim_start_matches(':');
                            let value = walk(ast, ast.get(items[i + 1]));
                            map.insert(key.to_string(), Box::new(value));
                            i += 2;
                        } else {
                            return Value::Error("Expected symbol as struct key".to_string());
                        }
                    }
                    return Value::Object(map);
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
            let value = walk(&ast, ast.root().unwrap());
            let value = to_json(value);
            serde_json::to_string_pretty(&value).unwrap()
        })
    }
}
