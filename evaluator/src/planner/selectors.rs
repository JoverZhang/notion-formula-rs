use analyzer::analysis::Ty;
use analyzer::ast::BinOpKind;

use crate::ir::nodes::{BinaryExecKey, BinaryPlan, CastPlan};

use super::PlanError;

struct AddSelector;

impl AddSelector {
    fn select(left_ty: &Ty, right_ty: &Ty) -> BinaryPlan {
        if *left_ty == Ty::Number && *right_ty == Ty::Number {
            return BinaryPlan {
                key: BinaryExecKey::AddF64,
                left_cast: CastPlan::ToF64,
                right_cast: CastPlan::ToF64,
            };
        }

        BinaryPlan {
            key: BinaryExecKey::AddAny,
            left_cast: CastPlan::None,
            right_cast: CastPlan::None,
        }
    }
}

struct SubSelector;

impl SubSelector {
    fn select(left_ty: &Ty, right_ty: &Ty) -> Result<BinaryPlan, PlanError> {
        if *left_ty == Ty::Number && *right_ty == Ty::Number {
            return Ok(BinaryPlan {
                key: BinaryExecKey::SubF64,
                left_cast: CastPlan::ToF64,
                right_cast: CastPlan::ToF64,
            });
        }
        Err(PlanError::TypeMismatch)
    }
}

struct MulSelector;

impl MulSelector {
    fn select(left_ty: &Ty, right_ty: &Ty) -> Result<BinaryPlan, PlanError> {
        if *left_ty == Ty::Number && *right_ty == Ty::Number {
            return Ok(BinaryPlan {
                key: BinaryExecKey::MulF64,
                left_cast: CastPlan::ToF64,
                right_cast: CastPlan::ToF64,
            });
        }
        Err(PlanError::TypeMismatch)
    }
}

struct DivSelector;

impl DivSelector {
    fn select(left_ty: &Ty, right_ty: &Ty) -> Result<BinaryPlan, PlanError> {
        if *left_ty == Ty::Number && *right_ty == Ty::Number {
            return Ok(BinaryPlan {
                key: BinaryExecKey::DivF64,
                left_cast: CastPlan::ToF64,
                right_cast: CastPlan::ToF64,
            });
        }
        Err(PlanError::TypeMismatch)
    }
}

pub(crate) fn select_binary_plan(
    op: BinOpKind,
    left_ty: &Ty,
    right_ty: &Ty,
) -> Result<BinaryPlan, PlanError> {
    match op {
        BinOpKind::Plus => Ok(AddSelector::select(left_ty, right_ty)),
        BinOpKind::Minus => SubSelector::select(left_ty, right_ty),
        BinOpKind::Star => MulSelector::select(left_ty, right_ty),
        BinOpKind::Slash => DivSelector::select(left_ty, right_ty),
        _ => Err(PlanError::InvalidArgument),
    }
}
