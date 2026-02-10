use crate::analyze;
use crate::semantic::{
    Context, FunctionCategory, FunctionSig, GenericId, GenericParam, GenericParamKind, ParamShape,
    ParamSig, Ty, TypeMap, infer_expr_with_map,
};

fn p(name: &str, ty: Ty) -> ParamSig {
    ParamSig {
        name: name.into(),
        ty,
        optional: false,
    }
}

fn infer(source: &str, ctx: &Context) -> (Ty, TypeMap, crate::ast::Expr) {
    let output = analyze(source).unwrap();
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    let mut map = TypeMap::default();
    let ty = infer_expr_with_map(&output.expr, ctx, &mut map);
    (ty, map, output.expr)
}

#[test]
fn generic_if_plain_unifies_then_else_into_union() {
    let t = GenericId(0);
    let ctx = Context {
        properties: vec![],
        functions: vec![FunctionSig::new(
            FunctionCategory::General,
            "if(condition, then, else)",
            "if",
            ParamShape::new(
                vec![
                    p("condition", Ty::Boolean),
                    p("then", Ty::Generic(t)),
                    p("else", Ty::Generic(t)),
                ],
                vec![],
                vec![],
            ),
            Ty::Generic(t),
            vec![GenericParam {
                id: t,
                kind: GenericParamKind::Plain,
            }],
        )],
    };

    let (ty, map, root) = infer("if(true, 1, \"x\")", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
    assert_eq!(map.get(root.id), Some(&ty));
}

#[test]
fn generic_list_unifies_inner_type() {
    let t = GenericId(0);
    let ctx = Context {
        properties: vec![],
        functions: vec![
            FunctionSig::new(
                FunctionCategory::List,
                "split(text, separator)",
                "split",
                ParamShape::new(
                    vec![p("text", Ty::String), p("separator", Ty::String)],
                    vec![],
                    vec![],
                ),
                Ty::List(Box::new(Ty::String)),
                vec![],
            ),
            FunctionSig::new(
                FunctionCategory::List,
                "first(list)",
                "first",
                ParamShape::new(
                    vec![p("list", Ty::List(Box::new(Ty::Generic(t))))],
                    vec![],
                    vec![],
                ),
                Ty::Generic(t),
                vec![GenericParam {
                    id: t,
                    kind: GenericParamKind::Plain,
                }],
            ),
        ],
    };

    let (ty, _, _) = infer("first(split(\"a\", \",\"))", &ctx);
    assert_eq!(ty, Ty::String);
}

#[test]
fn variant_generic_accumulates_union_when_no_unknown() {
    let t = GenericId(0);
    let ctx = Context {
        properties: vec![],
        functions: vec![FunctionSig::new(
            FunctionCategory::General,
            "ifs(condition1, value1, ..., else)",
            "ifs",
            ParamShape::new(
                vec![],
                vec![p("condition1", Ty::Unknown), p("value1", Ty::Generic(t))],
                vec![p("else", Ty::Generic(t))],
            ),
            Ty::Generic(t),
            vec![GenericParam {
                id: t,
                kind: GenericParamKind::Variant,
            }],
        )],
    };

    let (ty, _, _) = infer("ifs(true, 1, false, 2, \"a\")", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
}

#[test]
fn variant_generic_propagates_unknown() {
    let t = GenericId(0);
    let ctx = Context {
        properties: vec![],
        functions: vec![FunctionSig::new(
            FunctionCategory::General,
            "ifs(condition1, value1, ..., else)",
            "ifs",
            ParamShape::new(
                vec![],
                vec![p("condition1", Ty::Unknown), p("value1", Ty::Generic(t))],
                vec![p("else", Ty::Generic(t))],
            ),
            Ty::Generic(t),
            vec![GenericParam {
                id: t,
                kind: GenericParamKind::Variant,
            }],
        )],
    };

    let (ty, _, _) = infer("ifs(true, 1, false, x, \"a\")", &ctx);
    assert_eq!(ty, Ty::Unknown);
}
