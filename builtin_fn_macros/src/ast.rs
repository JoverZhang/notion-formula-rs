use proc_macro2::Span;
use syn::ext::IdentExt;
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, LitInt, Result, Token, braced, bracketed, parenthesized, token};

pub(crate) struct CategoryDecl {
    pub(crate) category: Ident,
    pub(crate) functions: Vec<FunctionDecl>,
    pub(crate) parse_errors: Vec<syn::Error>,
}

impl Parse for CategoryDecl {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let keyword = Ident::parse_any(input)?;
        if keyword != "category" {
            return Err(syn::Error::new(keyword.span(), "expected `category`"));
        }
        input.parse::<Token![:]>()?;
        let category = Ident::parse_any(input)?;
        input.parse::<Token![;]>()?;

        let mut functions = Vec::new();
        let mut parse_errors = Vec::new();
        while !input.is_empty() {
            // Failed parses must happen on a fork. Besides keeping the original
            // cursor at the declaration start for recovery, this prevents Syn's
            // nested-delimiter error tracker from poisoning the enclosing parse.
            let fork = input.fork();
            let parsed = fork
                .call(Attribute::parse_outer)
                .and_then(|attrs| FunctionDecl::parse_after_attrs(&fork, attrs));
            match parsed {
                Ok(function) => {
                    input.advance_to(&fork);
                    functions.push(function);
                }
                Err(error) => {
                    parse_errors.push(error);
                    recover_function(input)?;
                }
            }
        }
        Ok(Self {
            category,
            functions,
            parse_errors,
        })
    }
}

/// Skip the rest of a malformed declaration. Token groups are represented as one token
/// here, so a punctuation token found by this loop is necessarily a top-level semicolon.
fn recover_function(input: ParseStream<'_>) -> Result<()> {
    while !input.is_empty() {
        let token = input.parse::<proc_macro2::TokenTree>()?;
        if matches!(token, proc_macro2::TokenTree::Punct(ref punct) if punct.as_char() == ';') {
            break;
        }
    }
    Ok(())
}

pub(crate) struct FunctionDecl {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) name: Ident,
    pub(crate) generics: Vec<GenericDecl>,
    pub(crate) params: Vec<ParamItem>,
    pub(crate) ret: TypeAst,
}

impl FunctionDecl {
    fn parse_after_attrs(input: ParseStream<'_>, attrs: Vec<Attribute>) -> Result<Self> {
        let name = Ident::parse_any(input)?;
        let generics = if input.peek(Token![<]) {
            parse_generics(input)?
        } else {
            Vec::new()
        };

        let content;
        parenthesized!(content in input);
        let params = parse_param_items(&content)?;
        input.parse::<Token![->]>()?;
        let ret = input.parse()?;
        input.parse::<Token![;]>()?;

        Ok(Self {
            attrs,
            name,
            generics,
            params,
            ret,
        })
    }
}

pub(crate) struct GenericDecl {
    pub(crate) name: Ident,
    pub(crate) kind: Option<Ident>,
}

fn parse_generics(input: ParseStream<'_>) -> Result<Vec<GenericDecl>> {
    input.parse::<Token![<]>()?;
    let mut out = Vec::new();
    while !input.peek(Token![>]) {
        let name = Ident::parse_any(input)?;
        let kind = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(Ident::parse_any(input)?)
        } else {
            None
        };
        out.push(GenericDecl { name, kind });
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.peek(Token![>]) {
                break;
            }
        } else {
            break;
        }
    }
    input.parse::<Token![>]>()?;
    Ok(out)
}

pub(crate) enum ParamItem {
    Param(ParamDecl),
    Repeat(RepeatDecl),
}

pub(crate) struct RepeatDecl {
    pub(crate) keyword_span: Span,
    pub(crate) min: LitInt,
    pub(crate) params: Vec<ParamDecl>,
}

pub(crate) struct ParamDecl {
    pub(crate) name: Ident,
    pub(crate) optional: bool,
    pub(crate) ty: TypeAst,
}

fn parse_param_items(input: ParseStream<'_>) -> Result<Vec<ParamItem>> {
    let mut out = Vec::new();
    while !input.is_empty() {
        if next_is_repeat(input) {
            out.push(ParamItem::Repeat(parse_repeat(input)?));
        } else {
            out.push(ParamItem::Param(parse_param(input)?));
        }

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        } else if !input.is_empty() {
            return Err(input.error("expected `,` between parameters"));
        }
    }
    Ok(out)
}

fn next_is_repeat(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    let Ok(name) = Ident::parse_any(&fork) else {
        return false;
    };
    name == "repeat" && fork.peek(token::Paren)
}

fn parse_repeat(input: ParseStream<'_>) -> Result<RepeatDecl> {
    let keyword = Ident::parse_any(input)?;
    let keyword_span = keyword.span();
    debug_assert_eq!(keyword.to_string(), "repeat");

    let options;
    parenthesized!(options in input);
    let min_name = Ident::parse_any(&options)?;
    if min_name != "min" {
        return Err(syn::Error::new(min_name.span(), "expected `min`"));
    }
    options.parse::<Token![=]>()?;
    let min = options.parse::<LitInt>()?;
    if !options.is_empty() {
        return Err(options.error("repeat accepts only `min = <integer>`"));
    }

    let body;
    braced!(body in input);
    let mut params = Vec::new();
    while !body.is_empty() {
        params.push(parse_param(&body)?);
        if body.peek(Token![,]) {
            body.parse::<Token![,]>()?;
        } else if !body.is_empty() {
            return Err(body.error("expected `,` between repeat members"));
        }
    }

    Ok(RepeatDecl {
        keyword_span,
        min,
        params,
    })
}

fn parse_param(input: ParseStream<'_>) -> Result<ParamDecl> {
    let name = Ident::parse_any(input)?;
    let optional = if input.peek(Token![?]) {
        input.parse::<Token![?]>()?;
        true
    } else {
        false
    };
    input.parse::<Token![:]>()?;
    let ty = input.parse()?;
    Ok(ParamDecl { name, optional, ty })
}

#[derive(Clone)]
pub(crate) enum TypeAst {
    Number,
    String,
    Boolean,
    Date,
    Null,
    Any,
    Named(Ident),
    List(Box<TypeAst>),
    Union(Vec<TypeAst>),
    Fn {
        params: Vec<LambdaParamAst>,
        ret: Box<TypeAst>,
    },
    Ident {
        inner: Box<TypeAst>,
    },
}

#[derive(Clone)]
pub(crate) struct LambdaParamAst {
    pub(crate) name: Ident,
    pub(crate) ty: TypeAst,
}

impl Parse for TypeAst {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        parse_union(input)
    }
}

fn parse_union(input: ParseStream<'_>) -> Result<TypeAst> {
    let first = parse_postfix(input)?;
    if !input.peek(Token![|]) {
        return Ok(first);
    }

    let mut members = vec![first];
    while input.peek(Token![|]) {
        input.parse::<Token![|]>()?;
        members.push(parse_postfix(input)?);
    }
    Ok(TypeAst::Union(members))
}

fn parse_postfix(input: ParseStream<'_>) -> Result<TypeAst> {
    let mut ty = parse_primary(input)?;
    while input.peek(token::Bracket) {
        let content;
        bracketed!(content in input);
        if !content.is_empty() {
            return Err(content.error("list suffix must be `[]`"));
        }
        ty = TypeAst::List(Box::new(ty));
    }
    Ok(ty)
}

fn parse_primary(input: ParseStream<'_>) -> Result<TypeAst> {
    if input.peek(token::Paren) {
        let content;
        let paren = parenthesized!(content in input);
        let span = paren.span.open();
        if input.peek(Token![->]) {
            input.parse::<Token![->]>()?;
            let params = if content.is_empty() {
                Vec::new()
            } else {
                parse_lambda_params(&content)?
            };
            let ret = parse_union(input)?;
            return Ok(TypeAst::Fn {
                params,
                ret: Box::new(ret),
            });
        }
        if content.is_empty() {
            return Err(syn::Error::new(span, "empty grouped type is not valid"));
        }
        let ty = content.parse::<TypeAst>()?;
        if !content.is_empty() {
            return Err(content.error("unexpected token in grouped type"));
        }
        return Ok(ty);
    }

    let name = Ident::parse_any(input)?;
    match name.to_string().as_str() {
        "number" => Ok(TypeAst::Number),
        "string" => Ok(TypeAst::String),
        "boolean" => Ok(TypeAst::Boolean),
        "date" => Ok(TypeAst::Date),
        "null" => Ok(TypeAst::Null),
        "any" => Ok(TypeAst::Any),
        "Ident" if input.peek(Token![<]) => {
            input.parse::<Token![<]>()?;
            let inner = parse_union(input)?;
            input.parse::<Token![>]>()?;
            Ok(TypeAst::Ident {
                inner: Box::new(inner),
            })
        }
        _ => Ok(TypeAst::Named(name)),
    }
}

fn parse_lambda_params(input: ParseStream<'_>) -> Result<Vec<LambdaParamAst>> {
    let mut out = Vec::new();
    while !input.is_empty() {
        let name = Ident::parse_any(input)?;
        input.parse::<Token![:]>()?;
        let ty = input.parse::<TypeAst>()?;
        out.push(LambdaParamAst { name, ty });
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        } else if !input.is_empty() {
            return Err(input.error("expected `,` between lambda parameters"));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::CategoryDecl;

    #[test]
    fn malformed_function_recovers_at_its_semicolon() {
        let declaration: CategoryDecl = syn::parse_str(
            "category: General; malformed(value number) -> number; later(value: Missing) -> number;",
        )
        .expect("category header should remain parseable");

        assert_eq!(declaration.parse_errors.len(), 1);
        assert_eq!(declaration.functions.len(), 1);
        assert_eq!(declaration.functions[0].name, "later");
    }
}
