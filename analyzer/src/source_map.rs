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

    /// Returns `(line, col)`, both 1-based.
    ///
    /// `byte` is interpreted as a UTF-8 byte offset into `src` and is clamped down to the nearest
    /// valid UTF-8 char boundary (never panics).
    ///
    /// `col` is computed as the number of Rust `char`s (Unicode scalar values) from the start of
    /// the line to the clamped position.
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
