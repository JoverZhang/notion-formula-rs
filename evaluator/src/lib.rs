use analyzer::analysis::{Property, Ty};
use analyzer::ast::{BinOpKind, Expr, ExprKind};
use analyzer::LitKind;
use core::future::Future;
use std::collections::HashMap;
use std::pin::Pin;

pub type RowId = u64;
pub type Mask = Vec<bool>;

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
    Null,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueBlock {
    Any(Vec<Value>),
    F64(Vec<f64>),
}

impl ValueBlock {
    pub fn len(&self) -> usize {
        match self {
            Self::Any(values) => values.len(),
            Self::F64(values) => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_f64(&self) -> Option<&[f64]> {
        match self {
            Self::F64(values) => Some(values),
            Self::Any(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderError {
    NotFound,
    BackendError,
    Timeout,
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

/// Provider stays in place for later `prop(...)` reintegration.
pub trait Provider {
    fn get_prop<'a>(
        &'a self,
        prop: &'a Property,
        batch: RowBatch<'a>,
        mask: Option<&'a Mask>,
    ) -> impl Future<Output = Result<ValueBlock, ProviderError>> + 'a;

    fn now_epoch_ms(&self) -> i64 {
        0
    }
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

#[derive(Clone, Debug, PartialEq)]
pub struct EvalBlock {
    pub values: ValueBlock,
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

    fn fail_mask(mask: &Mask, error: EvalError) -> Self {
        let len = mask.len();
        let mut ok = vec![false; len];
        let mut errors = Vec::new();
        for (idx, active) in mask.iter().copied().enumerate() {
            if active {
                ok[idx] = false;
                errors.push((idx, error.clone()));
            }
        }
        Self {
            values: ValueBlock::Any(vec![Value::Null; len]),
            ok,
            errors,
        }
    }

}

#[derive(Clone, Debug)]
pub struct EvalContext {
    pub properties: Vec<Property>,
    prop_index: HashMap<String, usize>,
}

impl EvalContext {
    pub fn new(properties: Vec<Property>) -> Self {
        let mut prop_index = HashMap::with_capacity(properties.len());
        for (idx, prop) in properties.iter().enumerate() {
            // Keep first occurrence to match analyzer's current linear lookup behavior.
            prop_index.entry(prop.name.clone()).or_insert(idx);
        }
        Self {
            properties,
            prop_index,
        }
    }

    pub fn property(&self, name: &str) -> Option<&Property> {
        self.prop_index
            .get(name)
            .and_then(|idx| self.properties.get(*idx))
    }

    pub fn ty(&self, name: &str) -> Option<&Ty> {
        self.property(name).map(|property| &property.ty)
    }
}

#[derive(Debug, Default)]
pub struct Evaluator;

impl Evaluator {
    pub fn new() -> Self {
        Self
    }

    pub async fn eval<P: Provider>(
        &self,
        expr: &Expr,
        batch: RowBatch<'_>,
        ctx: &EvalContext,
        provider: &P,
    ) -> Result<EvalBlock, ProviderError> {
        self.eval_with_mask(expr, batch, ctx, provider, vec![true; batch.rows.len()])
            .await
    }

    pub async fn eval_simple_fail_batch<P: Provider>(
        &self,
        expr: &Expr,
        batch: RowBatch<'_>,
        ctx: &EvalContext,
        provider: &P,
    ) -> Result<ValueBlock, SimpleEvalError> {
        let out = self
            .eval(expr, batch, ctx, provider)
            .await
            .map_err(SimpleEvalError::Provider)?;

        if let Some((row_index, reason)) = out.errors.first().cloned() {
            return Err(SimpleEvalError::FirstRowError {
                row_index,
                reason,
                total: out.len(),
            });
        }

        if let Some(row_index) = out.ok.iter().position(|row_ok| !row_ok) {
            return Err(SimpleEvalError::FirstRowError {
                row_index,
                reason: EvalError::InvalidArgument,
                total: out.len(),
            });
        }

        Ok(out.values)
    }

    fn eval_with_mask<'a, P: Provider + 'a>(
        &'a self,
        expr: &'a Expr,
        batch: RowBatch<'a>,
        ctx: &'a EvalContext,
        provider: &'a P,
        mask: Mask,
    ) -> Pin<Box<dyn Future<Output = Result<EvalBlock, ProviderError>> + 'a>> {
        Box::pin(async move {
            if mask.len() != batch.rows.len() {
                return Err(ProviderError::BackendError);
            }

            match &expr.kind {
                ExprKind::Group { inner } => {
                    self.eval_with_mask(inner, batch, ctx, provider, mask).await
                }
                ExprKind::Lit(lit) => Ok(eval_literal(lit, &mask)),
                ExprKind::Binary { op, left, right } if is_arithmetic_op(op.node) => {
                    self.eval_binary_arith(op.node, left, right, batch, ctx, provider, &mask)
                        .await
                }
                _ => Ok(EvalBlock::fail_mask(&mask, EvalError::InvalidArgument)),
            }
        })
    }

    async fn eval_binary_arith<P: Provider>(
        &self,
        op: BinOpKind,
        left: &Expr,
        right: &Expr,
        batch: RowBatch<'_>,
        ctx: &EvalContext,
        provider: &P,
        mask: &Mask,
    ) -> Result<EvalBlock, ProviderError> {
        let len = batch.rows.len();
        let left_block = self
            .eval_with_mask(left, batch, ctx, provider, mask.clone())
            .await?;
        let right_block = self
            .eval_with_mask(right, batch, ctx, provider, mask.clone())
            .await?;

        let mut errors = left_block.errors.clone();
        errors.extend(right_block.errors.iter().cloned());

        let mut out = vec![0.0_f64; len];
        let mut ok = vec![false; len];

        let (left_values, right_values) = match (left_block.values.as_f64(), right_block.values.as_f64()) {
            (Some(left_values), Some(right_values)) => (left_values, right_values),
            _ => {
                if errors.is_empty() {
                    return Ok(EvalBlock::fail_mask(mask, EvalError::TypeMismatch));
                }
                return Ok(EvalBlock {
                    values: ValueBlock::F64(out),
                    ok,
                    errors,
                });
            }
        };

        if is_all_true(mask) && is_all_true(&left_block.ok) && is_all_true(&right_block.ok) {
            match op {
                BinOpKind::Plus => {
                    for idx in 0..len {
                        out[idx] = left_values[idx] + right_values[idx];
                    }
                    ok.fill(true);
                }
                BinOpKind::Minus => {
                    for idx in 0..len {
                        out[idx] = left_values[idx] - right_values[idx];
                    }
                    ok.fill(true);
                }
                BinOpKind::Star => {
                    for idx in 0..len {
                        out[idx] = left_values[idx] * right_values[idx];
                    }
                    ok.fill(true);
                }
                BinOpKind::Slash => {
                    for idx in 0..len {
                        let rhs = right_values[idx];
                        if rhs == 0.0 {
                            errors.push((idx, EvalError::DivideByZero));
                            continue;
                        }
                        out[idx] = left_values[idx] / rhs;
                        ok[idx] = true;
                    }
                }
                _ => return Ok(EvalBlock::fail_mask(mask, EvalError::InvalidArgument)),
            }
            return Ok(EvalBlock {
                values: ValueBlock::F64(out),
                ok,
                errors,
            });
        }

        for idx in 0..len {
            if !mask[idx] || !left_block.ok[idx] || !right_block.ok[idx] {
                continue;
            }
            match arith_apply_checked(op, left_values[idx], right_values[idx]) {
                Ok(value) => {
                    out[idx] = value;
                    ok[idx] = true;
                }
                Err(error) => errors.push((idx, error)),
            }
        }

        Ok(EvalBlock {
            values: ValueBlock::F64(out),
            ok,
            errors,
        })
    }
}

fn eval_literal(lit: &analyzer::Lit, mask: &Mask) -> EvalBlock {
    if lit.kind != LitKind::Number {
        return EvalBlock::fail_mask(mask, EvalError::InvalidArgument);
    }

    let number = match lit.symbol.text.parse::<f64>() {
        Ok(number) => number,
        Err(_) => return EvalBlock::fail_mask(mask, EvalError::InvalidArgument),
    };

    let len = mask.len();
    let mut rows = vec![0.0; len];
    let mut ok = vec![false; len];
    for (idx, active) in mask.iter().copied().enumerate() {
        if active {
            rows[idx] = number;
            ok[idx] = true;
        }
    }

    EvalBlock {
        values: ValueBlock::F64(rows),
        ok,
        errors: Vec::new(),
    }
}

fn arith_apply_checked(op: BinOpKind, left: f64, right: f64) -> Result<f64, EvalError> {
    match op {
        BinOpKind::Plus => Ok(left + right),
        BinOpKind::Minus => Ok(left - right),
        BinOpKind::Star => Ok(left * right),
        BinOpKind::Slash => {
            if right == 0.0 {
                Err(EvalError::DivideByZero)
            } else {
                Ok(left / right)
            }
        }
        _ => Err(EvalError::InvalidArgument),
    }
}

fn is_arithmetic_op(op: BinOpKind) -> bool {
    matches!(
        op,
        BinOpKind::Plus | BinOpKind::Minus | BinOpKind::Star | BinOpKind::Slash
    )
}

fn is_all_true(mask: &Mask) -> bool {
    mask.iter().all(|active| *active)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::{Context, Poll, Waker};

    #[derive(Default)]
    struct DummyProvider;

    impl Provider for DummyProvider {
        fn get_prop<'a>(
            &'a self,
            _prop: &'a Property,
            _batch: RowBatch<'a>,
            _mask: Option<&'a Mask>,
        ) -> impl Future<Output = Result<ValueBlock, ProviderError>> + 'a {
            async move { Err(ProviderError::NotFound) }
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        let mut future = std::pin::pin!(future);
        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(value) => return value,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    fn parse_expr(source: &str) -> Expr {
        analyzer::analyze_syntax(source).expr
    }

    #[test]
    fn number_literal_produces_f64_block() {
        let evaluator = Evaluator::new();
        let expr = parse_expr("1");
        let rows = [1_u64, 2, 3];
        let batch = RowBatch {
            rows: &rows,
            batch_id: 1,
        };
        let ctx = EvalContext::new(vec![]);
        let provider = DummyProvider;

        let out = block_on(evaluator.eval(&expr, batch, &ctx, &provider)).unwrap();

        assert_eq!(out.ok, vec![true, true, true]);
        assert!(out.errors.is_empty());
        match out.values {
            ValueBlock::F64(values) => assert_eq!(values, vec![1.0, 1.0, 1.0]),
            ValueBlock::Any(_) => panic!("expected f64 block"),
        }
    }

    #[test]
    fn add_runs_dense_f64_path() {
        let evaluator = Evaluator::new();
        let expr = parse_expr("1 + 2");
        let rows = [1_u64, 2, 3];
        let batch = RowBatch {
            rows: &rows,
            batch_id: 1,
        };
        let ctx = EvalContext::new(vec![]);
        let provider = DummyProvider;

        let out = block_on(evaluator.eval(&expr, batch, &ctx, &provider)).unwrap();

        assert_eq!(out.ok, vec![true, true, true]);
        assert!(out.errors.is_empty());
        match out.values {
            ValueBlock::F64(values) => assert_eq!(values, vec![3.0, 3.0, 3.0]),
            ValueBlock::Any(_) => panic!("expected f64 block"),
        }
    }

    #[test]
    fn nested_arithmetic_expression_works() {
        let evaluator = Evaluator::new();
        let expr = parse_expr("(1 + 2) * 3 - 4 / 2");
        let rows = [1_u64, 2, 3, 4];
        let batch = RowBatch {
            rows: &rows,
            batch_id: 1,
        };
        let ctx = EvalContext::new(vec![]);
        let provider = DummyProvider;

        let out = block_on(evaluator.eval(&expr, batch, &ctx, &provider)).unwrap();

        assert_eq!(out.ok, vec![true, true, true, true]);
        assert!(out.errors.is_empty());
        match out.values {
            ValueBlock::F64(values) => assert_eq!(values, vec![7.0, 7.0, 7.0, 7.0]),
            ValueBlock::Any(_) => panic!("expected f64 block"),
        }
    }

    #[test]
    fn divide_by_zero_marks_rows_invalid() {
        let evaluator = Evaluator::new();
        let expr = parse_expr("1 / 0");
        let rows = [1_u64, 2, 3];
        let batch = RowBatch {
            rows: &rows,
            batch_id: 1,
        };
        let ctx = EvalContext::new(vec![]);
        let provider = DummyProvider;

        let out = block_on(evaluator.eval(&expr, batch, &ctx, &provider)).unwrap();

        assert_eq!(out.ok, vec![false, false, false]);
        assert_eq!(
            out.errors,
            vec![
                (0, EvalError::DivideByZero),
                (1, EvalError::DivideByZero),
                (2, EvalError::DivideByZero),
            ]
        );
        match out.values {
            ValueBlock::F64(values) => assert_eq!(values, vec![0.0, 0.0, 0.0]),
            ValueBlock::Any(_) => panic!("expected f64 block"),
        }
    }

    #[test]
    fn mask_disables_rows_for_binary_arith() {
        let evaluator = Evaluator::new();
        let expr = parse_expr("1 + 2");
        let rows = [1_u64, 2, 3, 4];
        let batch = RowBatch {
            rows: &rows,
            batch_id: 1,
        };
        let ctx = EvalContext::new(vec![]);
        let provider = DummyProvider;

        let out = block_on(evaluator.eval_with_mask(
            &expr,
            batch,
            &ctx,
            &provider,
            vec![true, false, true, false],
        ))
        .unwrap();

        assert_eq!(out.ok, vec![true, false, true, false]);
        assert!(out.errors.is_empty());
        match out.values {
            ValueBlock::F64(values) => assert_eq!(values, vec![3.0, 0.0, 3.0, 0.0]),
            ValueBlock::Any(_) => panic!("expected f64 block"),
        }
    }
}
