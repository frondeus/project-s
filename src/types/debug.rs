use super::levels::WithLevel;
use super::*;

impl std::fmt::Display for InferedTypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TypeEnv {
    pub fn debug_dot(&self, asts: &ASTS, root: InferedTypeId) -> String {
        let mut buf = String::new();
        self.debug_dot_inner(root, asts, &mut buf)
            .expect("Written ");
        buf
    }

    fn debug_dot_inner(
        &self,
        root: InferedTypeId,
        asts: &ASTS,
        buf: &mut String,
    ) -> std::fmt::Result {
        use std::fmt::Write;
        writeln!(buf, "digraph G {{")?;
        for (id, node) in self.iter() {
            let sexp = self
                .code_map
                .get_sexps(id)
                .and_then(|mut sexps| sexps.next());
            let mut label = format!("{id} - lvl{} - {node:?}", id.level(self));
            if let Some(sexp) = sexp {
                label.push_str(&format!("\n`{}`", asts.fmt(sexp)));
            }
            let label = label.escape_debug();
            writeln!(buf, "N{id} [label=\"{label}\"];",)?;
        }
        writeln!(buf, "START -> N{root}")?;

        for (id, node) in self.iter() {
            for to in node.ids() {
                writeln!(buf, "N{id} -> N{to} [style=dotted];")?;
            }
            if let InferedType::Variable {
                id: var_id,
                span: _,
            } = node
            {
                let vars = &self.vars[var_id.0];
                for to in &vars.lower_bounds {
                    writeln!(buf, "N{id} -> N{to} [label=\"LO\"];")?;
                }
                for to in &vars.upper_bounds {
                    writeln!(buf, "N{to} -> N{id} [label=\"UP\"];")?;
                }
            }
        }

        for &(lhs_id, rhs_id) in &self.constraints {
            writeln!(buf, "N{rhs_id} -> N{lhs_id} [label=\"<:\";color=red];")?;
        }

        writeln!(buf, "}}")?;
        Ok(())
    }
}
