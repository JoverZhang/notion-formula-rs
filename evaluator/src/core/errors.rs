#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderError {
    NotFound,
    BackendError,
    Timeout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    TypeMismatch,
    DivideByZero,
    UnknownFunction,
    InvalidArgument,
    CycleDetected,
    PropertyDisabled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimpleEvalError {
    Provider(ProviderError),
    FirstRowError {
        row_index: usize,
        reason: EvalError,
        total: usize,
    },
}
