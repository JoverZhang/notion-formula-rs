use crate::analysis::{
    Context, ShapeValidity, Ty, analyze_expr_with_semantic_map, builtins_functions,
};
use crate::analyze_syntax;

#[test]
fn semantic_map_retains_only_the_final_lambda_resolution() {
    let mut syntax = analyze_syntax("ifs(true, 1, false, \"two\", 0)");
    let call_id = syntax.expr.id;
    let context = Context {
        properties: vec![],
        functions: builtins_functions(),
    };

    let (ty, map, diagnostics) = analyze_expr_with_semantic_map(&mut syntax.expr, &context);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
    let resolved = map.builtin_calls.get(&call_id).expect("resolved ifs call");
    assert_eq!(resolved.validity, ShapeValidity::Valid);
    assert_eq!(resolved.return_ty, ty);
    assert_eq!(resolved.arguments.len(), 5);
}

#[test]
fn semantic_map_keeps_flat_resolver_output_for_planning() {
    let mut syntax = analyze_syntax("flat([[[1]], [2]])");
    let call_id = syntax.expr.id;
    let context = Context {
        properties: vec![],
        functions: builtins_functions(),
    };

    let (ty, map, diagnostics) = analyze_expr_with_semantic_map(&mut syntax.expr, &context);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
    assert_eq!(map.builtin_calls[&call_id].return_ty, ty);
}
