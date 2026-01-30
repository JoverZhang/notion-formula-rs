use crate::analyze;
use crate::semantic::{self, Context, Ty};

fn infer_ok(source: &str) -> Ty {
    let output = analyze(source).unwrap();
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );

    let ctx = Context {
        properties: vec![],
        functions: vec![],
    };

    let (ty, diags) = semantic::analyze_expr(&output.expr, &ctx);
    assert!(diags.is_empty(), "unexpected semantic diagnostics: {:?}", diags);
    ty
}

#[test]
fn infer_list_union() {
    let ty = infer_ok("[1, \"x\"]");
    let expected_inner = semantic::normalize_union(vec![Ty::Number, Ty::String]);
    assert_eq!(
        ty,
        Ty::List(Box::new(expected_inner))
    );
}

#[test]
fn infer_list_unknown_propagates() {
    let ty = infer_ok("[1, unknownIdent]");
    assert_eq!(ty, Ty::List(Box::new(Ty::Unknown)));
}
