use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use super::errors::EvalError;
use super::types::{Mask, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AbiKind {
    Number,
    Boolean,
    Text,
    Date,
    List,
    Any,
}

pub struct SharedStorage<S> {
    inner: Arc<S>,
}

impl<S> SharedStorage<S> {
    pub fn from_owned(storage: S) -> Self {
        Self {
            inner: Arc::new(storage),
        }
    }

    pub fn get(&self) -> &S {
        &self.inner
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    pub fn shares_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn try_into_unique(self) -> Result<S, Self> {
        Arc::try_unwrap(self.inner).map_err(|inner| Self { inner })
    }
}

impl<S> Clone for SharedStorage<S> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<S: fmt::Debug> fmt::Debug for SharedStorage<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SharedStorage")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S: PartialEq> PartialEq for SharedStorage<S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedBitmap {
    inner: Arc<[bool]>,
}

impl SharedBitmap {
    pub fn new(bits: impl Into<Vec<bool>>) -> Self {
        Self {
            inner: Arc::from(bits.into().into_boxed_slice()),
        }
    }

    pub fn as_slice(&self) -> &[bool] {
        &self.inner
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Validity {
    AllValid,
    AllNull,
    Bitmap(SharedBitmap),
}

impl Validity {
    pub fn from_valid_bits(bits: Vec<bool>) -> Self {
        if bits.iter().all(|valid| *valid) {
            Self::AllValid
        } else if bits.iter().all(|valid| !*valid) {
            Self::AllNull
        } else {
            Self::Bitmap(SharedBitmap::new(bits))
        }
    }

    pub fn is_valid(&self, index: usize) -> bool {
        match self {
            Self::AllValid => true,
            Self::AllNull => false,
            Self::Bitmap(bits) => bits.as_slice()[index],
        }
    }

    pub fn bitmap_len(&self) -> Option<usize> {
        match self {
            Self::Bitmap(bits) => Some(bits.len()),
            Self::AllValid | Self::AllNull => None,
        }
    }

    pub(crate) fn normalize_inactive(&self, mask: &Mask) -> Self {
        debug_assert!(self.bitmap_len().is_none_or(|len| len == mask.len()));
        if mask.iter().all(|active| active) {
            return self.clone();
        }
        Self::from_valid_bits(
            (0..mask.len())
                .map(|row| !mask[row] || self.is_valid(row))
                .collect(),
        )
    }
}

pub trait ColumnKind: Sized + Send + Sync + 'static {
    type Scalar: Clone + fmt::Debug + PartialEq + Send + Sync + 'static;
    type Storage: AsRef<[Self::Scalar]>
        + AsMut<[Self::Scalar]>
        + From<Vec<Self::Scalar>>
        + fmt::Debug
        + PartialEq
        + Send
        + Sync
        + 'static;

    const ABI_KIND: AbiKind;
    fn placeholder() -> Self::Scalar;
    fn from_value(value: Value) -> Result<Self::Scalar, Value>;
    fn to_value(value: &Self::Scalar) -> Value;
    fn into_column(column: KernelColumn<Self>) -> Column;
    fn from_column(column: Column) -> Result<KernelColumn<Self>, Column>;
}

pub struct KernelColumn<K: ColumnKind> {
    storage: SharedStorage<K::Storage>,
    validity: Validity,
    kind: PhantomData<K>,
}

impl<K: ColumnKind> KernelColumn<K> {
    pub fn from_owned(storage: K::Storage, validity: Validity) -> Self {
        if let Some(validity_len) = validity.bitmap_len() {
            debug_assert_eq!(storage.as_ref().len(), validity_len);
        }
        Self {
            storage: SharedStorage::from_owned(storage),
            validity,
            kind: PhantomData,
        }
    }

    pub fn from_values(values: Vec<K::Scalar>, validity: Validity) -> Self {
        Self::from_owned(K::Storage::from(values), validity)
    }

    pub fn values(&self) -> &[K::Scalar] {
        self.storage.get().as_ref()
    }

    pub fn validity(&self) -> &Validity {
        &self.validity
    }

    pub fn len(&self) -> usize {
        self.values().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn value(&self, index: usize) -> Option<&K::Scalar> {
        self.validity.is_valid(index).then(|| &self.values()[index])
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        self.storage.shares_with(&other.storage)
    }

    pub fn storage_strong_count(&self) -> usize {
        self.storage.strong_count()
    }

    pub fn try_into_unique(self) -> Result<(K::Storage, Validity), Self> {
        let Self {
            storage,
            validity,
            kind,
        } = self;
        match storage.try_into_unique() {
            Ok(storage) => Ok((storage, validity)),
            Err(storage) => Err(Self {
                storage,
                validity,
                kind,
            }),
        }
    }

    pub fn into_column(self) -> Column {
        K::into_column(self)
    }

    pub(crate) fn with_validity(self, validity: Validity) -> Self {
        debug_assert!(validity.bitmap_len().is_none_or(|len| len == self.len()));
        Self { validity, ..self }
    }
}

impl<K: ColumnKind> Clone for KernelColumn<K> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
            validity: self.validity.clone(),
            kind: PhantomData,
        }
    }
}

impl<K: ColumnKind> fmt::Debug for KernelColumn<K> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KernelColumn")
            .field("abi_kind", &K::ABI_KIND)
            .field("storage", &self.storage)
            .field("validity", &self.validity)
            .finish()
    }
}

impl<K: ColumnKind> PartialEq for KernelColumn<K> {
    fn eq(&self, other: &Self) -> bool {
        self.storage == other.storage && self.validity == other.validity
    }
}

macro_rules! define_kind {
    (
        $kind:ident,
        $scalar:ty,
        $variant:ident,
        $abi:ident,
        $placeholder:expr,
        $pattern:pat => $from_value:expr,
        $to_value:expr
    ) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $kind;

        impl ColumnKind for $kind {
            type Scalar = $scalar;
            type Storage = Box<[$scalar]>;
            const ABI_KIND: AbiKind = AbiKind::$abi;

            fn placeholder() -> Self::Scalar {
                $placeholder
            }

            fn from_value(value: Value) -> Result<Self::Scalar, Value> {
                match value {
                    $pattern => Ok($from_value),
                    other => Err(other),
                }
            }

            fn to_value(value: &Self::Scalar) -> Value {
                ($to_value)(value)
            }

            fn into_column(column: KernelColumn<Self>) -> Column {
                Column::$variant(column)
            }

            fn from_column(column: Column) -> Result<KernelColumn<Self>, Column> {
                match column {
                    Column::$variant(column) => Ok(column),
                    other => Err(other),
                }
            }
        }
    };
}

define_kind!(
    NumberKind,
    f64,
    Number,
    Number,
    0.0,
    Value::Number(value) => value,
    |value: &f64| Value::Number(*value)
);
define_kind!(
    BooleanKind,
    bool,
    Boolean,
    Boolean,
    false,
    Value::Bool(value) => value,
    |value: &bool| Value::Bool(*value)
);
define_kind!(
    TextKind,
    String,
    Text,
    Text,
    String::new(),
    Value::Text(value) => value,
    |value: &String| Value::Text(value.clone())
);
define_kind!(
    DateKind,
    i64,
    Date,
    Date,
    0,
    Value::Date(value) => value,
    |value: &i64| Value::Date(*value)
);
define_kind!(
    ListKind,
    Vec<Value>,
    List,
    List,
    Vec::new(),
    Value::List(value) => value,
    |value: &Vec<Value>| Value::List(value.clone())
);

#[derive(Clone, Copy, Debug)]
pub struct AnyKind;

impl ColumnKind for AnyKind {
    type Scalar = Value;
    type Storage = Box<[Value]>;
    const ABI_KIND: AbiKind = AbiKind::Any;

    fn placeholder() -> Self::Scalar {
        Value::Number(0.0)
    }

    fn from_value(value: Value) -> Result<Self::Scalar, Value> {
        Ok(value)
    }

    fn to_value(value: &Self::Scalar) -> Value {
        value.clone()
    }

    fn into_column(column: KernelColumn<Self>) -> Column {
        Column::Any(column)
    }

    fn from_column(column: Column) -> Result<KernelColumn<Self>, Column> {
        match column {
            Column::Any(column) => Ok(column),
            other => Err(other),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Column {
    Number(KernelColumn<NumberKind>),
    Boolean(KernelColumn<BooleanKind>),
    Text(KernelColumn<TextKind>),
    Date(KernelColumn<DateKind>),
    List(KernelColumn<ListKind>),
    Any(KernelColumn<AnyKind>),
}

impl Column {
    pub fn abi_kind(&self) -> AbiKind {
        match self {
            Self::Number(_) => AbiKind::Number,
            Self::Boolean(_) => AbiKind::Boolean,
            Self::Text(_) => AbiKind::Text,
            Self::Date(_) => AbiKind::Date,
            Self::List(_) => AbiKind::List,
            Self::Any(_) => AbiKind::Any,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Number(column) => column.len(),
            Self::Boolean(column) => column.len(),
            Self::Text(column) => column.len(),
            Self::Date(column) => column.len(),
            Self::List(column) => column.len(),
            Self::Any(column) => column.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn validity(&self) -> &Validity {
        match self {
            Self::Number(column) => column.validity(),
            Self::Boolean(column) => column.validity(),
            Self::Text(column) => column.validity(),
            Self::Date(column) => column.validity(),
            Self::List(column) => column.validity(),
            Self::Any(column) => column.validity(),
        }
    }

    pub fn row_value(&self, index: usize) -> Option<Value> {
        if !self.validity().is_valid(index) {
            return None;
        }
        Some(match self {
            Self::Number(column) => Value::Number(column.values()[index]),
            Self::Boolean(column) => Value::Bool(column.values()[index]),
            Self::Text(column) => Value::Text(column.values()[index].clone()),
            Self::Date(column) => Value::Date(column.values()[index]),
            Self::List(column) => Value::List(column.values()[index].clone()),
            Self::Any(column) => column.values()[index].clone(),
        })
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left.shares_storage_with(right),
            (Self::Boolean(left), Self::Boolean(right)) => left.shares_storage_with(right),
            (Self::Text(left), Self::Text(right)) => left.shares_storage_with(right),
            (Self::Date(left), Self::Date(right)) => left.shares_storage_with(right),
            (Self::List(left), Self::List(right)) => left.shares_storage_with(right),
            (Self::Any(left), Self::Any(right)) => left.shares_storage_with(right),
            _ => false,
        }
    }

    pub(crate) fn normalize_inactive(self, mask: &Mask) -> Self {
        let validity = self.validity().normalize_inactive(mask);
        match self {
            Self::Number(column) => Self::Number(column.with_validity(validity)),
            Self::Boolean(column) => Self::Boolean(column.with_validity(validity)),
            Self::Text(column) => Self::Text(column.with_validity(validity)),
            Self::Date(column) => Self::Date(column.with_validity(validity)),
            Self::List(column) => Self::List(column.with_validity(validity)),
            Self::Any(column) => Self::Any(column.with_validity(validity)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KernelResult<K: ColumnKind> {
    pub column: KernelColumn<K>,
    pub ok: Mask,
    pub errors: Vec<(usize, EvalError)>,
}

impl<K: ColumnKind> KernelResult<K> {
    pub fn into_eval_block(self) -> super::types::EvalBlock {
        super::types::EvalBlock::new(K::into_column(self.column), self.ok, self.errors)
    }
}
