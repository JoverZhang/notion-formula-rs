use crate::{
    ArgumentObservation, BuiltinCategory, FunctionSig, ResolverInput, Ty, builtin_functions,
    normalize_union,
};

fn general_definitions() -> BuiltinCategory {
    builtin_functions! {
        category: General;

        if<T: Variant>(condition: boolean, then: () -> T, else: () -> T) -> T;

        ifs<T: Variant>(
            repeat(min = 1) {
                condition: boolean,
                value: () -> T,
            },
            else: () -> T,
        ) -> T;

        #[unsupported]
        /// Currently expressed by the `&&` operator rather than a builtin call.
        and(
            repeat(min = 2) {
                condition: boolean,
            },
        ) -> boolean;

        #[unsupported]
        /// Currently expressed by the `||` operator rather than a builtin call.
        or(
            repeat(min = 2) {
                condition: boolean,
            },
        ) -> boolean;

        #[unsupported]
        /// Currently expressed by the `not` prefix operator rather than a builtin call.
        not(condition: boolean) -> boolean;

        empty(value?: any) -> boolean;
        length(value: string | any[]) -> number;
        format(value: any) -> string;
        equal(a: any, b: any) -> boolean;
        unequal(a: any, b: any) -> boolean;

        let<T, U>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U;

        #[unsupported]
        /// Precise sequential binder typing requires a heterogeneous binder-pack model.
        lets(
            repeat(min = 1) {
                var: Ident<any>,
                value: any,
            },
            expr: () -> any,
        ) -> any;
    }
}

fn text_definitions() -> BuiltinCategory {
    builtin_functions! {
        category: Text;

        substring(text: string, start: number, end?: number) -> string;
        contains(text: string, search: string) -> boolean;
        test(text: string, regex: string) -> boolean;
        match(text: string, regex: string) -> string[];
        replace(text: string, regex: string, replacement: string) -> string;
        replaceAll(text: string, regex: string, replacement: string) -> string;
        lower(text: string) -> string;
        upper(text: string) -> string;
        trim(text: string) -> string;
        repeat(text: string, times: number) -> string;
        padStart(text: string | number, length: number, pad: string) -> string;
        padEnd(text: string | number, length: number, pad: string) -> string;

        #[unsupported]
        /// The semantic type model does not yet represent `Link`.
        link(label: string, url: string) -> Link;

        #[unsupported]
        /// The semantic type model does not yet represent `StyledText`.
        style(
            text: string,
            repeat(min = 1) {
                styles: string,
            },
        ) -> StyledText;

        #[unsupported]
        /// The semantic type model does not yet represent `StyledText`.
        unstyle(text: string | StyledText, styles?: string) -> string;

        concat<T>(
            repeat(min = 2) {
                lists: T[],
            },
        ) -> T[];

        join<T>(list: T[], separator: string) -> string;
        split(text: string, separator: string) -> string[];
    }
}

fn math_definitions() -> BuiltinCategory {
    builtin_functions! {
        category: Number;

        formatNumber(value: number, format: string, precision: number) -> string;
        add(a: number, b: number) -> number;
        subtract(a: number, b: number) -> number;
        multiply(a: number, b: number) -> number;
        mod(a: number, b: number) -> number;
        pow(base: number, exp: number) -> number;
        divide(a: number, b: number) -> number;

        min(repeat(min = 1) { values: number | number[] }) -> number;
        max(repeat(min = 1) { values: number | number[] }) -> number;
        sum(repeat(min = 1) { values: number | number[] }) -> number;
        median(repeat(min = 1) { values: number | number[] }) -> number;
        mean(repeat(min = 1) { values: number | number[] }) -> number;

        abs(value: number) -> number;
        round(value: number, places?: number) -> number;
        ceil(value: number) -> number;
        floor(value: number) -> number;
        sqrt(value: number) -> number;
        cbrt(value: number) -> number;
        exp(value: number) -> number;
        ln(value: number) -> number;
        log10(value: number) -> number;
        log2(value: number) -> number;
        sign(value: number) -> number;
        pi() -> number;
        e() -> number;
        toNumber(value: any) -> number;
    }
}

fn date_definitions() -> BuiltinCategory {
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

fn people_definitions() -> BuiltinCategory {
    builtin_functions! {
        category: People;

        #[unsupported]
        /// Runtime inputs do not currently provide a person's display name.
        name(person: any) -> string;

        #[unsupported]
        /// Runtime inputs do not currently provide a person's email address.
        email(person: any) -> string;
    }
}

fn resolve_flat(input: &ResolverInput<'_>) -> Ty {
    match input.arguments.first() {
        Some(ArgumentObservation::Typed(Ty::List(element))) => {
            let mut leaves = Vec::new();
            collect_leaf_types(element, &mut leaves);
            Ty::List(Box::new(normalize_union(leaves)))
        }
        _ => input.default_return_ty.clone(),
    }
}

fn collect_leaf_types(ty: &Ty, out: &mut Vec<Ty>) {
    match ty {
        Ty::List(inner) => collect_leaf_types(inner, out),
        Ty::Union(members) => {
            for member in members {
                collect_leaf_types(member, out);
            }
        }
        other => out.push(other.clone()),
    }
}

fn list_definitions() -> BuiltinCategory {
    builtin_functions! {
        category: List;

        at<T>(list: T[], index: number) -> T;
        first<T>(list: T[]) -> T;
        last<T>(list: T[]) -> T;
        slice<T>(list: T[], start: number, end?: number) -> T[];

        splice<T>(
            list: T[],
            startIndex: number,
            deleteCount: number,
            repeat(min = 0) {
                items: T,
            },
        ) -> T[];

        sort<T>(list: T[]) -> T[];
        reverse<T>(list: T[]) -> T[];
        unique<T>(list: T[]) -> T[];
        includes<T>(list: T[], value: T) -> boolean;
        map<T, U>(list: T[], mapper: (current: T) -> U) -> U[];
        filter<T>(list: T[], predicate: (current: T) -> boolean) -> T[];
        find<T>(list: T[], predicate: (current: T) -> boolean) -> T;
        findIndex<T>(list: T[], predicate: (current: T) -> boolean) -> number;
        some<T>(list: T[], predicate: (current: T) -> boolean) -> boolean;
        every<T>(list: T[], predicate: (current: T) -> boolean) -> boolean;
        count<T>(list: T[], predicate: (current: T) -> boolean) -> number;

        #[resolver(resolve_flat)]
        flat<T>(list: T[]) -> T[];
    }
}

fn special_definitions() -> BuiltinCategory {
    builtin_functions! {
        category: Special;

        id() -> string;
    }
}

/// Return the complete declaration catalog in stable category order.
pub fn builtin_categories() -> Vec<BuiltinCategory> {
    vec![
        general_definitions(),
        text_definitions(),
        math_definitions(),
        date_definitions(),
        people_definitions(),
        list_definitions(),
        special_definitions(),
    ]
}

/// Return only declarations that have semantic and runtime implementation obligations.
pub fn builtins_functions() -> Vec<FunctionSig> {
    builtin_categories()
        .into_iter()
        .flat_map(BuiltinCategory::into_functions)
        .collect()
}
