use crate::{BuiltinCategory, builtin_functions};

pub(super) fn definitions() -> BuiltinCategory {
    builtin_functions! {
        category: Date;

        now() -> date;
        today() -> date;
        minute(date: date) -> number;
        hour(date: date) -> number;
        day(date: date) -> number;
        date(date: date) -> number;
        week(date: date) -> number;
        month(date: date) -> number;
        year(date: date) -> number;
        dateAdd(date: date, amount: number, unit: string) -> date;
        dateSubtract(date: date, amount: number, unit: string) -> date;
        dateBetween(a: date, b: date, unit: string) -> number;

        #[unsupported]
        /// The semantic type model does not yet represent `DateRange`.
        dateRange(start: date, end: date) -> DateRange;

        #[unsupported]
        /// The semantic type model does not yet represent `DateRange`.
        dateStart(range: DateRange) -> date;

        #[unsupported]
        /// The semantic type model does not yet represent `DateRange`.
        dateEnd(range: DateRange) -> date;

        timestamp(date: date) -> number;
        fromTimestamp(timestamp: number) -> date;
        formatDate(date: date, format: string) -> string;
        parseDate(text: string) -> date;
    }
}
