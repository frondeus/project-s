#![allow(unused_variables)]

use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

use crate::{
    ast::ASTS,
    diagnostics::Diagnostics,
    source::{Span, WithSpan},
};

mod flow;

use super::reachability::Reachability;

pub type ID = usize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Use(ID);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Value(ID);

pub trait WithID {
    fn id(self) -> ID;
}

impl WithID for ID {
    fn id(self) -> ID {
        self
    }
}

impl WithID for Use {
    fn id(self) -> ID {
        self.0
    }
}

impl WithID for Value {
    fn id(self) -> ID {
        self.0
    }
}

pub type PolyFunc = Rc<dyn Fn(&mut super::TypeEnv, &ASTS, &mut Diagnostics) -> Value>;

#[derive(Clone)]
pub enum Scheme {
    Monomorphic(Value),
    Polymorphic(PolyFunc),
}

impl Scheme {
    pub fn as_mono(&self) -> Option<Value> {
        match self {
            Self::Monomorphic(value) => Some(*value),
            Self::Polymorphic(_) => None,
        }
    }
}

impl std::fmt::Debug for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Monomorphic(arg0) => f.debug_tuple("Monomorphic").field(arg0).finish(),
            Self::Polymorphic(arg0) => f.debug_tuple("Polymorphic").finish(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum VTypeHead {
    VBool,
    VNumber,
    VString,
    VError,
    VKeyword,
    VTuple {
        items: Vec<Value>,
    },
    VList {
        item: Value,
    },
    VStruct {
        fields: BTreeMap<String, Value>,
        proto: Option<Value>,
    },
    VFunc {
        pattern: Use,
        ret: Value,
    },
    VRef {
        write: Option<Use>,
        read: Option<Value>,
    },
}

impl std::fmt::Display for VTypeHead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VTypeHead::VBool => write!(f, "bool"),
            VTypeHead::VNumber => write!(f, "number"),
            VTypeHead::VString => write!(f, "string"),
            VTypeHead::VError => write!(f, "error"),
            VTypeHead::VKeyword => write!(f, "keyword"),
            VTypeHead::VTuple { items } => write!(f, "tuple"),
            VTypeHead::VList { item } => write!(f, "list"),
            VTypeHead::VStruct { .. } => write!(f, "struct"),
            VTypeHead::VFunc { .. } => write!(f, "function"),
            VTypeHead::VRef { .. } => write!(f, "ref"),
        }
    }
}

impl VTypeHead {
    pub fn ids(&self) -> impl Iterator<Item = ID> {
        let mut ids = Vec::new();
        match self {
            VTypeHead::VBool
            | VTypeHead::VNumber
            | VTypeHead::VString
            | VTypeHead::VError
            | VTypeHead::VKeyword => (),
            VTypeHead::VTuple { items } => {
                ids.extend(items.iter().copied().map(WithID::id));
            }
            VTypeHead::VList { item } => {
                ids.push(item.id());
            }
            VTypeHead::VStruct { fields, proto } => {
                ids.extend(fields.values().copied().map(WithID::id));
                if let Some(proto) = proto {
                    ids.push(proto.id());
                }
            }
            VTypeHead::VFunc { pattern, ret } => {
                ids.push(pattern.id());
                ids.push(ret.id());
            }
            VTypeHead::VRef { write, read } => {
                if let Some(write) = write {
                    ids.push(write.id());
                }
                if let Some(read) = read {
                    ids.push(read.id());
                }
            }
        }
        ids.into_iter()
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum UTypeHead {
    UBool,
    UNumber,
    UString,
    UError,
    UKeyword,
    /// A tuple where each element might have a different type.
    /// Tuple has a fixed number of elements.
    UTuple {
        items: Vec<Use>,
    },
    /// Access to a specific element of a tuple.
    UTupleAccess {
        index: Use,
    },
    UFunc {
        pattern: Value,
        ret: Use,
    },
    /// A list where all elements have the same type.
    /// It might have a fixed number of elements but it doesnt have to.
    UList {
        items: Use,
        min_len: usize,
        max_len: Option<usize>,
    },
    UStruct {
        fields: HashMap<String, Use>,
    },
    UStructAccess {
        field: (String, Use),
    },
    UApplication {
        args: Value,
        ret: Use,
        // In case its object access
        field: (Option<String>, Use),
        // In case its list access
        index: (Option<usize>, Use),
    },
    URef {
        write: Option<Value>,
        read: Option<Use>,
    },
}

impl std::fmt::Display for UTypeHead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UTypeHead::UBool => write!(f, "bool"),
            UTypeHead::UNumber => write!(f, "number"),
            UTypeHead::UError => write!(f, "error"),
            UTypeHead::UString => write!(f, "string"),
            UTypeHead::UKeyword => write!(f, "keyword"),
            UTypeHead::UTuple { items } => write!(f, "tuple"),
            UTypeHead::UTupleAccess { index } => write!(f, "tuple_access"),
            UTypeHead::UList {
                items,
                min_len,
                max_len,
            } => write!(f, "list"),
            UTypeHead::UFunc { pattern, ret } => write!(f, "function"),
            UTypeHead::UStruct { fields } => write!(f, "object"),
            UTypeHead::UStructAccess { field } => write!(f, "object_access"),
            UTypeHead::UApplication { .. } => write!(f, "function"),
            UTypeHead::URef { write, read } => write!(f, "ref"),
        }
    }
}
impl UTypeHead {
    pub fn ids(&self) -> impl Iterator<Item = ID> {
        let mut ids = Vec::new();
        match self {
            UTypeHead::UBool
            | UTypeHead::UNumber
            | UTypeHead::UString
            | UTypeHead::UKeyword
            | UTypeHead::UError => (),
            UTypeHead::UTuple { items } => {
                ids.extend(items.iter().copied().map(WithID::id));
            }
            UTypeHead::UTupleAccess { index } => {
                ids.push(index.id());
            }
            UTypeHead::UList {
                items,
                min_len,
                max_len,
            } => {
                ids.push(items.id());
            }
            UTypeHead::UStruct { fields } => {
                ids.extend(fields.values().copied().map(WithID::id));
            }
            UTypeHead::UStructAccess {
                field: (_, field_use),
            } => {
                ids.push(field_use.id());
            }
            UTypeHead::UFunc { pattern, ret } => {
                ids.push(pattern.id());
                ids.push(ret.id());
            }
            UTypeHead::UApplication {
                args,
                ret,
                field: (_, field_use),
                index: (_, index_use),
            } => {
                ids.push(args.id());
                ids.push(ret.id());
                ids.push(field_use.id());
                ids.push(index_use.id());
            }
            UTypeHead::URef { write, read } => {
                if let Some(write) = write {
                    ids.push(write.id());
                }
                if let Some(read) = read {
                    ids.push(read.id());
                }
            }
        }
        ids.into_iter()
    }
}

#[derive(Clone)]
pub enum TypeNode {
    Var(Span),
    Value(VTypeHead, Span),
    Use(UTypeHead, Span),
}

impl std::fmt::Debug for TypeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Var(_) => write!(f, "Var"),
            Self::Value(arg0, _) => f.debug_tuple("Value").field(arg0).finish(),
            Self::Use(arg0, _) => f.debug_tuple("Use").field(arg0).finish(),
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct TypeCheckerCore {
    r: Reachability,
    types: Vec<TypeNode>,
}

impl TypeCheckerCore {
    fn new_val(&mut self, val_type: VTypeHead, span: impl WithSpan) -> Value {
        // if let Some(i) = self
        //     .types
        //     .iter()
        //     .position(|t| matches!(t, TypeNode::Value(t, _) if t == &val_type))
        // // .position(|t| matches!(t, TypeNode::Value(t, s) if t == &val_type && s == &span))
        // {
        //     return Value(i);
        // }

        let i = self.r.add_node();
        assert!(i == self.types.len());
        self.types.push(TypeNode::Value(val_type, span.span()));
        Value(i)
    }

    fn new_use(&mut self, constraint: UTypeHead, span: impl WithSpan) -> Use {
        // if let Some(i) = self
        //     .types
        //     .iter()
        //     .position(|t| matches!(t, TypeNode::Use(t, _) if t == &constraint))
        // // .position(|t| matches!(t, TypeNode::Use(t, s) if t == &constraint && s == &span))
        // {
        //     return Use(i);
        // }

        let i = self.r.add_node();
        assert!(i == self.types.len());
        self.types.push(TypeNode::Use(constraint, span.span()));
        Use(i)
    }

    pub fn get(&self, id: impl WithID) -> &TypeNode {
        &self.types[id.id()]
    }

    pub fn reachability(&self) -> &Reachability {
        &self.r
    }

    pub fn predecessors(&self, id: impl WithID) -> impl Iterator<Item = (&TypeNode, ID)> {
        self.r.predecessors(id.id()).map(|id| (&self.types[id], id))
    }

    pub fn successors(&self, id: impl WithID) -> impl Iterator<Item = (&TypeNode, ID)> {
        self.r.successors(id.id()).map(|id| (&self.types[id], id))
    }

    pub fn all_linked(&self, id: impl WithID) -> impl Iterator<Item = &TypeNode> {
        self.r.all_linked(id.id()).map(|id| &self.types[id])
    }

    pub fn iter(&self) -> impl Iterator<Item = (ID, &TypeNode)> {
        self.types.iter().enumerate()
    }
}

impl TypeCheckerCore {
    pub fn var(&mut self, span: impl WithSpan) -> (Value, Use) {
        let i = self.r.add_node();
        assert!(i == self.types.len());
        self.types.push(TypeNode::Var(span.span()));
        (Value(i), Use(i))
    }

    pub fn bool(&mut self, span: impl WithSpan) -> Value {
        self.new_val(VTypeHead::VBool, span)
    }

    pub fn bool_use(&mut self, span: impl WithSpan) -> Use {
        self.new_use(UTypeHead::UBool, span)
    }

    pub fn keyword(&mut self, span: impl WithSpan) -> Value {
        self.new_val(VTypeHead::VKeyword, span)
    }

    pub fn keyword_use(&mut self, span: impl WithSpan) -> Use {
        self.new_use(UTypeHead::UKeyword, span)
    }

    pub fn string(&mut self, span: impl WithSpan) -> Value {
        self.new_val(VTypeHead::VString, span)
    }

    pub fn string_use(&mut self, span: impl WithSpan) -> Use {
        self.new_use(UTypeHead::UString, span)
    }

    pub fn number(&mut self, span: impl WithSpan) -> Value {
        self.new_val(VTypeHead::VNumber, span)
    }

    pub fn number_use(&mut self, span: impl WithSpan) -> Use {
        self.new_use(UTypeHead::UNumber, span)
    }

    pub fn error(&mut self, span: impl WithSpan) -> Value {
        self.new_val(VTypeHead::VError, span)
    }

    pub fn error_use(&mut self, span: impl WithSpan) -> Use {
        self.new_use(UTypeHead::UError, span)
    }

    pub fn func(&mut self, pattern: Use, ret: Value, span: impl WithSpan) -> Value {
        self.new_val(VTypeHead::VFunc { pattern, ret }, span)
    }

    pub fn func_use(&mut self, pattern: Value, ret: Use, span: impl WithSpan) -> Use {
        self.new_use(UTypeHead::UFunc { pattern, ret }, span)
    }

    pub fn application_use(
        &mut self,
        args: Vec<Value>,
        ret: Use,
        field: (Option<String>, Use),
        index: (Option<usize>, Use),
        args_span: impl WithSpan,
        span: impl WithSpan,
    ) -> Use {
        let args = self.tuple(args, args_span);
        self.new_use(
            UTypeHead::UApplication {
                args,
                ret,
                field,
                index,
            },
            span,
        )
    }

    pub fn list(&mut self, item: Value, span: impl WithSpan) -> Value {
        self.new_val(VTypeHead::VList { item }, span)
    }

    pub fn tuple(&mut self, items: Vec<Value>, span: impl WithSpan) -> Value {
        self.new_val(VTypeHead::VTuple { items }, span)
    }

    pub fn tuple_use(&mut self, items: Vec<Use>, span: impl WithSpan) -> Use {
        self.new_use(UTypeHead::UTuple { items }, span)
    }
    pub fn tuple_access_use(&mut self, index: Use, span: impl WithSpan) -> Use {
        self.new_use(UTypeHead::UTupleAccess { index }, span)
    }

    pub fn list_use(
        &mut self,
        items: Use,
        min_len: usize,
        max_len: Option<usize>,
        span: impl WithSpan,
    ) -> Use {
        self.new_use(
            UTypeHead::UList {
                items,
                min_len,
                max_len,
            },
            span,
        )
    }

    pub fn obj(
        &mut self,
        fields: Vec<(String, Value)>,
        proto: Option<Value>,
        span: impl WithSpan,
    ) -> Value {
        self.new_val(
            VTypeHead::VStruct {
                fields: fields.into_iter().collect(),
                proto,
            },
            span,
        )
    }

    pub fn obj_use(&mut self, fields: Vec<(String, Use)>, span: impl WithSpan) -> Use {
        self.new_use(
            UTypeHead::UStruct {
                fields: fields.into_iter().collect(),
            },
            span,
        )
    }
    pub fn obj_field_access_use(&mut self, field: (String, Use), span: impl WithSpan) -> Use {
        self.new_use(UTypeHead::UStructAccess { field }, span)
    }

    pub fn reference(
        &mut self,
        write: Option<Use>,
        read: Option<Value>,
        span: impl WithSpan,
    ) -> Value {
        self.new_val(VTypeHead::VRef { write, read }, span)
    }

    pub fn reference_use(
        &mut self,
        write: Option<Value>,
        read: Option<Use>,
        span: impl WithSpan,
    ) -> Use {
        self.new_use(UTypeHead::URef { write, read }, span)
    }
}
