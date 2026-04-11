# builtin_fn

Builtin function signature definitions and parsing infrastructure.

This crate owns:

- the shared formula type model used by builtin signatures
- builtin signature shapes and validation
- deterministic union normalization helpers
- the string-driven builtin signature parser
- the canonical builtin registry used by `analyzer`

`analyzer::semantic` re-exports these types so downstream crates can keep using
the existing API surface while the ownership boundary lives here.

## Entry points

- `builtin_fn::builtins_functions() -> Vec<FunctionSig>`
- `builtin_fn::default_parser() -> BuiltinSigParser`
- `BuiltinSigParser::parse(category, text) -> Result<FunctionSig, BuiltinSigParseError>`

## Notes

- The parser is intentionally narrow: it only supports the signature language
  needed by the builtin registry.
- `SigResolver` remains the escape hatch for builtins such as `flat`.
