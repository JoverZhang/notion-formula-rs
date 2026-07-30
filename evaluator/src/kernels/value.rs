use std::cmp::Ordering;

use chrono::{Datelike, FixedOffset, Months, NaiveDate, TimeZone, Timelike, Utc};
use regex::Regex;

use crate::builtins::contract::{ConcatArgs, SpliceArgs};
use crate::core::columns::{
    AnyKind, BooleanKind, ColumnKind, DateKind, KernelColumn, KernelResult, ListKind, NumberKind,
    TextKind, Validity,
};
use crate::core::context::BuiltinValueContext;
use crate::core::errors::EvalError;
use crate::core::types::{Mask, Value};
use crate::runtime::operators::stringify_value;

#[derive(Clone, Debug)]
enum TypedRow<T> {
    Value(T),
    Null,
    Inactive,
    Error(EvalError),
}

fn eval_rows<K: ColumnKind>(
    mask: &Mask,
    mut evaluate: impl FnMut(usize) -> TypedRow<K::Scalar>,
) -> KernelResult<K> {
    let mut values = Vec::with_capacity(mask.len());
    let mut valid = Vec::with_capacity(mask.len());
    let mut ok = Mask::all(mask.len());
    let mut errors = Vec::new();

    for row in 0..mask.len() {
        let outcome = if mask[row] {
            evaluate(row)
        } else {
            TypedRow::Inactive
        };
        match outcome {
            TypedRow::Value(value) => {
                values.push(value);
                valid.push(true);
            }
            TypedRow::Null => {
                values.push(K::placeholder());
                valid.push(false);
            }
            TypedRow::Inactive => {
                values.push(K::placeholder());
                valid.push(true);
            }
            TypedRow::Error(error) => {
                values.push(K::placeholder());
                valid.push(true);
                ok.set(row, false);
                errors.push((row, error));
            }
        }
    }

    KernelResult {
        column: KernelColumn::from_values(values, Validity::from_valid_bits(valid)),
        ok,
        errors,
    }
}

fn eval_unary<I: ColumnKind, O: ColumnKind>(
    input: &KernelColumn<I>,
    mask: &Mask,
    mut operation: impl FnMut(&I::Scalar) -> Result<O::Scalar, EvalError>,
) -> KernelResult<O> {
    eval_rows(mask, |row| match input.value(row) {
        Some(value) => operation(value)
            .map(TypedRow::Value)
            .unwrap_or_else(TypedRow::Error),
        None => TypedRow::Null,
    })
}

fn eval_binary<A: ColumnKind, B: ColumnKind, O: ColumnKind>(
    a: &KernelColumn<A>,
    b: &KernelColumn<B>,
    mask: &Mask,
    mut operation: impl FnMut(&A::Scalar, &B::Scalar) -> Result<O::Scalar, EvalError>,
) -> KernelResult<O> {
    eval_rows(mask, |row| match (a.value(row), b.value(row)) {
        (Some(a), Some(b)) => operation(a, b)
            .map(TypedRow::Value)
            .unwrap_or_else(TypedRow::Error),
        _ => TypedRow::Null,
    })
}

fn eval_ternary<A: ColumnKind, B: ColumnKind, C: ColumnKind, O: ColumnKind>(
    a: &KernelColumn<A>,
    b: &KernelColumn<B>,
    c: &KernelColumn<C>,
    mask: &Mask,
    mut operation: impl FnMut(&A::Scalar, &B::Scalar, &C::Scalar) -> Result<O::Scalar, EvalError>,
) -> KernelResult<O> {
    eval_rows(mask, |row| {
        match (a.value(row), b.value(row), c.value(row)) {
            (Some(a), Some(b), Some(c)) => operation(a, b, c)
                .map(TypedRow::Value)
                .unwrap_or_else(TypedRow::Error),
            _ => TypedRow::Null,
        }
    })
}

pub(crate) fn eval_empty(
    value: Option<KernelColumn<AnyKind>>,
    mask: &Mask,
) -> KernelResult<BooleanKind> {
    eval_rows(mask, |row| {
        let is_empty = match value.as_ref().and_then(|value| value.value(row)) {
            None => true,
            Some(Value::Number(value)) => *value == 0.0,
            Some(Value::Text(value)) => value.is_empty(),
            Some(Value::List(value)) => value.is_empty(),
            Some(Value::Bool(value)) => !value,
            Some(Value::Date(_)) => false,
        };
        TypedRow::Value(is_empty)
    })
}

pub(crate) fn eval_length(value: KernelColumn<AnyKind>, mask: &Mask) -> KernelResult<NumberKind> {
    eval_unary(&value, mask, |value| match value {
        Value::Text(value) => Ok(value.chars().count() as f64),
        Value::List(value) => Ok(value.len() as f64),
        _ => Err(EvalError::TypeMismatch),
    })
}

pub(crate) fn eval_format<C: BuiltinValueContext>(
    value: KernelColumn<AnyKind>,
    context: &C,
    mask: &Mask,
) -> KernelResult<TextKind> {
    eval_rows(mask, |row| match value.value(row) {
        None => TypedRow::Value(String::new()),
        Some(Value::Date(value)) => local_datetime(*value, context)
            .map(|date| date.format("%B %-d, %Y %H:%M").to_string())
            .map(TypedRow::Value)
            .unwrap_or_else(TypedRow::Error),
        Some(value) => TypedRow::Value(stringify_value(value)),
    })
}

pub(crate) fn eval_equality(
    a: KernelColumn<AnyKind>,
    b: KernelColumn<AnyKind>,
    negate: bool,
    mask: &Mask,
) -> KernelResult<BooleanKind> {
    eval_rows(mask, |row| {
        let equal = match (a.value(row), b.value(row)) {
            (None, None) => true,
            (Some(a), Some(b)) => a == b,
            _ => false,
        };
        TypedRow::Value(if negate { !equal } else { equal })
    })
}

pub(crate) fn eval_substring(
    text: KernelColumn<TextKind>,
    start: KernelColumn<NumberKind>,
    end: Option<KernelColumn<NumberKind>>,
    mask: &Mask,
) -> KernelResult<TextKind> {
    eval_rows(mask, |row| {
        let (Some(text), Some(start)) = (text.value(row), start.value(row)) else {
            return TypedRow::Null;
        };
        if !start.is_finite() {
            return TypedRow::Error(EvalError::InvalidArgument);
        }
        let characters = text.chars().collect::<Vec<_>>();
        let start = normalize_index(*start, characters.len(), true);
        let end = match end.as_ref() {
            None => characters.len(),
            Some(end) => {
                let Some(end) = end.value(row) else {
                    return TypedRow::Null;
                };
                if !end.is_finite() {
                    return TypedRow::Error(EvalError::InvalidArgument);
                }
                normalize_index(*end, characters.len(), true)
            }
        };
        let output = if start <= end {
            characters[start..end].iter().collect()
        } else {
            String::new()
        };
        TypedRow::Value(output)
    })
}

pub(crate) fn eval_contains(
    text: KernelColumn<TextKind>,
    search: KernelColumn<TextKind>,
    mask: &Mask,
) -> KernelResult<BooleanKind> {
    eval_binary(&text, &search, mask, |text, search| {
        Ok(text.contains(search))
    })
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum RegexOperation {
    Test,
    Match,
    ReplaceOne,
    ReplaceAll,
}

pub(crate) fn eval_regex<O: ColumnKind>(
    text: KernelColumn<TextKind>,
    pattern: KernelColumn<TextKind>,
    replacement: Option<KernelColumn<TextKind>>,
    operation: RegexOperation,
    mask: &Mask,
) -> KernelResult<O> {
    eval_rows(mask, |row| {
        let (Some(text), Some(pattern)) = (text.value(row), pattern.value(row)) else {
            return TypedRow::Null;
        };
        let replacement = match replacement.as_ref() {
            Some(replacement) => {
                let Some(replacement) = replacement.value(row) else {
                    return TypedRow::Null;
                };
                Some(replacement.as_str())
            }
            None => None,
        };
        let regex = match Regex::new(pattern) {
            Ok(regex) => regex,
            Err(_) => return TypedRow::Error(EvalError::InvalidRegex),
        };
        let value = match operation {
            RegexOperation::Test => Value::Bool(regex.is_match(text)),
            RegexOperation::Match => Value::List(
                regex
                    .find_iter(text)
                    .map(|matched| Value::Text(matched.as_str().to_string()))
                    .collect(),
            ),
            RegexOperation::ReplaceOne | RegexOperation::ReplaceAll => {
                let Some(replacement) = replacement else {
                    return TypedRow::Error(EvalError::InvalidArgument);
                };
                let replaced = if matches!(operation, RegexOperation::ReplaceOne) {
                    regex.replace(text, replacement)
                } else {
                    regex.replace_all(text, replacement)
                };
                Value::Text(replaced.into_owned())
            }
        };
        O::from_value(value)
            .map(TypedRow::Value)
            .unwrap_or_else(|_| TypedRow::Error(EvalError::TypeMismatch))
    })
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum TextTransform {
    Lower,
    Upper,
    Trim,
}

pub(crate) fn eval_text_transform(
    text: KernelColumn<TextKind>,
    operation: TextTransform,
    mask: &Mask,
) -> KernelResult<TextKind> {
    eval_unary(&text, mask, |text| {
        Ok(match operation {
            TextTransform::Lower => text.to_lowercase(),
            TextTransform::Upper => text.to_uppercase(),
            TextTransform::Trim => text.trim().to_string(),
        })
    })
}

pub(crate) fn eval_repeat(
    text: KernelColumn<TextKind>,
    times: KernelColumn<NumberKind>,
    mask: &Mask,
) -> KernelResult<TextKind> {
    eval_binary(&text, &times, mask, |text, times| {
        Ok(text.repeat(bounded_count(*times)?))
    })
}

pub(crate) fn eval_pad(
    text: KernelColumn<AnyKind>,
    length: KernelColumn<NumberKind>,
    pad: KernelColumn<TextKind>,
    at_start: bool,
    mask: &Mask,
) -> KernelResult<TextKind> {
    eval_ternary(&text, &length, &pad, mask, |value, length, pad| {
        let text = match value {
            Value::Text(text) => text.clone(),
            Value::Number(number) => stringify_value(&Value::Number(*number)),
            _ => return Err(EvalError::TypeMismatch),
        };
        let desired = bounded_count(*length)?;
        let current = text.chars().count();
        if current >= desired || pad.is_empty() {
            return Ok(text);
        }
        let fill = pad
            .chars()
            .cycle()
            .take(desired - current)
            .collect::<String>();
        Ok(if at_start { fill + &text } else { text + &fill })
    })
}

pub(crate) fn eval_concat(args: ConcatArgs, mask: &Mask) -> KernelResult<ListKind> {
    let groups = args.repeat_groups.into_vec();
    eval_rows(mask, |row| {
        let mut output = Vec::new();
        for group in &groups {
            let Some(values) = group.lists.value(row) else {
                return TypedRow::Null;
            };
            output.extend(values.iter().cloned());
        }
        TypedRow::Value(output)
    })
}

pub(crate) fn eval_join(
    list: KernelColumn<ListKind>,
    separator: KernelColumn<TextKind>,
    mask: &Mask,
) -> KernelResult<TextKind> {
    eval_binary(&list, &separator, mask, |list, separator| {
        Ok(list
            .iter()
            .map(stringify_value)
            .collect::<Vec<_>>()
            .join(separator))
    })
}

pub(crate) fn eval_split(
    text: KernelColumn<TextKind>,
    separator: KernelColumn<TextKind>,
    mask: &Mask,
) -> KernelResult<ListKind> {
    eval_binary(&text, &separator, mask, |text, separator| {
        if separator.is_empty() {
            Ok(text
                .chars()
                .map(|character| Value::Text(character.to_string()))
                .collect())
        } else {
            Ok(text
                .split(separator)
                .map(|part| Value::Text(part.to_string()))
                .collect())
        }
    })
}

pub(crate) fn eval_format_number(
    value: KernelColumn<NumberKind>,
    format: KernelColumn<TextKind>,
    precision: KernelColumn<NumberKind>,
    mask: &Mask,
) -> KernelResult<TextKind> {
    eval_ternary(
        &value,
        &format,
        &precision,
        mask,
        |value, format, precision| {
            if !value.is_finite() {
                return Err(EvalError::InvalidArgument);
            }
            let precision = bounded_count(*precision)?.min(100);
            let format = format.to_ascii_lowercase();
            match format.as_str() {
                "number" | "decimal" => Ok(render_fixed(*value, precision, false)),
                "number_with_commas" | "commas" => Ok(render_fixed(*value, precision, true)),
                "percent" | "%" => {
                    let percent = *value * 100.0;
                    if !percent.is_finite() {
                        return Err(EvalError::InvalidArgument);
                    }
                    Ok(format!("{}%", render_fixed(percent, precision, false)))
                }
                "scientific" => Ok(format!("{value:.*e}", precision)),
                "usd" => Ok(render_currency(*value, precision, "$")),
                "eur" => Ok(render_currency(*value, precision, "€")),
                "gbp" => Ok(render_currency(*value, precision, "£")),
                "jpy" => Ok(render_currency(*value, precision, "¥")),
                "cny" => Ok(render_currency(*value, precision, "CN¥")),
                "krw" => Ok(render_currency(*value, precision, "₩")),
                "inr" => Ok(render_currency(*value, precision, "₹")),
                "cad" => Ok(render_currency(*value, precision, "CA$")),
                "aud" => Ok(render_currency(*value, precision, "A$")),
                "chf" => Ok(render_currency(*value, precision, "CHF ")),
                _ => Err(EvalError::InvalidArgument),
            }
        },
    )
}

fn render_currency(value: f64, precision: usize, symbol: &str) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "" };
    format!(
        "{sign}{symbol}{}",
        render_fixed(value.abs(), precision, true)
    )
}

fn render_fixed(value: f64, precision: usize, grouped: bool) -> String {
    let rendered = format!("{value:.*}", precision);
    if !grouped {
        return rendered;
    }

    let (sign, unsigned) = rendered
        .strip_prefix('-')
        .map_or(("", rendered.as_str()), |value| ("-", value));
    let (integer, fraction) = unsigned
        .split_once('.')
        .map_or((unsigned, None), |(integer, fraction)| {
            (integer, Some(fraction))
        });
    let mut grouped_integer = String::with_capacity(integer.len() + integer.len() / 3);
    for (index, character) in integer.chars().enumerate() {
        if index > 0 && (integer.len() - index).is_multiple_of(3) {
            grouped_integer.push(',');
        }
        grouped_integer.push(character);
    }
    match fraction {
        Some(fraction) => format!("{sign}{grouped_integer}.{fraction}"),
        None => format!("{sign}{grouped_integer}"),
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum NumericBinary {
    Add,
    Subtract,
    Multiply,
    Mod,
    Pow,
    Divide,
}

pub(crate) fn eval_numeric_binary(
    a: KernelColumn<NumberKind>,
    b: KernelColumn<NumberKind>,
    operation: NumericBinary,
    mask: &Mask,
) -> KernelResult<NumberKind> {
    eval_binary(&a, &b, mask, |a, b| {
        let value = match operation {
            NumericBinary::Add => *a + *b,
            NumericBinary::Subtract => *a - *b,
            NumericBinary::Multiply => *a * *b,
            NumericBinary::Mod if *b != 0.0 => *a % *b,
            NumericBinary::Pow => a.powf(*b),
            NumericBinary::Divide if *b != 0.0 => *a / *b,
            NumericBinary::Mod | NumericBinary::Divide => return Err(EvalError::DivideByZero),
        };
        finite_number(value)
    })
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Aggregate {
    Min,
    Max,
    Sum,
    Median,
    Mean,
}

pub(crate) fn eval_aggregate(
    columns: Vec<KernelColumn<AnyKind>>,
    operation: Aggregate,
    mask: &Mask,
) -> KernelResult<NumberKind> {
    eval_rows(mask, |row| {
        let mut values = Vec::new();
        for column in &columns {
            let Some(value) = column.value(row) else {
                return TypedRow::Null;
            };
            match value {
                Value::Number(value) if value.is_finite() => values.push(*value),
                Value::List(items) => {
                    for item in items {
                        match item {
                            Value::Number(value) if value.is_finite() => values.push(*value),
                            _ => return TypedRow::Error(EvalError::TypeMismatch),
                        }
                    }
                }
                _ => return TypedRow::Error(EvalError::TypeMismatch),
            }
        }
        if values.is_empty() {
            return TypedRow::Null;
        }
        let result = match operation {
            Aggregate::Min => values.into_iter().fold(f64::INFINITY, f64::min),
            Aggregate::Max => values.into_iter().fold(f64::NEG_INFINITY, f64::max),
            Aggregate::Sum => values.into_iter().sum(),
            Aggregate::Mean => values.iter().sum::<f64>() / values.len() as f64,
            Aggregate::Median => {
                values.sort_by(f64::total_cmp);
                let middle = values.len() / 2;
                if values.len() % 2 == 0 {
                    (values[middle - 1] + values[middle]) / 2.0
                } else {
                    values[middle]
                }
            }
        };
        finite_number(result)
            .map(TypedRow::Value)
            .unwrap_or_else(TypedRow::Error)
    })
}

pub(crate) fn eval_round(
    value: KernelColumn<NumberKind>,
    places: Option<KernelColumn<NumberKind>>,
    mask: &Mask,
) -> KernelResult<NumberKind> {
    eval_rows(mask, |row| {
        let Some(value) = value.value(row) else {
            return TypedRow::Null;
        };
        let places = match places.as_ref() {
            None => 0,
            Some(places) => {
                let Some(places) = places.value(row) else {
                    return TypedRow::Null;
                };
                if !places.is_finite() {
                    return TypedRow::Error(EvalError::InvalidArgument);
                }
                places.trunc().clamp(-308.0, 308.0) as i32
            }
        };
        let factor = 10_f64.powi(places);
        finite_number((value * factor).round() / factor)
            .map(TypedRow::Value)
            .unwrap_or_else(TypedRow::Error)
    })
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum NumericUnary {
    Floor,
    Cbrt,
    Exp,
    Ln,
    Log10,
    Log2,
    Sign,
}

pub(crate) fn eval_numeric_unary(
    value: KernelColumn<NumberKind>,
    operation: NumericUnary,
    mask: &Mask,
) -> KernelResult<NumberKind> {
    eval_unary(&value, mask, |value| {
        let result = match operation {
            NumericUnary::Floor => value.floor(),
            NumericUnary::Cbrt => value.cbrt(),
            NumericUnary::Exp => value.exp(),
            NumericUnary::Ln if *value > 0.0 => value.ln(),
            NumericUnary::Log10 if *value > 0.0 => value.log10(),
            NumericUnary::Log2 if *value > 0.0 => value.log2(),
            NumericUnary::Sign if *value == 0.0 => 0.0,
            NumericUnary::Sign => value.signum(),
            NumericUnary::Ln | NumericUnary::Log10 | NumericUnary::Log2 => {
                return Err(EvalError::InvalidArgument);
            }
        };
        finite_number(result)
    })
}

pub(crate) fn eval_constant_number(value: f64, mask: &Mask) -> KernelResult<NumberKind> {
    eval_rows(mask, |_| TypedRow::Value(value))
}

pub(crate) fn eval_to_number(
    value: KernelColumn<AnyKind>,
    mask: &Mask,
) -> KernelResult<NumberKind> {
    eval_unary(&value, mask, |value| {
        let number = match value {
            Value::Number(value) => *value,
            Value::Bool(value) => {
                if *value {
                    1.0
                } else {
                    0.0
                }
            }
            Value::Date(value) => *value as f64,
            Value::Text(value) => value
                .trim()
                .parse::<f64>()
                .map_err(|_| EvalError::InvalidArgument)?,
            Value::List(_) => return Err(EvalError::TypeMismatch),
        };
        finite_number(number)
    })
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

pub(crate) fn eval_now<C: BuiltinValueContext>(context: &C, mask: &Mask) -> KernelResult<DateKind> {
    let value = context.runtime().evaluated_at_epoch_ms();
    eval_rows(mask, |_| TypedRow::Value(value))
}

pub(crate) fn eval_today<C: BuiltinValueContext>(
    context: &C,
    mask: &Mask,
) -> KernelResult<DateKind> {
    match today_epoch_ms(context) {
        Ok(value) => eval_rows(mask, |_| TypedRow::Value(value)),
        Err(error) => eval_rows(mask, |_| TypedRow::Error(error.clone())),
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DatePart {
    Minute,
    Hour,
    Day,
    Date,
    Week,
    Month,
    Year,
}

pub(crate) fn eval_date_part<C: BuiltinValueContext>(
    date: KernelColumn<DateKind>,
    part: DatePart,
    context: &C,
    mask: &Mask,
) -> KernelResult<NumberKind> {
    eval_unary(&date, mask, |date| {
        let date = local_datetime(*date, context)?;
        Ok(match part {
            DatePart::Minute => date.minute() as f64,
            DatePart::Hour => date.hour() as f64,
            DatePart::Day => date.weekday().number_from_monday() as f64,
            DatePart::Date => date.day() as f64,
            DatePart::Week => date.iso_week().week() as f64,
            DatePart::Month => date.month() as f64,
            DatePart::Year => date.year() as f64,
        })
    })
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DateShift {
    Add,
    Subtract,
}

pub(crate) fn eval_date_shift<C: BuiltinValueContext>(
    date: KernelColumn<DateKind>,
    amount: KernelColumn<NumberKind>,
    unit: KernelColumn<TextKind>,
    direction: DateShift,
    context: &C,
    mask: &Mask,
) -> KernelResult<DateKind> {
    eval_ternary(&date, &amount, &unit, mask, |date, amount, unit| {
        if !amount.is_finite() {
            return Err(EvalError::InvalidDate);
        }
        let mut amount = amount.trunc() as i64;
        if matches!(direction, DateShift::Subtract) {
            amount = amount.checked_neg().ok_or(EvalError::InvalidDate)?;
        }
        let unit = DateUnit::parse(unit)?;
        let shifted = if let Some(milliseconds) = unit.fixed_milliseconds() {
            date.checked_add(
                amount
                    .checked_mul(milliseconds)
                    .ok_or(EvalError::InvalidDate)?,
            )
        } else {
            let months = amount
                .checked_mul(unit.month_multiplier().ok_or(EvalError::InvalidArgument)?)
                .ok_or(EvalError::InvalidDate)?;
            shift_months(*date, months, context)
        }
        .ok_or(EvalError::InvalidDate)?;
        Ok(shifted)
    })
}

pub(crate) fn eval_date_between<C: BuiltinValueContext>(
    a: KernelColumn<DateKind>,
    b: KernelColumn<DateKind>,
    unit: KernelColumn<TextKind>,
    context: &C,
    mask: &Mask,
) -> KernelResult<NumberKind> {
    eval_ternary(&a, &b, &unit, mask, |a, b, unit| {
        let unit = DateUnit::parse(unit)?;
        let value = match unit {
            DateUnit::Minute | DateUnit::Hour | DateUnit::Day | DateUnit::Week => {
                let difference = a.checked_sub(*b).ok_or(EvalError::InvalidDate)?;
                let milliseconds = unit
                    .fixed_milliseconds()
                    .ok_or(EvalError::InvalidArgument)?;
                (difference as f64 / milliseconds as f64).trunc()
            }
            DateUnit::Month => complete_months_between(*a, *b, context)? as f64,
            DateUnit::Quarter => complete_months_between(*a, *b, context)? as f64 / 3.0,
            DateUnit::Year => complete_months_between(*a, *b, context)? as f64 / 12.0,
        };
        Ok(value.trunc())
    })
}

fn complete_months_between<C: BuiltinValueContext>(
    a: i64,
    b: i64,
    context: &C,
) -> Result<i64, EvalError> {
    if a < b {
        return complete_months_between(b, a, context)?
            .checked_neg()
            .ok_or(EvalError::InvalidDate);
    }

    let a_local = local_datetime(a, context)?;
    let b_local = local_datetime(b, context)?;
    let months = i64::from(a_local.year() - b_local.year())
        .checked_mul(12)
        .and_then(|years| {
            years.checked_add(i64::from(a_local.month()) - i64::from(b_local.month()))
        })
        .ok_or(EvalError::InvalidDate)?;
    let candidate = shift_months(b, months, context).ok_or(EvalError::InvalidDate)?;
    if candidate > a {
        months.checked_sub(1).ok_or(EvalError::InvalidDate)
    } else {
        Ok(months)
    }
}

#[derive(Clone, Copy, Debug)]
enum DateUnit {
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

impl DateUnit {
    fn parse(value: &str) -> Result<Self, EvalError> {
        match value.to_ascii_lowercase().as_str() {
            "minute" | "minutes" => Ok(Self::Minute),
            "hour" | "hours" => Ok(Self::Hour),
            "day" | "days" => Ok(Self::Day),
            "week" | "weeks" => Ok(Self::Week),
            "month" | "months" => Ok(Self::Month),
            "quarter" | "quarters" => Ok(Self::Quarter),
            "year" | "years" => Ok(Self::Year),
            _ => Err(EvalError::InvalidArgument),
        }
    }

    fn fixed_milliseconds(self) -> Option<i64> {
        match self {
            Self::Minute => Some(60_000),
            Self::Hour => Some(3_600_000),
            Self::Day => Some(86_400_000),
            Self::Week => Some(604_800_000),
            Self::Month | Self::Quarter | Self::Year => None,
        }
    }

    fn month_multiplier(self) -> Option<i64> {
        match self {
            Self::Month => Some(1),
            Self::Quarter => Some(3),
            Self::Year => Some(12),
            Self::Minute | Self::Hour | Self::Day | Self::Week => None,
        }
    }
}

pub(crate) fn eval_timestamp(
    date: KernelColumn<DateKind>,
    mask: &Mask,
) -> KernelResult<NumberKind> {
    eval_unary(&date, mask, |date| Ok(*date as f64))
}

pub(crate) fn eval_from_timestamp(
    timestamp: KernelColumn<NumberKind>,
    mask: &Mask,
) -> KernelResult<DateKind> {
    eval_unary(&timestamp, mask, |timestamp| {
        if !timestamp.is_finite() || *timestamp < i64::MIN as f64 || *timestamp >= i64::MAX as f64 {
            return Err(EvalError::InvalidDate);
        }
        let timestamp = timestamp.trunc() as i64;
        timestamp
            .checked_sub(timestamp.rem_euclid(60_000))
            .ok_or(EvalError::InvalidDate)
    })
}

pub(crate) fn eval_format_date<C: BuiltinValueContext>(
    date: KernelColumn<DateKind>,
    format: KernelColumn<TextKind>,
    context: &C,
    mask: &Mask,
) -> KernelResult<TextKind> {
    eval_binary(&date, &format, mask, |date, format| {
        Ok(local_datetime(*date, context)?
            .format(&translate_date_format(format))
            .to_string())
    })
}

pub(crate) fn eval_parse_date<C: BuiltinValueContext>(
    text: KernelColumn<TextKind>,
    context: &C,
    mask: &Mask,
) -> KernelResult<DateKind> {
    eval_unary(&text, mask, |text| {
        if let Ok(date) = chrono::DateTime::parse_from_rfc3339(text) {
            return Ok(date.timestamp_millis());
        }
        let date =
            NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| EvalError::InvalidDate)?;
        let local = timezone(context)?
            .from_local_datetime(&date.and_hms_opt(0, 0, 0).ok_or(EvalError::InvalidDate)?)
            .single()
            .ok_or(EvalError::InvalidDate)?;
        Ok(local.timestamp_millis())
    })
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ListPick {
    At,
    First,
    Last,
}

pub(crate) fn eval_list_pick(
    list: KernelColumn<ListKind>,
    index: Option<KernelColumn<NumberKind>>,
    operation: ListPick,
    mask: &Mask,
) -> KernelResult<AnyKind> {
    eval_rows(mask, |row| {
        let Some(list) = list.value(row) else {
            return TypedRow::Null;
        };
        let value = match operation {
            ListPick::First => list.first(),
            ListPick::Last => list.last(),
            ListPick::At => {
                let Some(index) = index.as_ref().and_then(|index| index.value(row)) else {
                    return TypedRow::Null;
                };
                if !index.is_finite() {
                    return TypedRow::Error(EvalError::InvalidArgument);
                }
                list_index(*index, list.len()).and_then(|index| list.get(index))
            }
        };
        value.cloned().map_or(TypedRow::Null, TypedRow::Value)
    })
}

pub(crate) fn eval_slice(
    list: KernelColumn<ListKind>,
    start: KernelColumn<NumberKind>,
    end: Option<KernelColumn<NumberKind>>,
    mask: &Mask,
) -> KernelResult<ListKind> {
    eval_rows(mask, |row| {
        let (Some(list), Some(start)) = (list.value(row), start.value(row)) else {
            return TypedRow::Null;
        };
        if !start.is_finite() {
            return TypedRow::Error(EvalError::InvalidArgument);
        }
        let start = normalize_index(*start, list.len(), true);
        let end = match end.as_ref() {
            None => list.len(),
            Some(end) => {
                let Some(end) = end.value(row) else {
                    return TypedRow::Null;
                };
                if !end.is_finite() {
                    return TypedRow::Error(EvalError::InvalidArgument);
                }
                normalize_index(*end, list.len(), true)
            }
        };
        TypedRow::Value(if start <= end {
            list[start..end].to_vec()
        } else {
            Vec::new()
        })
    })
}

pub(crate) fn eval_splice(args: SpliceArgs, mask: &Mask) -> KernelResult<ListKind> {
    let groups = args.repeat_groups.into_vec();
    eval_rows(mask, |row| {
        let (Some(list), Some(start), Some(delete_count)) = (
            args.list.value(row),
            args.start_index.value(row),
            args.delete_count.value(row),
        ) else {
            return TypedRow::Null;
        };
        if !start.is_finite() || !delete_count.is_finite() {
            return TypedRow::Error(EvalError::InvalidArgument);
        }
        let start = normalize_index(*start, list.len(), true);
        let delete_count = if *delete_count > 0.0 {
            delete_count.trunc() as usize
        } else {
            0
        };
        let end = start.saturating_add(delete_count).min(list.len());
        let mut output = list[..start].to_vec();
        for group in &groups {
            let Some(item) = group.items.value(row) else {
                return TypedRow::Null;
            };
            output.push(item.clone());
        }
        output.extend_from_slice(&list[end..]);
        TypedRow::Value(output)
    })
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ListTransform {
    Sort,
    Reverse,
    Unique,
}

pub(crate) fn eval_list_transform(
    list: KernelColumn<ListKind>,
    operation: ListTransform,
    mask: &Mask,
) -> KernelResult<ListKind> {
    eval_unary(&list, mask, |list| {
        let mut values = list.clone();
        match operation {
            ListTransform::Sort => values.sort_by(compare_value),
            ListTransform::Reverse => values.reverse(),
            ListTransform::Unique => {
                let mut unique = Vec::with_capacity(values.len());
                for value in values {
                    if !unique.contains(&value) {
                        unique.push(value);
                    }
                }
                values = unique;
            }
        }
        Ok(values)
    })
}

pub(crate) fn eval_includes(
    list: KernelColumn<ListKind>,
    value: KernelColumn<AnyKind>,
    mask: &Mask,
) -> KernelResult<BooleanKind> {
    eval_binary(&list, &value, mask, |list, value| Ok(list.contains(value)))
}

pub(crate) fn eval_flat(list: KernelColumn<ListKind>, mask: &Mask) -> KernelResult<ListKind> {
    eval_unary(&list, mask, |list| {
        let mut output = Vec::new();
        flatten_values(list, &mut output);
        Ok(output)
    })
}

pub(crate) fn eval_id<C: BuiltinValueContext>(context: &C, mask: &Mask) -> KernelResult<TextKind> {
    eval_rows(mask, |row| TypedRow::Value(context.rows()[row].to_string()))
}

fn finite_number(value: f64) -> Result<f64, EvalError> {
    value
        .is_finite()
        .then_some(value)
        .ok_or(EvalError::InvalidArgument)
}

fn bounded_count(value: f64) -> Result<usize, EvalError> {
    if !value.is_finite() || !(0.0..=1_000_000.0).contains(&value) {
        return Err(EvalError::InvalidArgument);
    }
    Ok(value.trunc() as usize)
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

fn translate_date_format(format: &str) -> String {
    const TOKENS: &[(&str, &str)] = &[
        ("YYYY", "%Y"),
        ("MMMM", "%B"),
        ("MMM", "%b"),
        ("dddd", "%A"),
        ("ddd", "%a"),
        ("YY", "%y"),
        ("MM", "%m"),
        ("DD", "%d"),
        ("HH", "%H"),
        ("hh", "%I"),
        ("mm", "%M"),
        ("ss", "%S"),
        ("Y", "%Y"),
        ("M", "%-m"),
        ("D", "%-d"),
        ("H", "%-H"),
        ("h", "%-I"),
        ("m", "%-M"),
        ("s", "%-S"),
        ("A", "%p"),
        ("a", "%P"),
    ];

    let mut output = String::with_capacity(format.len());
    let mut remaining = format;
    while !remaining.is_empty() {
        if let Some((token, replacement)) = TOKENS
            .iter()
            .find(|(token, _)| remaining.starts_with(token))
        {
            output.push_str(replacement);
            remaining = &remaining[token.len()..];
            continue;
        }

        let character = remaining
            .chars()
            .next()
            .expect("non-empty format has a character");
        if character == '%' {
            output.push_str("%%");
        } else {
            output.push(character);
        }
        remaining = &remaining[character.len_utf8()..];
    }
    output
}

fn list_index(index: f64, len: usize) -> Option<usize> {
    let index = index.trunc() as isize;
    let len = len as isize;
    let index = if index < 0 {
        len.checked_add(index)?
    } else {
        index
    };
    (0..len).contains(&index).then_some(index as usize)
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
    use super::*;

    #[test]
    fn abs_reuses_uniquely_owned_storage_for_in_place_work() {
        let input =
            KernelColumn::<NumberKind>::from_values(vec![-1.0, 2.0, -3.0], Validity::AllValid);
        let storage_address = input.values().as_ptr() as usize;

        let output = eval_abs(input, &Mask::all(3));

        assert_eq!(output.column.values(), &[1.0, 2.0, 3.0]);
        assert_eq!(output.column.values().as_ptr() as usize, storage_address);
    }
}
