use std::{ops::Deref, sync::Arc};

pub use tree_sitter::Range;

#[derive(Debug, Default)]
pub struct Sources {
    sources: Vec<Source>,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub filename: Arc<str>,
    pub source: Arc<str>,
}

impl Sources {
    pub fn single(filename: &str, source: &str) -> (Self, SourceId) {
        let mut sources = Self::default();
        let id = sources.add(filename, source);
        (sources, id)
    }

    pub fn add(&mut self, filename: &str, source: &str) -> SourceId {
        let id = SourceId(self.sources.len());
        self.sources.push(Source {
            filename: Arc::from(filename),
            source: Arc::from(source),
        });
        id
    }

    pub fn get(&self, id: SourceId) -> &Source {
        &self.sources[id.0]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Source> {
        self.sources.iter()
    }

    pub fn iter_with_id(&self) -> impl Iterator<Item = (SourceId, &Source)> {
        self.sources
            .iter()
            .enumerate()
            .map(|(i, s)| (SourceId(i), s))
    }
}

pub trait WithSpan {
    fn span(&self) -> Span;
}

impl WithSpan for Span {
    fn span(&self) -> Span {
        *self
    }
}

impl<T> WithSpan for Spanned<T> {
    fn span(&self) -> Span {
        self.span
    }
}

impl<T> WithSpan for &T
where
    T: WithSpan,
{
    fn span(&self) -> Span {
        (*self).span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    item: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(item: T, span: Span) -> Self {
        Self { item, span }
    }

    pub fn inner(self) -> T {
        self.item
    }
    pub fn map<O>(self, f: impl FnOnce(T) -> O) -> Spanned<O> {
        Spanned {
            item: f(self.item),
            span: self.span,
        }
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<T> Copy for Spanned<T> where T: Copy {}

impl<T> Spanned<Option<T>> {
    pub fn transpose(self) -> Option<Spanned<T>> {
        self.item.map(|item| Spanned {
            item,
            span: self.span,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Copy, Eq, Hash)]
pub struct SourceId(usize);

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Span {
    pub range: Range,
    pub source_id: SourceId,
    // pub filename: Arc<str>,
    // pub source: Arc<str>,
}

impl Span {
    pub fn new(range: Range, source_id: SourceId) -> Self {
        Self { range, source_id }
    }

    pub fn new_empty(source_id: SourceId) -> Self {
        Self {
            range: default_range(),
            source_id,
        }
    }
}

// impl Default for Span {
//     fn default() -> Self {
//         Self {
//             range: default_range(),
//             source_id: SourceId(0),
//             // filename: Arc::from(""),
//             // source: Arc::from(""),
//         }
//     }
// }

pub trait WithRange {
    fn range(&self) -> Range;
}

impl WithRange for tree_sitter::Node<'_> {
    fn range(&self) -> Range {
        self.range()
    }
}

impl WithRange for Option<tree_sitter::Node<'_>> {
    fn range(&self) -> Range {
        self.map(|n| n.range()).unwrap_or_else(default_range)
    }
}

fn default_range() -> Range {
    Range {
        start_byte: Default::default(),
        end_byte: Default::default(),
        start_point: Default::default(),
        end_point: Default::default(),
    }
}
