use std::collections::{BTreeMap, HashMap};

pub enum Value {
    String(String),
    Object(BTreeMap<String, Box<Value>>),
    /// For error handling 
    Error(String)
}

pub fn walk(ast: &crate::parser::SExp) -> Value {
    use crate::parser::SExp;
    match ast {
        SExp::Symbol(s) => {
            if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                // Remove the outer quotes for string literals
                Value::String(s[1..s.len()-1].to_string())
            } else {
                Value::String(s.clone())
            }
        },
        SExp::List(items) => {
            // Check for (:struct ...)
            if let Some(SExp::Symbol(tag)) = items.get(0) {
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

fn to_json(value: Value) -> String {
    match value {
        Value::String(s) => format!("\"{}\"", escape_json_string(&s)),
        Value::Object(map) => {
            let mut out = String::from("{\n");
            let mut first = true;
            for (k, v) in map {
                if !first {
                    out.push_str(",\n");
                }
                first = false;
                out.push_str("    \"");
                out.push_str(&escape_json_string(&k));
                out.push_str("\": ");
                out.push_str(&to_json(*v));
            }
            out.push_str("\n}");
            out
        }
        Value::Error(e) => e,
    }
}

fn escape_json_string(s: &str) -> String {
    s.replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "json", |input, _deps| {
            let ast = crate::parser::parse(input).unwrap();
            let value = walk(&ast);
            to_json(value)
        })
    }
}