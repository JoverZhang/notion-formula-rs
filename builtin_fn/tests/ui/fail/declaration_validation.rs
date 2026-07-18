use builtin_fn::builtin_functions;

fn main() {
    let _ = builtin_functions! {
        category: General;

        duplicate<T, T: Mystery>(first?: number, required: MissingType) -> T;
        duplicate() -> number;

        invalidRepeat(
            optionalHead?: number,
            repeat(min = 1u8) {
                values1?: number,
            },
            tail?: number,
            repeat(min = 0) {},
        ) -> number;

        #[unsupported]
        undocumented() -> FutureType;

        #[unsupported]
        #[resolver(crate::resolver)]
        /// Conflicting declaration attributes.
        conflicted() -> number;

        #[unknown]
        attributed() -> number;
    };
}

fn resolver(
    sig: &builtin_fn::FunctionSig,
    _arguments: &[builtin_fn::Ty],
) -> builtin_fn::FunctionSig {
    sig.clone()
}
