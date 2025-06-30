use super::TypeEnv;
use super::canonical::CanonId;
use super::canonical::Canonical;
use super::canonical::Canonicalized;
use super::canonical::Canonicalizer;
use super::core;

struct Formatter<'a> {
    f: &'a mut String,
}

fn variable_letters(mut i: usize) -> String {
    const LETTERS: &[char] = &[
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];
    let letter = LETTERS[i % LETTERS.len()];
    let mut result = format!("'{letter}");
    if i < LETTERS.len() {
        return result;
    }
    let mut num = 1;
    while i > 0 {
        i = match i.checked_sub(LETTERS.len()) {
            Some(i) => i,
            None => break,
        };
        num += 1;
    }
    result.push_str(&num.to_string());
    result
}

impl Formatter<'_> {
    fn print_canon(&mut self, id: CanonId, canonical: &Canonicalized) {
        match canonical.get(id) {
            Canonical::Todo(todo, _) => self.f.push_str(&format!("TODO: {}", todo)),
            Canonical::Any(None, _) => self.f.push_str("Any"),
            Canonical::Literal(lit, _) => self.f.push_str(&format!("{lit}")),
            Canonical::Any(Some(i), _) => self.f.push_str(&variable_letters(*i)),
            Canonical::As(i, canon_id, _) => {
                self.f.push('(');
                self.print_canon(*canon_id, canonical);
                self.f.push_str(") as ");
                self.f.push_str(&variable_letters(*i));
            }
            Canonical::Or(canon_ids, _) => {
                // assert!(canon_ids.len() > 1);
                if canon_ids.len() == 1 {
                    self.f.push_str("or<");
                    self.print_canon(canon_ids[0], canonical);
                    self.f.push('>');
                    return;
                }

                for (i, canon_id) in canon_ids.iter().enumerate() {
                    if i > 0 {
                        self.f.push_str(" | ");
                    }
                    self.print_canon(*canon_id, canonical);
                }
            }
            Canonical::And(canon_ids, _) => {
                // assert!(canon_ids.len() > 1);
                if canon_ids.len() == 1 {
                    self.f.push_str("and<");
                    self.print_canon(canon_ids[0], canonical);
                    self.f.push('>');
                    return;
                }

                for (i, canon_id) in canon_ids.iter().enumerate() {
                    if i > 0 {
                        self.f.push_str(" & ");
                    }
                    self.print_canon(*canon_id, canonical);
                }
            }
            Canonical::Wildcard(_) => self.f.push('_'),
            Canonical::Error(_) => self.f.push_str("Error"),
            Canonical::Primitive(name, _) => self.f.push_str(name),
            Canonical::Tuple { items, span: _ } => {
                self.f.push('(');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.f.push_str(", ");
                    }
                    self.print_canon(*item, canonical);
                }
                self.f.push(')');
            }
            Canonical::List { item, span: _ } => {
                self.f.push('[');
                self.print_canon(*item, canonical);
                self.f.push(']');
            }
            Canonical::Func {
                pattern,
                ret,
                span: _,
            } => {
                self.print_canon(*pattern, canonical);
                self.f.push_str(" -> ");
                self.print_canon(*ret, canonical);
            }
            Canonical::Applicable { args, ret, span: _ } => {
                self.f.push_str("Applicable(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.f.push_str(", ");
                    }
                    self.print_canon(*arg, canonical);
                }
                self.f.push_str(" -> ");
                self.print_canon(*ret, canonical);
                self.f.push(')');
            }
            Canonical::Record {
                fields,
                proto: _,
                span: _,
            } => {
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
                span: _,
            } if read == write => {
                self.f.push_str("refmut<");
                self.print_canon(read, canonical);
                self.f.push('>');
            }
            &Canonical::Reference {
                read: Some(read),
                write: Some(write),
                span: _,
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
                span: _,
            } => {
                self.f.push_str("ref<");
                self.print_canon(read, canonical);
                self.f.push('>');
            }
            &Canonical::Reference {
                read: None,
                write: Some(write),
                span: _,
            } => {
                self.f.push_str("mut<");
                self.print_canon(write, canonical);
                self.f.push('>');
            }
            Canonical::Reference {
                read: None,
                write: None,
                span: _,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters() {
        assert_eq!(variable_letters(0), "'a");
        assert_eq!(variable_letters(25), "'z");
        assert_eq!(variable_letters(26), "'a2");
        assert_eq!(variable_letters(27), "'b2");
        assert_eq!(variable_letters(28), "'c2");
        assert_eq!(variable_letters(29), "'d2");
        assert_eq!(variable_letters(30), "'e2");
        assert_eq!(variable_letters(31), "'f2");
        assert_eq!(variable_letters(51), "'z2");
        assert_eq!(variable_letters(52), "'a3");
        assert_eq!(variable_letters(53), "'b3");
        assert_eq!(variable_letters(54), "'c3");
        assert_eq!(variable_letters(55), "'d3");
        assert_eq!(variable_letters(56), "'e3");
        assert_eq!(variable_letters(57), "'f3");
    }
}
