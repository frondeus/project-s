use super::core;
use super::core::TypeNode;
use super::core::WithID;

#[derive(Debug, PartialEq, Clone, Copy, PartialOrd, Ord, Eq)]
pub struct CanonId(core::ID);

#[derive(Debug, PartialEq, Clone)]
pub enum Canonical {
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
}

impl Canonical {
    fn ids(&self) -> impl Iterator<Item = CanonId> {
        match self {
            Canonical::Any(_) => vec![].into_iter(),
            Canonical::Recursive(canon_id) => vec![*canon_id].into_iter(),
            Canonical::Or(canon_ids) => canon_ids.clone().into_iter(),
            Canonical::Bool => vec![].into_iter(),
            Canonical::Number => vec![].into_iter(),
            Canonical::String => vec![].into_iter(),
            Canonical::Error => vec![].into_iter(),
            Canonical::Keyword => vec![].into_iter(),
            Canonical::Tuple { items } => items.clone().into_iter(),
            Canonical::List { item } => vec![*item].into_iter(),
            Canonical::Func { pattern, ret } => vec![*pattern, *ret].into_iter(),
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
            core::VTypeHead::VObj { .. } => todo!(),
            core::VTypeHead::VFunc { pattern, ret } => {
                let pattern = self.canon_use(*pattern, engine);
                let ret = self.canon_value(*ret, engine);
                self.add_canon(Canonical::Func { pattern, ret })
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
            core::UTypeHead::UTuple { items } => {
                let items = items
                    .iter()
                    .map(|item| self.canon_use(*item, engine))
                    .collect();
                self.add_canon(Canonical::Tuple { items })
            }
            core::UTypeHead::UTupleAccess { .. } => todo!(),
            core::UTypeHead::UList {
                items,
                min_len: _,
                max_len: _,
            } => {
                let item = self.canon_use(*items, engine);
                self.add_canon(Canonical::List { item })
            }
            core::UTypeHead::UObj { .. } => todo!(),
            core::UTypeHead::UObjAccess { .. } => todo!(),
            core::UTypeHead::UFunc { .. } => todo!(),
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

// impl Formatter<'_> {
//     fn def_ids(&mut self, def_ids: Vec<usize>, engine: &core::TypeCheckerCore) {
//         let defs = def_ids
//             .iter()
//             .map(|id| engine.get(*id))
//             .filter_map(|node| match node {
//                 core::TypeNode::Def(def, _span) => Some(def),
//                 _ => None,
//             })
//             .collect::<Vec<_>>();

//         if defs.is_empty() {
//             self.f.push_str("Any");
//         } else {
//             for (i, def) in defs.into_iter().enumerate() {
//                 if i > 0 {
//                     self.f.push_str(" | ");
//                 }
//                 self.def(def, engine);
//             }
//         }
//     }
//     fn refdef(&mut self, def: core::Def, engine: &core::TypeCheckerCore) {
//         if self.visited.contains(&def.id()) {
//             self.f.push_str("<recursive>");
//             return;
//         }
//         self.visited.push(def.id());
//         let def = engine.get(def);
//         match def {
//             core::TypeNode::Def(def, _span) => self.def(def, engine),
//             _ => unreachable!(),
//         }
//         self.visited.pop();
//     }
//     fn def(&mut self, def: &core::TypeDef, engine: &core::TypeCheckerCore) {
//         match def {
//             core::TypeDef::Bool
//             | core::TypeDef::Number
//             | core::TypeDef::String
//             | core::TypeDef::Error
//             | core::TypeDef::Keyword => {
//                 self.f.push_str(&def.to_string());
//             }
//             core::TypeDef::Tuple(items) => {
//                 self.f.push('(');
//                 for (i, item) in items.iter().enumerate() {
//                     if i > 0 {
//                         self.f.push_str(", ");
//                     }
//                     self.refdef(*item, engine);
//                 }
//                 self.f.push(')');
//             }
//             core::TypeDef::Obj => todo!(),
//             core::TypeDef::Func => {
//                 self.f.push_str("function");
//             }
//         }
//     }
//     fn value(&mut self, value: core::Value, engine: &core::TypeCheckerCore) {
//         match engine.get(value) {
//             core::TypeNode::Var => {
//                 let def_ids = engine
//                     .predecessors(value)
//                     .flat_map(|(_pred, pred_id)| {
//                         engine
//                             .successors(pred_id)
//                             .filter_map(|(succ, succ_id)| match succ {
//                                 core::TypeNode::Def(_def, _span) => Some(succ_id),
//                                 _ => None,
//                             })
//                     })
//                     .unique()
//                     .collect::<Vec<_>>();

//                 self.def_ids(def_ids, engine);
//             }
//             core::TypeNode::Use(_, _) | core::TypeNode::Value(_, _) => {
//                 let def_ids = engine
//                     .successors(value)
//                     .filter_map(|(succ, succ_id)| match succ {
//                         core::TypeNode::Def(_def, _span) => Some(succ_id),
//                         _ => None,
//                     })
//                     .unique()
//                     .collect::<Vec<_>>();

//                 self.def_ids(def_ids, engine);
//             }
//             core::TypeNode::Def(type_def, _span) => {
//                 self.def(type_def, engine);
//             }
//         }
//     }
// }

/*
fn fmt_value_head(&self, value: &core::VTypeHead, f: &mut String, visited: &mut Vec<ID>) {
    match value {
        core::VTypeHead::VBool => f.push_str("Bool"),
        core::VTypeHead::VNumber => f.push_str("Number"),
        core::VTypeHead::VString => f.push_str("String"),
        core::VTypeHead::VError => f.push_str("Error"),
        core::VTypeHead::VKeyword => f.push_str("Keyword"),
        core::VTypeHead::VList { items } => {
            f.push('(');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    f.push_str(", ");
                }
                self.fmt_value(*item, f, visited);
            }
            f.push(')');
        }
        core::VTypeHead::VObj { .. } => todo!(),
        core::VTypeHead::VFunc { pattern, ret } => {
            self.fmt_use(*pattern, f, visited);
            f.push_str(" -> ");
            self.fmt_value(*ret, f, visited);
        }
    }
}

fn fmt_use_head(&self, u: &core::UTypeHead, f: &mut String, visited: &mut Vec<ID>) {
    match u {
        core::UTypeHead::UBool => f.push_str("Bool"),
        core::UTypeHead::UNumber => f.push_str("Number"),
        core::UTypeHead::UString => f.push_str("String"),
        core::UTypeHead::UKeyword => f.push_str("Keyword"),
        core::UTypeHead::UTuple { items } => {
            f.push('(');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    f.push_str(", ");
                }
                self.fmt_use(*item, f, visited);
            }
            f.push(')');
        }
        core::UTypeHead::UList {
            items,
            min_len,
            max_len,
        } => {
            f.push('[');
            self.fmt_use(*items, f, visited);
            f.push(';');
            f.push_str(&min_len.to_string());
            if let Some(max_len) = max_len {
                f.push(':');
                f.push_str(&max_len.to_string());
            }
            f.push(']');
        }
        core::UTypeHead::UTupleAccess { .. } => todo!(),
        core::UTypeHead::UObj { .. } => todo!(),
        core::UTypeHead::UObjAccess { .. } => todo!(),
        core::UTypeHead::UFunc { .. } => todo!(),
    }
}

fn fmt_use(&self, use_: core::Use, f: &mut String, visited: &mut Vec<ID>) {
    use core::WithID;
    if self.check_visited(use_, visited) {
        f.push_str("<recursive>");
        return;
    }

    let mut has_value = false;

    for (i, node) in self
        .engine
        .predecessors(use_)
        .filter_map(|(pred, _)| match pred {
            core::TypeNode::Value(value, _) => Some(value),
            _ => None,
        })
        .enumerate()
    {
        has_value = true;
        if i > 0 {
            f.push_str(" | ");
        }
        self.fmt_value_head(node, f, visited);
    }
    visited.pop();

    if !has_value {
        self.fmt_use_node(use_.id(), self.engine.get(use_), f, visited);
    }
}

fn fmt_use_node(&self, id: ID, node: &core::TypeNode, f: &mut String, visited: &mut Vec<ID>) {
    if self.check_visited(id, visited) {
        f.push_str("<recursive>");
        return;
    }

    match node {
        core::TypeNode::Var => {
            let mut first = true;
            let mut any = true;
            for (pred, pred_id) in self.engine.successors(id) {
                any = false;
                if first {
                    first = false;
                } else {
                    f.push_str(" | ");
                }
                self.fmt_use_node(pred_id, pred, f, visited);
            }
            if any {
                f.push_str("Any");
            }
        }
        core::TypeNode::Use(u, _) => self.fmt_use_head(u, f, visited),
        node => unreachable!("{:?}", node),
    }
    visited.pop();
}

fn check_visited(&self, id: impl WithID, visited: &mut Vec<ID>) -> bool {
    let id = id.id();
    if visited.contains(&id) {
        return true;
    }
    visited.push(id);
    false
}

fn fmt_value(&self, value: core::Value, f: &mut String, visited: &mut Vec<ID>) {
    if self.check_visited(value, visited) {
        f.push_str("<recursive>");
        return;
    }
    match self.engine.get(value) {
        core::TypeNode::Value(value, _) => {
            self.fmt_value_head(value, f, visited);
        }
        core::TypeNode::Def(def, _) => {
            f.push_str(&def.to_string());
        }
        core::TypeNode::Use(_u, _) => unreachable!(),
        core::TypeNode::Var => {
            let mut first = true;
            let mut any = true;
            for (pred, pred_id) in self.engine.predecessors(value) {
                match pred {
                    core::TypeNode::Use(_u, _) => continue,
                    core::TypeNode::Def(def, _) => {
                        f.push_str(&def.to_string());
                    }
                    core::TypeNode::Value(value, _) => {
                        any = false;
                        if first {
                            first = false;
                        } else {
                            f.push_str(" | ");
                        }
                        self.fmt_value_head(value, f, visited);
                    }
                    core::TypeNode::Var => {
                        // Only if it is a var without predecessors.
                        if self.engine.predecessors(pred_id).count() == 0 {
                            if first {
                                first = false;
                            } else {
                                f.push_str(" | ");
                            }
                            f.push_str("Any");
                        } else {
                            continue;
                        }
                    }
                }
            }
            if any {
                f.push_str("Any");
            }
        }
    }
    visited.pop();
}
*/
