use analyzer::analysis::Ty;
use std::fmt;

use super::columns::AbiKind;
use super::inputs::InputSlot;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    TypeMismatch,
    DivideByZero,
    InvalidArgument,
    InvalidRegex,
    InvalidDate,
    UnknownFunction,
    CycleDetected,
    PropertyDisabled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputContractError {
    MissingColumn {
        slot: InputSlot,
        name: String,
    },
    DuplicateColumn {
        slot: InputSlot,
    },
    WrongKind {
        slot: InputSlot,
        expected: AbiKind,
        actual: AbiKind,
    },
    WrongLength {
        slot: InputSlot,
        expected: usize,
        actual: usize,
    },
    WrongInputLayout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrepareError {
    Semantic(Vec<String>),
    UnsupportedExpression,
    MissingResolvedCall,
    InvalidResolvedShape,
    UnknownProperty(String),
    UnsupportedType(Ty),
}

impl fmt::Display for InputContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for InputContractError {}

impl fmt::Display for PrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PrepareError {}
