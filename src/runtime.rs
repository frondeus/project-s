use std::collections::BTreeMap;

use itertools::Itertools;

use crate::{
    ast::{AST, SExp, SExpId},
    types::{Type, TypeEnv},
};

#[derive(Debug)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Object(BTreeMap<String, Box<Value>>),
    SExp(SExpId),
    DynamicSExp(AST),
    /// For error handling
    Error(String),
}

impl Value {
    fn as_sexp<'a>(&'a self, ast: &'a AST) -> Option<(&'a SExp, &'a AST)> {
        match self {
            Value::SExp(id) => Some((ast.get(*id), ast)),
            Value::DynamicSExp(ast) => Some((ast.root().unwrap(), ast)),
            _ => None,
        }
    }

    fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    fn to_sexp(&self, source: &AST, target: &mut AST) -> SExpId {
        match self {
            Value::Number(n) => target.add_node(SExp::Number(*n)),
            Value::String(s) => target.add_node(SExp::String(s.clone())),
            Value::Bool(_b) => todo!(),
            Value::Object(_btree_map) => todo!(),
            Value::SExp(sexp_id) => copy_sexp(source, target, sexp_id),
            Value::DynamicSExp(_) => todo!(),
            Value::Error(_) => todo!(),
        }
    }
}

fn make_struct(ast: &AST, items: &[SExpId]) -> Value {
    let Some(sexp) = items.first() else {
        return Value::Error("Expected SExpression. Found None".to_string());
    };
    let sexp = ast.get(*sexp);
    let evaled = eval(ast, sexp);
    let Some((sexp, ast)) = evaled.as_sexp(ast) else {
        return Value::Error(format!("Expected SExpression. Found {evaled:?}",));
    };
    let Some(items) = sexp.as_list() else {
        return Value::Error("Expected list".to_string());
    };

    let mut map = BTreeMap::new();
    for (key, value) in items.iter().map(|id| ast.get(*id)).tuples() {
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
    let Some(sexp) = items.first() else {
        return Value::Error("Expected SExpression".to_string());
    };
    let mut env = TypeEnv::default();
    let infered = env.infer(ast, *sexp);
    let result = env.get(infered);

    let Some(ty) = items.get(1) else {
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

fn quote(ast: &AST, id: &SExpId) -> Value {
    let sexp = ast.get(*id);
    match sexp {
        SExp::Number(n) => Value::Number(*n),
        SExp::String(s) => Value::String(s.clone()),
        SExp::Symbol(s) => Value::String(s.clone()),
        SExp::Error => Value::Error("AST Error".to_string()),
        SExp::List(_) => Value::SExp(*id),
    }
}

fn quasiquote(ast: &AST, id: &SExpId) -> Value {
    let sexp = ast.get(*id);
    match sexp {
        SExp::Number(n) => Value::Number(*n),
        SExp::String(s) => Value::String(s.clone()),
        SExp::Symbol(s) => Value::String(s.clone()),
        SExp::Error => Value::Error("AST Error".to_string()),
        SExp::List(_) => {
            let mut new_ast = AST::default();
            traverse_unquote(ast, &mut new_ast, id);
            Value::DynamicSExp(new_ast)
        }
    }
}

fn copy_sexp(source: &AST, target: &mut AST, id: &SExpId) -> SExpId {
    let sexp = source.get(*id);
    let sexp = sexp.clone();
    target.add_node(sexp)
}

fn traverse_unquote(ast: &AST, new_ast: &mut AST, id: &SExpId) -> SExpId {
    let sexp = ast.get(*id);
    match sexp {
        SExp::List(items) => {
            let Some(first) = items.first() else {
                return copy_sexp(ast, new_ast, id);
            };
            if is_unquote(ast, first) {
                let Some(first) = items.get(1) else {
                    todo!("Should return error somehow");
                };
                let evaled = eval(ast, ast.get(*first));
                evaled.to_sexp(ast, new_ast)
            } else {
                let parent = new_ast.reserve();
                let mut result = Vec::new();
                for item in items {
                    result.push(traverse_unquote(ast, new_ast, item));
                }
                if &result == items {
                    return copy_sexp(ast, new_ast, id);
                }
                let new_list = SExp::List(result);
                new_ast.set(parent, new_list);
                parent
            }
        }
        _ => copy_sexp(ast, new_ast, id),
    }
}

fn is_unquote(ast: &AST, id: &SExpId) -> bool {
    let sexp = ast.get(*id);
    match sexp {
        SExp::Symbol(s) => s == "unquote",
        _ => false,
    }
}

fn add(ast: &AST, items: &[SExpId]) -> Value {
    let mut sum = 0.0;
    for item in items {
        let value = eval(ast, ast.get(*item));
        if let Some(n) = value.as_number() {
            sum += n;
        } else {
            return Value::Error("Expected number".to_string());
        }
    }
    Value::Number(sum)
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
                    return make_struct(ast, &items[1..]);
                } else if tag == "is-type" {
                    return is_type(ast, &items[1..]);
                } else if tag == "quote" {
                    let Some(item) = items.get(1) else {
                        return Value::Error("Expected item after quote".to_string());
                    };
                    return quote(ast, item);
                } else if tag == "+" {
                    return add(ast, &items[1..]);
                } else if tag == "quasiquote" {
                    let Some(item) = items.get(1) else {
                        return Value::Error("Expected item after quasiquote".to_string());
                    };
                    return quasiquote(ast, item);
                }
            }
            // Otherwise, just return error for now
            Value::Error("Only (struct, is-type, quote, ...) supported for now".to_string())
        }
    }
}

pub fn to_json(ast: &AST, value: Value) -> serde_json::Value {
    match value {
        Value::Number(n) => serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap()),
        Value::String(s) => serde_json::Value::String(s),
        Value::Bool(b) => serde_json::Value::Bool(b),
        Value::Object(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k, to_json(ast, *v));
            }
            serde_json::Value::Object(obj)
        }
        Value::Error(e) => serde_json::Value::String(e),
        Value::SExp(id) => {
            let sexp = ast.get(id);
            let sexp = sexp.fmt(ast).to_string();
            serde_json::Value::String(sexp)
        }
        Value::DynamicSExp(ast) => {
            let root = ast.root().unwrap();
            let sexp = root.fmt(&ast).to_string();
            serde_json::Value::String(sexp)
        }
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
            let value = to_json(&ast, value);
            serde_json::to_string_pretty(&value).unwrap()
        })
    }
}
