use super::TypeEnv;
use super::core;
use core::{ID, WithID};

impl TypeEnv {
    pub fn to_string(&self, value: core::Value) -> String {
        let mut f = String::new();
        let mut visited = Vec::new();
        self.fmt_value(value, &mut f, &mut visited);
        f
    }

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
                core::TypeNode::Value(value) => Some(value),
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
            core::TypeNode::Use(u) => self.fmt_use_head(u, f, visited),
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
            core::TypeNode::Value(value) => {
                self.fmt_value_head(value, f, visited);
            }
            core::TypeNode::Use(_u) => unreachable!(),
            core::TypeNode::Var => {
                let mut first = true;
                let mut any = true;
                for (pred, pred_id) in self.engine.predecessors(value) {
                    match pred {
                        core::TypeNode::Use(_) => continue,
                        core::TypeNode::Value(value) => {
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
}
