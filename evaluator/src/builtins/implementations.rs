use crate::core::columns::{
    AnyKind, BooleanKind, DateKind, KernelResult, ListKind, NumberKind, TextKind,
};
use crate::core::context::BuiltinValueContext;
use crate::core::types::Mask;
use crate::kernels::controlled::{
    eval_count, eval_every, eval_filter, eval_find, eval_find_index, eval_if, eval_ifs, eval_let,
    eval_map, eval_some,
};
use crate::kernels::helpers::{eval_fallible_selected, eval_infallible_all_slots, eval_null_aware};
use crate::kernels::value::{ceil_number, eval_abs, eval_value, is_empty_value, sqrt_number};

use super::contract::*;

macro_rules! impl_value_kernel {
    ($marker:ident, $kernel:ident, $args:ident, $return_kind:ty) => {
        impl $kernel for $marker {
            fn eval<C: BuiltinValueContext>(
                args: $args,
                context: &C,
                mask: &Mask,
            ) -> KernelResult<$return_kind> {
                eval_value::<$return_kind, _>(
                    BuiltinKey::$marker,
                    args.into_dynamic(),
                    context,
                    mask,
                )
            }
        }
    };
}

impl EmptyKernel for Empty {
    fn eval<C: BuiltinValueContext>(
        args: EmptyArgs,
        _context: &C,
        mask: &Mask,
    ) -> KernelResult<BooleanKind> {
        let std::option::Option::Some(value) = args.value else {
            return KernelResult {
                column: crate::KernelColumn::from_values(
                    vec![is_empty_value(None); mask.len()],
                    crate::Validity::AllValid,
                ),
                ok: Mask::all(mask.len()),
                errors: Vec::new(),
            };
        };
        eval_null_aware::<_, BooleanKind>(&value, mask, |value| {
            Ok(std::option::Option::Some(is_empty_value(value)))
        })
    }
}
impl_value_kernel!(Length, LengthKernel, LengthArgs, NumberKind);
impl_value_kernel!(Format, FormatKernel, FormatArgs, TextKind);
impl_value_kernel!(Equal, EqualKernel, EqualArgs, BooleanKind);
impl_value_kernel!(Unequal, UnequalKernel, UnequalArgs, BooleanKind);

impl_value_kernel!(Substring, SubstringKernel, SubstringArgs, TextKind);
impl_value_kernel!(Contains, ContainsKernel, ContainsArgs, BooleanKind);
impl_value_kernel!(Test, TestKernel, TestArgs, BooleanKind);
impl_value_kernel!(Match, MatchKernel, MatchArgs, ListKind);
impl_value_kernel!(Replace, ReplaceKernel, ReplaceArgs, TextKind);
impl_value_kernel!(ReplaceAll, ReplaceAllKernel, ReplaceAllArgs, TextKind);
impl_value_kernel!(Lower, LowerKernel, LowerArgs, TextKind);
impl_value_kernel!(Upper, UpperKernel, UpperArgs, TextKind);
impl_value_kernel!(Trim, TrimKernel, TrimArgs, TextKind);
impl_value_kernel!(Repeat, RepeatKernel, RepeatArgs, TextKind);
impl_value_kernel!(PadStart, PadStartKernel, PadStartArgs, TextKind);
impl_value_kernel!(PadEnd, PadEndKernel, PadEndArgs, TextKind);
impl_value_kernel!(Concat, ConcatKernel, ConcatArgs, ListKind);
impl_value_kernel!(Join, JoinKernel, JoinArgs, TextKind);
impl_value_kernel!(Split, SplitKernel, SplitArgs, ListKind);

impl_value_kernel!(FormatNumber, FormatNumberKernel, FormatNumberArgs, TextKind);
impl_value_kernel!(Add, AddKernel, AddArgs, NumberKind);
impl_value_kernel!(Subtract, SubtractKernel, SubtractArgs, NumberKind);
impl_value_kernel!(Multiply, MultiplyKernel, MultiplyArgs, NumberKind);
impl_value_kernel!(Mod, ModKernel, ModArgs, NumberKind);
impl_value_kernel!(Pow, PowKernel, PowArgs, NumberKind);
impl_value_kernel!(Divide, DivideKernel, DivideArgs, NumberKind);
impl_value_kernel!(Min, MinKernel, MinArgs, NumberKind);
impl_value_kernel!(Max, MaxKernel, MaxArgs, NumberKind);
impl_value_kernel!(Sum, SumKernel, SumArgs, NumberKind);
impl_value_kernel!(Median, MedianKernel, MedianArgs, NumberKind);
impl_value_kernel!(Mean, MeanKernel, MeanArgs, NumberKind);

impl AbsKernel for Abs {
    fn eval<C: BuiltinValueContext>(
        args: AbsArgs,
        _context: &C,
        mask: &Mask,
    ) -> KernelResult<NumberKind> {
        eval_abs(args.value, mask)
    }
}

impl_value_kernel!(Round, RoundKernel, RoundArgs, NumberKind);
impl CeilKernel for Ceil {
    fn eval<C: BuiltinValueContext>(
        args: CeilArgs,
        _context: &C,
        mask: &Mask,
    ) -> KernelResult<NumberKind> {
        eval_infallible_all_slots(&args.value, mask, |value| ceil_number(*value))
    }
}
impl_value_kernel!(Floor, FloorKernel, FloorArgs, NumberKind);
impl SqrtKernel for Sqrt {
    fn eval<C: BuiltinValueContext>(
        args: SqrtArgs,
        _context: &C,
        mask: &Mask,
    ) -> KernelResult<NumberKind> {
        eval_fallible_selected(&args.value, mask, |value| sqrt_number(*value))
    }
}
impl_value_kernel!(Cbrt, CbrtKernel, CbrtArgs, NumberKind);
impl_value_kernel!(Exp, ExpKernel, ExpArgs, NumberKind);
impl_value_kernel!(Ln, LnKernel, LnArgs, NumberKind);
impl_value_kernel!(Log10, Log10Kernel, Log10Args, NumberKind);
impl_value_kernel!(Log2, Log2Kernel, Log2Args, NumberKind);
impl_value_kernel!(Sign, SignKernel, SignArgs, NumberKind);
impl_value_kernel!(Pi, PiKernel, PiArgs, NumberKind);
impl_value_kernel!(E, EKernel, EArgs, NumberKind);
impl_value_kernel!(ToNumber, ToNumberKernel, ToNumberArgs, NumberKind);

impl_value_kernel!(Now, NowKernel, NowArgs, DateKind);
impl_value_kernel!(Today, TodayKernel, TodayArgs, DateKind);
impl_value_kernel!(Minute, MinuteKernel, MinuteArgs, NumberKind);
impl_value_kernel!(Hour, HourKernel, HourArgs, NumberKind);
impl_value_kernel!(Day, DayKernel, DayArgs, NumberKind);
impl_value_kernel!(Date, DateKernel, DateArgs, NumberKind);
impl_value_kernel!(Week, WeekKernel, WeekArgs, NumberKind);
impl_value_kernel!(Month, MonthKernel, MonthArgs, NumberKind);
impl_value_kernel!(Year, YearKernel, YearArgs, NumberKind);
impl_value_kernel!(DateAdd, DateAddKernel, DateAddArgs, DateKind);
impl_value_kernel!(DateSubtract, DateSubtractKernel, DateSubtractArgs, DateKind);
impl_value_kernel!(DateBetween, DateBetweenKernel, DateBetweenArgs, NumberKind);
impl_value_kernel!(Timestamp, TimestampKernel, TimestampArgs, NumberKind);
impl_value_kernel!(
    FromTimestamp,
    FromTimestampKernel,
    FromTimestampArgs,
    DateKind
);
impl_value_kernel!(FormatDate, FormatDateKernel, FormatDateArgs, TextKind);
impl_value_kernel!(ParseDate, ParseDateKernel, ParseDateArgs, DateKind);

impl_value_kernel!(At, AtKernel, AtArgs, AnyKind);
impl_value_kernel!(First, FirstKernel, FirstArgs, AnyKind);
impl_value_kernel!(Last, LastKernel, LastArgs, AnyKind);
impl_value_kernel!(Slice, SliceKernel, SliceArgs, ListKind);
impl_value_kernel!(Splice, SpliceKernel, SpliceArgs, ListKind);
impl_value_kernel!(Sort, SortKernel, SortArgs, ListKind);
impl_value_kernel!(Reverse, ReverseKernel, ReverseArgs, ListKind);
impl_value_kernel!(Unique, UniqueKernel, UniqueArgs, ListKind);
impl_value_kernel!(Includes, IncludesKernel, IncludesArgs, BooleanKind);
impl_value_kernel!(Flat, FlatKernel, FlatArgs, ListKind);

impl_value_kernel!(Id, IdKernel, IdArgs, TextKind);

// Controlled implementations are kept below the shared controlled kernel module so their
// generated method signatures remain compile-time obligations too.

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
