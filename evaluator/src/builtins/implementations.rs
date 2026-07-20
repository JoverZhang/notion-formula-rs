use crate::core::columns::{
    AnyKind, BooleanKind, DateKind, KernelResult, ListKind, NumberKind, TextKind,
};
use crate::core::context::BuiltinValueContext;
use crate::core::types::Mask;

use super::contract::*;

macro_rules! impl_value_kernel {
    ($marker:ident, $kernel:ident, $args:ident, $return_kind:ty) => {
        impl $kernel for $marker {
            fn eval<C: BuiltinValueContext>(
                _args: $args,
                _context: &C,
                _mask: &Mask,
            ) -> KernelResult<$return_kind> {
                todo!("builtin behavior lands in PR3")
            }
        }
    };
}

impl_value_kernel!(Empty, EmptyKernel, EmptyArgs, BooleanKind);
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
impl_value_kernel!(Abs, AbsKernel, AbsArgs, NumberKind);
impl_value_kernel!(Round, RoundKernel, RoundArgs, NumberKind);
impl_value_kernel!(Ceil, CeilKernel, CeilArgs, NumberKind);
impl_value_kernel!(Floor, FloorKernel, FloorArgs, NumberKind);
impl_value_kernel!(Sqrt, SqrtKernel, SqrtArgs, NumberKind);
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

macro_rules! impl_controlled_kernel {
    ($marker:ident, $kernel:ident, $plans:ident, $return_kind:ty) => {
        impl $kernel for $marker {
            fn eval<C: super::BuiltinEvalContext>(
                _context: &mut C,
                _args: $plans,
                _mask: &Mask,
            ) -> KernelResult<$return_kind> {
                todo!("builtin behavior lands in PR3")
            }
        }
    };
}

impl_controlled_kernel!(If, IfKernel, IfPlans, AnyKind);
impl_controlled_kernel!(Ifs, IfsKernel, IfsPlans, AnyKind);
impl_controlled_kernel!(Let, LetKernel, LetPlans, AnyKind);
impl_controlled_kernel!(Map, MapKernel, MapPlans, ListKind);
impl_controlled_kernel!(Filter, FilterKernel, FilterPlans, ListKind);
impl_controlled_kernel!(Find, FindKernel, FindPlans, AnyKind);
impl_controlled_kernel!(FindIndex, FindIndexKernel, FindIndexPlans, NumberKind);
impl_controlled_kernel!(Some, SomeKernel, SomePlans, BooleanKind);
impl_controlled_kernel!(Every, EveryKernel, EveryPlans, BooleanKind);
impl_controlled_kernel!(Count, CountKernel, CountPlans, NumberKind);
