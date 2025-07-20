use std::{cell::RefCell, collections::BTreeMap, ops::Deref, rc::Rc};

use crate::{
    ast::{AST, ASTS, SExp, SExpId},
    patterns::Pattern,
    source::{Span, Spanned},
};

use super::Runtime;

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Object(BTreeMap<String, (Value, Option<SExpId>)>),
    List(Vec<Value>),
    SExp(SExpId),
    Macro(Macro),
    Function(Function),
    Thunk(Thunk),
    Constructor(Constructor),
    /// Mutable reference that lives on a heap
    Ref(Ref),
    /// For error handling
    Error(String),
}

#[derive(Clone, Debug)]
pub struct Ref(Rc<RefCell<Value>>);

impl Deref for Ref {
    type Target = Rc<RefCell<Value>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub enum Macro {
    Lisp {
        pattern: Pattern,
        body: Spanned<SExpId>,
    },
    Rust {
        body: NativeMacro,
    },
}

impl std::fmt::Debug for Macro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lisp { pattern, body } => f
                .debug_struct("LispMacro")
                .field("pattern", pattern)
                .field("body", body)
                .finish(),
            Self::Rust { .. } => f.debug_struct("RustMacro").finish(),
        }
    }
}

pub type NativeMacro = Rc<dyn Fn(&mut ASTS, Span, Vec<Spanned<SExpId>>) -> Spanned<SExpId>>;
pub type NativeFn = Rc<dyn Fn(&mut Runtime, Vec<Value>) -> Value>;

#[derive(Clone)]
pub enum Function {
    Lisp {
        // signature: Vec<String>,
        pattern: Pattern,
        captured: BTreeMap<String, Value>,
        body: SExpId,
    },
    Rust {
        body: NativeFn,
    },
}

impl Function {
    pub fn body(&self) -> Option<SExpId> {
        match self {
            Self::Lisp { body, .. } => Some(*body),
            Self::Rust { .. } => None,
        }
    }
}

impl<F> From<F> for Function
where
    F: Fn(&mut Runtime, Vec<Value>) -> Value + 'static,
{
    fn from(f: F) -> Self {
        Self::Rust { body: Rc::new(f) }
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lisp { .. } => f
                .debug_struct("LispFn")
                // .field("signature", signature)
                // .field("captured", captured)
                // .field("body", body)
                .finish(),
            Self::Rust { .. } => f.debug_struct("RustFn").finish(),
        }
    }
}

#[derive(Clone)]
pub struct Thunk {
    pub(crate) inner: Rc<RefCell<InnerThunk>>,
}

impl Thunk {
    pub fn new(body: SExpId) -> Self {
        Self {
            inner: Rc::new(RefCell::new(InnerThunk::ToEvaluate {
                captured: BTreeMap::new(),
                body,
            })),
        }
    }

    pub fn new_for_let() -> Self {
        Self {
            inner: Rc::new(RefCell::new(InnerThunk::Evaluating)),
        }
    }
}

impl std::fmt::Debug for Thunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &*self.inner.borrow() {
            InnerThunk::Evaluated(value) => f.debug_tuple("Thunk/Evaluated").field(value).finish(),
            InnerThunk::Evaluating => f.debug_struct("Thunk/Evaluating").finish(),
            InnerThunk::ToEvaluate { .. } => f.debug_struct("Thunk/ToEvaluate").finish(),
        }
    }
}

#[derive(Debug)]
pub enum InnerThunk {
    Evaluated(Value),
    Evaluating,
    ToEvaluate {
        captured: BTreeMap<String, Value>,
        body: SExpId,
    },
}

/// Creates a new object every time its called.
#[derive(Clone, Debug)]
pub struct Constructor {
    pub(crate) constructor: Function,
}

impl Value {
    pub fn error(s: impl ToString) -> Self {
        Value::Error(s.to_string())
    }
    pub fn ok(self) -> Result<Self, String> {
        if let Value::Error(e) = self {
            Err(e)
        } else {
            Ok(self)
        }
    }

    pub fn ref_(val: Value) -> Self {
        Value::Ref(Ref(Rc::new(RefCell::new(val))))
    }

    pub fn deref(&self) -> Value {
        match self {
            Value::Ref(rc) => rc.0.borrow().clone(),
            _ => self.clone(),
        }
    }

    pub fn is_constructor(&self) -> bool {
        matches!(self, Value::Constructor(_))
    }

    pub fn is_lazy(&self, include_constructor: bool) -> bool {
        if include_constructor && matches!(self, Value::Constructor(_)) {
            return true;
        }
        matches!(self, Value::Thunk(_) | Value::Ref(_))
    }

    pub fn as_sexp(&self) -> Option<&SExpId> {
        match self {
            Value::SExp(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_thunk(&self) -> Option<&Thunk> {
        match self {
            Value::Thunk(thunk) => Some(thunk),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> Option<&Ref> {
        match self {
            Value::Ref(rc) => Some(rc),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut BTreeMap<String, (Value, Option<SExpId>)>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, (Value, Option<SExpId>)>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<Value>> {
        match self {
            Value::List(list) => Some(list),
            _ => None,
        }
    }

    pub fn as_function(&self) -> Option<&Function> {
        match self {
            Value::Function(function) => Some(function),
            _ => None,
        }
    }

    pub fn into_object(self) -> Option<BTreeMap<String, (Value, Option<SExpId>)>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn eager(self, rt: &mut Runtime, include_constructor: bool) -> Self {
        rt.to_eager(self, include_constructor)
    }

    pub fn eager_rec(mut self, rt: &mut Runtime, include_constructor: bool) -> Self {
        while self.is_lazy(include_constructor) {
            self = rt.to_eager(self, include_constructor);
        }
        self
    }

    pub fn to_sexp(&self, target: &mut AST, span: Span) -> SExpId {
        match self {
            Value::SExp(sexp_id) => *sexp_id,
            Value::Number(n) => target.add_node(SExp::Number(*n), span, None),
            Value::String(s) => target.add_node(SExp::String(s.clone()), span, None),
            Value::Bool(b) => target.add_node(SExp::Bool(*b), span, None),
            Value::Error(err) => {
                tracing::error!("Error: {err}");
                target.add_node(SExp::Error, span, None)
            }
            Value::List(list) => {
                if list.iter().all(|v| matches!(v, Value::SExp(_))) {
                    let list = list.iter().filter_map(|v| v.as_sexp()).copied().collect();
                    target.add_node(SExp::List(list), span, None)
                } else {
                    todo!("Could not convert List to SExp: {:?}", list)
                }
            }
            Value::Ref(rc) => {
                todo!("Could not convert Ref to SExp: {:?}", rc)
            }
            Value::Object(_btree_map) => {
                todo!("Could not convert Object to SExp: {:?}", self)
            }
            Value::Macro(macro_) => {
                todo!("Could not convert Macro to SExp: {:?}", macro_)
            }
            Value::Function(function) => {
                todo!("Could not convert Function to SExp: {:?}", function)
            }
            Value::Thunk(thunk) => {
                todo!("Could not convert Thunk to SExp: {:?}", thunk)
            }
            Value::Constructor(constructor) => {
                todo!("Could not convert Constructor to SExp: {:?}", constructor)
            }
        }
    }
}
