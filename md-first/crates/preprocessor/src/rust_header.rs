//! Rust declarations with explicit private storage and omitted method bodies.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
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
        if let Err(error) = transform(&mut file.items) {
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

fn transform(items: &mut [Item]) -> syn::Result<()> {
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
                    let (_, arguments, _) = item.generics.split_for_impl();
                    fields
                        .named
                        .push(syn::parse_quote!(inner: #inner #arguments));
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
                    *member = forward(syn::parse2(tokens.clone())?)?;
                }
            }
            Item::Mod(item) => {
                if let Some((_, items)) = &mut item.content {
                    transform(items)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
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

fn forward(mut method: Method) -> syn::Result<ImplItem> {
    require_public(&method.vis, method.sig.span(), "header methods")?;
    if let Some(variadic) = &method.sig.variadic {
        return Err(syn::Error::new_spanned(
            variadic,
            "C variadic arguments cannot be forwarded by spec::header",
        ));
    }
    let implementation = format_ident!("{}_impl", method.sig.ident.unraw());
    let mut arguments = Vec::new();
    for (index, input) in method.sig.inputs.iter_mut().enumerate() {
        match input {
            FnArg::Receiver(_) => arguments.push(quote!(self)),
            FnArg::Typed(argument) => {
                let name = match &*argument.pat {
                    Pat::Ident(pattern) if pattern.by_ref.is_none() && pattern.subpat.is_none() => {
                        pattern.ident.clone()
                    }
                    _ => format_ident!("__md_first_argument_{index}"),
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
    let generics: Vec<_> = method
        .sig
        .generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Lifetime(_) => None,
            GenericParam::Type(param) => Some(param.ident.clone()),
            GenericParam::Const(param) => Some(param.ident.clone()),
        })
        .collect();
    let parameters = if generics.is_empty() {
        TokenStream::new()
    } else {
        quote!(::<#(#generics),*>)
    };
    let mut call = quote!(Self::#implementation #parameters (#(#arguments),*));
    if method.sig.unsafety.is_some() {
        call = quote!(unsafe { #call });
    }
    if method.sig.asyncness.is_some() {
        call = quote!((#call).await);
    }
    let Method {
        attrs,
        vis,
        defaultness,
        sig,
    } = method;
    syn::parse2(quote!(#(#attrs)* #vis #defaultness #sig { #call }))
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
