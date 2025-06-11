use std::collections::HashMap;

use crate::ast::{ASTS, SExp, SExpId};

#[derive(Debug, Clone)]
pub enum Pattern {
    Single(String),
    List(Vec<Pattern>),
    Object(HashMap<String, Pattern>),
}

impl Pattern {
    fn is_special_case(asts: &ASTS, ident: SExpId, name: &str) -> bool {
        let ident = asts.get(ident);
        let Some(ident) = ident.as_symbol() else {
            return false;
        };
        ident == name
    }

    pub fn parse(ident: SExpId, asts: &ASTS) -> Result<Self, String> {
        match asts.get(ident).clone() {
            SExp::Keyword(k) => Ok(Pattern::Single(k)),
            SExp::List(items) if items.is_empty() => Ok(Pattern::List(vec![])),
            SExp::List(items) => {
                let first = items[0];
                if Self::is_special_case(asts, first, "obj/struct") {
                    let mut patterns = HashMap::new();
                    let mut items = items.into_iter().skip(1).peekable();
                    while let Some(item) = items.next() {
                        let Some(key) = asts.get(item).as_keyword() else {
                            return Err(format!("Expected keyword, found: {:?}", asts.fmt(item)));
                        };

                        if let Some(next) = items.peek() {
                            let next = asts.get(*next);
                            match next {
                                SExp::Symbol(renamed) => {
                                    patterns.insert(
                                        key.to_owned(),
                                        Pattern::Single(renamed.to_owned()),
                                    );
                                    items.next();
                                    continue;
                                }
                                SExp::Keyword(_) => (),
                                _ => {
                                    let next = items.next().unwrap();
                                    patterns.insert(key.to_owned(), Self::parse(next, asts)?);
                                    continue;
                                }
                            }
                        }
                        patterns.insert(key.to_owned(), Pattern::Single(key.to_owned()));
                    }

                    return Ok(Pattern::Object(patterns));
                }

                let mut patterns = vec![];
                for item in items {
                    let pattern = Self::parse(item, asts)?;
                    patterns.push(pattern);
                }
                Ok(Pattern::List(patterns))
            }
            ident => Err(format!("Expected keyword or list, found: {:?}", ident)),
        }
    }

    // pub fn destruct<T>(self, value: T, with: impl Fn(String, T)) {
    //     match self {
    //         Self::Single(key) => with(key, value),
    //         Self::List(patterns) =
    //     }

    // }
}
