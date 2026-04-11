use crate::{BuiltinSigParser, FunctionCategory, FunctionSig, SigResolver, default_parser};

mod date;
mod general;
mod list;
mod math;
mod people;
mod special;
mod text;

pub fn builtins_functions() -> Vec<FunctionSig> {
    let parser = default_parser();
    let mut out = Vec::new();
    out.extend(general::builtins(&parser));
    out.extend(text::builtins(&parser));
    out.extend(math::builtins(&parser));
    out.extend(date::builtins(&parser));
    out.extend(people::builtins(&parser));
    out.extend(list::builtins(&parser));
    out.extend(special::builtins(&parser));
    out
}

fn sig(parser: &BuiltinSigParser, category: FunctionCategory, text: &str) -> FunctionSig {
    parser
        .parse(category, text)
        .unwrap_or_else(|err| panic!("invalid builtin signature `{text}`: {err}"))
}

fn sig_with_detail(
    parser: &BuiltinSigParser,
    category: FunctionCategory,
    text: &str,
    detail: &str,
) -> FunctionSig {
    let mut sig = sig(parser, category, text);
    sig.detail = detail.to_string();
    sig
}

fn sig_with_resolver(
    parser: &BuiltinSigParser,
    category: FunctionCategory,
    text: &str,
    resolver: SigResolver,
) -> FunctionSig {
    let mut sig = sig(parser, category, text);
    sig.resolver = Some(resolver);
    sig
}
