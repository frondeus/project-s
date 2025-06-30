use std::collections::HashMap;

use crate::source::Span;

use super::core;
use super::core::Literal;
use super::core::TypeNode;
use super::core::WithID;

#[derive(Debug, PartialEq, Clone, Copy, PartialOrd, Ord, Eq)]
pub struct CanonId(core::ID);

#[derive(Debug, PartialEq, Clone)]
pub enum Canonical {
    /// A type that is not yet implemented. Better than panic
    Todo(String, Option<Span>),

    /// "_" type
    Wildcard(Option<Span>),
    /// Any type. If there is integer it means it is generic type
    /// It allows us to express polymorphic functions like (T0) -> T0 where we
    /// have guarantee of "any type in the input is going to be used in the output"
    Any(Option<usize>, Option<Span>),

    Literal(Literal, Option<Span>),

    /// A new representation of recursive types.
    As(usize, CanonId, Option<Span>),

    Or(Vec<CanonId>, Option<Span>),
    And(Vec<CanonId>, Option<Span>),
    Error(Option<Span>),
    Primitive(String, Option<Span>),
    Tuple {
        items: Vec<CanonId>,
        span: Option<Span>,
    },
    List {
        item: CanonId,
        span: Option<Span>,
    },
    Func {
        pattern: CanonId,
        ret: CanonId,
        span: Option<Span>,
    },
    Record {
        fields: Vec<(String, CanonId)>,
        proto: Option<CanonId>,
        span: Option<Span>,
    },
    Reference {
        read: Option<CanonId>,
        write: Option<CanonId>,
        span: Option<Span>,
    },
    Applicable {
        args: Vec<CanonId>,
        ret: CanonId,
        span: Option<Span>,
    }, // Applicable {

       // }
}

impl Canonical {
    pub fn span(&self) -> Option<Span> {
        match self {
            Canonical::Todo(_, span)
            | Canonical::Wildcard(span)
            | Canonical::Any(_, span)
            | Canonical::Literal(_, span)
            | Canonical::As(_, _, span)
            | Canonical::Or(_, span)
            | Canonical::And(_, span)
            | Canonical::Error(span)
            | Canonical::Primitive(_, span)
            | Canonical::Tuple { span, .. }
            | Canonical::List { span, .. }
            | Canonical::Func { span, .. }
            | Canonical::Record { span, .. }
            | Canonical::Reference { span, .. }
            | Canonical::Applicable { span, .. } => *span,
        }
    }

    #[cfg(test)]
    fn ids(&self) -> impl Iterator<Item = CanonId> {
        match self {
            Canonical::Todo(_, _)
            | Canonical::Any(_, _)
            | Canonical::Wildcard(_)
            | Canonical::Error(_)
            | Canonical::Primitive(_, _)
            | Canonical::Literal(_, _) => vec![].into_iter(),
            Canonical::As(_, canon_id, _) => vec![*canon_id].into_iter(),
            Canonical::Or(canon_ids, _) => canon_ids.clone().into_iter(),
            Canonical::And(canon_ids, _) => canon_ids.clone().into_iter(),
            Canonical::Tuple { items, span: _ } => items.clone().into_iter(),
            Canonical::List { item, span: _ } => vec![*item].into_iter(),
            Canonical::Func {
                pattern,
                ret,
                span: _,
            } => vec![*pattern, *ret].into_iter(),
            Canonical::Record {
                fields,
                proto,
                span: _,
            } => fields
                .iter()
                .map(|(_, id)| *id)
                .chain(*proto)
                .collect::<Vec<_>>()
                .into_iter(),
            Canonical::Reference {
                read,
                write,
                span: _,
            } => {
                let mut ids = Vec::new();
                if let Some(read) = read {
                    ids.push(*read);
                }
                if let Some(write) = write {
                    ids.push(*write);
                }
                ids.into_iter()
            }
            Canonical::Applicable { args, ret, span: _ } => {
                let mut ids = Vec::new();
                ids.extend(args.iter().copied());
                ids.push(*ret);
                ids.into_iter()
            }
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct CanonicalBuilder {
    canonical: Vec<Canonical>,
}

impl CanonicalBuilder {
    pub fn add(&mut self, canon: Canonical) -> CanonId {
        if let Some(i) = self.canonical.iter().position(|c| c == &canon) {
            CanonId(i)
        } else {
            let i = self.canonical.len();
            self.canonical.push(canon);
            CanonId(i)
        }
    }
    pub fn get(&self, id: CanonId) -> &Canonical {
        &self.canonical[id.0]
    }
    pub fn finish(self) -> Canonicalized {
        Canonicalized {
            canonical: self.canonical,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Canonicalizer {
    visited: Vec<core::ID>,
    recursive: HashMap<core::ID, usize>,
    builder: CanonicalBuilder,
}

pub struct Canonicalized {
    canonical: Vec<Canonical>,
}

impl Canonicalized {
    pub fn get(&self, id: CanonId) -> &Canonical {
        &self.canonical[id.0]
    }

    #[cfg(test)]
    pub fn dot(&self, root: CanonId) -> String {
        use std::fmt::Write;
        let mut buffer = String::new();
        writeln!(buffer, "digraph Canonical {{").unwrap();
        for (id, canon) in self.canonical.iter().enumerate() {
            writeln!(buffer, "C{id} [label=\"{id}: {canon:?}\"];").unwrap();
        }
        writeln!(buffer, "ROOT -> C{}", root.0).unwrap();
        for (id, ids) in self.canonical.iter().enumerate() {
            for to in ids.ids() {
                writeln!(buffer, "C{id} -> C{};", to.0).unwrap();
            }
        }
        writeln!(buffer, "}}").unwrap();
        buffer
    }
}

impl Canonicalizer {
    pub fn canonicalize(
        mut self,
        value: core::Value,
        engine: &core::TypeCheckerCore,
    ) -> (CanonId, Canonicalized) {
        let id = self.canon_value(value, engine);
        (id, self.builder.finish())
    }

    fn canon_value(&mut self, value: core::Value, engine: &core::TypeCheckerCore) -> CanonId {
        self.recursive_with(value, |this, value| match engine.get(value) {
            TypeNode::Value(value, _) => this.canon_value_head(value, engine),
            TypeNode::Var(_) => this.canon_value_var(value, engine),
            _ => unreachable!(),
        })
    }

    fn canon_value_var(&mut self, value: core::Value, engine: &core::TypeCheckerCore) -> CanonId {
        let mut ids = self.value_predecessors(value, engine);
        ids.sort_unstable();
        ids.dedup();

        match &ids[..] {
            [] => self.add_canon(Canonical::Any(None, None)),
            [id] => *id,
            ids => self.add_canon(Canonical::Or(ids.to_vec(), None)),
        }
    }

    fn value_predecessors(
        &mut self,
        value: impl WithID,
        engine: &core::TypeCheckerCore,
    ) -> Vec<CanonId> {
        let mut ids = Vec::new();
        for (pred, pred_id) in engine.predecessors(value) {
            match pred {
                TypeNode::Value(value, _) => {
                    ids.push(self.canon_value_head(value, engine));
                }
                TypeNode::Var(_) => {
                    // Only if it is a var without predecessors.
                    if engine.predecessors(pred_id).count() == 0 {
                        ids.push(self.add_canon(Canonical::Any(None, None)));
                    }
                }
                _ => continue,
            }
        }
        ids
    }

    fn canon_value_head(
        &mut self,
        value: &core::VTypeHead,
        engine: &core::TypeCheckerCore,
    ) -> CanonId {
        match value {
            core::VTypeHead::VError => self.add_canon(Canonical::Error(None)),
            core::VTypeHead::VLiteral(lit) => self.add_canon(Canonical::Literal(lit.clone(), None)),
            core::VTypeHead::VPrimitive(name) => {
                self.add_canon(Canonical::Primitive(name.clone(), None))
            }
            core::VTypeHead::VTuple { items } => {
                let items = items
                    .iter()
                    .map(|item| self.canon_value(*item, engine))
                    .collect();
                self.add_canon(Canonical::Tuple { items, span: None })
            }
            core::VTypeHead::VList { item } => {
                let item = self.canon_value(*item, engine);
                self.add_canon(Canonical::List { item, span: None })
            }
            core::VTypeHead::VStruct { fields, proto } => {
                let mut proto = proto
                    .map(|proto| self.canon_value(proto, engine))
                    .and_then(|proto| match self.builder.get(proto) {
                        Canonical::Record {
                            fields,
                            proto: _,
                            span: _,
                        } => Some(fields.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();

                let new_fields: Vec<(String, CanonId)> = fields
                    .iter()
                    .map(|(name, value)| (name.clone(), self.canon_value(*value, engine)))
                    .collect();

                proto.extend(new_fields);

                self.add_canon(Canonical::Record {
                    fields: proto,
                    proto: None,
                    span: None,
                })
            }
            core::VTypeHead::VFunc { pattern, ret } => {
                let pattern = self.canon_use(*pattern, engine);
                let ret = self.canon_value(*ret, engine);
                self.add_canon(Canonical::Func {
                    pattern,
                    ret,
                    span: None,
                })
            }
            core::VTypeHead::VRef { read, write } => {
                let read = read.map(|read| self.canon_value(read, engine));
                let write = write.map(|write| self.canon_use(write, engine));
                self.add_canon(Canonical::Reference {
                    read,
                    write,
                    span: None,
                })
            }
        }
    }

    fn canon_use(&mut self, use_: core::Use, engine: &core::TypeCheckerCore) -> CanonId {
        self.recursive_with(use_, |this, use_| match engine.get(use_) {
            TypeNode::Use(use_, _) => this.canon_use_head(use_, engine),
            TypeNode::Var(_) => this.canon_use_var(use_, engine),
            _ => unreachable!(),
        })
    }

    fn canon_use_head(
        &mut self,
        use_: &core::UTypeHead,
        engine: &core::TypeCheckerCore,
    ) -> CanonId {
        match use_ {
            core::UTypeHead::UError => self.add_canon(Canonical::Error(None)),
            core::UTypeHead::ULiteral(lit) => self.add_canon(Canonical::Literal(lit.clone(), None)),
            core::UTypeHead::UPrimitive(name) => {
                self.add_canon(Canonical::Primitive(name.clone(), None))
            }
            core::UTypeHead::UTuple { items } => {
                let items = items
                    .iter()
                    .map(|item| self.canon_use(*item, engine))
                    .collect();
                self.add_canon(Canonical::Tuple { items, span: None })
            }
            core::UTypeHead::UFunc { pattern, ret } => {
                let pattern = self.canon_value(*pattern, engine);
                let ret = self.canon_use(*ret, engine);
                self.add_canon(Canonical::Func {
                    pattern,
                    ret,
                    span: None,
                })
            }
            core::UTypeHead::UList {
                items,
                min_len: _,
                max_len: _,
            } => {
                let item = self.canon_use(*items, engine);
                self.add_canon(Canonical::List { item, span: None })
            }
            core::UTypeHead::UStruct { fields } => {
                let fields = fields
                    .iter()
                    .map(|(name, id)| (name.clone(), self.canon_use(*id, engine)))
                    .collect();
                self.add_canon(Canonical::Record {
                    fields,
                    proto: None,
                    span: None,
                })
            }
            core::UTypeHead::UApplication {
                args,
                ret,
                first_arg: _,
            } => {
                let args = self.canon_value(*args, engine);
                let args = self.builder.get(args);
                let Canonical::Tuple {
                    items: args,
                    span: _,
                } = args
                else {
                    panic!("Expected a tuple")
                };
                let args = args.clone();
                let ret = self.canon_use(*ret, engine);
                self.add_canon(Canonical::Applicable {
                    args,
                    ret,
                    span: None,
                })
            }
            core::UTypeHead::URef { read, write } => {
                let read = read.map(|read| self.canon_use(read, engine));
                let write = write.map(|write| self.canon_value(write, engine));
                self.add_canon(Canonical::Reference {
                    read,
                    write,
                    span: None,
                })
            }
        }
    }

    fn canon_use_var(&mut self, use_: core::Use, engine: &core::TypeCheckerCore) -> CanonId {
        let mut ids = self.value_predecessors(use_, engine);
        if ids.is_empty() {
            for (succ, _) in engine.successors(use_) {
                match succ {
                    TypeNode::Use(use_, _) => {
                        ids.push(self.canon_use_head(use_, engine));
                    }
                    _ => continue,
                }
            }
        }
        let mut ids = ids.to_vec();
        ids.sort_unstable();
        ids.dedup();
        match &ids[..] {
            [] => self.add_canon(Canonical::Any(None, None)),
            [id] => *id,
            ids => self.add_canon(Canonical::And(ids.to_vec(), None)),
        }
    }

    // ----

    fn is_visited(&mut self, id: impl WithID) -> bool {
        let id = id.id();
        let is = self.visited.contains(&id);
        self.visited.push(id);
        is
    }

    fn is_recursive(&mut self, id: impl WithID) -> Option<usize> {
        self.recursive.get(&id.id()).copied()
    }

    fn recursive_with<ID: WithID + Copy>(
        &mut self,
        id: ID,
        f: impl FnOnce(&mut Self, ID) -> CanonId,
    ) -> CanonId {
        if self.is_visited(id) {
            return self.recursive(id);
        }
        let result = f(self, id);
        self.visited.pop();
        if let Some(i) = self.is_recursive(id) {
            return self.add_canon(Canonical::As(i, result, None));
        }
        result
    }

    fn recursive(&mut self, id: impl WithID) -> CanonId {
        let i = self.recursive.len();
        self.recursive.insert(id.id(), i);
        self.add_canon(Canonical::Any(Some(i), None))
        // self.add_canon(Canonical::Recursive(CanonId(id.id())))
    }
    fn add_canon(&mut self, canon: Canonical) -> CanonId {
        self.builder.add(canon)
    }
}
