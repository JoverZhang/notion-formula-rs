use analyzer::analysis::Ty;
use std::fmt;

use super::columns::AbiKind;
use super::inputs::InputSlot;
use super::types::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum EvalError {
    InvalidValueType {
        expected: Ty,
        actual: Ty,
    },
    InvalidValue {
        actual: Value,
        constraint: String,
    },
    InvalidRegex {
        pattern: String,
        detail: String,
    },
    InvalidDateText {
        text: String,
    },
    DateOutOfRange,
    /// Formula boundaries assign an identity once, then dependency reads preserve it.
    Originated {
        origin_formula_id: String,
        occurrence: u64,
        error: Box<EvalError>,
    },
    // Retained for internal plan/ABI invariants and compatibility.
    TypeMismatch,
    DivideByZero,
    InvalidArgument,
    InvalidDate,
    UnknownFunction,
    CycleDetected,
    PropertyDisabled,
}

impl EvalError {
    pub(crate) fn invalid_type(expected: Ty, actual: &Value) -> Self {
        Self::InvalidValueType {
            expected,
            actual: actual.value_type(),
        }
    }

    pub(crate) fn invalid_value(actual: Value, constraint: &str) -> Self {
        Self::InvalidValue {
            actual,
            constraint: constraint.to_string(),
        }
    }
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
    UnboundVariable(String),
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
