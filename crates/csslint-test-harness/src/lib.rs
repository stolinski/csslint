#![forbid(unsafe_code)]

use csslint_core::{FileId, Span};

pub mod stylelint_compat;

pub fn fixture_root() -> &'static str {
    "tests"
}

pub fn fixture_span(file_id: FileId, end: usize) -> (FileId, Span) {
    (file_id, Span::new(0, end))
}
