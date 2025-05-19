use std::collections::BTreeMap;

pub enum Value {
    String(String),
    Object(BTreeMap<String, Box<Value>>),
    /// For error handling
    Error(String),
}

pub fn walk(ast: &crate::parser::SExp) -> Value {
    use crate::parser::SExp;
    match ast {
        SExp::Symbol(s) => {
            if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                // Remove the outer quotes for string literals
                Value::String(s[1..s.len() - 1].to_string())
            } else {
                Value::String(s.clone())
            }
        }
        SExp::List(items) => {
            // Check for (:struct ...)
            if let Some(SExp::Symbol(tag)) = items.first() {
                if tag == ":struct" {
                    let mut map = BTreeMap::new();
                    let mut i = 1;
                    while i + 1 < items.len() {
                        if let SExp::Symbol(key) = &items[i] {
                            let key = key.trim_start_matches(':');
                            let value = walk(&items[i + 1]);
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
            let ast = crate::parser::parse(input).unwrap();
            let value = walk(&ast);
            let value = to_json(value);
            serde_json::to_string_pretty(&value).unwrap()
        })
    }
}
