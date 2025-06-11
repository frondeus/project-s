use super::{Runtime, value::Value};

impl Runtime {
    pub fn to_json(&mut self, value: Value, eager: bool) -> serde_json::Value {
        self.to_json_inner(value, eager, 5)
    }
    pub fn to_json_inner(
        &mut self,
        mut value: Value,
        eager: bool,
        depth: usize,
    ) -> serde_json::Value {
        tracing::trace!("To json");
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
