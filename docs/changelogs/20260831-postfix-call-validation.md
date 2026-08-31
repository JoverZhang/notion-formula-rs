# Postfix call validation

- Type: Fixed
- Component: analyzer

## Summary

Semantic analysis now reports unsupported and unknown postfix member calls instead of silently
leaving their type as `Unknown`. Supported postfix calls continue to use the same inference and
validation rules as their prefix form.

## Compatibility notes

- Formulas such as `true.sum()` and `true.noSuchFn()` now produce semantic diagnostics.
- Prefix calls and supported postfix calls are unchanged.

## Tests

- `cargo test -p analyzer`

## Links

- `docs/design/analyzer.md`
