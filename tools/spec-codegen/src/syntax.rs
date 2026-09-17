use std::collections::BTreeSet;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{
    Attribute, Fields, FnArg, Ident, Item, Pat, Signature, Token, UseTree, Visibility, braced,
};

use crate::{Error, markdown::Block};

struct Method {
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
}

struct Methods {
    attrs: Vec<Attribute>,
    ty: Ident,
    methods: Vec<Method>,
}

impl Parse for Methods {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        input.parse::<Token![impl]>()?;
        let ty = input.parse()?;
        let content;
        braced!(content in input);
        let mut methods = Vec::new();
        while !content.is_empty() {
            let attrs = content.call(Attribute::parse_outer)?;
            let vis = content.parse()?;
            let sig = content.parse()?;
            content.parse::<Token![;]>()?;
            methods.push(Method { attrs, vis, sig });
        }
        Ok(Self { attrs, ty, methods })
    }
}

enum Declaration {
    Item(Box<Item>),
    Methods(Methods),
}

impl Declaration {
    fn span(&self) -> Span {
        match self {
            Self::Item(item) => item.span(),
            Self::Methods(methods) => methods.ty.span(),
        }
    }
}

struct Declarations(Vec<Declaration>);

impl Parse for Declarations {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut declarations = Vec::new();
        while !input.is_empty() {
            let ahead = input.fork();
            ahead.call(Attribute::parse_outer)?;
            if ahead.peek(Token![impl]) {
                declarations.push(Declaration::Methods(input.parse()?));
            } else {
                declarations.push(Declaration::Item(input.parse()?));
            }
        }
        Ok(Self(declarations))
    }
}

#[derive(Default)]
struct AttributeCheck(Option<syn::Error>);

impl<'ast> Visit<'ast> for AttributeCheck {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if !attr.path().is_ident("doc") && !attr.path().is_ident("derive") && self.0.is_none() {
            self.0 = Some(syn::Error::new_spanned(
                attr,
                "unsupported attribute; only doc and derive are accepted",
            ));
        }
    }
}

fn validate_item(item: &Item) -> syn::Result<Option<&Ident>> {
    let mut attrs = AttributeCheck::default();
    attrs.visit_item(item);
    if let Some(error) = attrs.0 {
        return Err(error);
    }
    match item {
        Item::Struct(item) => Ok(Some(&item.ident)),
        Item::Enum(item) => Ok(Some(&item.ident)),
        Item::Type(item) => Ok(Some(&item.ident)),
        Item::Use(_) => Ok(None),
        _ => Err(syn::Error::new_spanned(
            item,
            "expected use, struct, enum, type, or a bodyless inherent impl",
        )),
    }
}

fn import_names(tree: &UseTree, parent: Option<&Ident>, names: &mut BTreeSet<String>) {
    match tree {
        UseTree::Path(path) => import_names(&path.tree, Some(&path.ident), names),
        UseTree::Group(group) => {
            for item in &group.items {
                import_names(item, parent, names);
            }
        }
        UseTree::Name(name) => {
            let ident = if name.ident == "self" {
                parent.unwrap_or(&name.ident)
            } else {
                &name.ident
            };
            names.insert(ident.unraw().to_string());
        }
        UseTree::Rename(rename) => {
            names.insert(rename.rename.unraw().to_string());
        }
        UseTree::Glob(_) => {} // Wildcard imports are resolved by the Rust compiler.
    }
}

pub(crate) fn render(blocks: &[Block]) -> Result<String, Error> {
    let mut declarations = Vec::new();
    for block in blocks {
        let parsed: Declarations =
            syn::parse_str(&block.code).map_err(|error| block.syntax_error(error))?;
        declarations.extend(parsed.0.into_iter().map(|declaration| (block, declaration)));
    }
    let mut names = BTreeSet::new();
    let mut structs = BTreeSet::new();
    let mut imports = BTreeSet::new();
    let mut facades = BTreeSet::new();
    for (block, declaration) in &declarations {
        match declaration {
            Declaration::Item(item) => {
                if let Some(ident) =
                    validate_item(item).map_err(|error| block.syntax_error(error))?
                {
                    let name = ident.unraw().to_string();
                    if !names.insert(name.clone()) {
                        return Err(block.error(
                            block.source_line(ident.span()),
                            format!("duplicate declaration: {name}"),
                        ));
                    }
                    if matches!(item.as_ref(), Item::Struct(_)) {
                        structs.insert(name);
                    }
                }
                if let Item::Use(item) = item.as_ref() {
                    import_names(&item.tree, None, &mut imports);
                }
            }
            Declaration::Methods(methods) => {
                facades.insert(methods.ty.unraw().to_string());
            }
        }
    }
    for (block, declaration) in &declarations {
        if let Declaration::Methods(methods) = declaration {
            let name = methods.ty.unraw().to_string();
            if !structs.contains(&name) {
                return Err(block.error(
                    block.source_line(methods.ty.span()),
                    format!("impl {name} has no matching struct declaration in this header"),
                ));
            }
            let inner = format!("{name}Inner");
            if names.contains(&inner) || imports.contains(&inner) {
                return Err(block.error(
                    block.source_line(methods.ty.span()),
                    format!("{inner} is reserved for handwritten private storage"),
                ));
            }
        }
    }

    let mut output = String::from(crate::output::GENERATED_MARKER);
    let mut seen_methods = BTreeSet::new();
    for (block, declaration) in declarations {
        output.push_str(&format!(
            "\n// Source: {}\n",
            block.provenance(declaration.span())
        ));
        let tokens = match declaration {
            Declaration::Item(mut item) => {
                if let Item::Struct(item) = item.as_mut()
                    && facades.contains(&item.ident.unraw().to_string())
                {
                    if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
                        return Err(block.error(
                            block.source_line(item.ident.span()),
                            "facade structs must be non-generic",
                        ));
                    }
                    let Fields::Named(fields) = &mut item.fields else {
                        return Err(block.error(
                            block.source_line(item.ident.span()),
                            "facades require a named-field struct, including empty braces",
                        ));
                    };
                    if let Some(field) = fields.named.iter().find(|field| {
                        field
                            .ident
                            .as_ref()
                            .is_some_and(|ident| ident.unraw() == "inner")
                    }) {
                        return Err(block.error(
                            block.source_line(field.span()),
                            "inner is reserved for injected private storage",
                        ));
                    }
                    let inner = format_ident!("{}Inner", item.ident);
                    fields.named.push(syn::parse_quote!(inner: #inner));
                }
                quote!(#item)
            }
            Declaration::Methods(methods) => {
                let mut attrs = AttributeCheck::default();
                for attr in &methods.attrs {
                    attrs.visit_attribute(attr);
                }
                if let Some(error) = attrs.0 {
                    return Err(block.syntax_error(error));
                }
                let mut body = TokenStream::new();
                for method in methods.methods {
                    let name = method.sig.ident.unraw().to_string();
                    if !seen_methods.insert((methods.ty.unraw().to_string(), name.clone())) {
                        return Err(block.error(
                            block.source_line(method.sig.ident.span()),
                            format!("duplicate method: {name}"),
                        ));
                    }
                    output.push_str(&format!(
                        "// Method: {} ({name})\n",
                        block.provenance(method.sig.ident.span())
                    ));
                    body.extend(emit_method(method).map_err(|error| block.syntax_error(error))?);
                }
                let ty = methods.ty;
                let attrs = methods.attrs;
                quote!(#(#attrs)* impl #ty { #body })
            }
        };
        let file = syn::parse2(tokens).map_err(|error| block.syntax_error(error))?;
        output.push_str(&prettyplease::unparse(&file));
    }
    Ok(output)
}

fn emit_method(method: Method) -> syn::Result<TokenStream> {
    let Method { attrs, vis, sig } = method;
    let mut check = AttributeCheck::default();
    for attr in &attrs {
        check.visit_attribute(attr);
    }
    check.visit_signature(&sig);
    if let Some(error) = check.0 {
        return Err(error);
    }
    if sig.ident.unraw().to_string().ends_with("_impl") {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "*_impl names are reserved for handwritten hooks",
        ));
    }
    if !sig.generics.params.is_empty()
        || sig.generics.where_clause.is_some()
        || sig.asyncness.is_some()
        || sig.constness.is_some()
        || sig.unsafety.is_some()
        || sig.abi.is_some()
        || sig.variadic.is_some()
    {
        return Err(syn::Error::new_spanned(
            &sig,
            "methods must be synchronous and non-generic, without const, unsafe, extern, or variadic qualifiers",
        ));
    }
    let mut args = Vec::new();
    for input in &sig.inputs {
        match input {
            FnArg::Receiver(receiver) => {
                if receiver.colon_token.is_some() {
                    return Err(syn::Error::new_spanned(
                        receiver,
                        "typed receivers are unsupported; use self, &self, or &mut self",
                    ));
                }
            }
            FnArg::Typed(arg) => match arg.pat.as_ref() {
                Pat::Ident(ident) if ident.by_ref.is_none() && ident.subpat.is_none() => {
                    args.push(&ident.ident);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &arg.pat,
                        "parameters must be named identifiers",
                    ));
                }
            },
        }
    }
    let hook = format_ident!("{}_impl", sig.ident);
    let call = if sig.receiver().is_some() {
        quote!(self.#hook(#(#args),*))
    } else {
        quote!(Self::#hook(#(#args),*))
    };
    Ok(quote!(#(#attrs)* #vis #sig { #call }))
}
