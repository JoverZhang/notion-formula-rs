#[macro_use]
#[cfg(test)]
mod common;
#[cfg(test)]
mod completion_dsl;
#[cfg(test)]
mod test_ast_regression;
#[cfg(test)]
mod test_completion_position;
#[cfg(test)]
mod test_completion_ranking;
#[cfg(test)]
mod test_completion_signature_help;
#[cfg(test)]
mod test_completion_smoke;
#[cfg(test)]
mod test_errors;
#[cfg(test)]
mod test_format_idempotence;
#[cfg(test)]
mod test_invariants;
#[cfg(test)]
mod test_lexer;
#[cfg(test)]
mod test_generic_infer;
#[cfg(test)]
mod test_normalize_union;
#[cfg(test)]
mod test_parser;
#[cfg(test)]
mod test_parser_spans;
#[cfg(test)]
mod test_semantic;
#[cfg(test)]
mod test_semantic_infer_builtins;
#[cfg(test)]
mod test_token_query;
#[cfg(test)]
mod test_tokens_in_span;
