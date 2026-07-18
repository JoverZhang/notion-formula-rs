use std::sync::atomic::{AtomicU64, Ordering};

use analyzer::analysis::Ty;
use analyzer::ast::{BinOpKind, UnOp};
use builtin_fn::ParamRef;

use crate::builtins::BuiltinKey;
use crate::core::columns::AbiKind;
use crate::core::inputs::InputSlot;
use crate::core::types::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PlanId(u32);

impl PlanId {
    pub(crate) fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("execution plan exceeds u32 node capacity"))
    }

    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PlanOwner(u64);

impl PlanOwner {
    pub(crate) fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ExecPlan {
    owner: PlanOwner,
    nodes: Box<[ExecNode]>,
    root: PlanId,
}

impl ExecPlan {
    pub(crate) fn new(owner: PlanOwner, nodes: Vec<ExecNode>, root: PlanId) -> Self {
        debug_assert!(root.index() < nodes.len());
        Self {
            owner,
            nodes: nodes.into_boxed_slice(),
            root,
        }
    }

    pub(crate) fn owner(&self) -> PlanOwner {
        self.owner
    }

    pub(crate) fn root(&self) -> PlanId {
        self.root
    }

    pub(crate) fn node(&self, id: PlanId) -> &ExecNode {
        &self.nodes[id.index()]
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ExecNode {
    Literal(Value),
    List(Box<[PlanId]>),
    Input(InputSlot),
    Variable(String),
    Unary {
        op: UnOp,
        input: PlanId,
    },
    Binary {
        op: BinOpKind,
        left: PlanId,
        right: PlanId,
    },
    Ternary {
        condition: PlanId,
        then_plan: PlanId,
        else_plan: PlanId,
    },
    Cast {
        input: PlanId,
        target: AbiKind,
    },
    Builtin(BuiltinCallNode),
}

#[derive(Clone, Debug)]
pub(crate) struct BuiltinCallNode {
    pub key: BuiltinKey,
    pub arguments: Box<[PlannedArgument]>,
    #[cfg(debug_assertions)]
    pub debug_contract: DebugCallContract,
}

impl BuiltinCallNode {
    pub(crate) fn debug_contract(&self) -> Option<&DebugCallContract> {
        #[cfg(debug_assertions)]
        {
            Some(&self.debug_contract)
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PlannedArgument {
    pub parameter: ParamRef,
    pub repeat_group: Option<usize>,
    pub kind: PlannedArgumentKind,
}

#[derive(Clone, Debug)]
pub(crate) enum PlannedArgumentKind {
    Value(PlanId),
    Thunk {
        body: PlanId,
    },
    Lambda {
        body: PlanId,
        parameters: Box<[String]>,
    },
    Binder {
        name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DebugCallContract {
    pub arguments: Box<[DebugArgumentContract]>,
    pub return_ty: Ty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DebugArgumentContract {
    pub parameter: ParamRef,
    pub repeat_group: Option<usize>,
    pub expected_ty: Ty,
}
