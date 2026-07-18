use builtin_fn::{FunctionSig, builtin_functions};

fn wrong_resolver(_input: &FunctionSig) -> FunctionSig {
    unreachable!()
}

fn main() {
    let _ = builtin_functions! {
        category: List;

        #[resolver(wrong_resolver)]
        flat<T>(list: T[]) -> T[];

        #[resolver(path::that::does_not_exist)]
        compact<T>(list: T[]) -> T[];
    };
}
