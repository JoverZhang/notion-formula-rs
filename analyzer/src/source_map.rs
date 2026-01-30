//! Line/column lookup for a source string.
//!
//! Input byte offsets are clamped down to a UTF-8 char boundary.
//! The column is a Rust `char` count (Unicode scalar values), not bytes or UTF-16.

pub struct SourceMap<'a> {
    src: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> SourceMap<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { src, line_starts }
    }

    /// Return `(line, col)`, both 1-based.
    /// `byte` is a UTF-8 byte offset into `src`.
    pub fn line_col(&self, byte: u32) -> (usize, usize) {
        let b = clamp_to_char_boundary(self.src, byte as usize);
        let line_idx = match self.line_starts.binary_search(&b) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line_start = self.line_starts[line_idx];
        let col = self.src[line_start..b].chars().count();
        (line_idx + 1, col + 1)
    }
}

fn clamp_to_char_boundary(source: &str, mut byte: usize) -> usize {
    if byte > source.len() {
        byte = source.len();
    }
    while !source.is_char_boundary(byte) {
        byte = byte.saturating_sub(1);
    }
    byte
}
