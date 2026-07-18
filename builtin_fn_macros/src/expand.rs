use std::collections::{HashMap, HashSet};

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Attribute, Expr, ExprLit, Lit, Meta, Path, spanned::Spanned};

use crate::ast::{
    CategoryDecl, FunctionDecl, LambdaParamAst, ParamDecl, ParamItem, RepeatDecl, TypeAst,
};

const ERROR_LIMIT: usize = 32;

pub(crate) fn expand(category: CategoryDecl) -> syn::Result<TokenStream> {
    validate(&category)?;

    let category_variant = category_variant(&category.category.to_string()).expect("validated");
    let entries = category
        .functions
        .iter()
        .map(|function| expand_function(category_variant.clone(), function))
        .collect::<Vec<_>>();

    Ok(quote! {
        ::builtin_fn::BuiltinCategory::new(
            ::builtin_fn::FunctionCategory::#category_variant,
            vec![#(#entries),*],
        )
    })
}

fn expand_function(category: syn::Ident, function: &FunctionDecl) -> TokenStream {
    let attrs = parse_attrs(&function.attrs, &mut Errors::default());
    let name = function.name.to_string();
    let signature = canonical_signature(function);
    let detail = canonical_detail(function);
    let docs = attrs.docs.iter().map(|doc| quote!(#doc.to_string()));

    if attrs.unsupported {
        return quote! {
            ::builtin_fn::BuiltinCatalogEntry::unsupported(
                ::builtin_fn::FunctionCategory::#category,
                #name,
                #signature,
                #detail,
                vec![#(#docs),*],
            )
        };
    }

    let mut generic_ids = HashMap::new();
    let mut generic_values = Vec::new();
    for (index, generic) in function.generics.iter().enumerate() {
        generic_ids.insert(generic.name.to_string(), index as u32);
        let id = index as u32;
        let kind = match generic.kind.as_ref().map(ToString::to_string).as_deref() {
            Some("Variant") => quote!(::builtin_fn::GenericParamKind::Variant),
            _ => quote!(::builtin_fn::GenericParamKind::Plain),
        };
        generic_values.push(quote! {
            ::builtin_fn::GenericParam {
                id: ::builtin_fn::GenericId(#id),
                kind: #kind,
            }
        });
    }

    let any_id = contains_any(function).then_some(function.generics.len() as u32);
    if let Some(id) = any_id {
        generic_values.push(quote! {
            ::builtin_fn::GenericParam {
                id: ::builtin_fn::GenericId(#id),
                kind: ::builtin_fn::GenericParamKind::Plain,
            }
        });
    }

    let shape = split_shape(function);
    let head = shape
        .head
        .iter()
        .map(|param| expand_param(param, &generic_ids, any_id));
    let repeat = shape
        .repeat
        .iter()
        .map(|param| expand_param(param, &generic_ids, any_id));
    let tail = shape
        .tail
        .iter()
        .map(|param| expand_param(param, &generic_ids, any_id));
    let repeat_min = shape.min;
    let ret = expand_type(&function.ret, &generic_ids, any_id);
    let function_name = function.name.to_string();

    let sig = if let Some(resolver) = attrs.resolver {
        quote! {
            ::builtin_fn::FunctionSig::new_builtin_with_resolver(
                ::builtin_fn::FunctionCategory::#category,
                #detail,
                #function_name,
                ::builtin_fn::ParamShape::new(
                    vec![#(#head),*],
                    vec![#(#repeat),*],
                    vec![#(#tail),*],
                ).with_repeat_min_groups(#repeat_min),
                #ret,
                vec![#(#generic_values),*],
                #resolver,
            )
        }
    } else {
        quote! {
            ::builtin_fn::FunctionSig::new_builtin(
                ::builtin_fn::FunctionCategory::#category,
                #detail,
                #function_name,
                ::builtin_fn::ParamShape::new(
                    vec![#(#head),*],
                    vec![#(#repeat),*],
                    vec![#(#tail),*],
                ).with_repeat_min_groups(#repeat_min),
                #ret,
                vec![#(#generic_values),*],
            )
        }
    };

    quote! {
        {
            let implementation = #sig;
            ::builtin_fn::BuiltinCatalogEntry::supported(
                ::builtin_fn::FunctionCategory::#category,
                #signature,
                #detail,
                vec![#(#docs),*],
                implementation,
            )
        }
    }
}

fn expand_param(
    param: &ParamDecl,
    generics: &HashMap<String, u32>,
    any_id: Option<u32>,
) -> TokenStream {
    let name = param.name.to_string();
    let optional = param.optional;
    let ty = expand_type(&param.ty, generics, any_id);
    quote! {
        ::builtin_fn::ParamSig {
            name: #name.to_string(),
            ty: #ty,
            optional: #optional,
        }
    }
}

fn expand_type(ty: &TypeAst, generics: &HashMap<String, u32>, any_id: Option<u32>) -> TokenStream {
    match ty {
        TypeAst::Number => quote!(::builtin_fn::Ty::Number),
        TypeAst::String => quote!(::builtin_fn::Ty::String),
        TypeAst::Boolean => quote!(::builtin_fn::Ty::Boolean),
        TypeAst::Date => quote!(::builtin_fn::Ty::Date),
        TypeAst::Null => quote!(::builtin_fn::Ty::Null),
        TypeAst::Any => {
            let id = any_id.expect("contains_any supplies a hidden generic");
            quote!(::builtin_fn::Ty::Generic(::builtin_fn::GenericId(#id)))
        }
        TypeAst::Named(name) => {
            let id = generics[&name.to_string()];
            quote!(::builtin_fn::Ty::Generic(::builtin_fn::GenericId(#id)))
        }
        TypeAst::List(inner) => {
            let inner = expand_type(inner, generics, any_id);
            quote!(::builtin_fn::Ty::List(Box::new(#inner)))
        }
        TypeAst::Union(members) => {
            let members = members
                .iter()
                .map(|member| expand_type(member, generics, any_id));
            quote!(::builtin_fn::Ty::Union(vec![#(#members),*]))
        }
        TypeAst::Fn { params, ret } => {
            let params = params.iter().map(|param| {
                let name = param.name.to_string();
                let lambda_param = if name == "current" {
                    quote!(::builtin_fn::LambdaParam::Current)
                } else {
                    quote!(::builtin_fn::LambdaParam::ParamRef(#name.to_string()))
                };
                let ty = expand_type(&param.ty, generics, any_id);
                quote!((#lambda_param, #ty))
            });
            let ret = expand_type(ret, generics, any_id);
            quote! {
                ::builtin_fn::Ty::Fn {
                    params: vec![#(#params),*],
                    ret: Box::new(#ret),
                }
            }
        }
        TypeAst::Ident { inner } => {
            let inner = expand_type(inner, generics, any_id);
            quote!(::builtin_fn::Ty::Ident(Box::new(#inner)))
        }
    }
}

#[derive(Default)]
struct AttrInfo {
    unsupported: bool,
    unsupported_span: Option<Span>,
    resolver: Option<Path>,
    docs: Vec<String>,
}

fn parse_attrs(attrs: &[Attribute], errors: &mut Errors) -> AttrInfo {
    let mut out = AttrInfo::default();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            let Meta::NameValue(meta) = &attr.meta else {
                errors.push(syn::Error::new(attr.span(), "malformed doc comment"));
                continue;
            };
            let Expr::Lit(ExprLit {
                lit: Lit::Str(text),
                ..
            }) = &meta.value
            else {
                errors.push(syn::Error::new(attr.span(), "doc comment must be text"));
                continue;
            };
            out.docs.push(text.value().trim().to_string());
            continue;
        }

        if attr.path().is_ident("unsupported") {
            if out.unsupported {
                errors.push(syn::Error::new(attr.span(), "duplicate `#[unsupported]`"));
            }
            if !matches!(attr.meta, Meta::Path(_)) {
                errors.push(syn::Error::new(
                    attr.span(),
                    "`#[unsupported]` takes no arguments",
                ));
            }
            if out.resolver.is_some() {
                errors.push(syn::Error::new(
                    attr.span(),
                    "`#[unsupported]` cannot be combined with `#[resolver(...)]`",
                ));
            }
            out.unsupported = true;
            out.unsupported_span = Some(attr.span());
            continue;
        }

        if attr.path().is_ident("resolver") {
            if out.resolver.is_some() {
                errors.push(syn::Error::new(attr.span(), "duplicate `#[resolver(...)]`"));
                continue;
            }
            if out.unsupported {
                errors.push(syn::Error::new(
                    attr.span(),
                    "`#[unsupported]` cannot be combined with `#[resolver(...)]`",
                ));
            }
            match attr.parse_args::<Path>() {
                Ok(path) => {
                    out.resolver = Some(path);
                }
                Err(error) => errors.push(error),
            }
            continue;
        }

        errors.push(syn::Error::new(
            attr.span(),
            "unsupported builtin declaration attribute",
        ));
    }
    out
}

fn validate(category: &CategoryDecl) -> syn::Result<()> {
    let mut errors = Errors::default();
    for error in &category.parse_errors {
        errors.push(syn::Error::new(error.span(), error.to_string()));
    }
    if category_variant(&category.category.to_string()).is_none() {
        errors.push(syn::Error::new(
            category.category.span(),
            "unknown builtin category",
        ));
    }

    let mut function_names = HashMap::<String, Span>::new();
    for function in &category.functions {
        let function_name = function.name.to_string();
        if let Some(original) = function_names.insert(function_name.clone(), function.name.span()) {
            errors.push(syn::Error::new(
                function.name.span(),
                format!("duplicate builtin `{function_name}`"),
            ));
            errors.push(syn::Error::new(original, "first declaration is here"));
        }
        validate_function(function, &mut errors);
    }
    errors.finish()
}

fn validate_function(function: &FunctionDecl, errors: &mut Errors) {
    let attrs = parse_attrs(&function.attrs, errors);
    if attrs.unsupported && attrs.docs.is_empty() {
        let span = attrs.unsupported_span.unwrap_or(function.name.span());
        errors.push(syn::Error::new(
            span,
            "`#[unsupported]` declarations require a doc comment",
        ));
    }
    let mut generics = HashSet::new();
    for generic in &function.generics {
        let name = generic.name.to_string();
        if !generics.insert(name.clone()) {
            errors.push(syn::Error::new(
                generic.name.span(),
                format!("duplicate generic `{name}`"),
            ));
        }
        if let Some(kind) = &generic.kind
            && kind != "Plain"
            && kind != "Variant"
        {
            errors.push(syn::Error::new(
                kind.span(),
                format!("unknown generic kind `{kind}`; expected `Plain` or `Variant`"),
            ));
        }
    }

    let repeats = function
        .params
        .iter()
        .filter_map(|item| match item {
            ParamItem::Repeat(repeat) => Some(repeat),
            ParamItem::Param(_) => None,
        })
        .collect::<Vec<_>>();
    if repeats.len() > 1 {
        for repeat in repeats.iter().skip(1) {
            errors.push(syn::Error::new(
                repeat.keyword_span,
                "a function may contain at most one repeat block",
            ));
        }
    }

    if !repeats.is_empty() {
        for repeat in &repeats {
            validate_repeat(repeat, errors);
        }
        for item in &function.params {
            if let ParamItem::Param(param) = item
                && param.optional
            {
                errors.push(syn::Error::new(
                    param.name.span(),
                    "fixed parameters cannot be optional when repeat is present",
                ));
            }
        }
    } else {
        let mut saw_optional = false;
        for item in &function.params {
            let ParamItem::Param(param) = item else {
                continue;
            };
            if param.optional {
                saw_optional = true;
            } else if saw_optional {
                errors.push(syn::Error::new(
                    param.name.span(),
                    "required fixed parameter cannot follow an optional parameter",
                ));
            }
        }
    }

    let mut logical_names = HashSet::new();
    let mut rust_names = HashMap::<String, Span>::new();
    for param in all_params(function) {
        let name = param.name.to_string();
        if !logical_names.insert(name.clone()) {
            errors.push(syn::Error::new(
                param.name.span(),
                format!("duplicate parameter `{name}`"),
            ));
        }
        let rust_name = rust_field_name(&name);
        if rust_names
            .insert(rust_name.clone(), param.name.span())
            .is_some()
        {
            errors.push(syn::Error::new(
                param.name.span(),
                format!("parameter field name `{rust_name}` collides after snake_case conversion"),
            ));
        }
    }

    if !attrs.unsupported {
        for param in all_params(function) {
            validate_type(&param.ty, &generics, errors);
        }
        validate_type(&function.ret, &generics, errors);
    }
}

fn validate_repeat(repeat: &RepeatDecl, errors: &mut Errors) {
    if repeat.params.is_empty() {
        errors.push(syn::Error::new(
            repeat.keyword_span,
            "repeat block cannot be empty",
        ));
    }
    if !repeat.min.suffix().is_empty() || repeat.min.base10_parse::<usize>().is_err() {
        errors.push(syn::Error::new(
            repeat.min.span(),
            "repeat minimum must be a non-negative unsuffixed integer",
        ));
    }
    for param in &repeat.params {
        if param.optional {
            errors.push(syn::Error::new(
                param.name.span(),
                "repeat members cannot be optional",
            ));
        }
        let name = param.name.to_string();
        if name.ends_with('N') || name.chars().last().is_some_and(|ch| ch.is_ascii_digit()) {
            errors.push(syn::Error::new(
                param.name.span(),
                "repeat member names must be logical base names without numeric or `N` suffixes",
            ));
        }
    }
}

fn validate_type(ty: &TypeAst, generics: &HashSet<String>, errors: &mut Errors) {
    match ty {
        TypeAst::Named(name) => {
            let name_text = name.to_string();
            if !generics.contains(&name_text) {
                errors.push(syn::Error::new(
                    name.span(),
                    format!("unknown type or generic `{name_text}`"),
                ));
            }
        }
        TypeAst::List(inner) | TypeAst::Ident { inner } => validate_type(inner, generics, errors),
        TypeAst::Union(members) => {
            for member in members {
                validate_type(member, generics, errors);
            }
        }
        TypeAst::Fn { params, ret } => {
            for param in params {
                validate_type(&param.ty, generics, errors);
            }
            validate_type(ret, generics, errors);
        }
        TypeAst::Number
        | TypeAst::String
        | TypeAst::Boolean
        | TypeAst::Date
        | TypeAst::Null
        | TypeAst::Any => {}
    }
}

fn all_params(function: &FunctionDecl) -> impl Iterator<Item = &ParamDecl> {
    function.params.iter().flat_map(|item| match item {
        ParamItem::Param(param) => std::slice::from_ref(param).iter(),
        ParamItem::Repeat(repeat) => repeat.params.iter(),
    })
}

struct Shape<'a> {
    head: Vec<&'a ParamDecl>,
    repeat: Vec<&'a ParamDecl>,
    tail: Vec<&'a ParamDecl>,
    min: usize,
}

fn split_shape(function: &FunctionDecl) -> Shape<'_> {
    let mut head = Vec::new();
    let mut repeat = Vec::new();
    let mut tail = Vec::new();
    let mut saw_repeat = false;
    let mut min = 1;
    for item in &function.params {
        match item {
            ParamItem::Param(param) if saw_repeat => tail.push(param),
            ParamItem::Param(param) => head.push(param),
            ParamItem::Repeat(group) => {
                saw_repeat = true;
                repeat.extend(group.params.iter());
                min = group.min.base10_parse().expect("validated repeat minimum");
            }
        }
    }
    Shape {
        head,
        repeat,
        tail,
        min,
    }
}

fn contains_any(function: &FunctionDecl) -> bool {
    all_params(function).any(|param| type_contains_any(&param.ty))
        || type_contains_any(&function.ret)
}

fn type_contains_any(ty: &TypeAst) -> bool {
    match ty {
        TypeAst::Any => true,
        TypeAst::List(inner) | TypeAst::Ident { inner } => type_contains_any(inner),
        TypeAst::Union(members) => members.iter().any(type_contains_any),
        TypeAst::Fn { params, ret } => {
            params.iter().any(|param| type_contains_any(&param.ty)) || type_contains_any(ret)
        }
        TypeAst::Number
        | TypeAst::String
        | TypeAst::Boolean
        | TypeAst::Date
        | TypeAst::Null
        | TypeAst::Named(_) => false,
    }
}

fn canonical_signature(function: &FunctionDecl) -> String {
    let mut out = function.name.to_string();
    if !function.generics.is_empty() {
        out.push('<');
        for (index, generic) in function.generics.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            out.push_str(&generic.name.to_string());
            if generic.kind.as_ref().is_some_and(|kind| kind == "Variant") {
                out.push_str(": Variant");
            }
        }
        out.push('>');
    }
    out.push('(');
    out.push_str(&render_params(function, true));
    out.push_str(") -> ");
    out.push_str(&render_type(&function.ret, 0));
    out
}

fn canonical_detail(function: &FunctionDecl) -> String {
    format!("{}({})", function.name, render_params(function, false))
}

fn render_params(function: &FunctionDecl, include_types: bool) -> String {
    let shape = split_shape(function);
    let mut rendered = Vec::new();
    for param in &shape.head {
        rendered.push(render_param(param, None, include_types));
    }
    if !shape.repeat.is_empty() {
        for group in 1..=shape.min.max(2) {
            for param in &shape.repeat {
                rendered.push(render_param(param, Some(group), include_types));
            }
        }
        rendered.push("...".to_string());
    }
    for param in &shape.tail {
        rendered.push(render_param(param, None, include_types));
    }
    rendered.join(", ")
}

fn render_param(param: &ParamDecl, group: Option<usize>, include_types: bool) -> String {
    let mut out = param.name.to_string();
    if let Some(group) = group {
        out.push_str(&group.to_string());
    }
    if param.optional {
        out.push('?');
    }
    if include_types {
        out.push_str(": ");
        out.push_str(&render_type(&param.ty, 0));
    }
    out
}

fn render_type(ty: &TypeAst, parent_precedence: u8) -> String {
    let precedence = match ty {
        TypeAst::Fn { .. } => 0,
        TypeAst::Union(_) => 1,
        TypeAst::List(_) => 2,
        _ => 3,
    };
    let mut out = match ty {
        TypeAst::Number => "number".to_string(),
        TypeAst::String => "string".to_string(),
        TypeAst::Boolean => "boolean".to_string(),
        TypeAst::Date => "date".to_string(),
        TypeAst::Null => "null".to_string(),
        TypeAst::Any => "any".to_string(),
        TypeAst::Named(name) => name.to_string(),
        TypeAst::List(inner) => format!("{}[]", render_type(inner, precedence)),
        TypeAst::Union(members) => members
            .iter()
            .map(|member| render_type(member, precedence))
            .collect::<Vec<_>>()
            .join(" | "),
        TypeAst::Fn { params, ret } => {
            let params = params
                .iter()
                .map(render_lambda_param)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({params}) -> {}", render_type(ret, 0))
        }
        TypeAst::Ident { inner } => format!("Ident<{}>", render_type(inner, 0)),
    };
    if precedence < parent_precedence {
        out = format!("({out})");
    }
    out
}

fn render_lambda_param(param: &LambdaParamAst) -> String {
    format!("{}: {}", param.name, render_type(&param.ty, 0))
}

fn category_variant(name: &str) -> Option<syn::Ident> {
    matches!(
        name,
        "General" | "Text" | "Number" | "Date" | "People" | "List" | "Special"
    )
    .then(|| format_ident!("{name}"))
}

fn rust_field_name(name: &str) -> String {
    let mut out = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    if is_rust_keyword(&out) {
        out.push('_');
    }
    out
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}

#[derive(Default)]
struct Errors {
    errors: Vec<syn::Error>,
    suppressed: usize,
}

impl Errors {
    fn push(&mut self, error: syn::Error) {
        if self.errors.len() < ERROR_LIMIT {
            self.errors.push(error);
        } else {
            self.suppressed += 1;
        }
    }

    fn finish(mut self) -> syn::Result<()> {
        if self.suppressed > 0 {
            let retained = ERROR_LIMIT.saturating_sub(1);
            if self.errors.len() > retained {
                self.suppressed += self.errors.len() - retained;
                self.errors.truncate(retained);
            }
            self.errors.push(syn::Error::new(
                Span::call_site(),
                format!(
                    "{} additional declaration errors were suppressed",
                    self.suppressed
                ),
            ));
        }
        let mut errors = self.errors.into_iter();
        let Some(mut combined) = errors.next() else {
            return Ok(());
        };
        for error in errors {
            combined.combine(error);
        }
        Err(combined)
    }
}
