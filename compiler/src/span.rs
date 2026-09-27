//! Source positions.
//!
//! A [`Span`] is a half-open byte range `start..end` into the (newline-normalized)
//! source text. Every token, AST node, and diagnostic carries one. Never drop them:
//! spans are what make error underlines and current-line highlighting possible.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        debug_assert!(start <= end, "span start {start} is after end {end}");
        Span { start, end }
    }

    /// The smallest span covering both `self` and `other`.
    pub fn to(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }

    pub fn len(self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// The source text this span covers.
    pub fn text(self, source: &str) -> &str {
        &source[self.start as usize..self.end as usize]
    }
}

/// Maps byte offsets to 1-based line and column numbers.
///
/// Columns count Unicode scalar values (chars), not bytes, so an underline
/// lines up under the right character in a terminal.
pub struct LineIndex<'a> {
    source: &'a str,
    line_starts: Vec<u32>,
}

impl<'a> LineIndex<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i as u32 + 1);
            }
        }
        LineIndex {
            source,
            line_starts,
        }
    }

    /// 0-based line index containing `offset`.
    fn line_of(&self, offset: u32) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next) => next - 1,
        }
    }

    /// 1-based `(line, column)` of a byte offset.
    pub fn line_col(&self, offset: u32) -> (u32, u32) {
        let line = self.line_of(offset);
        let start = self.line_starts[line] as usize;
        let col = self.source[start..offset as usize].chars().count();
        (line as u32 + 1, col as u32 + 1)
    }

    /// Text of a 1-based line, without its trailing newline.
    pub fn line_text(&self, line: u32) -> &'a str {
        let i = line as usize - 1;
        let start = self.line_starts[i] as usize;
        let end = self
            .line_starts
            .get(i + 1)
            .map_or(self.source.len(), |&s| s as usize - 1);
        &self.source[start..end]
    }
}

/// Converts `\r\n` and lone `\r` to `\n`.
///
/// `compile` does this before lexing so that byte offsets agree with the
/// editor, which normalizes line endings the same way.
pub fn normalize_newlines(source: &str) -> std::borrow::Cow<'_, str> {
    if source.contains('\r') {
        source.replace("\r\n", "\n").replace('\r', "\n").into()
    } else {
        source.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_basics() {
        let src = "ab\ncd\n\nef";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_col(0), (1, 1));
        assert_eq!(idx.line_col(1), (1, 2));
        assert_eq!(idx.line_col(2), (1, 3)); // the '\n' itself
        assert_eq!(idx.line_col(3), (2, 1));
        assert_eq!(idx.line_col(6), (3, 1));
        assert_eq!(idx.line_col(7), (4, 1));
        assert_eq!(idx.line_col(9), (4, 3)); // end of file
        assert_eq!(idx.line_text(2), "cd");
        assert_eq!(idx.line_text(3), "");
        assert_eq!(idx.line_text(4), "ef");
    }

    #[test]
    fn columns_count_chars_not_bytes() {
        let src = "// é\nx";
        let idx = LineIndex::new(src);
        let e_end = src.find('\n').unwrap() as u32;
        assert_eq!(idx.line_col(e_end), (1, 5));
    }

    #[test]
    fn normalizes_crlf() {
        assert_eq!(normalize_newlines("a\r\nb\rc\n"), "a\nb\nc\n");
    }
}
