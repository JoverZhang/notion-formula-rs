use crate::builtins::contract::{
    AndPlans, CountPlans, EveryPlans, FilterPlans, FindIndexPlans, FindPlans, IfPlans, IfsPlans,
    LetPlans, MapPlans, OrPlans, SomePlans,
};
use crate::builtins::{
    BuiltinEvalContext, BuiltinKey, LambdaBindings, LambdaPlan, RowOutcome, ThunkPlan, ValuePlan,
    block_into_kernel, rows_to_kernel,
};
use crate::core::columns::{
    AnyKind, BooleanKind, Column, ColumnKind, KernelColumn, KernelResult, ListKind, Validity,
};
use crate::core::errors::EvalError;
use crate::core::types::{EvalBlock, Mask, Value};
use crate::runtime::operators::{eval_cast, eval_logical_and, eval_logical_or};

pub(crate) fn eval_and<C: BuiltinEvalContext>(
    context: &mut C,
    args: AndPlans,
    mask: &Mask,
) -> KernelResult<BooleanKind> {
    eval_boolean_call(
        context,
        args.repeat_groups
            .into_vec()
            .into_iter()
            .map(|group| group.condition),
        mask,
        true,
    )
}

pub(crate) fn eval_or<C: BuiltinEvalContext>(
    context: &mut C,
    args: OrPlans,
    mask: &Mask,
) -> KernelResult<BooleanKind> {
    eval_boolean_call(
        context,
        args.repeat_groups
            .into_vec()
            .into_iter()
            .map(|group| group.condition),
        mask,
        false,
    )
}

fn eval_boolean_call<C: BuiltinEvalContext>(
    context: &mut C,
    mut conditions: impl Iterator<Item = ThunkPlan<BooleanKind>>,
    mask: &Mask,
    conjunction: bool,
) -> KernelResult<BooleanKind> {
    let Some(first) = conditions.next() else {
        return block_into_kernel(EvalBlock::fail_mask(mask, EvalError::InvalidArgument), mask);
    };
    let mut result = context.eval_thunk(first, mask).into_eval_block();
    for condition in conditions {
        let evaluate =
            |remaining: &Mask| context.eval_thunk(condition, remaining).into_eval_block();
        result = if conjunction {
            eval_logical_and(result, mask, evaluate)
        } else {
            eval_logical_or(result, mask, evaluate)
        };
    }
    block_into_kernel(eval_cast(result, BooleanKind::ABI_KIND, mask), mask)
}

pub(crate) fn eval_if<C: BuiltinEvalContext>(
    context: &mut C,
    args: IfPlans,
    mask: &Mask,
) -> KernelResult<AnyKind> {
    let condition = context.eval(args.condition, mask);
    let split = context.split_mask(&condition, mask);
    let then_block = context
        .eval_thunk(args.then, &split.when_true)
        .into_eval_block();
    let else_block = context
        .eval_thunk(args.else_, &split.when_false)
        .into_eval_block();
    merge_branches(
        condition.into_eval_block(),
        then_block,
        else_block,
        mask,
        &split.when_true,
    )
}

pub(crate) fn eval_ifs<C: BuiltinEvalContext>(
    context: &mut C,
    args: IfsPlans,
    mask: &Mask,
) -> KernelResult<AnyKind> {
    let mut remaining = mask.clone();
    let mut outcomes = vec![RowOutcome::Inactive; mask.len()];
    let mut errors = Vec::new();

    for group in args.repeat_groups.into_vec() {
        if !remaining.any() {
            break;
        }
        let condition = context.eval(group.condition, &remaining);
        errors.extend(condition.errors.iter().cloned());
        for (row, outcome) in outcomes.iter_mut().enumerate() {
            if remaining[row] && !condition.ok[row] {
                *outcome = RowOutcome::Failed;
            }
        }
        let split = context.split_mask(&condition, &remaining);
        let value = context
            .eval_thunk(group.value, &split.when_true)
            .into_eval_block();
        errors.extend(value.errors.iter().cloned());
        for (row, outcome) in outcomes.iter_mut().enumerate() {
            if split.when_true[row] {
                *outcome = block_outcome(&value, row);
            }
        }
        remaining = split.when_false;
    }

    if let Some(otherwise) = args.else_ {
        let else_block = context.eval_thunk(otherwise, &remaining).into_eval_block();
        errors.extend(else_block.errors.iter().cloned());
        for (row, outcome) in outcomes.iter_mut().enumerate() {
            if remaining[row] {
                *outcome = block_outcome(&else_block, row);
            }
        }
    } else {
        for (row, outcome) in outcomes.iter_mut().enumerate() {
            if remaining[row] {
                *outcome = RowOutcome::Null;
            }
        }
    }

    let mut result = rows_to_kernel::<AnyKind>(outcomes, mask);
    result.errors.extend(errors);
    result
}

pub(crate) fn eval_let<C: BuiltinEvalContext>(
    context: &mut C,
    args: LetPlans,
    mask: &Mask,
) -> KernelResult<AnyKind> {
    if args.ident.owner() != context.plan_owner() {
        return block_into_kernel(EvalBlock::fail_mask(mask, EvalError::InvalidArgument), mask);
    }
    let name = args.ident.name().to_string();
    let value = context.eval(args.value, mask).into_eval_block();
    let body_mask = mask.and(&value.ok);
    let body = context
        .apply_lambda(
            args.body,
            LambdaBindings::new(vec![(name, value.column.clone())]),
            &body_mask,
        )
        .into_eval_block();
    let rows = (0..mask.len())
        .map(|row| {
            if !mask[row] {
                RowOutcome::Inactive
            } else if !value.ok[row] {
                RowOutcome::Failed
            } else {
                block_outcome(&body, row)
            }
        })
        .collect();
    let mut result = rows_to_kernel::<AnyKind>(rows, mask);
    result.errors.extend(value.errors);
    result.errors.extend(body.errors);
    result
}

pub(crate) fn eval_map<C: BuiltinEvalContext>(
    context: &mut C,
    args: MapPlans,
    mask: &Mask,
) -> KernelResult<ListKind> {
    eval_map_impl(context, args.list, args.mapper, mask)
}

pub(crate) fn eval_filter<C: BuiltinEvalContext>(
    context: &mut C,
    args: FilterPlans,
    mask: &Mask,
) -> KernelResult<ListKind> {
    eval_predicate_list(context, BuiltinKey::Filter, args.list, args.predicate, mask)
}

pub(crate) fn eval_find<C: BuiltinEvalContext>(
    context: &mut C,
    args: FindPlans,
    mask: &Mask,
) -> KernelResult<AnyKind> {
    eval_predicate_list(context, BuiltinKey::Find, args.list, args.predicate, mask)
}

pub(crate) fn eval_find_index<C: BuiltinEvalContext>(
    context: &mut C,
    args: FindIndexPlans,
    mask: &Mask,
) -> KernelResult<crate::core::columns::NumberKind> {
    eval_predicate_list(
        context,
        BuiltinKey::FindIndex,
        args.list,
        args.predicate,
        mask,
    )
}

pub(crate) fn eval_some<C: BuiltinEvalContext>(
    context: &mut C,
    args: SomePlans,
    mask: &Mask,
) -> KernelResult<BooleanKind> {
    eval_predicate_list(context, BuiltinKey::Some, args.list, args.predicate, mask)
}

pub(crate) fn eval_every<C: BuiltinEvalContext>(
    context: &mut C,
    args: EveryPlans,
    mask: &Mask,
) -> KernelResult<BooleanKind> {
    eval_predicate_list(context, BuiltinKey::Every, args.list, args.predicate, mask)
}

pub(crate) fn eval_count<C: BuiltinEvalContext>(
    context: &mut C,
    args: CountPlans,
    mask: &Mask,
) -> KernelResult<crate::core::columns::NumberKind> {
    eval_predicate_list(context, BuiltinKey::Count, args.list, args.predicate, mask)
}

fn eval_map_impl<C: BuiltinEvalContext>(
    context: &mut C,
    list_plan: ValuePlan<ListKind>,
    mapper: LambdaPlan<AnyKind>,
    mask: &Mask,
) -> KernelResult<ListKind> {
    let list = context.eval(list_plan, mask).into_eval_block();
    let (lists, mut outcomes, mut active) = initialize_lists(&list, mask, ListInitial::EmptyList);
    let mut errors = list.errors.clone();
    let max_len = lists
        .iter()
        .filter_map(Option::as_ref)
        .map(Vec::len)
        .max()
        .unwrap_or(0);
    for index in 0..max_len {
        let element_mask = element_mask(&lists, &active, index, mask.len());
        if !element_mask.any() {
            continue;
        }
        let bindings = element_bindings(mapper.parameters(), &lists, index, &element_mask);
        let mapped = context
            .apply_lambda(mapper.clone(), bindings, &element_mask)
            .into_eval_block();
        errors.extend(mapped.errors.iter().cloned());
        for row in 0..mask.len() {
            if !element_mask[row] {
                continue;
            }
            if !mapped.ok[row] {
                active.set(row, false);
                outcomes[row] = RowOutcome::Failed;
            } else {
                if let RowOutcome::Value(Value::List(output)) = &mut outcomes[row] {
                    output.push(mapped.column.row_value(row));
                }
            }
        }
    }

    let mut result = rows_to_kernel::<ListKind>(outcomes, mask);
    result.errors.extend(errors);
    result
}

fn eval_predicate_list<C: BuiltinEvalContext, K: ColumnKind>(
    context: &mut C,
    key: BuiltinKey,
    list_plan: ValuePlan<ListKind>,
    predicate: LambdaPlan<BooleanKind>,
    mask: &Mask,
) -> KernelResult<K> {
    let list = context.eval(list_plan, mask).into_eval_block();
    let initial = match key {
        BuiltinKey::Filter => ListInitial::EmptyList,
        BuiltinKey::Find => ListInitial::Null,
        BuiltinKey::FindIndex => ListInitial::Number(-1.0),
        BuiltinKey::Some => ListInitial::Bool(false),
        BuiltinKey::Every => ListInitial::Bool(true),
        BuiltinKey::Count => ListInitial::Number(0.0),
        _ => ListInitial::Null,
    };
    let (lists, mut outcomes, mut active) = initialize_lists(&list, mask, initial);
    let mut errors = list.errors.clone();
    let max_len = lists
        .iter()
        .filter_map(Option::as_ref)
        .map(Vec::len)
        .max()
        .unwrap_or(0);
    for index in 0..max_len {
        let element_mask = element_mask(&lists, &active, index, mask.len());
        if !element_mask.any() {
            continue;
        }
        let bindings = element_bindings(predicate.parameters(), &lists, index, &element_mask);
        let tested = context
            .apply_lambda(predicate.clone(), bindings, &element_mask)
            .into_eval_block();
        errors.extend(tested.errors.iter().cloned());

        for row in 0..mask.len() {
            if !element_mask[row] {
                continue;
            }
            if !tested.ok[row] {
                active.set(row, false);
                outcomes[row] = RowOutcome::Failed;
                continue;
            }
            let passed = matches!(tested.column.row_value(row), Some(Value::Bool(true)));
            let element = lists[row]
                .as_ref()
                .and_then(|list| list.get(index))
                .cloned()
                .expect("element mask only selects existing elements");
            match key {
                BuiltinKey::Filter if passed => {
                    if let RowOutcome::Value(Value::List(output)) = &mut outcomes[row] {
                        output.push(element);
                    }
                }
                BuiltinKey::Find if passed => {
                    outcomes[row] = element.map_or(RowOutcome::Null, RowOutcome::Value);
                    active.set(row, false);
                }
                BuiltinKey::FindIndex if passed => {
                    outcomes[row] = RowOutcome::Value(Value::Number(index as f64));
                    active.set(row, false);
                }
                BuiltinKey::Some if passed => {
                    outcomes[row] = RowOutcome::Value(Value::Bool(true));
                    active.set(row, false);
                }
                BuiltinKey::Every if !passed => {
                    outcomes[row] = RowOutcome::Value(Value::Bool(false));
                    active.set(row, false);
                }
                BuiltinKey::Count if passed => {
                    if let RowOutcome::Value(Value::Number(count)) = &mut outcomes[row] {
                        *count += 1.0;
                    }
                }
                _ => {}
            }
        }
    }

    let mut result = rows_to_kernel::<K>(outcomes, mask);
    result.errors.extend(errors);
    result
}

#[derive(Clone, Copy)]
enum ListInitial {
    EmptyList,
    Null,
    Number(f64),
    Bool(bool),
}

type RowList = Option<Vec<Option<Value>>>;

fn initialize_lists(
    list: &EvalBlock,
    mask: &Mask,
    initial: ListInitial,
) -> (Vec<RowList>, Vec<RowOutcome>, Mask) {
    let mut lists = Vec::with_capacity(mask.len());
    let mut outcomes = Vec::with_capacity(mask.len());
    let mut active = Mask::none(mask.len());
    for row in 0..mask.len() {
        if !mask[row] {
            lists.push(None);
            outcomes.push(RowOutcome::Inactive);
            continue;
        }
        if !list.ok[row] {
            lists.push(None);
            outcomes.push(RowOutcome::Failed);
            continue;
        }
        match list.column.row_value(row) {
            Some(Value::List(values)) => {
                lists.push(Some(values));
                outcomes.push(match initial {
                    ListInitial::EmptyList => RowOutcome::Value(Value::List(Vec::new())),
                    ListInitial::Null => RowOutcome::Null,
                    ListInitial::Number(value) => RowOutcome::Value(Value::Number(value)),
                    ListInitial::Bool(value) => RowOutcome::Value(Value::Bool(value)),
                });
                active.set(row, true);
            }
            None => {
                lists.push(None);
                outcomes.push(RowOutcome::Null);
            }
            Some(value) => {
                lists.push(None);
                outcomes.push(RowOutcome::Error(EvalError::invalid_type(
                    analyzer::analysis::Ty::List(Box::new(analyzer::analysis::Ty::Unknown)),
                    &value,
                )));
            }
        }
    }
    (lists, outcomes, active)
}

fn element_mask(lists: &[RowList], active: &Mask, index: usize, len: usize) -> Mask {
    (0..len)
        .map(|row| active[row] && lists[row].as_ref().is_some_and(|list| index < list.len()))
        .collect()
}

fn element_bindings(
    parameters: &[String],
    lists: &[RowList],
    index: usize,
    mask: &Mask,
) -> LambdaBindings {
    let current_name = parameters
        .first()
        .cloned()
        .unwrap_or_else(|| "current".to_string());
    let index_name = parameters
        .get(1)
        .cloned()
        .unwrap_or_else(|| "index".to_string());
    // Lexical variables use the Any physical ABI even when their semantic type is Number.
    let index_column = Column::Any(KernelColumn::from_values(
        vec![Value::Number(index as f64); mask.len()],
        Validity::AllValid,
    ));
    LambdaBindings::new(vec![
        (current_name, element_binding(lists, index, mask)),
        (index_name, index_column),
    ])
}

fn element_binding(lists: &[RowList], index: usize, mask: &Mask) -> Column {
    let mut valid = Vec::with_capacity(mask.len());
    let values = (0..mask.len())
        .map(|row| {
            if mask[row] {
                let value = lists[row]
                    .as_ref()
                    .and_then(|list| list.get(index))
                    .cloned()
                    .expect("element mask only selects existing elements");
                valid.push(value.is_some());
                value.unwrap_or(Value::Number(0.0))
            } else {
                valid.push(true);
                Value::Number(0.0)
            }
        })
        .collect();
    Column::Any(KernelColumn::from_values(
        values,
        Validity::from_valid_bits(valid),
    ))
}

fn merge_branches(
    condition: EvalBlock,
    then_block: EvalBlock,
    else_block: EvalBlock,
    mask: &Mask,
    then_mask: &Mask,
) -> KernelResult<AnyKind> {
    let rows = (0..mask.len())
        .map(|row| {
            if !mask[row] {
                return RowOutcome::Inactive;
            }
            if !condition.ok[row] {
                return RowOutcome::Failed;
            }
            if then_mask[row] {
                block_outcome(&then_block, row)
            } else {
                block_outcome(&else_block, row)
            }
        })
        .collect();
    let mut result = rows_to_kernel::<AnyKind>(rows, mask);
    result.errors.extend(condition.errors);
    result.errors.extend(then_block.errors);
    result.errors.extend(else_block.errors);
    result
}

fn block_outcome(block: &EvalBlock, row: usize) -> RowOutcome {
    if !block.ok[row] {
        RowOutcome::Failed
    } else {
        block
            .column
            .row_value(row)
            .map(RowOutcome::Value)
            .unwrap_or(RowOutcome::Null)
    }
}
