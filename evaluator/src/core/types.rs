use super::errors::EvalError;

pub type RowId = u64;
pub type Mask = Vec<bool>;
pub type NullMap = Vec<bool>;

#[derive(Clone, Copy, Debug)]
pub struct RowBatch<'a> {
    pub rows: &'a [RowId],
    pub batch_id: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
    Bool(bool),
    Date(i64),
    List(Vec<Value>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Column {
    F64(Vec<f64>),
    Any(Vec<Value>),
}

impl Column {
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::F64(values) => values.len(),
            Self::Any(values) => values.len(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColumnBlock {
    pub column: Column,
    pub nulls: NullMap,
}

impl ColumnBlock {
    pub(crate) fn len(&self) -> usize {
        self.column.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvalBlock {
    pub values: ColumnBlock,
    pub ok: Mask,
    pub errors: Vec<(usize, EvalError)>,
}

impl EvalBlock {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn fail_mask(mask: &Mask, error: EvalError) -> Self {
        let len = mask.len();
        let mut errors = Vec::new();
        for (idx, active) in mask.iter().copied().enumerate() {
            if active {
                errors.push((idx, error.clone()));
            }
        }
        Self {
            values: ColumnBlock {
                column: Column::Any(vec![Value::Number(0.0); len]),
                nulls: vec![true; len],
            },
            ok: vec![false; len],
            errors,
        }
    }
}
