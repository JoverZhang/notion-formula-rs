use analyzer::analysis::Property;
use analyzer::ast::Expr;
use core::future::Future;
use std::task::{Context, Poll, Waker};

use crate::core::context::EvalContext;
use crate::core::errors::{EvalError, ProviderError};
use crate::core::provider::Provider;
use crate::core::types::{Column, ColumnBlock, Mask, RowBatch, Value};
use crate::runtime::cast::cast_block_to_f64;
use crate::runtime::evaluator::Evaluator;

#[derive(Default)]
struct DummyProvider;

impl Provider for DummyProvider {
    fn get_prop<'a>(
        &'a self,
        _prop: &'a Property,
        _batch: RowBatch<'a>,
        _mask: Option<&'a Mask>,
    ) -> impl Future<Output = Result<ColumnBlock, ProviderError>> + 'a {
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
fn number_literal_produces_f64_column() {
    let ctx = EvalContext::new(vec![]);
    let provider = DummyProvider;
    let evaluator = Evaluator::new(&ctx, &provider);
    let expr = parse_expr("1");
    let rows = [1_u64, 2, 3];
    let batch = RowBatch {
        rows: &rows,
        batch_id: 1,
    };

    let out = block_on(evaluator.eval(&expr, batch)).unwrap();

    assert_eq!(out.ok, vec![true, true, true]);
    assert!(out.errors.is_empty());
    assert_eq!(out.values.nulls, vec![false, false, false]);
    match out.values.column {
        Column::F64(values) => assert_eq!(values, vec![1.0, 1.0, 1.0]),
        Column::Any(_) => panic!("expected f64 column"),
    }
}

#[test]
fn add_runs_f64_specialized_kernel() {
    let ctx = EvalContext::new(vec![]);
    let provider = DummyProvider;
    let evaluator = Evaluator::new(&ctx, &provider);
    let expr = parse_expr("1 + 2");
    let rows = [1_u64, 2, 3];
    let batch = RowBatch {
        rows: &rows,
        batch_id: 1,
    };

    let out = block_on(evaluator.eval(&expr, batch)).unwrap();

    assert_eq!(out.ok, vec![true, true, true]);
    assert!(out.errors.is_empty());
    assert_eq!(out.values.nulls, vec![false, false, false]);
    match out.values.column {
        Column::F64(values) => assert_eq!(values, vec![3.0, 3.0, 3.0]),
        Column::Any(_) => panic!("expected f64 column"),
    }
}

#[test]
fn nested_arithmetic_expression_works() {
    let ctx = EvalContext::new(vec![]);
    let provider = DummyProvider;
    let evaluator = Evaluator::new(&ctx, &provider);
    let expr = parse_expr("(1 + 2) * 3 - 4 / 2");
    let rows = [1_u64, 2, 3, 4];
    let batch = RowBatch {
        rows: &rows,
        batch_id: 1,
    };

    let out = block_on(evaluator.eval(&expr, batch)).unwrap();

    assert_eq!(out.ok, vec![true, true, true, true]);
    assert!(out.errors.is_empty());
    assert_eq!(out.values.nulls, vec![false, false, false, false]);
    match out.values.column {
        Column::F64(values) => assert_eq!(values, vec![7.0, 7.0, 7.0, 7.0]),
        Column::Any(_) => panic!("expected f64 column"),
    }
}

#[test]
fn divide_by_zero_marks_rows_invalid() {
    let ctx = EvalContext::new(vec![]);
    let provider = DummyProvider;
    let evaluator = Evaluator::new(&ctx, &provider);
    let expr = parse_expr("1 / 0");
    let rows = [1_u64, 2, 3];
    let batch = RowBatch {
        rows: &rows,
        batch_id: 1,
    };

    let out = block_on(evaluator.eval(&expr, batch)).unwrap();

    assert_eq!(out.ok, vec![false, false, false]);
    assert_eq!(out.values.nulls, vec![true, true, true]);
    assert_eq!(
        out.errors,
        vec![
            (0, EvalError::DivideByZero),
            (1, EvalError::DivideByZero),
            (2, EvalError::DivideByZero),
        ]
    );
}

#[test]
fn planner_rejects_invalid_subtraction_shape() {
    let ctx = EvalContext::new(vec![]);
    let provider = DummyProvider;
    let evaluator = Evaluator::new(&ctx, &provider);
    let expr = parse_expr("1 - \"x\"");
    let rows = [1_u64, 2];
    let batch = RowBatch {
        rows: &rows,
        batch_id: 1,
    };

    let out = block_on(evaluator.eval(&expr, batch)).unwrap();

    assert_eq!(out.ok, vec![false, false]);
    assert_eq!(
        out.errors,
        vec![(0, EvalError::TypeMismatch), (1, EvalError::TypeMismatch)]
    );
    assert_eq!(out.values.nulls, vec![true, true]);
}

#[test]
fn add_any_supports_string_number() {
    let ctx = EvalContext::new(vec![]);
    let provider = DummyProvider;
    let evaluator = Evaluator::new(&ctx, &provider);
    let expr = parse_expr(r#""x" + 2"#);
    let rows = [1_u64, 2, 3];
    let batch = RowBatch {
        rows: &rows,
        batch_id: 1,
    };

    let out = block_on(evaluator.eval(&expr, batch)).unwrap();

    assert_eq!(out.ok, vec![true, true, true]);
    assert!(out.errors.is_empty());
    assert_eq!(out.values.nulls, vec![false, false, false]);
    match out.values.column {
        Column::Any(values) => assert_eq!(
            values,
            vec![
                Value::Text("x2".to_string()),
                Value::Text("x2".to_string()),
                Value::Text("x2".to_string()),
            ]
        ),
        Column::F64(_) => panic!("expected any column"),
    }
}

#[test]
fn add_any_supports_string_list() {
    let ctx = EvalContext::new(vec![]);
    let provider = DummyProvider;
    let evaluator = Evaluator::new(&ctx, &provider);
    let expr = parse_expr(r#""x" + [1, 2]"#);
    let rows = [1_u64, 2];
    let batch = RowBatch {
        rows: &rows,
        batch_id: 1,
    };

    let out = block_on(evaluator.eval(&expr, batch)).unwrap();

    assert_eq!(out.ok, vec![true, true]);
    assert!(out.errors.is_empty());
    match out.values.column {
        Column::Any(values) => assert_eq!(
            values,
            vec![
                Value::Text("x[1, 2]".to_string()),
                Value::Text("x[1, 2]".to_string()),
            ]
        ),
        Column::F64(_) => panic!("expected any column"),
    }
}

#[test]
fn cast_to_f64_reports_row_error_on_non_number() {
    let input = crate::core::types::EvalBlock {
        values: ColumnBlock {
            column: Column::Any(vec![Value::Number(1.0), Value::Text("x".to_string())]),
            nulls: vec![false, false],
        },
        ok: vec![true, true],
        errors: vec![],
    };

    let out = cast_block_to_f64(input, &vec![true, true]);

    assert_eq!(out.ok, vec![true, false]);
    assert_eq!(out.values.nulls, vec![false, true]);
    assert_eq!(out.errors, vec![(1, EvalError::TypeMismatch)]);
    match out.values.column {
        Column::F64(values) => assert_eq!(values, vec![1.0, 0.0]),
        Column::Any(_) => panic!("expected f64 column"),
    }
}

#[test]
fn mask_disables_rows_for_binary_arith() {
    let ctx = EvalContext::new(vec![]);
    let provider = DummyProvider;
    let evaluator = Evaluator::new(&ctx, &provider);
    let expr = parse_expr("1 + 2");
    let rows = [1_u64, 2, 3, 4];
    let batch = RowBatch {
        rows: &rows,
        batch_id: 1,
    };

    let out = block_on(evaluator.eval_with_mask(&expr, batch, vec![true, false, true, false]))
        .unwrap();

    assert_eq!(out.ok, vec![true, false, true, false]);
    assert!(out.errors.is_empty());
    assert_eq!(out.values.nulls, vec![false, true, false, true]);
    match out.values.column {
        Column::F64(values) => assert_eq!(values, vec![3.0, 0.0, 3.0, 0.0]),
        Column::Any(_) => panic!("expected f64 column"),
    }
}
