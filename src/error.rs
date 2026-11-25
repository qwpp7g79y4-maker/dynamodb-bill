use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(String);

impl Error {
    pub fn msg(m: impl Into<String>) -> Self {
        Self(m.into())
    }
    pub fn message(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&clean(&self.0))
    }
}

/// Replace control characters (newline, tab, ESC, BEL, C1, line/paragraph
/// separators) with `?` so a value or a path cannot fake a line or emit a
/// terminal sequence. Used for the text table and for everything on stderr.
pub(crate) fn clean(s: &str) -> String {
    clean_with(s, false)
}

/// Same as [`clean`], but keeps `\n` (for multi-line clap messages).
pub fn clean_with(s: &str, keep_newline: bool) -> String {
    s.chars()
        .map(|c| {
            if (keep_newline && c == '\n')
                || !(c.is_control() || c == '\u{2028}' || c == '\u{2029}')
            {
                c
            } else {
                '?'
            }
        })
        .collect()
}
impl std::error::Error for Error {}
