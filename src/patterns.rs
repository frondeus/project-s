use indexmap::IndexMap;

use crate::{
    ast::{ASTS, SExp, SExpId},
    source::Span,
};

#[derive(Clone)]
pub enum Pattern {
    Hole(Span, SExpId),
    Single(String, Span, SExpId),
    Splice(Box<Pattern>, Span, SExpId),
    List(Vec<Pattern>, Span, SExpId),
    Object(IndexMap<String, Pattern>, Span, SExpId),
}

impl std::fmt::Debug for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hole(_span, _id) => f.debug_tuple("_").finish(),
            Self::Single(name, _span, _id) => f.debug_tuple("Single").field(name).finish(),
            Self::List(list, _span, _id) => f.debug_tuple("List").field(list).finish(),
            Self::Object(obj, _span, _id) => f.debug_tuple("Object").field(obj).finish(),
            Self::Splice(splice, _span, _id) => f.debug_tuple("Splice").field(splice).finish(),
        }
    }
}

impl Pattern {
    fn is_special_case(asts: &ASTS, ident: SExpId, name: &str) -> bool {
        let ident = asts.get(ident);
        let Some(ident) = ident.as_symbol() else {
            return false;
        };
        ident == name
    }

    pub fn parse_list(
        items: Vec<SExpId>,
        asts: &ASTS,
        span: Span,
        id: SExpId,
    ) -> Result<Self, String> {
        if items.is_empty() {
            return Ok(Self::List(vec![], span, id));
        }
        let first = items[0];
        if Self::is_special_case(asts, first, "obj/plain") {
            let mut patterns = IndexMap::new();
            let mut items = items.into_iter().skip(1).peekable();
            while let Some(item) = items.next() {
                let key = asts.get(item);
                let key_span = key.span;
                let Some(key) = key.as_keyword() else {
                    return Err(format!("Expected keyword, found: {:?}", asts.fmt(item)));
                };

                if let Some(next_id) = items.peek() {
                    let next = asts.get(*next_id);
                    match &**next {
                        SExp::Symbol(renamed) => {
                            patterns.insert(
                                key.to_owned(),
                                Pattern::Single(renamed.to_owned(), next.span, item),
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
                patterns.insert(
                    key.to_owned(),
                    Pattern::Single(key.to_owned(), key_span, item),
                );
            }

            return Ok(Pattern::Object(patterns, span, id));
        } else if Self::is_special_case(asts, first, "splice") {
            let Some(next) = items.into_iter().nth(1) else {
                return Err("Expected pattern after 'splice' keyword".to_string());
            };
            let pattern = Self::parse(next, asts)?;
            return Ok(Pattern::Splice(Box::new(pattern), span, id));
        }

        let mut patterns = vec![];
        for item in items {
            let pattern = Self::parse(item, asts)?;
            patterns.push(pattern);
        }
        Ok(Pattern::List(patterns, span, id))
    }

    pub fn parse(id: SExpId, asts: &ASTS) -> Result<Self, String> {
        let sexp = asts.get(id).clone();
        let span = sexp.span;
        match sexp.inner() {
            SExp::Symbol(s) if s == "_" => Ok(Pattern::Hole(span, id)),
            SExp::Keyword(k) => Ok(Pattern::Single(k, span, id)),
            SExp::List(items) => Self::parse_list(items, asts, span, id),
            ident => Err(format!("Expected keyword or list, found: {ident:?}")),
        }
    }

    // pub fn destruct<T>(self, value: T, with: impl Fn(String, T)) {
    //     match self {
    //         Self::Single(key) => with(key, value),
    //         Self::List(patterns) =
    //     }

    // }
}
