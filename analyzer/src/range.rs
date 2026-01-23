#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BytePos(u32);

impl BytePos {
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl From<u32> for BytePos {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

/// A half-open byte span in source text: `[start, end)`.
///
/// `start` and `end` are byte offsets. `start == end` is an empty span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteSpan {
    pub start: BytePos,
    pub end: BytePos,
}

impl ByteSpan {
    pub fn new(start: u32, end: u32) -> Self {
        debug_assert!(start <= end);
        Self {
            start: BytePos::from(start),
            end: BytePos::from(end),
        }
    }

    pub fn start_u32(self) -> u32 {
        self.start.as_u32()
    }

    pub fn end_u32(self) -> u32 {
        self.end.as_u32()
    }
}

impl From<(u32, u32)> for ByteSpan {
    fn from((start, end): (u32, u32)) -> Self {
        Self::new(start, end)
    }
}
