use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use crate::ast::{AST, SExp, SExpId};

use super::Runtime;

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Object(BTreeMap<String, Value>),
    Symbol(String),
    SExp(SExpId),
    Macro(Macro),
    Function(Function),
    Closure(Closure),
    Thunk(Thunk),
    /// For error handling
    Error(String),
}

#[derive(Clone)]
pub enum Macro {
    Lisp {
        signature: Vec<String>,
        body: SExpId,
    },
    Rust {
        body: NativeMacro,
    },
}

impl std::fmt::Debug for Macro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lisp { signature, body } => f
                .debug_struct("LispMacro")
                .field("signature", signature)
                .field("body", body)
                .finish(),
            Self::Rust { .. } => f.debug_struct("RustMacro").finish(),
        }
    }
}

pub type NativeMacro = Rc<dyn Fn(&mut Runtime, Vec<SExpId>) -> SExpId>;
pub type NativeFn = Rc<dyn Fn(&mut Runtime, Vec<Value>) -> Value>;

#[derive(Clone)]
pub enum Function {
    Lisp {
        signature: Vec<String>,
        body: SExpId,
    },
    Rust {
        body: NativeFn,
    },
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lisp { signature, body } => f
                .debug_struct("LispFn")
                .field("signature", signature)
                .field("body", body)
                .finish(),
            Self::Rust { .. } => f.debug_struct("RustFn").finish(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Closure {
    pub(crate) signature: Vec<String>,
    pub(crate) captured: BTreeMap<String, Value>,
    pub(crate) body: SExpId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Thunk {
    pub(crate) inner: Rc<RefCell<InnerThunk>>,
}

#[derive(Debug)]
pub enum InnerThunk {
    Evaluated(Value),
    ToEvaluate {
        captured: BTreeMap<String, Value>,
        body: SExpId,
    },
}

impl Value {
    pub fn ok(self) -> Result<Self, String> {
        if let Value::Error(e) = self {
            Err(e)
        } else {
            Ok(self)
        }
    }

    pub fn as_sexp(&self) -> Option<&SExpId> {
        match self {
            Value::SExp(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            Value::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn into_object(self) -> Option<BTreeMap<String, Value>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn to_sexp(&self, target: &mut AST) -> SExpId {
        match self {
            Value::Number(n) => target.add_node(SExp::Number(*n)),
            Value::String(s) => target.add_node(SExp::String(s.clone())),
            Value::Bool(b) => target.add_node(SExp::Bool(*b)),
            Value::Symbol(s) => target.add_node(SExp::Symbol(s.clone())),
            Value::Object(_btree_map) => {
                todo!("Could not convert Object to SExp: {:?}", self)
            }
            Value::Macro(macro_) => {
                todo!("Could not convert Macro to SExp: {:?}", macro_)
            }
            Value::Function(function) => {
                todo!("Could not convert Function to SExp: {:?}", function)
            }
            Value::Closure(closure) => {
                todo!("Could not convert Closure to SExp: {:?}", closure)
            }
            Value::Thunk(thunk) => {
                todo!("Could not convert Thunk to SExp: {:?}", thunk)
            }
            Value::SExp(sexp_id) => *sexp_id,
            Value::Error(err) => {
                println!("Error: {err}");
                target.add_node(SExp::Error)
            }
        }
    }
}
