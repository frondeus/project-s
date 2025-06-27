use std::{
    collections::{HashMap, hash_map::Entry},
    io::BufWriter,
    sync::Arc,
};

use ariadne::{Color, Label, Report};

use crate::{
    ast::{ASTS, SExpId},
    source::{SourceId, Sources, Span, WithSpan},
};

pub struct Diag {
    pub span: Span,
    pub message: String,

    pub extras: Vec<Extra>,
}

impl Diag {
    pub fn add_extra(&mut self, message: impl ToString, span: Option<Span>) -> &mut Diag {
        self.extras.push(Extra {
            message: message.to_string(),
            span,
        });
        self
    }
}

pub struct Extra {
    pub message: String,
    pub span: Option<Span>,
}

#[derive(Default)]
pub struct Diagnostics {
    pub diags: Vec<Diag>,
}

pub struct SourcesCache<'a> {
    sources: &'a Sources,
    cache: HashMap<SourceId, ariadne::Source<Arc<str>>>,
}

impl<'a> SourcesCache<'a> {
    pub fn new(sources: &'a Sources) -> Self {
        Self {
            sources,
            cache: HashMap::new(),
        }
    }
}

impl ariadne::Cache<SourceId> for SourcesCache<'_> {
    type Storage = Arc<str>;

    fn fetch(
        &mut self,
        id: &SourceId,
    ) -> Result<&ariadne::Source<Self::Storage>, impl std::fmt::Debug> {
        match self.cache.entry(*id) {
            Entry::Occupied(entry) => Ok::<_, ()>(entry.into_mut()),
            Entry::Vacant(entry) => {
                let source = self.sources.get(*id);
                let source = ariadne::Source::from(source.source.clone());
                Ok(entry.insert(source))
            }
        }
    }

    fn display<'a>(&self, id: &'a SourceId) -> Option<impl std::fmt::Display + 'a> {
        let source = self.sources.get(*id);
        Some(source.filename.clone())
    }
}

impl Diagnostics {
    pub fn add(&mut self, span: impl WithSpan, message: impl ToString) -> &mut Diag {
        self.diags.push(Diag {
            span: span.span(),
            message: message.to_string(),
            extras: Vec::new(),
        });
        self.diags.last_mut().unwrap()
    }

    pub fn has_errors(&self) -> bool {
        !self.diags.is_empty()
    }

    pub fn print(self, sources: &Sources) -> String {
        let mut out = String::new();
        for diag in self.diags {
            let source = sources.get(diag.span.source_id);
            out.push_str(&format!("{}: {}\n", source.filename, diag.message));
        }
        out
    }

    pub fn pretty_print(&self, sources: &Sources) -> String {
        let mut cache = SourcesCache::new(sources);
        // let mut cache = ariadne::sources(
        //     sources.iter_with_id().map(|(_id, s)| (s.filename.clone(), s.source.clone())).collect::<Vec<_>>(),
        // self.diags
        //     .iter()
        //     .flat_map(|d| {
        //         std::iter::once(d.span.clone())
        //             .chain(d.extras.iter().filter_map(|e| e.span.clone()))
        //     })
        //     .map(|span| (span.filename.clone(), span.source.clone()))
        //     .unique()
        //     .collect::<Vec<_>>(),
        // );
        let mut out = Vec::new();
        let mut output_buf = BufWriter::new(&mut out);

        for report in self.diags.iter().map(|d| d.to_report()) {
            report.write(&mut cache, &mut output_buf).unwrap();
            // diag.write(
        }

        drop(output_buf);
        String::from_utf8(out).expect("Valid utf8")
    }
}

pub trait SExpDiag {
    fn add_sexp(&mut self, asts: &ASTS, sexp: SExpId, message: impl ToString) -> &mut Diag;
}

impl SExpDiag for Diagnostics {
    fn add_sexp(&mut self, asts: &ASTS, sexp: SExpId, message: impl ToString) -> &mut Diag {
        let sexp = asts.get(sexp);
        self.add(sexp, message)
    }
}

impl ariadne::Span for Span {
    type SourceId = SourceId;

    #[allow(clippy::misnamed_getters)]
    fn source(&self) -> &Self::SourceId {
        &self.source_id
    }

    fn start(&self) -> usize {
        self.range.start_byte
    }

    fn end(&self) -> usize {
        self.range.end_byte
    }
}

impl Diag {
    pub fn to_report(&self) -> Report<'_, Span> {
        let mut builder = Report::build(ariadne::ReportKind::Error, self.span)
            .with_message(&self.message)
            .with_label(Label::new(self.span).with_color(Color::Red));

        for extra in self.extras.iter() {
            if let Some(span) = &extra.span {
                builder = builder.with_label(
                    Label::new(*span)
                        .with_message(&extra.message)
                        .with_color(Color::Blue),
                );
            } else {
                builder = builder.with_note(extra.message.clone());
            }
        }

        builder.finish()
    }
}
