use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinSigParseErrorKind {
    UnexpectedChar { ch: char },
    UnexpectedToken { expected: String, found: String },
    MissingArrow,
    MalformedParameter,
    MalformedType,
    InvalidRepeatGroupPlacement,
    DuplicateGenericName { name: String },
    UnknownGenericKind { name: String },
    UnknownGenericReference { name: String },
    RestParamMustUseListType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinSigParseError {
    pub kind: BuiltinSigParseErrorKind,
    pub position: usize,
}

impl BuiltinSigParseError {
    pub(crate) fn new(kind: BuiltinSigParseErrorKind, position: usize) -> Self {
        Self { kind, position }
    }
}

impl fmt::Display for BuiltinSigParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BuiltinSigParseErrorKind as Kind;

        match &self.kind {
            Kind::UnexpectedChar { ch } => {
                write!(f, "unexpected character `{ch}` at byte {}", self.position)
            }
            Kind::UnexpectedToken { expected, found } => write!(
                f,
                "expected {expected}, found {found} at byte {}",
                self.position
            ),
            Kind::MissingArrow => write!(f, "missing `->` at byte {}", self.position),
            Kind::MalformedParameter => write!(f, "malformed parameter at byte {}", self.position),
            Kind::MalformedType => write!(f, "malformed type at byte {}", self.position),
            Kind::InvalidRepeatGroupPlacement => {
                write!(
                    f,
                    "invalid repeat-group placement at byte {}",
                    self.position
                )
            }
            Kind::DuplicateGenericName { name } => {
                write!(f, "duplicate generic `{name}` at byte {}", self.position)
            }
            Kind::UnknownGenericKind { name } => {
                write!(f, "unknown generic kind `{name}` at byte {}", self.position)
            }
            Kind::UnknownGenericReference { name } => write!(
                f,
                "unknown generic reference `{name}` at byte {}",
                self.position
            ),
            Kind::RestParamMustUseListType => write!(
                f,
                "rest parameters must use a list type at byte {}",
                self.position
            ),
        }
    }
}

impl std::error::Error for BuiltinSigParseError {}
