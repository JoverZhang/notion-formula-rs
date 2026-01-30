use crate::ide::display::{TypeFormatOptions, format_ty};
use crate::semantic::Ty;

#[test]
fn format_ty_list_union_parens() {
    let ty = Ty::List(Box::new(Ty::Union(vec![Ty::Number, Ty::String])));
    let out = format_ty(&ty, &TypeFormatOptions::default());
    assert_eq!(out, "(number | string)[]");
}

