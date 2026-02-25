mod lower_lit;
mod planner;
mod selectors;

use crate::core::errors::EvalError;

pub(crate) use planner::Planner;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PlanError {
    TypeMismatch,
    InvalidArgument,
}

impl From<PlanError> for EvalError {
    fn from(error: PlanError) -> Self {
        match error {
            PlanError::TypeMismatch => Self::TypeMismatch,
            PlanError::InvalidArgument => Self::InvalidArgument,
        }
    }
}
