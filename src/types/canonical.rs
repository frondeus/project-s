use super::core;
use super::core::TypeNode;
use super::core::WithID;

#[derive(Debug, PartialEq, Clone, Copy, PartialOrd, Ord, Eq)]
pub struct CanonId(core::ID);

#[derive(Debug, PartialEq, Clone)]
pub enum Canonical {
    /// A type that is not yet implemented. Better than panic
    Todo(String),

    /// "_" type
    Skip,
    /// Any type. If there is integer it means it is generic type
    /// It allows us to express polymorphic functions like (T0) -> T0 where we
    /// have guarantee of "any type in the input is going to be used in the output"
    Any(Option<usize>),
    Recursive(CanonId),
    Or(Vec<CanonId>),
    Bool,
    Number,
    String,
    Error,
    Keyword,
    Tuple {
        items: Vec<CanonId>,
    },
    List {
        item: CanonId,
    },
    Func {
        pattern: CanonId,
        ret: CanonId,
    },
    Struct {
        fields: Vec<(String, CanonId)>,
    },
    Reference {
        read: Option<CanonId>,
        write: Option<CanonId>,
    }, // Applicable {

       // }
}

impl Canonical {
    fn ids(&self) -> impl Iterator<Item = CanonId> {
        match self {
            Canonical::Todo(_)
            | Canonical::Any(_)
            | Canonical::Bool
            | Canonical::Number
            | Canonical::Skip
            | Canonical::String
            | Canonical::Error
            | Canonical::Keyword => vec![].into_iter(),
            Canonical::Recursive(canon_id) => vec![*canon_id].into_iter(),
            Canonical::Or(canon_ids) => canon_ids.clone().into_iter(),
            Canonical::Tuple { items } => items.clone().into_iter(),
            Canonical::List { item } => vec![*item].into_iter(),
            Canonical::Func { pattern, ret } => vec![*pattern, *ret].into_iter(),
            Canonical::Struct { fields } => fields
                .iter()
                .map(|(_, id)| *id)
                .collect::<Vec<_>>()
                .into_iter(),
            Canonical::Reference { read, write } => {
                let mut ids = Vec::new();
                if let Some(read) = read {
                    ids.push(*read);
                }
                if let Some(write) = write {
                    ids.push(*write);
                }
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
    fn get(&self, id: CanonId) -> &Canonical {
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
    builder: CanonicalBuilder,
}

pub struct Canonicalized {
    canonical: Vec<Canonical>,
}

impl Canonicalized {
    pub fn get(&self, id: CanonId) -> &Canonical {
        &self.canonical[id.0]
    }

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
            TypeNode::Var => this.canon_value_var(value, engine),
            _ => unreachable!(),
        })
    }

    fn canon_value_var(&mut self, value: core::Value, engine: &core::TypeCheckerCore) -> CanonId {
        let ids = self.value_predecessors(value, engine);
        match &ids[..] {
            [] => self.add_canon(Canonical::Any(None)),
            [id] => *id,
            ids => {
                let mut ids = ids.to_vec();
                ids.sort_unstable();
                ids.dedup();
                self.add_canon(Canonical::Or(ids))
            }
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
                TypeNode::Var => {
                    // Only if it is a var without predecessors.
                    if engine.predecessors(pred_id).count() == 0 {
                        ids.push(self.add_canon(Canonical::Any(None)));
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
            core::VTypeHead::VBool => self.add_canon(Canonical::Bool),
            core::VTypeHead::VNumber => self.add_canon(Canonical::Number),
            core::VTypeHead::VString => self.add_canon(Canonical::String),
            core::VTypeHead::VError => self.add_canon(Canonical::Error),
            core::VTypeHead::VKeyword => self.add_canon(Canonical::Keyword),
            core::VTypeHead::VTuple { items } => {
                let items = items
                    .iter()
                    .map(|item| self.canon_value(*item, engine))
                    .collect();
                self.add_canon(Canonical::Tuple { items })
            }
            core::VTypeHead::VList { item } => {
                let item = self.canon_value(*item, engine);
                self.add_canon(Canonical::List { item })
            }
            core::VTypeHead::VStruct { fields, proto } => {
                let mut proto = proto
                    .map(|proto| self.canon_value(proto, engine))
                    .and_then(|proto| match self.builder.get(proto) {
                        Canonical::Struct { fields } => Some(fields.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();

                let new_fields: Vec<(String, CanonId)> = fields
                    .iter()
                    .map(|(name, value)| (name.clone(), self.canon_value(*value, engine)))
                    .collect();

                proto.extend(new_fields);

                self.add_canon(Canonical::Struct { fields: proto })
            }
            core::VTypeHead::VFunc { pattern, ret } => {
                let pattern = self.canon_use(*pattern, engine);
                let ret = self.canon_value(*ret, engine);
                self.add_canon(Canonical::Func { pattern, ret })
            }
            core::VTypeHead::VRef { read, write } => {
                let read = read.map(|read| self.canon_value(read, engine));
                let write = write.map(|write| self.canon_use(write, engine));
                self.add_canon(Canonical::Reference { read, write })
            }
        }
    }

    fn canon_use(&mut self, use_: core::Use, engine: &core::TypeCheckerCore) -> CanonId {
        self.recursive_with(use_, |this, use_| match engine.get(use_) {
            TypeNode::Use(use_, _) => this.canon_use_head(use_, engine),
            TypeNode::Var => this.canon_use_var(use_, engine),
            _ => unreachable!(),
        })
    }

    fn canon_use_head(
        &mut self,
        use_: &core::UTypeHead,
        engine: &core::TypeCheckerCore,
    ) -> CanonId {
        match use_ {
            core::UTypeHead::UBool => self.add_canon(Canonical::Bool),
            core::UTypeHead::UNumber => self.add_canon(Canonical::Number),
            core::UTypeHead::UString => self.add_canon(Canonical::String),
            core::UTypeHead::UKeyword => self.add_canon(Canonical::Keyword),
            core::UTypeHead::UError => self.add_canon(Canonical::Error),
            core::UTypeHead::UTuple { items } => {
                let items = items
                    .iter()
                    .map(|item| self.canon_use(*item, engine))
                    .collect();
                self.add_canon(Canonical::Tuple { items })
            }
            core::UTypeHead::UTupleAccess { .. } => {
                self.add_canon(Canonical::Todo(format!("{:?}", use_)))
            }
            core::UTypeHead::UFunc { pattern, ret } => {
                let pattern = self.canon_value(*pattern, engine);
                let ret = self.canon_use(*ret, engine);
                self.add_canon(Canonical::Func { pattern, ret })
            }
            core::UTypeHead::UList {
                items,
                min_len: _,
                max_len: _,
            } => {
                let item = self.canon_use(*items, engine);
                self.add_canon(Canonical::List { item })
            }
            core::UTypeHead::UStruct { fields } => {
                let fields = fields
                    .iter()
                    .map(|(name, id)| (name.clone(), self.canon_use(*id, engine)))
                    .collect();
                self.add_canon(Canonical::Struct { fields })
            }
            core::UTypeHead::UStructAccess { .. } => {
                self.add_canon(Canonical::Todo(format!("{:?}", use_)))
            }
            app @ core::UTypeHead::UApplication { .. } => {
                self.add_canon(Canonical::Todo(format!("{:?}", app)))
            }
            core::UTypeHead::URef { read, write } => {
                let read = read.map(|read| self.canon_use(read, engine));
                let write = write.map(|write| self.canon_value(write, engine));
                self.add_canon(Canonical::Reference { read, write })
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
        match &ids[..] {
            [] => self.add_canon(Canonical::Any(None)),
            [id] => *id,
            ids => {
                let mut ids = ids.to_vec();
                ids.sort_unstable();
                ids.dedup();
                self.add_canon(Canonical::Or(ids))
            }
        }
    }

    // ----

    fn is_visited(&mut self, id: impl WithID) -> bool {
        let id = id.id();
        let is = self.visited.contains(&id);
        self.visited.push(id);
        is
    }

    fn recursive_with<ID: WithID + Copy>(
        &mut self,
        id: ID,
        f: impl FnOnce(&mut Self, ID) -> CanonId,
    ) -> CanonId {
        if self.is_visited(id) {
            return self.recursive(id);
        }
        let id = f(self, id);
        self.visited.pop();
        id
    }

    fn recursive(&mut self, id: impl WithID) -> CanonId {
        self.add_canon(Canonical::Recursive(CanonId(id.id())))
    }
    fn add_canon(&mut self, canon: Canonical) -> CanonId {
        self.builder.add(canon)
    }
}
