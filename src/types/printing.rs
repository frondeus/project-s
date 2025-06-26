use super::TypeEnv;
use super::canonical::CanonId;
use super::canonical::Canonical;
use super::canonical::Canonicalized;
use super::canonical::Canonicalizer;
use super::core;

struct Formatter<'a> {
    f: &'a mut String,
}

impl Formatter<'_> {
    fn print_canon(&mut self, id: CanonId, canonical: &Canonicalized) {
        match canonical.get(id) {
            Canonical::Todo(todo) => self.f.push_str(&format!("TODO: {}", todo)),
            Canonical::Any(None) => self.f.push_str("Any"),
            Canonical::Any(Some(i)) => self.f.push_str(&format!("?T{}", i)),
            Canonical::Recursive(_canon_id) => self.f.push_str("<recursive>"),
            Canonical::Or(canon_ids) => {
                for (i, canon_id) in canon_ids.iter().enumerate() {
                    if i > 0 {
                        self.f.push_str(" | ");
                    }
                    self.print_canon(*canon_id, canonical);
                }
            }
            Canonical::Skip => self.f.push('_'),
            Canonical::Bool => self.f.push_str("Bool"),
            Canonical::Number => self.f.push_str("Number"),
            Canonical::String => self.f.push_str("String"),
            Canonical::Error => self.f.push_str("Error"),
            Canonical::Keyword => self.f.push_str("Keyword"),
            Canonical::Tuple { items } => {
                self.f.push('(');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.f.push_str(", ");
                    }
                    self.print_canon(*item, canonical);
                }
                self.f.push(')');
            }
            Canonical::List { item } => {
                self.f.push('[');
                self.print_canon(*item, canonical);
                self.f.push(']');
            }
            Canonical::Func { pattern, ret } => {
                self.print_canon(*pattern, canonical);
                self.f.push_str(" -> ");
                self.print_canon(*ret, canonical);
            }
            Canonical::Struct { fields } => {
                self.f.push('{');
                for (i, (name, id)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.f.push_str(", ");
                    }
                    self.f.push_str(name);
                    self.f.push_str(": ");
                    self.print_canon(*id, canonical);
                }
                self.f.push('}');
            }
            &Canonical::Reference {
                read: Some(read),
                write: Some(write),
            } if read == write => {
                self.f.push_str("refmut<");
                self.print_canon(read, canonical);
                self.f.push('>');
            }
            &Canonical::Reference {
                read: Some(read),
                write: Some(write),
            } => {
                self.f.push_str("ref<");
                self.print_canon(read, canonical);
                self.f.push_str(", mut ");
                self.print_canon(write, canonical);
                self.f.push('>');
            }
            &Canonical::Reference {
                read: Some(read),
                write: None,
            } => {
                self.f.push_str("ref<");
                self.print_canon(read, canonical);
                self.f.push('>');
            }
            &Canonical::Reference {
                read: None,
                write: Some(write),
            } => {
                self.f.push_str("mut<");
                self.print_canon(write, canonical);
                self.f.push('>');
            }
            Canonical::Reference {
                read: None,
                write: None,
            } => {
                unreachable!()
            }
        }
    }

    fn value(&mut self, value: core::Value, engine: &core::TypeCheckerCore) {
        let (id, canonical) = Canonicalizer::default().canonicalize(value, engine);
        self.print_canon(id, &canonical);
    }
}

impl TypeEnv {
    pub fn to_string(&self, value: core::Value) -> String {
        let mut f = String::new();
        Formatter { f: &mut f }.value(value, &self.engine);
        f
    }
}
