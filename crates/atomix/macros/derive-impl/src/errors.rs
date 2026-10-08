//! [`DeriveError`], why a derive refuses its input.

use proc_macro2::Span;
use syn::Error as SyntaxError;

/// Why a derive refuses its input: a message at a span of the user's code, with notes that say why
/// and a help that says what to write instead.
#[derive(Debug)]
pub struct DeriveError {
    /// Where it points.
    pub span: Span,
    /// What is wrong.
    pub message: String,
    /// Each note, at a span of its own or at the error's.
    pub notes: Vec<(Option<Span>, String)>,
    /// What to write instead.
    pub help: Option<String>,
}

impl DeriveError {
    /// An error of `message`, pointing at `span`, with no note or help.
    pub(crate) const fn new(span: Span, message: String) -> Self {
        Self { span, message, notes: Vec::new(), help: None }
    }

    /// The error with `note` added, pointing at `span`, or at the error's for `None`.
    pub(crate) fn note(mut self, span: Option<Span>, note: String) -> Self {
        self.notes.push((span, note));
        self
    }

    /// The error with `help`.
    pub(crate) fn help(mut self, help: String) -> Self {
        self.help = Some(help);
        self
    }
}

impl From<SyntaxError> for DeriveError {
    /// The error of `error`, one message of those syn gives where a definition does not parse.
    fn from(error: SyntaxError) -> Self {
        Self::new(error.span(), error.to_string())
    }
}
