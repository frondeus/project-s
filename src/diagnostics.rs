use std::{io::BufWriter, sync::Arc};

use ariadne::{Color, Label, Report};
use itertools::Itertools;

use crate::source::Span;

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

impl Diagnostics {
    pub fn add(&mut self, span: Span, message: impl ToString) -> &mut Diag {
        self.diags.push(Diag {
            span,
            message: message.to_string(),
            extras: Vec::new(),
        });
        self.diags.last_mut().unwrap()
    }

    pub fn has_errors(&self) -> bool {
        !self.diags.is_empty()
    }

    pub fn print(self) -> String {
        let mut out = String::new();
        for diag in self.diags {
            out.push_str(&format!("{}: {}\n", diag.span.filename, diag.message));
        }
        out
    }

    pub fn pretty_print(&self) -> String {
        let mut cache = ariadne::sources(
            self.diags
                .iter()
                .flat_map(|d| {
                    std::iter::once(d.span.clone())
                        .chain(d.extras.iter().filter_map(|e| e.span.clone()))
                })
                .map(|span| (span.filename.clone(), span.source.clone()))
                .unique()
                .collect::<Vec<_>>(),
        );
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

impl ariadne::Span for Span {
    type SourceId = Arc<str>;

    #[allow(clippy::misnamed_getters)]
    fn source(&self) -> &Self::SourceId {
        &self.filename
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
        let mut builder = Report::build(ariadne::ReportKind::Error, self.span.clone())
            .with_message(&self.message)
            .with_label(Label::new(self.span.clone()).with_color(Color::Red));

        for extra in self.extras.iter() {
            if let Some(span) = &extra.span {
                builder = builder.with_label(
                    Label::new(span.clone())
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
