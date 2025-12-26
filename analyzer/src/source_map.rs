pub struct SourceMap<'a> {
    _src: &'a str,
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
        Self {
            _src: src,
            line_starts,
        }
    }

    /// Returns (line, col), both 1-based.
    pub fn line_col(&self, byte: u32) -> (usize, usize) {
        let b = byte as usize;
        let line_idx = match self.line_starts.binary_search(&b) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let col = b.saturating_sub(self.line_starts[line_idx]);
        (line_idx + 1, col + 1)
    }
}
