use crate::semantic::{
    Context, FunctionCategory, FunctionSig, GenericId, GenericParam, GenericParamKind, ParamLayout,
    ParamSig, Ty, TypeMap, infer_expr_with_map,
};
use crate::analyze;

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
        functions: vec![FunctionSig {
            name: "if".into(),
            layout: ParamLayout::Flat(vec![
                ParamSig {
                    name: "condition".into(),
                    ty: Ty::Boolean,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: "then".into(),
                    ty: Ty::Generic(t),
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: "else".into(),
                    ty: Ty::Generic(t),
                    optional: false,
                    variadic: false,
                },
            ]),
            ret: Ty::Generic(t),
            detail: None,
            category: FunctionCategory::General,
            generics: vec![GenericParam {
                id: t,
                name: "T".into(),
                kind: GenericParamKind::Plain,
            }],
        }],
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
            FunctionSig {
                name: "split".into(),
                layout: ParamLayout::Flat(vec![
                    ParamSig {
                        name: "text".into(),
                        ty: Ty::String,
                        optional: false,
                        variadic: false,
                    },
                    ParamSig {
                        name: "separator".into(),
                        ty: Ty::String,
                        optional: false,
                        variadic: false,
                    },
                ]),
                ret: Ty::List(Box::new(Ty::String)),
                detail: None,
                category: FunctionCategory::List,
                generics: vec![],
            },
            FunctionSig {
                name: "first".into(),
                layout: ParamLayout::Flat(vec![ParamSig {
                    name: "list".into(),
                    ty: Ty::List(Box::new(Ty::Generic(t))),
                    optional: false,
                    variadic: false,
                }]),
                ret: Ty::Generic(t),
                detail: None,
                category: FunctionCategory::List,
                generics: vec![GenericParam {
                    id: t,
                    name: "T".into(),
                    kind: GenericParamKind::Plain,
                }],
            },
        ],
    };

    let (ty, _, _) = infer("first(split(\"a\", \",\"))", &ctx);
    assert_eq!(ty, Ty::String);
}

#[test]
fn variant_generic_skips_unknown_when_accumulating_union() {
    let t = GenericId(0);
    let ctx = Context {
        properties: vec![],
        functions: vec![FunctionSig {
            name: "ifs".into(),
            layout: ParamLayout::RepeatGroup {
                head: vec![],
                repeat: vec![
                    ParamSig {
                        name: "condition".into(),
                        ty: Ty::Unknown,
                        optional: false,
                        variadic: false,
                    },
                    ParamSig {
                        name: "value".into(),
                        ty: Ty::Generic(t),
                        optional: false,
                        variadic: false,
                    },
                ],
                tail: vec![ParamSig {
                    name: "default".into(),
                    ty: Ty::Generic(t),
                    optional: false,
                    variadic: false,
                }],
            },
            ret: Ty::Generic(t),
            detail: None,
            category: FunctionCategory::General,
            generics: vec![GenericParam {
                id: t,
                name: "T".into(),
                kind: GenericParamKind::Variant,
            }],
        }],
    };

    let (ty, _, _) = infer("ifs(true, 1, false, x, \"a\")", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
}

