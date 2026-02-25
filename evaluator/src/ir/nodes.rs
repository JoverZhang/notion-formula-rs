use crate::core::types::Value;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinaryExecKey {
    AddF64 = 0,
    AddAny = 1,
    SubF64 = 2,
    MulF64 = 3,
    DivF64 = 4,
}

pub(crate) const BINARY_EXEC_KEY_COUNT: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CastPlan {
    None,
    ToF64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BinaryPlan {
    pub key: BinaryExecKey,
    pub left_cast: CastPlan,
    pub right_cast: CastPlan,
}

#[derive(Clone, Debug)]
pub(crate) enum ExecNode {
    LiteralF64(f64),
    LiteralAny(Value),
    CastToF64 {
        input: Box<ExecNode>,
    },
    Binary {
        key: BinaryExecKey,
        left: Box<ExecNode>,
        right: Box<ExecNode>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ExecPlan {
    pub root: ExecNode,
}
