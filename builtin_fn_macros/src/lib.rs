mod ast;
mod expand;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

/// Declare all builtin functions belonging to one category.
#[proc_macro]
pub fn builtin_functions(input: TokenStream) -> TokenStream {
    let declaration = parse_macro_input!(input as ast::CategoryDecl);
    match expand::expand(declaration) {
        Ok(expansion) => expansion.into(),
        Err(error) => {
            // The macro is an expression. A block keeps multiple `compile_error!`
            // expansions syntactically independent instead of letting adjacent absolute
            // paths be parsed as one malformed expression.
            let diagnostics = error.into_compile_error();
            quote!({ #diagnostics }).into()
        }
    }
}
