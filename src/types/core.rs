#![allow(unused_variables)]

use std::{collections::HashMap, rc::Rc};

use crate::{ast::ASTS, diagnostics::Diagnostics, source::Span};

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
    VTuple { items: Vec<Value> },
    VList { item: Value },
    VObj { fields: HashMap<String, Value> },
    VFunc { pattern: Use, ret: Value },
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
            VTypeHead::VObj { fields } => write!(f, "object"),
            VTypeHead::VFunc { pattern, ret } => write!(f, "function"),
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
    UApplication {
        args: Value,
        ret: Use,
        // In case its object access
        // field: (String, Use),
        // In case its list access
        index: (Option<usize>, Use),
    },
}

impl std::fmt::Display for UTypeHead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UTypeHead::UBool => write!(f, "bool"),
            UTypeHead::UNumber => write!(f, "number"),
            UTypeHead::UString => write!(f, "string"),
            UTypeHead::UKeyword => write!(f, "keyword"),
            UTypeHead::UTuple { items } => write!(f, "tuple"),
            UTypeHead::UTupleAccess { index } => write!(f, "tuple_access"),
            UTypeHead::UList {
                items,
                min_len,
                max_len,
            } => write!(f, "list"),
            UTypeHead::UObj { fields } => write!(f, "object"),
            UTypeHead::UObjAccess { field } => write!(f, "object_access"),
            UTypeHead::UApplication { .. } => write!(f, "function"),
        }
    }
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
            UTypeHead::UApplication {
                args,
                ret,
                // field: (_, field_use),
                index: (_, index_use),
            } => {
                ids.push(args.id());
                ids.push(ret.id());
                // ids.push(field_use.id());
                ids.push(index_use.id());
            }
        }
        ids.into_iter()
    }
}

#[derive(Clone)]
pub enum TypeNode {
    Var,
    Value(VTypeHead, Span),
    Use(UTypeHead, Span),
}

impl std::fmt::Debug for TypeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Var => write!(f, "Var"),
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
    fn new_val(&mut self, val_type: VTypeHead, span: Span) -> Value {
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
        self.types.push(TypeNode::Value(val_type, span));
        Value(i)
    }

    fn new_use(&mut self, constraint: UTypeHead, span: Span) -> Use {
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
        self.types.push(TypeNode::Use(constraint, span));
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
    pub fn var(&mut self) -> (Value, Use) {
        let i = self.r.add_node();
        assert!(i == self.types.len());
        self.types.push(TypeNode::Var);
        (Value(i), Use(i))
    }

    pub fn bool(&mut self, span: Span) -> Value {
        self.new_val(VTypeHead::VBool, span)
    }

    pub fn bool_use(&mut self, span: Span) -> Use {
        self.new_use(UTypeHead::UBool, span)
    }

    pub fn keyword(&mut self, span: Span) -> Value {
        self.new_val(VTypeHead::VKeyword, span)
    }

    pub fn keyword_use(&mut self, span: Span) -> Use {
        self.new_use(UTypeHead::UKeyword, span)
    }

    pub fn string(&mut self, span: Span) -> Value {
        self.new_val(VTypeHead::VString, span)
    }

    pub fn string_use(&mut self, span: Span) -> Use {
        self.new_use(UTypeHead::UString, span)
    }

    pub fn number(&mut self, span: Span) -> Value {
        self.new_val(VTypeHead::VNumber, span)
    }

    pub fn number_use(&mut self, span: Span) -> Use {
        self.new_use(UTypeHead::UNumber, span)
    }

    pub fn error(&mut self, span: Span) -> Value {
        self.new_val(VTypeHead::VError, span)
    }

    pub fn func(&mut self, pattern: Use, ret: Value, span: Span) -> Value {
        self.new_val(VTypeHead::VFunc { pattern, ret }, span)
    }

    pub fn application_use(
        &mut self,
        args: Vec<Value>,
        ret: Use,
        index: (Option<usize>, Use),
        span: Span,
    ) -> Use {
        let args = self.tuple(args, span.clone());
        self.new_use(UTypeHead::UApplication { args, ret, index }, span)
    }

    pub fn list(&mut self, item: Value, span: Span) -> Value {
        self.new_val(VTypeHead::VList { item }, span)
    }

    pub fn tuple(&mut self, items: Vec<Value>, span: Span) -> Value {
        self.new_val(VTypeHead::VTuple { items }, span)
    }

    pub fn tuple_use(&mut self, items: Vec<Use>, span: Span) -> Use {
        self.new_use(UTypeHead::UTuple { items }, span)
    }
    pub fn tuple_access_use(&mut self, index: Use, span: Span) -> Use {
        self.new_use(UTypeHead::UTupleAccess { index }, span)
    }

    pub fn list_use(
        &mut self,
        items: Use,
        min_len: usize,
        max_len: Option<usize>,
        span: Span,
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

    pub fn obj(&mut self, fields: Vec<(String, Value)>, span: Span) -> Value {
        self.new_val(
            VTypeHead::VObj {
                fields: fields.into_iter().collect(),
            },
            span,
        )
    }

    pub fn obj_use(&mut self, fields: Vec<(String, Use)>, span: Span) -> Use {
        self.new_use(
            UTypeHead::UObj {
                fields: fields.into_iter().collect(),
            },
            span,
        )
    }
    pub fn obj_field_access_use(&mut self, field: (String, Use), span: Span) -> Use {
        self.new_use(UTypeHead::UObjAccess { field }, span)
    }

    #[allow(clippy::result_large_err)]
    pub fn flow(&mut self, lhs: Value, rhs: Use, diagnostics: &mut Diagnostics) {
        let mut pending_edges = vec![(lhs, rhs)];
        let mut type_pairs_to_check = Vec::new();
        while let Some((lhs, rhs)) = pending_edges.pop() {
            self.r.add_edge(lhs.0, rhs.0, &mut type_pairs_to_check);

            // Check if adding that edge resulted in any new type pairs needing to be checked
            while let Some((lhs, rhs)) = type_pairs_to_check.pop() {
                if let TypeNode::Value(lhs_head, lhs_span) = &self.types[lhs] {
                    if let TypeNode::Use(rhs_head, rhs_span) = &self.types[rhs] {
                        Self::check_heads(
                            lhs_head,
                            rhs_head,
                            lhs_span,
                            rhs_span,
                            &mut pending_edges,
                            diagnostics,
                        );
                    }
                }
            }
        }
        assert!(pending_edges.is_empty() && type_pairs_to_check.is_empty());
    }

    #[allow(clippy::result_large_err)]
    fn check_heads(
        lhs: &VTypeHead,
        rhs: &UTypeHead,
        lhs_span: &Span,
        rhs_span: &Span,
        out: &mut Vec<(Value, Use)>,
        diagnostics: &mut Diagnostics,
    ) {
        use UTypeHead::*;
        use VTypeHead::*;

        match (lhs, rhs) {
            (VError, _) => (), // We assume that error type is like ! type in Rust.
            (VBool, UBool) => (),
            (VNumber, UNumber) => (),
            (VString, UString) => (),
            (VKeyword, UKeyword) => (),
            (
                &VList { item },
                &UApplication {
                    args,
                    ret: ret_use,
                    // field: (ref field_name, field_use),
                    index: (index, index_use),
                },
            ) => {
                out.push((item, ret_use));
                out.push((args, index_use));
            }
            (
                VTuple { items },
                &UApplication {
                    args,
                    ret: ret_use,
                    index: (index, index_use),
                },
            ) => {
                out.push((args, index_use));
                let Some(index) = index else {
                    diagnostics.add(
                        lhs_span.clone(),
                        "Expected int literal to access tuple element",
                    );
                    return;
                };
                if index >= items.len() {
                    diagnostics.add(
                        lhs_span.clone(),
                        format!(
                            "Tuple index out of bounds: {} expected {}",
                            index,
                            items.len()
                        ),
                    );
                    return;
                }
                out.push((items[index], ret_use));
            }
            (
                &VFunc { pattern, ret },
                &UApplication {
                    args, ret: ret_use, ..
                },
            ) => {
                out.push((args, pattern));
                out.push((ret, ret_use));
            }
            (
                VObj { fields },
                &UObjAccess {
                    field: (ref field, field_use),
                },
            ) => match fields.get(field) {
                None => {
                    diagnostics.add(rhs_span.clone(), format!("Undefined field: {}", field));
                }
                Some(field_ty) => {
                    out.push((*field_ty, field_use));
                }
            },
            (
                VTuple { items },
                &UList {
                    items: args,
                    min_len,
                    max_len,
                },
            ) => {
                if items.len() < min_len {
                    diagnostics.add(
                        lhs_span.clone(),
                        format!(
                            "Wrong number of arguments: {} expected {}",
                            items.len(),
                            min_len
                        ),
                    );
                }
                if let Some(max_len) = max_len {
                    if items.len() > max_len {
                        diagnostics.add(
                            lhs_span.clone(),
                            format!(
                                "Wrong number of arguments: {} expected {}",
                                items.len(),
                                max_len
                            ),
                        );
                    }
                }
                for item in items {
                    out.push((*item, args));
                }
            }
            (VList { item }, UTuple { items: args }) => {
                // TODO: Length
                for arg in args {
                    out.push((*item, *arg));
                }
            }
            (VTuple { items }, UTuple { items: args }) => {
                if items.len() != args.len() {
                    diagnostics.add(
                        lhs_span.clone(),
                        format!(
                            "Wrong number of arguments: {} expected {}",
                            items.len(),
                            args.len()
                        ),
                    );
                }

                for (item, arg) in items.iter().zip(args) {
                    out.push((*item, *arg));
                }
            }
            _ => {
                diagnostics
                    .add(rhs_span.clone(), "Incompatible types")
                    .add_extra(format!("Expected {rhs}"), Some(rhs_span.clone()))
                    .add_extra(format!("But got {lhs}"), Some(lhs_span.clone()));
            }
        }
    }
}
