//! Rust declarations with explicit private storage and omitted method bodies.

use std::collections::BTreeSet;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, Fields, FnArg, GenericParam, ImplItem, Item, Pat, Signature, Token, Visibility,
};

use md_first_core::{Context, Preprocessor};

/// Preprocess `.h.rs` declarations. Handwritten methods use the `_impl` suffix.
pub struct RustHeader;

impl Preprocessor for RustHeader {
    fn preprocess(&self, blocks: Vec<TokenStream>, context: &Context) -> TokenStream {
        let origins: Vec<_> = blocks
            .iter()
            .map(|block| {
                block
                    .clone()
                    .into_iter()
                    .next()
                    .map(|token| token.span().file())
            })
            .collect();
        let report = |error: syn::Error| {
            for error in error {
                let file = error.span().file();
                let index = origins
                    .iter()
                    .position(|origin| origin.as_ref() == Some(&file))
                    .unwrap_or_else(|| blocks.len().saturating_sub(1));
                context.error(index, error.span(), error.to_string());
            }
        };
        let tokens = blocks.iter().cloned().collect();
        let mut file: syn::File = match syn::parse2(tokens) {
            Ok(file) => file,
            Err(error) => {
                report(error);
                return TokenStream::new();
            }
        };
        let mut reserved = ReservedNames::default();
        reserved.visit_file(&file);
        if let Err(error) = transform(&mut file.items, &reserved) {
            report(error);
            return TokenStream::new();
        }
        let mut markers = Markers::default();
        markers.visit_file(&file);
        for error in markers.errors {
            report(error);
        }
        file.into_token_stream()
    }
}

fn transform(items: &mut [Item], reserved: &ReservedNames) -> syn::Result<()> {
    for item in items {
        match item {
            Item::Struct(item) => {
                let private = take_marker(&mut item.attrs, "private_fields")?;
                for field in &item.fields {
                    require_public(&field.vis, field.span(), "struct fields")?;
                }
                if private {
                    let Fields::Named(fields) = &mut item.fields else {
                        return Err(syn::Error::new_spanned(
                            item,
                            "spec::private_fields requires named fields (empty braces are allowed)",
                        ));
                    };
                    if let Some(field) = fields.named.iter().find(|field| {
                        field
                            .ident
                            .as_ref()
                            .is_some_and(|name| name.unraw() == "inner")
                    }) {
                        return Err(syn::Error::new_spanned(
                            field,
                            "spec::private_fields conflicts with the declared inner field",
                        ));
                    }
                    let inner = format_ident!("{}Inner", item.ident.unraw());
                    for arguments in generic_arguments(&item.generics.params)? {
                        let cfg = arguments.configuration();
                        let parameters = arguments.parameters();
                        fields
                            .named
                            .push(syn::parse_quote!(#cfg inner: #inner #parameters));
                    }
                }
            }
            Item::Impl(item) => {
                if !take_marker(&mut item.attrs, "header")? {
                    continue;
                }
                if item.trait_.is_some() {
                    return Err(syn::Error::new_spanned(
                        item,
                        "spec::header requires an inherent impl",
                    ));
                }
                for member in &mut item.items {
                    let ImplItem::Verbatim(tokens) = member else {
                        return Err(syn::Error::new_spanned(
                            member,
                            "spec::header expects method declarations ending with ;",
                        ));
                    };
                    *member = forward(syn::parse2(tokens.clone())?, reserved)?;
                }
            }
            Item::Mod(item) => {
                if let Some((_, items)) = &mut item.content {
                    transform(items, reserved)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[derive(Clone, Default)]
struct ReservedNames(BTreeSet<String>);

impl<'ast> Visit<'ast> for ReservedNames {
    fn visit_ident(&mut self, ident: &'ast syn::Ident) {
        self.0.insert(ident.unraw().to_string());
    }
}

struct Method {
    attrs: Vec<Attribute>,
    vis: Visibility,
    defaultness: Option<Token![default]>,
    sig: Signature,
}

impl Parse for Method {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            defaultness: input.parse()?,
            sig: input.parse()?,
        };
        input.parse::<Token![;]>()?;
        Ok(method)
    }
}

fn forward(mut method: Method, reserved: &ReservedNames) -> syn::Result<ImplItem> {
    require_public(&method.vis, method.sig.span(), "header methods")?;
    if let Some(variadic) = &method.sig.variadic {
        return Err(syn::Error::new_spanned(
            variadic,
            "C variadic arguments cannot be forwarded by spec::header",
        ));
    }
    let implementation = format_ident!("{}_impl", method.sig.ident.unraw());
    let mut used = reserved.clone();
    used.visit_signature(&method.sig);
    let mut arguments = Vec::new();
    for (index, input) in method.sig.inputs.iter_mut().enumerate() {
        match input {
            FnArg::Receiver(_) => arguments.push(quote!(self)),
            FnArg::Typed(argument) => {
                let name = match &*argument.pat {
                    Pat::Ident(pattern) if pattern.by_ref.is_none() && pattern.subpat.is_none() => {
                        pattern.ident.clone()
                    }
                    _ => {
                        let mut suffix = index;
                        loop {
                            let name = format_ident!("__md_first_argument_{suffix}");
                            if used.0.insert(name.to_string()) {
                                break name;
                            }
                            suffix += 1;
                        }
                    }
                };
                // Destructuring belongs in the handwritten method. Forward the
                // whole value rather than trying to reconstruct a Rust pattern.
                *argument.pat = syn::parse_quote!(#name);
                let cfg = argument
                    .attrs
                    .iter()
                    .filter(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"));
                arguments.push(quote!(#(#cfg)* #name));
            }
        }
    }
    let generics = method
        .sig
        .generics
        .params
        .iter()
        .filter(|param| !matches!(param, GenericParam::Lifetime(_)));
    let mut calls = Vec::new();
    for generics in generic_arguments(generics)? {
        let parameters = generics.parameters();
        let parameters = if parameters.is_empty() {
            parameters
        } else {
            quote!(::#parameters)
        };
        let mut call = quote!(Self::#implementation #parameters (#(#arguments),*));
        if method.sig.unsafety.is_some() {
            call = quote!(unsafe { #call });
        }
        if method.sig.asyncness.is_some() {
            call = quote!((#call).await);
        }
        let cfg = generics.configuration();
        calls.push(if cfg.is_empty() {
            call
        } else {
            quote!(#cfg { #call })
        });
    }
    let Method {
        attrs,
        vis,
        defaultness,
        sig,
    } = method;
    syn::parse2(quote!(#(#attrs)* #vis #defaultness #sig { #(#calls)* }))
}

#[derive(Clone, Default)]
struct GenericArguments {
    conditions: Vec<TokenStream>,
    values: Vec<TokenStream>,
}

impl GenericArguments {
    fn configuration(&self) -> TokenStream {
        let conditions = &self.conditions;
        if conditions.is_empty() {
            TokenStream::new()
        } else {
            quote!(#[cfg(all(#(#conditions),*))])
        }
    }

    fn parameters(&self) -> TokenStream {
        let values = &self.values;
        if values.is_empty() {
            TokenStream::new()
        } else {
            quote!(<#(#values),*>)
        }
    }
}

fn generic_arguments<'a>(
    params: impl IntoIterator<Item = &'a GenericParam>,
) -> syn::Result<Vec<GenericArguments>> {
    let mut variants = vec![GenericArguments::default()];
    for param in params {
        let (attrs, value) = match param {
            GenericParam::Lifetime(param) => (&param.attrs, param.lifetime.to_token_stream()),
            GenericParam::Type(param) => (&param.attrs, param.ident.to_token_stream()),
            GenericParam::Const(param) => (&param.attrs, param.ident.to_token_stream()),
        };
        let conditions = attrs
            .iter()
            .map(|attr| cfg_predicate(&attr.meta))
            .collect::<syn::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if !conditions.is_empty() {
            // Generic arguments cannot carry cfg attributes. Emit mutually
            // exclusive fields/calls and let the consumer's compiler select one.
            let condition = quote!(all(#(#conditions),*));
            let mut absent = variants.clone();
            for variant in &mut absent {
                variant.conditions.push(quote!(not(#condition)));
            }
            for variant in &mut variants {
                variant.conditions.push(condition.clone());
                variant.values.push(value.clone());
            }
            variants.extend(absent);
        } else {
            for variant in &mut variants {
                variant.values.push(value.clone());
            }
        }
    }
    Ok(variants)
}

fn cfg_predicate(meta: &syn::Meta) -> syn::Result<Option<TokenStream>> {
    if meta.path().is_ident("cfg") {
        return Ok(Some(meta.require_list()?.tokens.clone()));
    }
    if !meta.path().is_ident("cfg_attr") {
        return Ok(None);
    }
    let mut arguments = meta
        .require_list()?
        .parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated)?
        .into_iter();
    let condition = arguments
        .next()
        .ok_or_else(|| syn::Error::new_spanned(meta, "cfg_attr requires a predicate"))?;
    let predicates = arguments
        .map(|meta| cfg_predicate(&meta))
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    Ok((!predicates.is_empty()).then(|| quote!(any(not(#condition), all(#(#predicates),*)))))
}

fn require_public(vis: &Visibility, span: Span, description: &str) -> syn::Result<()> {
    if matches!(vis, Visibility::Public(_)) {
        Ok(())
    } else {
        Err(syn::Error::new(
            span,
            format!("{description} must explicitly use pub"),
        ))
    }
}

fn take_marker(attrs: &mut Vec<Attribute>, name: &str) -> syn::Result<bool> {
    let mut found = false;
    let mut error = None;
    attrs.retain(|attr| {
        let path = attr.path();
        if path.leading_colon.is_none()
            && path.segments.len() == 2
            && path.segments[0].ident == "spec"
            && path.segments[1].ident == name
        {
            if found || !matches!(attr.meta, syn::Meta::Path(_)) {
                error = Some(syn::Error::new_spanned(
                    attr,
                    format!("use #[spec::{name}] once, without arguments"),
                ));
            }
            found = true;
            false
        } else {
            true
        }
    });
    error.map_or(Ok(found), Err)
}

#[derive(Default)]
struct Markers {
    errors: Vec<syn::Error>,
}

impl<'ast> Visit<'ast> for Markers {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if attr
            .path()
            .segments
            .first()
            .is_some_and(|segment| segment.ident == "spec")
        {
            self.errors.push(syn::Error::new_spanned(
                attr,
                "unknown or misplaced spec attribute",
            ));
        }
        visit::visit_attribute(self, attr);
    }
}
