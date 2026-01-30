use crate::semantic::Ty;

#[test]
fn format_ty_list_union_parens() {
    let ty = Ty::List(Box::new(Ty::Union(vec![Ty::Number, Ty::String])));
    assert_eq!(ty.to_string(), "(number | string)[]");
}

#[test]
fn format_ty_union_prec_does_not_parenthesize_list() {
    let ty = Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::String))]);
    assert_eq!(ty.to_string(), "number | string[]");
}
