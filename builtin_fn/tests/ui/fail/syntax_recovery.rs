use builtin_fn::builtin_functions;

fn main() {
    let _ = builtin_functions! {
        category: General;

        malformed(value number) -> number;

        later(value: MissingType) -> number;
    };
}
