use std::sync::atomic::{AtomicU64, Ordering};

use analyzer::analysis::Ty;

use super::columns::{AbiKind, Column};
use super::context::BuiltinRuntimeContext;
use super::errors::InputContractError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InputSlot {
    layout: InputLayoutId,
    index: u32,
}

impl InputSlot {
    pub(crate) fn new(layout: InputLayoutId, index: usize) -> Self {
        Self {
            layout,
            index: u32::try_from(index).expect("input slot count exceeds u32"),
        }
    }

    pub fn index(self) -> usize {
        self.index as usize
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiredColumn {
    pub slot: InputSlot,
    pub name: String,
    pub expected_type: Ty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct InputLayoutId(u64);

impl InputLayoutId {
    pub(crate) fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub struct EvalInputsBuilder {
    runtime: BuiltinRuntimeContext,
    columns: Vec<(InputSlot, Column)>,
}

impl EvalInputsBuilder {
    pub fn new(runtime: BuiltinRuntimeContext) -> Self {
        Self {
            runtime,
            columns: Vec::new(),
        }
    }

    pub fn insert(&mut self, slot: InputSlot, column: Column) -> &mut Self {
        self.columns.push((slot, column));
        self
    }

    pub fn with_column(mut self, slot: InputSlot, column: Column) -> Self {
        self.insert(slot, column);
        self
    }

    pub fn finish(
        self,
        prepared: &crate::PreparedFormula,
        batch_len: usize,
    ) -> Result<EvalInputs, InputContractError> {
        let required = prepared.required_columns();
        let mut columns = std::iter::repeat_with(|| None)
            .take(required.len())
            .collect::<Vec<Option<Column>>>();

        for (slot, column) in self.columns {
            if slot.layout != prepared.input_layout_id() {
                return Err(InputContractError::WrongInputLayout);
            }
            let Some(requirement) = required.get(slot.index()) else {
                return Err(InputContractError::WrongInputLayout);
            };
            if requirement.slot != slot {
                return Err(InputContractError::WrongInputLayout);
            }
            if columns[slot.index()].is_some() {
                return Err(InputContractError::DuplicateColumn { slot });
            }

            let expected = abi_kind_for_ty(&requirement.expected_type);
            let actual = column.abi_kind();
            if expected != actual {
                return Err(InputContractError::WrongKind {
                    slot,
                    expected,
                    actual,
                });
            }
            if column.len() != batch_len {
                return Err(InputContractError::WrongLength {
                    slot,
                    expected: batch_len,
                    actual: column.len(),
                });
            }
            if let Some(actual) = column.validity().bitmap_len()
                && actual != batch_len
            {
                return Err(InputContractError::WrongLength {
                    slot,
                    expected: batch_len,
                    actual,
                });
            }
            columns[slot.index()] = Some(column);
        }

        let columns = columns
            .into_iter()
            .zip(required)
            .map(|(column, requirement)| {
                column.ok_or_else(|| InputContractError::MissingColumn {
                    slot: requirement.slot,
                    name: requirement.name.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(EvalInputs {
            layout: prepared.input_layout_id(),
            batch_len,
            columns: columns.into_boxed_slice(),
            runtime: self.runtime,
        })
    }
}

#[derive(Clone, Debug)]
pub struct EvalInputs {
    layout: InputLayoutId,
    batch_len: usize,
    columns: Box<[Column]>,
    runtime: BuiltinRuntimeContext,
}

impl EvalInputs {
    pub(crate) fn layout(&self) -> InputLayoutId {
        self.layout
    }

    pub fn batch_len(&self) -> usize {
        self.batch_len
    }

    pub fn column(&self, slot: InputSlot) -> Option<&Column> {
        self.columns.get(slot.index())
    }

    pub fn runtime(&self) -> &BuiltinRuntimeContext {
        &self.runtime
    }
}

pub fn abi_kind_for_ty(ty: &Ty) -> AbiKind {
    match ty {
        Ty::Number => AbiKind::Number,
        Ty::Boolean => AbiKind::Boolean,
        Ty::String => AbiKind::Text,
        Ty::Date => AbiKind::Date,
        Ty::List(_) => AbiKind::List,
        Ty::Union(members) => {
            let mut kinds = members
                .iter()
                .filter(|member| !matches!(member, Ty::Null))
                .map(abi_kind_for_ty);
            let Some(first) = kinds.next() else {
                return AbiKind::Any;
            };
            if kinds.all(|kind| kind == first) {
                first
            } else {
                AbiKind::Any
            }
        }
        Ty::Null | Ty::Unknown | Ty::Generic(_) | Ty::Fn { .. } | Ty::Ident(_) => AbiKind::Any,
    }
}
