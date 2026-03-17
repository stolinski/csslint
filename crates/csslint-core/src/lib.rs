#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(u32);

impl FileId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
    line_starts: Vec<usize>,
    source_len: usize,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let bytes = source.as_bytes();
        let mut line_starts = vec![0];
        let mut index = 0;

        while index < bytes.len() {
            match bytes[index] {
                b'\r' => {
                    if bytes.get(index + 1) == Some(&b'\n') {
                        line_starts.push(index + 2);
                        index += 2;
                    } else {
                        line_starts.push(index + 1);
                        index += 1;
                    }
                }
                b'\n' => {
                    line_starts.push(index + 1);
                    index += 1;
                }
                _ => {
                    index += 1;
                }
            }
        }

        Self {
            line_starts,
            source_len: bytes.len(),
        }
    }

    pub fn line_starts(&self) -> &[usize] {
        &self.line_starts
    }

    pub fn offset_to_line_column(&self, offset: usize) -> (usize, usize) {
        let clamped = offset.min(self.source_len);
        let line_index = match self.line_starts.binary_search(&clamped) {
            Ok(index) => index,
            Err(0) => 0,
            Err(index) => index - 1,
        };

        let line_start = self.line_starts[line_index];
        (line_index + 1, clamped - line_start + 1)
    }
}

pub const fn map_local_span_to_global(start_offset: usize, local: Span) -> Span {
    Span::new(start_offset + local.start, start_offset + local.end)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuleId(Cow<'static, str>);

impl RuleId {
    pub fn new(value: impl Into<Cow<'static, str>>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl PartialOrd for RuleId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RuleId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl From<&'static str> for RuleId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RuleId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Off,
    Warn,
    Error,
}

impl Severity {
    pub const fn emits_diagnostic(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scope {
    Global,
    VueScoped,
    VueModule,
    SvelteScoped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub span: Span,
    pub replacement: String,
    pub rule_id: RuleId,
    pub priority: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub rule_id: RuleId,
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    pub file_id: FileId,
    pub fix: Option<Fix>,
}

impl Diagnostic {
    pub fn new(
        rule_id: RuleId,
        severity: Severity,
        message: impl Into<String>,
        span: Span,
        file_id: FileId,
    ) -> Self {
        Self {
            rule_id,
            severity,
            message: message.into(),
            span,
            file_id,
            fix: None,
        }
    }

    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{map_local_span_to_global, LineIndex, Severity, Span};

    #[test]
    fn span_len_is_non_negative() {
        let span = Span::new(10, 4);
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn severity_off_does_not_emit() {
        assert!(!Severity::Off.emits_diagnostic());
        assert!(Severity::Warn.emits_diagnostic());
    }

    #[test]
    fn line_index_maps_lf_offsets() {
        let index = LineIndex::new("a\nbc\n");

        assert_eq!(index.line_starts(), &[0, 2, 5]);
        assert_eq!(index.offset_to_line_column(0), (1, 1));
        assert_eq!(index.offset_to_line_column(2), (2, 1));
        assert_eq!(index.offset_to_line_column(4), (2, 3));
        assert_eq!(index.offset_to_line_column(5), (3, 1));
    }

    #[test]
    fn line_index_maps_crlf_offsets() {
        let index = LineIndex::new("a\r\nbc\r\nz");

        assert_eq!(index.line_starts(), &[0, 3, 7]);
        assert_eq!(index.offset_to_line_column(3), (2, 1));
        assert_eq!(index.offset_to_line_column(6), (2, 4));
        assert_eq!(index.offset_to_line_column(7), (3, 1));
    }

    #[test]
    fn maps_local_spans_to_global_offsets() {
        let local = Span::new(3, 8);
        let global = map_local_span_to_global(40, local);

        assert_eq!(global.start, 43);
        assert_eq!(global.end, 48);
    }
}
