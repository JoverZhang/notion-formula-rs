use std::cmp::Ordering;

use chrono::{Datelike, FixedOffset, Months, NaiveDate, TimeZone, Timelike, Utc};
use regex::Regex;

use crate::builtins::{BuiltinKey, DynamicValueArgs, RowOutcome, rows_to_kernel};
use crate::core::columns::{ColumnKind, KernelColumn, KernelResult, NumberKind, Validity};
use crate::core::context::BuiltinValueContext;
use crate::core::errors::EvalError;
use crate::core::types::{Mask, Value};
use crate::runtime::operators::stringify_value;

pub(crate) fn eval_value<K: ColumnKind, C: BuiltinValueContext>(
    key: BuiltinKey,
    args: DynamicValueArgs,
    context: &C,
    mask: &Mask,
) -> KernelResult<K> {
    let rows = (0..mask.len())
        .map(|row| {
            if mask[row] {
                eval_row(key, &args, context, row)
            } else {
                RowOutcome::Inactive
            }
        })
        .collect();
    rows_to_kernel(rows, mask)
}

pub(crate) fn is_empty_value(value: Option<&Value>) -> bool {
    match value {
        None => true,
        Some(Value::Text(value)) => value.is_empty(),
        Some(Value::List(value)) => value.is_empty(),
        Some(_) => false,
    }
}

pub(crate) fn abs_number(value: f64) -> f64 {
    value.abs()
}

pub(crate) fn ceil_number(value: f64) -> f64 {
    value.ceil()
}

pub(crate) fn sqrt_number(value: f64) -> Result<f64, EvalError> {
    if value < 0.0 || !value.is_finite() {
        Err(EvalError::InvalidArgument)
    } else {
        Ok(value.sqrt())
    }
}

pub(crate) fn eval_abs(column: KernelColumn<NumberKind>, mask: &Mask) -> KernelResult<NumberKind> {
    let input_validity = column.validity().clone();
    let valid = (0..mask.len())
        .map(|row| !mask[row] || input_validity.is_valid(row))
        .collect();
    let validity = Validity::from_valid_bits(valid);
    let column = match column.try_into_unique() {
        Ok((mut storage, _)) => {
            for (row, value) in storage.iter_mut().enumerate() {
                if mask[row] && input_validity.is_valid(row) {
                    *value = abs_number(*value);
                }
            }
            KernelColumn::from_owned(storage, validity)
        }
        Err(column) => {
            let values = column
                .values()
                .iter()
                .enumerate()
                .map(|(row, value)| {
                    if mask[row] && input_validity.is_valid(row) {
                        abs_number(*value)
                    } else {
                        *value
                    }
                })
                .collect();
            KernelColumn::from_values(values, validity)
        }
    };
    KernelResult {
        column,
        ok: Mask::all(mask.len()),
        errors: Vec::new(),
    }
}

#[derive(Clone, Debug)]
enum Cell {
    Omitted,
    Null,
    Value(Value),
}

fn eval_row(
    key: BuiltinKey,
    args: &DynamicValueArgs,
    context: &impl BuiltinValueContext,
    row: usize,
) -> RowOutcome {
    use BuiltinKey::*;
    match key {
        Empty => RowOutcome::Value(Value::Bool(match head(args, 0, row) {
            Cell::Omitted | Cell::Null => is_empty_value(None),
            Cell::Value(value) => is_empty_value(std::option::Option::Some(&value)),
        })),
        Length => with_value(head(args, 0, row), |value| match value {
            Value::Text(value) => Ok(Value::Number(value.chars().count() as f64)),
            Value::List(value) => Ok(Value::Number(value.len() as f64)),
            _ => Err(EvalError::TypeMismatch),
        }),
        Format => match head(args, 0, row) {
            Cell::Null | Cell::Omitted => RowOutcome::Value(Value::Text(String::new())),
            Cell::Value(value) => RowOutcome::Value(Value::Text(stringify_value(&value))),
        },
        Equal | Unequal => {
            let left = head(args, 0, row);
            let right = head(args, 1, row);
            let equal = match (left, right) {
                (Cell::Null | Cell::Omitted, Cell::Null | Cell::Omitted) => true,
                (Cell::Value(left), Cell::Value(right)) => left == right,
                _ => false,
            };
            RowOutcome::Value(Value::Bool(if key == Equal { equal } else { !equal }))
        }
        Substring => eval_substring(args, row),
        Contains => with_two(args, row, |left, right| {
            Ok(Value::Bool(as_text(left)?.contains(as_text(right)?)))
        }),
        Test => eval_regex(args, row, RegexOperation::Test),
        Match => eval_regex(args, row, RegexOperation::Match),
        Replace => eval_regex(args, row, RegexOperation::ReplaceOne),
        ReplaceAll => eval_regex(args, row, RegexOperation::ReplaceAll),
        Lower | Upper | Trim => with_value(head(args, 0, row), |value| {
            let text = as_text(value)?;
            let text = if key == Lower {
                text.to_lowercase()
            } else if key == Upper {
                text.to_uppercase()
            } else {
                text.trim().to_string()
            };
            Ok(Value::Text(text))
        }),
        Repeat => with_two(args, row, |text, times| {
            let text = as_text(text)?;
            let times = bounded_count(as_number(times)?)?;
            Ok(Value::Text(text.repeat(times)))
        }),
        PadStart | PadEnd => eval_pad(key, args, row),
        Concat => eval_concat(args, row),
        Join => with_two(args, row, |list, separator| {
            let list = as_list(list)?;
            let separator = as_text(separator)?;
            Ok(Value::Text(
                list.iter()
                    .map(stringify_value)
                    .collect::<Vec<_>>()
                    .join(separator),
            ))
        }),
        Split => with_two(args, row, |text, separator| {
            let text = as_text(text)?;
            let separator = as_text(separator)?;
            let parts = if separator.is_empty() {
                text.chars()
                    .map(|character| Value::Text(character.to_string()))
                    .collect()
            } else {
                text.split(separator)
                    .map(|part| Value::Text(part.to_string()))
                    .collect()
            };
            Ok(Value::List(parts))
        }),
        FormatNumber => eval_format_number(args, row),
        Add | Subtract | Multiply | Mod | Pow | Divide => eval_numeric_binary(key, args, row),
        Min | Max | Sum | Median | Mean => eval_aggregate(key, args, row),
        Abs => with_value(head(args, 0, row), |value| {
            Ok(Value::Number(abs_number(as_number(value)?)))
        }),
        Round => eval_round(args, row),
        Ceil | Floor | Sqrt | Cbrt | Exp | Ln | Log10 | Log2 | Sign => {
            with_value(head(args, 0, row), |value| {
                let value = as_number(value)?;
                let result = match key {
                    Ceil => return Ok(Value::Number(ceil_number(value))),
                    Floor => value.floor(),
                    Sqrt => sqrt_number(value)?,
                    Cbrt => value.cbrt(),
                    Exp => value.exp(),
                    Ln if value > 0.0 => value.ln(),
                    Log10 if value > 0.0 => value.log10(),
                    Log2 if value > 0.0 => value.log2(),
                    Sign => value.signum(),
                    _ => return Err(EvalError::InvalidArgument),
                };
                finite_number(result)
            })
        }
        Pi => RowOutcome::Value(Value::Number(std::f64::consts::PI)),
        E => RowOutcome::Value(Value::Number(std::f64::consts::E)),
        ToNumber => match head(args, 0, row) {
            Cell::Null => RowOutcome::Null,
            Cell::Omitted => RowOutcome::Error(EvalError::InvalidArgument),
            Cell::Value(value) => result_to_outcome(to_number(value)),
        },
        Now => RowOutcome::Value(Value::Date(context.runtime().evaluated_at_epoch_ms())),
        Today => result_to_outcome(today_epoch_ms(context).map(Value::Date)),
        Minute | Hour | Day | Date | Week | Month | Year => eval_date_part(key, args, row, context),
        DateAdd | DateSubtract => eval_date_shift(key, args, row, context),
        DateBetween => eval_date_between(args, row),
        Timestamp => with_value(head(args, 0, row), |value| {
            Ok(Value::Number(as_date(value)? as f64))
        }),
        FromTimestamp => with_value(head(args, 0, row), |value| {
            let timestamp = as_number(value)?;
            if !timestamp.is_finite() {
                return Err(EvalError::InvalidDate);
            }
            Ok(Value::Date(timestamp.trunc() as i64))
        }),
        FormatDate => eval_format_date(args, row, context),
        ParseDate => eval_parse_date(args, row, context),
        At | First | Last => eval_list_pick(key, args, row),
        Slice => eval_slice(args, row),
        Splice => eval_splice(args, row),
        Sort | Reverse | Unique => eval_list_transform(key, args, row),
        Includes => with_two(args, row, |list, needle| {
            Ok(Value::Bool(as_list(list)?.contains(needle)))
        }),
        Flat => with_value(head(args, 0, row), |value| {
            let list = as_list(value)?;
            let mut output = Vec::new();
            flatten_values(list, &mut output);
            Ok(Value::List(output))
        }),
        Id => RowOutcome::Value(Value::Text(context.rows()[row].to_string())),
        If | Ifs | Let | Map | Filter | Find | FindIndex | Some | Every | Count => {
            RowOutcome::Error(EvalError::InvalidArgument)
        }
    }
}

fn head(args: &DynamicValueArgs, index: usize, row: usize) -> Cell {
    column_cell(args.head(index), row)
}

fn column_cell(column: Option<&crate::core::columns::Column>, row: usize) -> Cell {
    match column {
        None => Cell::Omitted,
        Some(column) if !column.validity().is_valid(row) => Cell::Null,
        Some(column) => Cell::Value(column.row_value(row).expect("valid row has a value")),
    }
}

fn with_value(
    cell: Cell,
    operation: impl FnOnce(&Value) -> Result<Value, EvalError>,
) -> RowOutcome {
    match cell {
        Cell::Value(value) => result_to_outcome(operation(&value)),
        Cell::Null => RowOutcome::Null,
        Cell::Omitted => RowOutcome::Error(EvalError::InvalidArgument),
    }
}

fn with_two(
    args: &DynamicValueArgs,
    row: usize,
    operation: impl FnOnce(&Value, &Value) -> Result<Value, EvalError>,
) -> RowOutcome {
    match (head(args, 0, row), head(args, 1, row)) {
        (Cell::Value(left), Cell::Value(right)) => result_to_outcome(operation(&left, &right)),
        (Cell::Null, _) | (_, Cell::Null) => RowOutcome::Null,
        _ => RowOutcome::Error(EvalError::InvalidArgument),
    }
}

fn with_three(
    args: &DynamicValueArgs,
    row: usize,
    operation: impl FnOnce(&Value, &Value, &Value) -> Result<Value, EvalError>,
) -> RowOutcome {
    match (head(args, 0, row), head(args, 1, row), head(args, 2, row)) {
        (Cell::Value(first), Cell::Value(second), Cell::Value(third)) => {
            result_to_outcome(operation(&first, &second, &third))
        }
        (Cell::Null, _, _) | (_, Cell::Null, _) | (_, _, Cell::Null) => RowOutcome::Null,
        _ => RowOutcome::Error(EvalError::InvalidArgument),
    }
}

fn result_to_outcome(result: Result<Value, EvalError>) -> RowOutcome {
    result
        .map(RowOutcome::Value)
        .unwrap_or_else(RowOutcome::Error)
}

fn as_number(value: &Value) -> Result<f64, EvalError> {
    match value {
        Value::Number(value) => Ok(*value),
        _ => Err(EvalError::TypeMismatch),
    }
}

fn as_text(value: &Value) -> Result<&str, EvalError> {
    match value {
        Value::Text(value) => Ok(value),
        _ => Err(EvalError::TypeMismatch),
    }
}

fn as_list(value: &Value) -> Result<&[Value], EvalError> {
    match value {
        Value::List(value) => Ok(value),
        _ => Err(EvalError::TypeMismatch),
    }
}

fn as_date(value: &Value) -> Result<i64, EvalError> {
    match value {
        Value::Date(value) => Ok(*value),
        _ => Err(EvalError::TypeMismatch),
    }
}

fn finite_number(value: f64) -> Result<Value, EvalError> {
    value
        .is_finite()
        .then_some(Value::Number(value))
        .ok_or(EvalError::InvalidArgument)
}

fn bounded_count(value: f64) -> Result<usize, EvalError> {
    if !value.is_finite() || !(0.0..=1_000_000.0).contains(&value) {
        return Err(EvalError::InvalidArgument);
    }
    Ok(value.trunc() as usize)
}

fn normalize_index(index: f64, len: usize, allow_end: bool) -> usize {
    let index = index.trunc() as isize;
    let len = len as isize;
    let normalized = if index < 0 { len + index } else { index };
    let maximum = if allow_end {
        len
    } else {
        len.saturating_sub(1)
    };
    normalized.clamp(0, maximum) as usize
}

fn eval_substring(args: &DynamicValueArgs, row: usize) -> RowOutcome {
    let (Cell::Value(text), Cell::Value(start)) = (head(args, 0, row), head(args, 1, row)) else {
        return if matches!(head(args, 0, row), Cell::Null)
            || matches!(head(args, 1, row), Cell::Null)
        {
            RowOutcome::Null
        } else {
            RowOutcome::Error(EvalError::InvalidArgument)
        };
    };
    let Ok(text) = as_text(&text) else {
        return RowOutcome::Error(EvalError::TypeMismatch);
    };
    let Ok(start) = as_number(&start) else {
        return RowOutcome::Error(EvalError::TypeMismatch);
    };
    let characters = text.chars().collect::<Vec<_>>();
    let start = normalize_index(start, characters.len(), true);
    let end = match head(args, 2, row) {
        Cell::Omitted => characters.len(),
        Cell::Null => return RowOutcome::Null,
        Cell::Value(end) => match as_number(&end) {
            Ok(end) => normalize_index(end, characters.len(), true),
            Err(error) => return RowOutcome::Error(error),
        },
    };
    RowOutcome::Value(Value::Text(
        characters[start.min(end)..end.max(start)].iter().collect(),
    ))
}

enum RegexOperation {
    Test,
    Match,
    ReplaceOne,
    ReplaceAll,
}

fn eval_regex(args: &DynamicValueArgs, row: usize, operation: RegexOperation) -> RowOutcome {
    let required = match operation {
        RegexOperation::Test | RegexOperation::Match => 2,
        RegexOperation::ReplaceOne | RegexOperation::ReplaceAll => 3,
    };
    let cells = (0..required)
        .map(|index| head(args, index, row))
        .collect::<Vec<_>>();
    if cells.iter().any(|cell| matches!(cell, Cell::Null)) {
        return RowOutcome::Null;
    }
    let values = cells
        .iter()
        .map(|cell| match cell {
            Cell::Value(value) => as_text(value),
            _ => Err(EvalError::InvalidArgument),
        })
        .collect::<Result<Vec<_>, _>>();
    let Ok(values) = values else {
        return RowOutcome::Error(EvalError::TypeMismatch);
    };
    let Ok(regex) = Regex::new(values[1]) else {
        return RowOutcome::Error(EvalError::InvalidRegex);
    };
    match operation {
        RegexOperation::Test => RowOutcome::Value(Value::Bool(regex.is_match(values[0]))),
        RegexOperation::Match => RowOutcome::Value(Value::List(
            regex
                .find_iter(values[0])
                .map(|matched| Value::Text(matched.as_str().to_string()))
                .collect(),
        )),
        RegexOperation::ReplaceOne => RowOutcome::Value(Value::Text(
            regex.replace(values[0], values[2]).into_owned(),
        )),
        RegexOperation::ReplaceAll => RowOutcome::Value(Value::Text(
            regex.replace_all(values[0], values[2]).into_owned(),
        )),
    }
}

fn eval_pad(key: BuiltinKey, args: &DynamicValueArgs, row: usize) -> RowOutcome {
    with_three(args, row, |value, length, pad| {
        let text = match value {
            Value::Text(text) => text.clone(),
            Value::Number(number) => stringify_value(&Value::Number(*number)),
            _ => return Err(EvalError::TypeMismatch),
        };
        let desired = bounded_count(as_number(length)?)?;
        let pad = as_text(pad)?;
        let current = text.chars().count();
        if current >= desired || pad.is_empty() {
            return Ok(Value::Text(text));
        }
        let missing = desired - current;
        let fill = pad.chars().cycle().take(missing).collect::<String>();
        Ok(Value::Text(if key == BuiltinKey::PadStart {
            fill + &text
        } else {
            text + &fill
        }))
    })
}

fn eval_concat(args: &DynamicValueArgs, row: usize) -> RowOutcome {
    let mut result = Vec::new();
    for group in args.repeat_groups() {
        match column_cell(group.first().and_then(Option::as_ref), row) {
            Cell::Value(Value::List(values)) => result.extend(values),
            Cell::Null => return RowOutcome::Null,
            _ => return RowOutcome::Error(EvalError::TypeMismatch),
        }
    }
    RowOutcome::Value(Value::List(result))
}

fn eval_format_number(args: &DynamicValueArgs, row: usize) -> RowOutcome {
    with_three(args, row, |value, format, precision| {
        let value = as_number(value)?;
        let format = as_text(format)?;
        let precision = bounded_count(as_number(precision)?)?.min(100);
        let rendered = match format {
            "percent" | "%" => format!("{:.*}%", precision, value * 100.0),
            "scientific" => format!("{:.*e}", precision, value),
            _ => format!("{:.*}", precision, value),
        };
        Ok(Value::Text(rendered))
    })
}

fn eval_numeric_binary(key: BuiltinKey, args: &DynamicValueArgs, row: usize) -> RowOutcome {
    with_two(args, row, |left, right| {
        let left = as_number(left)?;
        let right = as_number(right)?;
        let value = match key {
            BuiltinKey::Add => left + right,
            BuiltinKey::Subtract => left - right,
            BuiltinKey::Multiply => left * right,
            BuiltinKey::Mod if right != 0.0 => left % right,
            BuiltinKey::Pow => left.powf(right),
            BuiltinKey::Divide if right != 0.0 => left / right,
            BuiltinKey::Mod | BuiltinKey::Divide => return Err(EvalError::DivideByZero),
            _ => return Err(EvalError::InvalidArgument),
        };
        finite_number(value)
    })
}

fn eval_aggregate(key: BuiltinKey, args: &DynamicValueArgs, row: usize) -> RowOutcome {
    let mut values = Vec::new();
    for group in args.repeat_groups() {
        match column_cell(group.first().and_then(Option::as_ref), row) {
            Cell::Value(Value::Number(value)) => values.push(value),
            Cell::Value(Value::List(items)) => {
                for item in items {
                    match item {
                        Value::Number(value) => values.push(value),
                        _ => return RowOutcome::Error(EvalError::TypeMismatch),
                    }
                }
            }
            Cell::Null => return RowOutcome::Null,
            _ => return RowOutcome::Error(EvalError::TypeMismatch),
        }
    }
    if values.is_empty() {
        return RowOutcome::Null;
    }
    let value = match key {
        BuiltinKey::Min => values.into_iter().fold(f64::INFINITY, f64::min),
        BuiltinKey::Max => values.into_iter().fold(f64::NEG_INFINITY, f64::max),
        BuiltinKey::Sum => values.into_iter().sum(),
        BuiltinKey::Mean => values.iter().sum::<f64>() / values.len() as f64,
        BuiltinKey::Median => {
            values.sort_by(f64::total_cmp);
            let middle = values.len() / 2;
            if values.len() % 2 == 0 {
                (values[middle - 1] + values[middle]) / 2.0
            } else {
                values[middle]
            }
        }
        _ => return RowOutcome::Error(EvalError::InvalidArgument),
    };
    RowOutcome::Value(Value::Number(value))
}

fn eval_round(args: &DynamicValueArgs, row: usize) -> RowOutcome {
    let Cell::Value(value) = head(args, 0, row) else {
        return match head(args, 0, row) {
            Cell::Null => RowOutcome::Null,
            _ => RowOutcome::Error(EvalError::InvalidArgument),
        };
    };
    let Ok(value) = as_number(&value) else {
        return RowOutcome::Error(EvalError::TypeMismatch);
    };
    let places = match head(args, 1, row) {
        Cell::Omitted => 0,
        Cell::Null => return RowOutcome::Null,
        Cell::Value(places) => match as_number(&places) {
            Ok(places) if places.is_finite() => places.trunc().clamp(-308.0, 308.0) as i32,
            _ => return RowOutcome::Error(EvalError::InvalidArgument),
        },
    };
    let factor = 10_f64.powi(places);
    result_to_outcome(finite_number((value * factor).round() / factor))
}

fn to_number(value: Value) -> Result<Value, EvalError> {
    match value {
        Value::Number(value) => Ok(Value::Number(value)),
        Value::Bool(value) => Ok(Value::Number(if value { 1.0 } else { 0.0 })),
        Value::Date(value) => Ok(Value::Number(value as f64)),
        Value::Text(value) => value
            .trim()
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| EvalError::InvalidArgument),
        Value::List(_) => Err(EvalError::TypeMismatch),
    }
}

fn timezone(context: &impl BuiltinValueContext) -> Result<FixedOffset, EvalError> {
    context
        .runtime()
        .timezone_offset_minutes()
        .checked_mul(60)
        .and_then(FixedOffset::east_opt)
        .ok_or(EvalError::InvalidDate)
}

fn local_datetime(
    epoch_ms: i64,
    context: &impl BuiltinValueContext,
) -> Result<chrono::DateTime<FixedOffset>, EvalError> {
    let utc =
        chrono::DateTime::<Utc>::from_timestamp_millis(epoch_ms).ok_or(EvalError::InvalidDate)?;
    Ok(utc.with_timezone(&timezone(context)?))
}

fn today_epoch_ms(context: &impl BuiltinValueContext) -> Result<i64, EvalError> {
    let offset_ms = i64::from(timezone(context)?.local_minus_utc())
        .checked_mul(1_000)
        .ok_or(EvalError::InvalidDate)?;
    let local_epoch = context
        .runtime()
        .evaluated_at_epoch_ms()
        .checked_add(offset_ms)
        .ok_or(EvalError::InvalidDate)?;
    local_epoch
        .div_euclid(86_400_000)
        .checked_mul(86_400_000)
        .and_then(|midnight| midnight.checked_sub(offset_ms))
        .ok_or(EvalError::InvalidDate)
}

fn eval_date_part(
    key: BuiltinKey,
    args: &DynamicValueArgs,
    row: usize,
    context: &impl BuiltinValueContext,
) -> RowOutcome {
    with_value(head(args, 0, row), |value| {
        let date = local_datetime(as_date(value)?, context)?;
        let result = match key {
            BuiltinKey::Minute => date.minute(),
            BuiltinKey::Hour => date.hour(),
            BuiltinKey::Day => date.weekday().number_from_monday(),
            BuiltinKey::Date => date.day(),
            BuiltinKey::Week => date.iso_week().week(),
            BuiltinKey::Month => date.month(),
            BuiltinKey::Year => date.year() as u32,
            _ => return Err(EvalError::InvalidArgument),
        };
        Ok(Value::Number(result as f64))
    })
}

fn eval_date_shift(
    key: BuiltinKey,
    args: &DynamicValueArgs,
    row: usize,
    context: &impl BuiltinValueContext,
) -> RowOutcome {
    with_three(args, row, |date, amount, unit| {
        let date = as_date(date)?;
        let amount = as_number(amount)?;
        if !amount.is_finite() {
            return Err(EvalError::InvalidDate);
        }
        let mut amount = amount.trunc() as i64;
        if key == BuiltinKey::DateSubtract {
            amount = amount.checked_neg().ok_or(EvalError::InvalidDate)?;
        }
        let unit = as_text(unit)?.to_ascii_lowercase();
        let shifted = match unit.as_str() {
            "millisecond" | "milliseconds" => date.checked_add(amount),
            "second" | "seconds" => {
                date.checked_add(amount.checked_mul(1_000).ok_or(EvalError::InvalidDate)?)
            }
            "minute" | "minutes" => {
                date.checked_add(amount.checked_mul(60_000).ok_or(EvalError::InvalidDate)?)
            }
            "hour" | "hours" => date.checked_add(
                amount
                    .checked_mul(3_600_000)
                    .ok_or(EvalError::InvalidDate)?,
            ),
            "day" | "days" => date.checked_add(
                amount
                    .checked_mul(86_400_000)
                    .ok_or(EvalError::InvalidDate)?,
            ),
            "week" | "weeks" => date.checked_add(
                amount
                    .checked_mul(604_800_000)
                    .ok_or(EvalError::InvalidDate)?,
            ),
            "month" | "months" | "quarter" | "quarters" | "year" | "years" => {
                let multiplier = match unit.as_str() {
                    "quarter" | "quarters" => 3,
                    "year" | "years" => 12,
                    _ => 1,
                };
                let months = amount
                    .checked_mul(multiplier)
                    .ok_or(EvalError::InvalidDate)?;
                shift_months(date, months, context)
            }
            _ => None,
        }
        .ok_or(EvalError::InvalidDate)?;
        Ok(Value::Date(shifted))
    })
}

fn shift_months(epoch_ms: i64, amount: i64, context: &impl BuiltinValueContext) -> Option<i64> {
    let local = local_datetime(epoch_ms, context).ok()?;
    let magnitude = u32::try_from(amount.unsigned_abs()).ok()?;
    let shifted = if amount >= 0 {
        local.checked_add_months(Months::new(magnitude))?
    } else {
        local.checked_sub_months(Months::new(magnitude))?
    };
    Some(shifted.timestamp_millis())
}

fn eval_date_between(args: &DynamicValueArgs, row: usize) -> RowOutcome {
    with_three(args, row, |left, right, unit| {
        let difference = as_date(left)?
            .checked_sub(as_date(right)?)
            .ok_or(EvalError::InvalidDate)?;
        let divisor = match as_text(unit)?.to_ascii_lowercase().as_str() {
            "millisecond" | "milliseconds" => 1.0,
            "second" | "seconds" => 1_000.0,
            "minute" | "minutes" => 60_000.0,
            "hour" | "hours" => 3_600_000.0,
            "day" | "days" => 86_400_000.0,
            "week" | "weeks" => 604_800_000.0,
            "month" | "months" => 2_629_746_000.0,
            "quarter" | "quarters" => 7_889_238_000.0,
            "year" | "years" => 31_556_952_000.0,
            _ => return Err(EvalError::InvalidArgument),
        };
        Ok(Value::Number((difference as f64 / divisor).trunc()))
    })
}

fn eval_format_date(
    args: &DynamicValueArgs,
    row: usize,
    context: &impl BuiltinValueContext,
) -> RowOutcome {
    with_two(args, row, |date, format| {
        let date = local_datetime(as_date(date)?, context)?;
        let pattern = translate_date_format(as_text(format)?);
        Ok(Value::Text(date.format(&pattern).to_string()))
    })
}

fn translate_date_format(format: &str) -> String {
    let replacements = [
        ("YYYY", "%Y"),
        ("YY", "%y"),
        ("MMMM", "%B"),
        ("MMM", "%b"),
        ("MM", "%m"),
        ("DD", "%d"),
        ("HH", "%H"),
        ("hh", "%I"),
        ("mm", "%M"),
        ("ss", "%S"),
    ];
    let mut output = format.to_string();
    for (token, replacement) in replacements {
        output = output.replace(token, replacement);
    }
    output
}

fn eval_parse_date(
    args: &DynamicValueArgs,
    row: usize,
    context: &impl BuiltinValueContext,
) -> RowOutcome {
    with_value(head(args, 0, row), |value| {
        let text = as_text(value)?;
        if let Ok(date) = chrono::DateTime::parse_from_rfc3339(text) {
            return Ok(Value::Date(date.timestamp_millis()));
        }
        let date =
            NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| EvalError::InvalidDate)?;
        let local = timezone(context)?
            .from_local_datetime(&date.and_hms_opt(0, 0, 0).ok_or(EvalError::InvalidDate)?)
            .single()
            .ok_or(EvalError::InvalidDate)?;
        Ok(Value::Date(local.timestamp_millis()))
    })
}

fn eval_list_pick(key: BuiltinKey, args: &DynamicValueArgs, row: usize) -> RowOutcome {
    let Cell::Value(list) = head(args, 0, row) else {
        return match head(args, 0, row) {
            Cell::Null => RowOutcome::Null,
            _ => RowOutcome::Error(EvalError::InvalidArgument),
        };
    };
    let Ok(list) = as_list(&list) else {
        return RowOutcome::Error(EvalError::TypeMismatch);
    };
    let index = match key {
        BuiltinKey::First => 0,
        BuiltinKey::Last => list.len().saturating_sub(1),
        BuiltinKey::At => match head(args, 1, row) {
            Cell::Value(index) => match as_number(&index) {
                Ok(index) => normalize_index(index, list.len(), false),
                Err(error) => return RowOutcome::Error(error),
            },
            Cell::Null => return RowOutcome::Null,
            _ => return RowOutcome::Error(EvalError::InvalidArgument),
        },
        _ => return RowOutcome::Error(EvalError::InvalidArgument),
    };
    list.get(index)
        .cloned()
        .map(RowOutcome::Value)
        .unwrap_or(RowOutcome::Null)
}

fn eval_slice(args: &DynamicValueArgs, row: usize) -> RowOutcome {
    let (Cell::Value(list), Cell::Value(start)) = (head(args, 0, row), head(args, 1, row)) else {
        return if matches!(head(args, 0, row), Cell::Null)
            || matches!(head(args, 1, row), Cell::Null)
        {
            RowOutcome::Null
        } else {
            RowOutcome::Error(EvalError::InvalidArgument)
        };
    };
    let Ok(list) = as_list(&list) else {
        return RowOutcome::Error(EvalError::TypeMismatch);
    };
    let Ok(start) = as_number(&start) else {
        return RowOutcome::Error(EvalError::TypeMismatch);
    };
    let start = normalize_index(start, list.len(), true);
    let end = match head(args, 2, row) {
        Cell::Omitted => list.len(),
        Cell::Null => return RowOutcome::Null,
        Cell::Value(end) => match as_number(&end) {
            Ok(end) => normalize_index(end, list.len(), true),
            Err(error) => return RowOutcome::Error(error),
        },
    };
    RowOutcome::Value(Value::List(list[start.min(end)..start.max(end)].to_vec()))
}

fn eval_splice(args: &DynamicValueArgs, row: usize) -> RowOutcome {
    let cells = (head(args, 0, row), head(args, 1, row), head(args, 2, row));
    let (Cell::Value(list), Cell::Value(start), Cell::Value(delete_count)) = cells else {
        return if matches!(cells.0, Cell::Null)
            || matches!(cells.1, Cell::Null)
            || matches!(cells.2, Cell::Null)
        {
            RowOutcome::Null
        } else {
            RowOutcome::Error(EvalError::InvalidArgument)
        };
    };
    let Ok(list) = as_list(&list) else {
        return RowOutcome::Error(EvalError::TypeMismatch);
    };
    let (Ok(start), Ok(delete_count)) = (as_number(&start), as_number(&delete_count)) else {
        return RowOutcome::Error(EvalError::TypeMismatch);
    };
    let start = normalize_index(start, list.len(), true);
    let delete_count = bounded_count(delete_count).unwrap_or(0);
    let end = start.saturating_add(delete_count).min(list.len());
    let mut output = list[..start].to_vec();
    for group in args.repeat_groups() {
        match column_cell(group.first().and_then(Option::as_ref), row) {
            Cell::Value(value) => output.push(value),
            Cell::Null => return RowOutcome::Null,
            Cell::Omitted => return RowOutcome::Error(EvalError::InvalidArgument),
        }
    }
    output.extend_from_slice(&list[end..]);
    RowOutcome::Value(Value::List(output))
}

fn eval_list_transform(key: BuiltinKey, args: &DynamicValueArgs, row: usize) -> RowOutcome {
    with_value(head(args, 0, row), |value| {
        let mut values = as_list(value)?.to_vec();
        match key {
            BuiltinKey::Sort => values.sort_by(compare_value),
            BuiltinKey::Reverse => values.reverse(),
            BuiltinKey::Unique => {
                let mut unique = Vec::with_capacity(values.len());
                for value in values {
                    if !unique.contains(&value) {
                        unique.push(value);
                    }
                }
                values = unique;
            }
            _ => return Err(EvalError::InvalidArgument),
        }
        Ok(Value::List(values))
    })
}

fn compare_value(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left.total_cmp(right),
        (Value::Text(left), Value::Text(right)) => left.cmp(right),
        (Value::Bool(left), Value::Bool(right)) => left.cmp(right),
        (Value::Date(left), Value::Date(right)) => left.cmp(right),
        (Value::List(left), Value::List(right)) => left.len().cmp(&right.len()),
        (left, right) => value_rank(left).cmp(&value_rank(right)),
    }
}

fn value_rank(value: &Value) -> u8 {
    match value {
        Value::Number(_) => 0,
        Value::Text(_) => 1,
        Value::Bool(_) => 2,
        Value::Date(_) => 3,
        Value::List(_) => 4,
    }
}

fn flatten_values(values: &[Value], output: &mut Vec<Value>) {
    for value in values {
        match value {
            Value::List(nested) => flatten_values(nested, output),
            value => output.push(value.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::columns::{KernelColumn, NumberKind, Validity};
    use crate::core::types::Mask;

    use super::eval_abs;

    #[test]
    fn abs_recovers_unique_storage() {
        let input = KernelColumn::<NumberKind>::from_values(vec![-1.0, -2.0], Validity::AllValid);
        let output = eval_abs(input, &Mask::all(2));
        assert_eq!(output.column.values(), &[1.0, 2.0]);
        assert_eq!(output.column.storage_strong_count(), 1);
    }
}
