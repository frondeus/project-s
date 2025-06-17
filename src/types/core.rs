#![allow(unused_variables)]

use std::{collections::HashMap, rc::Rc};

use thiserror::Error;

use crate::ast::ASTS;

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

pub type PolyFunc = Rc<dyn Fn(&mut super::TypeEnv, &ASTS) -> Result<Value>>;

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
    VList { items: Vec<Value> },
    VObj { fields: HashMap<String, Value> },
    VFunc { pattern: Use, ret: Value },
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
            VTypeHead::VList { items } => {
                ids.extend(items.iter().copied().map(WithID::id));
            }
            VTypeHead::VObj { fields } => {
                ids.extend(fields.values().copied().map(WithID::id));
            }
            VTypeHead::VFunc { pattern, ret } => {
                ids.push(pattern.id());
                ids.push(ret.id());
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
    /// A list where all elements have the same type.
    /// It might have a fixed number of elements but it doesnt have to.
    UList {
        items: Use,
        min_len: usize,
        max_len: Option<usize>,
    },
    UObj {
        fields: HashMap<String, Use>,
    },
    UObjAccess {
        field: (String, Use),
    },
    UFunc {
        args: Value,
        ret: Use,
    },
}

impl UTypeHead {
    pub fn ids(&self) -> impl Iterator<Item = ID> {
        let mut ids = Vec::new();
        match self {
            UTypeHead::UBool | UTypeHead::UNumber | UTypeHead::UString | UTypeHead::UKeyword => (),
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
            UTypeHead::UObj { fields } => {
                ids.extend(fields.values().copied().map(WithID::id));
            }
            UTypeHead::UObjAccess {
                field: (_, field_use),
            } => {
                ids.push(field_use.id());
            }
            UTypeHead::UFunc { args, ret } => {
                ids.push(args.id());
                ids.push(ret.id());
            }
        }
        ids.into_iter()
    }
}

#[derive(Debug, Clone)]
pub enum TypeNode {
    Var,
    Value(VTypeHead),
    Use(UTypeHead),
}

#[derive(Debug, Error)]
pub enum TypeError {
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),

    #[error("Unreadable pattern: {0}")]
    UnreadablePattern(String),

    #[error("Undefined field: {0}")]
    UndefinedField(String),

    #[error("Incompatible types: {0:?} and {1:?}")]
    IncompatibleTypes(VTypeHead, UTypeHead),

    #[error("Wrong number of arguments: {0}. Expected {1}.")]
    WrongNumberOfArguments(usize, usize),
}

pub type Result<T, E = TypeError> = std::result::Result<T, E>;

#[derive(Default, Debug)]
pub(crate) struct TypeCheckerCore {
    r: Reachability,
    types: Vec<TypeNode>,
}

impl TypeCheckerCore {
    fn new_val(&mut self, val_type: VTypeHead) -> Value {
        if let Some(i) = self
            .types
            .iter()
            .position(|t| matches!(t, TypeNode::Value(t) if t == &val_type))
        {
            return Value(i);
        }

        let i = self.r.add_node();
        assert!(i == self.types.len());
        self.types.push(TypeNode::Value(val_type));
        Value(i)
    }

    fn new_use(&mut self, constraint: UTypeHead) -> Use {
        if let Some(i) = self
            .types
            .iter()
            .position(|t| matches!(t, TypeNode::Use(t) if t == &constraint))
        {
            return Use(i);
        }

        let i = self.r.add_node();
        assert!(i == self.types.len());
        self.types.push(TypeNode::Use(constraint));
        Use(i)
    }

    pub fn get(&self, id: impl WithID) -> &TypeNode {
        &self.types[id.id()]
    }

    pub fn reachability(&self) -> &Reachability {
        &self.r
    }

    pub fn predecessors(&self, id: impl WithID) -> impl Iterator<Item = &TypeNode> {
        self.r.predecessors(id.id()).map(|id| &self.types[id])
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
    pub fn var(&mut self) -> (Value, Use) {
        let i = self.r.add_node();
        assert!(i == self.types.len());
        self.types.push(TypeNode::Var);
        (Value(i), Use(i))
    }

    pub fn bool(&mut self) -> Value {
        self.new_val(VTypeHead::VBool)
    }

    pub fn bool_use(&mut self) -> Use {
        self.new_use(UTypeHead::UBool)
    }

    pub fn keyword(&mut self) -> Value {
        self.new_val(VTypeHead::VKeyword)
    }

    pub fn keyword_use(&mut self) -> Use {
        self.new_use(UTypeHead::UKeyword)
    }

    pub fn string(&mut self) -> Value {
        self.new_val(VTypeHead::VString)
    }

    pub fn string_use(&mut self) -> Use {
        self.new_use(UTypeHead::UString)
    }

    pub fn number(&mut self) -> Value {
        self.new_val(VTypeHead::VNumber)
    }

    pub fn number_use(&mut self) -> Use {
        self.new_use(UTypeHead::UNumber)
    }

    pub fn error(&mut self) -> Value {
        self.new_val(VTypeHead::VError)
    }

    pub fn func(&mut self, pattern: Use, ret: Value) -> Value {
        self.new_val(VTypeHead::VFunc { pattern, ret })
    }

    pub fn func_use(&mut self, args: Vec<Value>, ret: Use) -> Use {
        let args = self.list(args);
        self.new_use(UTypeHead::UFunc { args, ret })
    }

    pub fn list(&mut self, items: Vec<Value>) -> Value {
        self.new_val(VTypeHead::VList { items })
    }

    pub fn tuple_use(&mut self, items: Vec<Use>) -> Use {
        self.new_use(UTypeHead::UTuple { items })
    }
    pub fn tuple_access_use(&mut self, index: Use) -> Use {
        self.new_use(UTypeHead::UTupleAccess { index })
    }

    pub fn list_use(&mut self, items: Use, min_len: usize, max_len: Option<usize>) -> Use {
        self.new_use(UTypeHead::UList {
            items,
            min_len,
            max_len,
        })
    }

    pub fn obj(&mut self, fields: Vec<(String, Value)>) -> Value {
        self.new_val(VTypeHead::VObj {
            fields: fields.into_iter().collect(),
        })
    }
    pub fn obj_use(&mut self, fields: Vec<(String, Use)>) -> Use {
        self.new_use(UTypeHead::UObj {
            fields: fields.into_iter().collect(),
        })
    }
    pub fn obj_field_access_use(&mut self, field: (String, Use)) -> Use {
        self.new_use(UTypeHead::UObjAccess { field })
    }

    pub fn flow(&mut self, lhs: Value, rhs: Use) -> Result<()> {
        let mut pending_edges = vec![(lhs, rhs)];
        let mut type_pairs_to_check = Vec::new();
        while let Some((lhs, rhs)) = pending_edges.pop() {
            self.r.add_edge(lhs.0, rhs.0, &mut type_pairs_to_check);

            // Check if adding that edge resulted in any new type pairs needing to be checked
            while let Some((lhs, rhs)) = type_pairs_to_check.pop() {
                if let TypeNode::Value(lhs_head) = &self.types[lhs] {
                    if let TypeNode::Use(rhs_head) = &self.types[rhs] {
                        Self::check_heads(lhs_head, rhs_head, &mut pending_edges)?;
                    }
                }
            }
        }
        assert!(pending_edges.is_empty() && type_pairs_to_check.is_empty());
        Ok(())
    }

    fn check_heads(lhs: &VTypeHead, rhs: &UTypeHead, out: &mut Vec<(Value, Use)>) -> Result<()> {
        use UTypeHead::*;
        use VTypeHead::*;

        match (lhs, rhs) {
            (VError, _) => Ok(()), // We assume that error type is like ! type in Rust.
            (VBool, UBool) => Ok(()),
            (VNumber, UNumber) => Ok(()),
            (VString, UString) => Ok(()),
            (VKeyword, UKeyword) => Ok(()),
            (&VFunc { pattern, ret }, &UFunc { args, ret: ret_use }) => {
                out.push((args, pattern));
                out.push((ret, ret_use));
                Ok(())
            }
            (
                VObj { fields },
                &UObjAccess {
                    field: (ref field, field_use),
                },
            ) => match fields.get(field) {
                None => Err(TypeError::UndefinedField(field.to_owned())),
                Some(field_ty) => {
                    out.push((*field_ty, field_use));
                    Ok(())
                }
            },
            (
                VList { items },
                &UList {
                    items: args,
                    min_len,
                    max_len,
                },
            ) => {
                if items.len() < min_len {
                    return Err(TypeError::WrongNumberOfArguments(min_len, items.len()));
                }
                if let Some(max_len) = max_len {
                    if items.len() > max_len {
                        return Err(TypeError::WrongNumberOfArguments(max_len, items.len()));
                    }
                }
                for item in items {
                    out.push((*item, args));
                }
                Ok(())
            }
            (VList { items }, UTuple { items: args }) => {
                if items.len() != args.len() {
                    return Err(TypeError::WrongNumberOfArguments(args.len(), items.len()));
                }

                for (item, arg) in items.iter().zip(args) {
                    out.push((*item, *arg));
                }
                Ok(())
            }
            _ => Err(TypeError::IncompatibleTypes(lhs.clone(), rhs.clone())),
        }
    }
}
