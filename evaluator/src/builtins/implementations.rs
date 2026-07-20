use crate::core::columns::{
    AnyKind, BooleanKind, DateKind, KernelResult, ListKind, NumberKind, TextKind,
};
use crate::core::context::BuiltinValueContext;
use crate::core::types::Mask;
use crate::kernels::controlled::{
    eval_count, eval_every, eval_filter, eval_find, eval_find_index, eval_if, eval_ifs, eval_let,
    eval_map, eval_some,
};
use crate::kernels::helpers::{eval_fallible_selected, eval_infallible_all_slots};
use crate::kernels::value::{
    Aggregate, DatePart, DateShift, ListPick, ListTransform, NumericBinary, NumericUnary,
    RegexOperation, TextTransform, ceil_number, eval_abs, eval_aggregate, eval_concat,
    eval_constant_number, eval_contains, eval_date_between, eval_date_part, eval_date_shift,
    eval_empty, eval_equality, eval_flat, eval_format, eval_format_date, eval_format_number,
    eval_from_timestamp, eval_id, eval_includes, eval_join, eval_length, eval_list_pick,
    eval_list_transform, eval_now, eval_numeric_binary, eval_numeric_unary, eval_pad,
    eval_parse_date, eval_regex, eval_repeat, eval_round, eval_slice, eval_splice, eval_split,
    eval_substring, eval_text_transform, eval_timestamp, eval_to_number, eval_today, sqrt_number,
};

use super::contract::*;

macro_rules! impl_typed_value_kernel {
    (
        $marker:ident,
        $kernel:ident,
        $args_ty:ident,
        $return_kind:ty;
        $args:ident,
        $context:ident,
        $mask:ident => $body:expr
    ) => {
        impl $kernel for $marker {
            fn eval<C: BuiltinValueContext>(
                $args: $args_ty,
                $context: &C,
                $mask: &Mask,
            ) -> KernelResult<$return_kind> {
                $body
            }
        }
    };
}

impl_typed_value_kernel!(
    Empty, EmptyKernel, EmptyArgs, BooleanKind;
    args, _context, mask => eval_empty(args.value, mask)
);
impl_typed_value_kernel!(
    Length, LengthKernel, LengthArgs, NumberKind;
    args, _context, mask => eval_length(args.value, mask)
);
impl_typed_value_kernel!(
    Format, FormatKernel, FormatArgs, TextKind;
    args, context, mask => eval_format(args.value, context, mask)
);
impl_typed_value_kernel!(
    Equal, EqualKernel, EqualArgs, BooleanKind;
    args, _context, mask => eval_equality(args.a, args.b, false, mask)
);
impl_typed_value_kernel!(
    Unequal, UnequalKernel, UnequalArgs, BooleanKind;
    args, _context, mask => eval_equality(args.a, args.b, true, mask)
);

impl_typed_value_kernel!(
    Substring, SubstringKernel, SubstringArgs, TextKind;
    args, _context, mask => eval_substring(args.text, args.start, args.end, mask)
);
impl_typed_value_kernel!(
    Contains, ContainsKernel, ContainsArgs, BooleanKind;
    args, _context, mask => eval_contains(args.text, args.search, mask)
);
impl_typed_value_kernel!(
    Test, TestKernel, TestArgs, BooleanKind;
    args, _context, mask => eval_regex::<BooleanKind>(
        args.text, args.regex, None, RegexOperation::Test, mask
    )
);
impl_typed_value_kernel!(
    Match, MatchKernel, MatchArgs, ListKind;
    args, _context, mask => eval_regex::<ListKind>(
        args.text, args.regex, None, RegexOperation::Match, mask
    )
);
impl_typed_value_kernel!(
    Replace, ReplaceKernel, ReplaceArgs, TextKind;
    args, _context, mask => eval_regex::<TextKind>(
        args.text,
        args.regex,
        Option::Some(args.replacement),
        RegexOperation::ReplaceOne,
        mask,
    )
);
impl_typed_value_kernel!(
    ReplaceAll, ReplaceAllKernel, ReplaceAllArgs, TextKind;
    args, _context, mask => eval_regex::<TextKind>(
        args.text,
        args.regex,
        Option::Some(args.replacement),
        RegexOperation::ReplaceAll,
        mask,
    )
);
impl_typed_value_kernel!(
    Lower, LowerKernel, LowerArgs, TextKind;
    args, _context, mask => eval_text_transform(args.text, TextTransform::Lower, mask)
);
impl_typed_value_kernel!(
    Upper, UpperKernel, UpperArgs, TextKind;
    args, _context, mask => eval_text_transform(args.text, TextTransform::Upper, mask)
);
impl_typed_value_kernel!(
    Trim, TrimKernel, TrimArgs, TextKind;
    args, _context, mask => eval_text_transform(args.text, TextTransform::Trim, mask)
);
impl_typed_value_kernel!(
    Repeat, RepeatKernel, RepeatArgs, TextKind;
    args, _context, mask => eval_repeat(args.text, args.times, mask)
);
impl_typed_value_kernel!(
    PadStart, PadStartKernel, PadStartArgs, TextKind;
    args, _context, mask => eval_pad(args.text, args.length, args.pad, true, mask)
);
impl_typed_value_kernel!(
    PadEnd, PadEndKernel, PadEndArgs, TextKind;
    args, _context, mask => eval_pad(args.text, args.length, args.pad, false, mask)
);
impl_typed_value_kernel!(
    Concat, ConcatKernel, ConcatArgs, ListKind;
    args, _context, mask => eval_concat(args, mask)
);
impl_typed_value_kernel!(
    Join, JoinKernel, JoinArgs, TextKind;
    args, _context, mask => eval_join(args.list, args.separator, mask)
);
impl_typed_value_kernel!(
    Split, SplitKernel, SplitArgs, ListKind;
    args, _context, mask => eval_split(args.text, args.separator, mask)
);

impl_typed_value_kernel!(
    FormatNumber, FormatNumberKernel, FormatNumberArgs, TextKind;
    args, _context, mask => eval_format_number(args.value, args.format, args.precision, mask)
);
impl_typed_value_kernel!(
    Add, AddKernel, AddArgs, NumberKind;
    args, _context, mask => eval_numeric_binary(args.a, args.b, NumericBinary::Add, mask)
);
impl_typed_value_kernel!(
    Subtract, SubtractKernel, SubtractArgs, NumberKind;
    args, _context, mask => eval_numeric_binary(args.a, args.b, NumericBinary::Subtract, mask)
);
impl_typed_value_kernel!(
    Multiply, MultiplyKernel, MultiplyArgs, NumberKind;
    args, _context, mask => eval_numeric_binary(args.a, args.b, NumericBinary::Multiply, mask)
);
impl_typed_value_kernel!(
    Mod, ModKernel, ModArgs, NumberKind;
    args, _context, mask => eval_numeric_binary(args.a, args.b, NumericBinary::Mod, mask)
);
impl_typed_value_kernel!(
    Pow, PowKernel, PowArgs, NumberKind;
    args, _context, mask => eval_numeric_binary(args.base, args.exp, NumericBinary::Pow, mask)
);
impl_typed_value_kernel!(
    Divide, DivideKernel, DivideArgs, NumberKind;
    args, _context, mask => eval_numeric_binary(args.a, args.b, NumericBinary::Divide, mask)
);
impl_typed_value_kernel!(
    Min, MinKernel, MinArgs, NumberKind;
    args, _context, mask => eval_aggregate(
        args.repeat_groups
            .into_vec()
            .into_iter()
            .map(|group| group.values)
            .collect(),
        Aggregate::Min,
        mask,
    )
);
impl_typed_value_kernel!(
    Max, MaxKernel, MaxArgs, NumberKind;
    args, _context, mask => eval_aggregate(
        args.repeat_groups
            .into_vec()
            .into_iter()
            .map(|group| group.values)
            .collect(),
        Aggregate::Max,
        mask,
    )
);
impl_typed_value_kernel!(
    Sum, SumKernel, SumArgs, NumberKind;
    args, _context, mask => eval_aggregate(
        args.repeat_groups
            .into_vec()
            .into_iter()
            .map(|group| group.values)
            .collect(),
        Aggregate::Sum,
        mask,
    )
);
impl_typed_value_kernel!(
    Median, MedianKernel, MedianArgs, NumberKind;
    args, _context, mask => eval_aggregate(
        args.repeat_groups
            .into_vec()
            .into_iter()
            .map(|group| group.values)
            .collect(),
        Aggregate::Median,
        mask,
    )
);
impl_typed_value_kernel!(
    Mean, MeanKernel, MeanArgs, NumberKind;
    args, _context, mask => eval_aggregate(
        args.repeat_groups
            .into_vec()
            .into_iter()
            .map(|group| group.values)
            .collect(),
        Aggregate::Mean,
        mask,
    )
);

impl_typed_value_kernel!(
    Abs, AbsKernel, AbsArgs, NumberKind;
    args, _context, mask => eval_abs(args.value, mask)
);
impl_typed_value_kernel!(
    Round, RoundKernel, RoundArgs, NumberKind;
    args, _context, mask => eval_round(args.value, args.places, mask)
);
impl_typed_value_kernel!(
    Ceil, CeilKernel, CeilArgs, NumberKind;
    args, _context, mask => {
        eval_infallible_all_slots(&args.value, mask, |value| ceil_number(*value))
    }
);
impl_typed_value_kernel!(
    Floor, FloorKernel, FloorArgs, NumberKind;
    args, _context, mask => eval_numeric_unary(args.value, NumericUnary::Floor, mask)
);
impl_typed_value_kernel!(
    Sqrt, SqrtKernel, SqrtArgs, NumberKind;
    args, _context, mask => {
        eval_fallible_selected(&args.value, mask, |value| sqrt_number(*value))
    }
);
impl_typed_value_kernel!(
    Cbrt, CbrtKernel, CbrtArgs, NumberKind;
    args, _context, mask => eval_numeric_unary(args.value, NumericUnary::Cbrt, mask)
);
impl_typed_value_kernel!(
    Exp, ExpKernel, ExpArgs, NumberKind;
    args, _context, mask => eval_numeric_unary(args.value, NumericUnary::Exp, mask)
);
impl_typed_value_kernel!(
    Ln, LnKernel, LnArgs, NumberKind;
    args, _context, mask => eval_numeric_unary(args.value, NumericUnary::Ln, mask)
);
impl_typed_value_kernel!(
    Log10, Log10Kernel, Log10Args, NumberKind;
    args, _context, mask => eval_numeric_unary(args.value, NumericUnary::Log10, mask)
);
impl_typed_value_kernel!(
    Log2, Log2Kernel, Log2Args, NumberKind;
    args, _context, mask => eval_numeric_unary(args.value, NumericUnary::Log2, mask)
);
impl_typed_value_kernel!(
    Sign, SignKernel, SignArgs, NumberKind;
    args, _context, mask => eval_numeric_unary(args.value, NumericUnary::Sign, mask)
);
impl_typed_value_kernel!(
    Pi, PiKernel, PiArgs, NumberKind;
    _args, _context, mask => eval_constant_number(std::f64::consts::PI, mask)
);
impl_typed_value_kernel!(
    E, EKernel, EArgs, NumberKind;
    _args, _context, mask => eval_constant_number(std::f64::consts::E, mask)
);
impl_typed_value_kernel!(
    ToNumber, ToNumberKernel, ToNumberArgs, NumberKind;
    args, _context, mask => eval_to_number(args.value, mask)
);

impl_typed_value_kernel!(
    Now, NowKernel, NowArgs, DateKind;
    _args, context, mask => eval_now(context, mask)
);
impl_typed_value_kernel!(
    Today, TodayKernel, TodayArgs, DateKind;
    _args, context, mask => eval_today(context, mask)
);
impl_typed_value_kernel!(
    Minute, MinuteKernel, MinuteArgs, NumberKind;
    args, context, mask => eval_date_part(args.date, DatePart::Minute, context, mask)
);
impl_typed_value_kernel!(
    Hour, HourKernel, HourArgs, NumberKind;
    args, context, mask => eval_date_part(args.date, DatePart::Hour, context, mask)
);
impl_typed_value_kernel!(
    Day, DayKernel, DayArgs, NumberKind;
    args, context, mask => eval_date_part(args.date, DatePart::Day, context, mask)
);
impl_typed_value_kernel!(
    Date, DateKernel, DateArgs, NumberKind;
    args, context, mask => eval_date_part(args.date, DatePart::Date, context, mask)
);
impl_typed_value_kernel!(
    Week, WeekKernel, WeekArgs, NumberKind;
    args, context, mask => eval_date_part(args.date, DatePart::Week, context, mask)
);
impl_typed_value_kernel!(
    Month, MonthKernel, MonthArgs, NumberKind;
    args, context, mask => eval_date_part(args.date, DatePart::Month, context, mask)
);
impl_typed_value_kernel!(
    Year, YearKernel, YearArgs, NumberKind;
    args, context, mask => eval_date_part(args.date, DatePart::Year, context, mask)
);
impl_typed_value_kernel!(
    DateAdd, DateAddKernel, DateAddArgs, DateKind;
    args, context, mask => eval_date_shift(
        args.date,
        args.amount,
        args.unit,
        DateShift::Add,
        context,
        mask,
    )
);
impl_typed_value_kernel!(
    DateSubtract, DateSubtractKernel, DateSubtractArgs, DateKind;
    args, context, mask => eval_date_shift(
        args.date,
        args.amount,
        args.unit,
        DateShift::Subtract,
        context,
        mask,
    )
);
impl_typed_value_kernel!(
    DateBetween, DateBetweenKernel, DateBetweenArgs, NumberKind;
    args, context, mask => eval_date_between(args.a, args.b, args.unit, context, mask)
);
impl_typed_value_kernel!(
    Timestamp, TimestampKernel, TimestampArgs, NumberKind;
    args, _context, mask => eval_timestamp(args.date, mask)
);
impl_typed_value_kernel!(
    FromTimestamp, FromTimestampKernel, FromTimestampArgs, DateKind;
    args, _context, mask => eval_from_timestamp(args.timestamp, mask)
);
impl_typed_value_kernel!(
    FormatDate, FormatDateKernel, FormatDateArgs, TextKind;
    args, context, mask => eval_format_date(args.date, args.format, context, mask)
);
impl_typed_value_kernel!(
    ParseDate, ParseDateKernel, ParseDateArgs, DateKind;
    args, context, mask => eval_parse_date(args.text, context, mask)
);

impl_typed_value_kernel!(
    At, AtKernel, AtArgs, AnyKind;
    args, _context, mask => eval_list_pick(args.list, Option::Some(args.index), ListPick::At, mask)
);
impl_typed_value_kernel!(
    First, FirstKernel, FirstArgs, AnyKind;
    args, _context, mask => eval_list_pick(args.list, None, ListPick::First, mask)
);
impl_typed_value_kernel!(
    Last, LastKernel, LastArgs, AnyKind;
    args, _context, mask => eval_list_pick(args.list, None, ListPick::Last, mask)
);
impl_typed_value_kernel!(
    Slice, SliceKernel, SliceArgs, ListKind;
    args, _context, mask => eval_slice(args.list, args.start, args.end, mask)
);
impl_typed_value_kernel!(
    Splice, SpliceKernel, SpliceArgs, ListKind;
    args, _context, mask => eval_splice(args, mask)
);
impl_typed_value_kernel!(
    Sort, SortKernel, SortArgs, ListKind;
    args, _context, mask => eval_list_transform(args.list, ListTransform::Sort, mask)
);
impl_typed_value_kernel!(
    Reverse, ReverseKernel, ReverseArgs, ListKind;
    args, _context, mask => eval_list_transform(args.list, ListTransform::Reverse, mask)
);
impl_typed_value_kernel!(
    Unique, UniqueKernel, UniqueArgs, ListKind;
    args, _context, mask => eval_list_transform(args.list, ListTransform::Unique, mask)
);
impl_typed_value_kernel!(
    Includes, IncludesKernel, IncludesArgs, BooleanKind;
    args, _context, mask => eval_includes(args.list, args.value, mask)
);
impl_typed_value_kernel!(
    Flat, FlatKernel, FlatArgs, ListKind;
    args, _context, mask => eval_flat(args.list, mask)
);
impl_typed_value_kernel!(
    Id, IdKernel, IdArgs, TextKind;
    _args, context, mask => eval_id(context, mask)
);

impl IfKernel for If {
    fn eval<C: super::BuiltinEvalContext>(
        context: &mut C,
        args: IfPlans,
        mask: &Mask,
    ) -> KernelResult<AnyKind> {
        eval_if(context, args, mask)
    }
}

impl IfsKernel for Ifs {
    fn eval<C: super::BuiltinEvalContext>(
        context: &mut C,
        args: IfsPlans,
        mask: &Mask,
    ) -> KernelResult<AnyKind> {
        eval_ifs(context, args, mask)
    }
}

impl LetKernel for Let {
    fn eval<C: super::BuiltinEvalContext>(
        context: &mut C,
        args: LetPlans,
        mask: &Mask,
    ) -> KernelResult<AnyKind> {
        eval_let(context, args, mask)
    }
}

impl MapKernel for Map {
    fn eval<C: super::BuiltinEvalContext>(
        context: &mut C,
        args: MapPlans,
        mask: &Mask,
    ) -> KernelResult<ListKind> {
        eval_map(context, args, mask)
    }
}

impl FilterKernel for Filter {
    fn eval<C: super::BuiltinEvalContext>(
        context: &mut C,
        args: FilterPlans,
        mask: &Mask,
    ) -> KernelResult<ListKind> {
        eval_filter(context, args, mask)
    }
}

impl FindKernel for Find {
    fn eval<C: super::BuiltinEvalContext>(
        context: &mut C,
        args: FindPlans,
        mask: &Mask,
    ) -> KernelResult<AnyKind> {
        eval_find(context, args, mask)
    }
}

impl FindIndexKernel for FindIndex {
    fn eval<C: super::BuiltinEvalContext>(
        context: &mut C,
        args: FindIndexPlans,
        mask: &Mask,
    ) -> KernelResult<NumberKind> {
        eval_find_index(context, args, mask)
    }
}

impl SomeKernel for Some {
    fn eval<C: super::BuiltinEvalContext>(
        context: &mut C,
        args: SomePlans,
        mask: &Mask,
    ) -> KernelResult<BooleanKind> {
        eval_some(context, args, mask)
    }
}

impl EveryKernel for Every {
    fn eval<C: super::BuiltinEvalContext>(
        context: &mut C,
        args: EveryPlans,
        mask: &Mask,
    ) -> KernelResult<BooleanKind> {
        eval_every(context, args, mask)
    }
}

impl CountKernel for Count {
    fn eval<C: super::BuiltinEvalContext>(
        context: &mut C,
        args: CountPlans,
        mask: &Mask,
    ) -> KernelResult<NumberKind> {
        eval_count(context, args, mask)
    }
}
