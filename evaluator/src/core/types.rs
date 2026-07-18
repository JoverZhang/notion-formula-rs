use std::fmt;
use std::ops::Index;
use std::sync::Arc;

use super::columns::{AnyKind, Column, KernelColumn, Validity};
use super::errors::EvalError;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RowId(Arc<str>);

impl RowId {
    pub fn new(value: impl Into<Arc<str>>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for RowId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RowId {
    fn from(value: String) -> Self {
        Self::new(Arc::<str>::from(value))
    }
}

impl From<u64> for RowId {
    fn from(value: u64) -> Self {
        Self::from(value.to_string())
    }
}

impl fmt::Display for RowId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowBatch {
    rows: Box<[RowId]>,
    pub batch_id: u64,
}

impl RowBatch {
    pub fn new(rows: impl IntoIterator<Item = RowId>, batch_id: u64) -> Self {
        Self {
            rows: rows.into_iter().collect(),
            batch_id,
        }
    }

    pub fn rows(&self) -> &[RowId] {
        &self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
    Bool(bool),
    Date(i64),
    List(Vec<Value>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mask(Box<[bool]>);

impl Mask {
    pub fn all(len: usize) -> Self {
        Self(vec![true; len].into_boxed_slice())
    }

    pub fn none(len: usize) -> Self {
        Self(vec![false; len].into_boxed_slice())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_slice(&self) -> &[bool] {
        &self.0
    }

    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.0.iter().copied()
    }

    pub fn get(&self, index: usize) -> bool {
        self.0[index]
    }

    pub fn set(&mut self, index: usize, value: bool) {
        self.0[index] = value;
    }

    pub fn and(&self, other: &Self) -> Self {
        assert_eq!(self.len(), other.len(), "mask length mismatch");
        Self::from(
            self.iter()
                .zip(other.iter())
                .map(|(left, right)| left && right)
                .collect::<Vec<_>>(),
        )
    }

    pub fn any(&self) -> bool {
        self.iter().any(|active| active)
    }
}

impl From<Vec<bool>> for Mask {
    fn from(value: Vec<bool>) -> Self {
        Self(value.into_boxed_slice())
    }
}

impl FromIterator<bool> for Mask {
    fn from_iter<T: IntoIterator<Item = bool>>(iter: T) -> Self {
        Self::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl Index<usize> for Mask {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvalBlock {
    pub column: Column,
    pub ok: Mask,
    pub errors: Vec<(usize, EvalError)>,
}

impl EvalBlock {
    pub fn new(column: Column, ok: Mask, errors: Vec<(usize, EvalError)>) -> Self {
        debug_assert_eq!(column.len(), ok.len());
        debug_assert!(errors.iter().all(|(row, _)| *row < ok.len() && !ok[*row]));
        Self { column, ok, errors }
    }

    pub fn len(&self) -> usize {
        self.column.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn validity(&self) -> &Validity {
        self.column.validity()
    }

    pub(crate) fn fail_mask(mask: &Mask, error: EvalError) -> Self {
        let errors = mask
            .iter()
            .enumerate()
            .filter(|(_, active)| *active)
            .map(|(index, _)| (index, error.clone()))
            .collect();
        let mut ok = Mask::all(mask.len());
        for (row, active) in mask.iter().enumerate() {
            if active {
                ok.set(row, false);
            }
        }
        Self {
            column: Column::Any(KernelColumn::<AnyKind>::from_values(
                vec![Value::Number(0.0); mask.len()],
                Validity::AllValid,
            )),
            ok,
            errors,
        }
    }
}
