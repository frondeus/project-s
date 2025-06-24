use std::sync::Arc;

pub use tree_sitter::Range;

#[derive(Debug, Clone)]
pub struct WithSpan<T> {
    pub item: T,
    pub span: Span,
}

impl<T> WithSpan<T> {
    pub fn map<O>(self, f: impl FnOnce(T) -> O) -> WithSpan<O> {
        WithSpan {
            item: f(self.item),
            span: self.span,
        }
    }
}

impl<T> WithSpan<Option<T>> {
    pub fn transpose(self) -> Option<WithSpan<T>> {
        self.item.map(|item| WithSpan {
            item,
            span: self.span,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub range: Range,
    pub filename: Arc<str>,
    pub source: Arc<str>,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            range: default_range(),
            filename: Arc::from(""),
            source: Arc::from(""),
        }
    }
}

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
