use builtin_fn::{
    FunctionCategory, FunctionSig, GenericId, GenericParam, GenericParamKind, LambdaParam,
    ParamShape, ParamSig, Ty, builtins_functions,
};

fn p(name: &str, ty: Ty) -> ParamSig {
    ParamSig {
        name: name.to_string(),
        ty,
        optional: false,
    }
}

fn opt(name: &str, ty: Ty) -> ParamSig {
    ParamSig {
        name: name.to_string(),
        ty,
        optional: true,
    }
}

fn builtin_by_name(name: &str) -> FunctionSig {
    builtins_functions()
        .into_iter()
        .find(|sig| sig.name == name)
        .unwrap_or_else(|| panic!("missing builtin `{name}`"))
}

#[test]
fn representative_builtins_match_legacy_shapes() {
    let t0 = GenericId(0);
    let t1 = GenericId(1);

    let cases = vec![
        (
            "if",
            FunctionSig::new_builtin(
                FunctionCategory::General,
                "if(condition, then, else)",
                "if",
                ParamShape::new(
                    vec![
                        p("condition", Ty::Boolean),
                        p(
                            "then",
                            Ty::Fn {
                                params: vec![],
                                ret: Box::new(Ty::Generic(t0)),
                            },
                        ),
                        p(
                            "else",
                            Ty::Fn {
                                params: vec![],
                                ret: Box::new(Ty::Generic(t0)),
                            },
                        ),
                    ],
                    vec![],
                    vec![],
                ),
                Ty::Generic(t0),
                vec![GenericParam {
                    id: t0,
                    kind: GenericParamKind::Variant,
                }],
            ),
        ),
        (
            "ifs",
            FunctionSig::new_builtin(
                FunctionCategory::General,
                "ifs(condition1, value1, ..., else)",
                "ifs",
                ParamShape::new(
                    vec![],
                    vec![
                        p("condition1", Ty::Boolean),
                        p(
                            "value1",
                            Ty::Fn {
                                params: vec![],
                                ret: Box::new(Ty::Generic(t0)),
                            },
                        ),
                    ],
                    vec![p(
                        "else",
                        Ty::Fn {
                            params: vec![],
                            ret: Box::new(Ty::Generic(t0)),
                        },
                    )],
                ),
                Ty::Generic(t0),
                vec![GenericParam {
                    id: t0,
                    kind: GenericParamKind::Variant,
                }],
            ),
        ),
        (
            "empty",
            FunctionSig::new_builtin(
                FunctionCategory::General,
                "empty(value?)",
                "empty",
                ParamShape::new(vec![opt("value", Ty::Generic(t0))], vec![], vec![]),
                Ty::Boolean,
                vec![GenericParam {
                    id: t0,
                    kind: GenericParamKind::Plain,
                }],
            ),
        ),
        (
            "length",
            FunctionSig::new_builtin(
                FunctionCategory::General,
                "length(value)",
                "length",
                ParamShape::new(
                    vec![p(
                        "value",
                        Ty::Union(vec![Ty::String, Ty::List(Box::new(Ty::Generic(t0)))]),
                    )],
                    vec![],
                    vec![],
                ),
                Ty::Number,
                vec![GenericParam {
                    id: t0,
                    kind: GenericParamKind::Plain,
                }],
            ),
        ),
        (
            "let",
            FunctionSig::new_builtin(
                FunctionCategory::General,
                "let(ident, value, body)",
                "let",
                ParamShape::new(
                    vec![
                        p("ident", Ty::Ident(Box::new(Ty::Generic(t0)))),
                        p("value", Ty::Generic(t0)),
                        p(
                            "body",
                            Ty::Fn {
                                params: vec![(
                                    LambdaParam::ParamRef("ident".into()),
                                    Ty::Generic(t0),
                                )],
                                ret: Box::new(Ty::Generic(t1)),
                            },
                        ),
                    ],
                    vec![],
                    vec![],
                ),
                Ty::Generic(t1),
                vec![
                    GenericParam {
                        id: t0,
                        kind: GenericParamKind::Plain,
                    },
                    GenericParam {
                        id: t1,
                        kind: GenericParamKind::Plain,
                    },
                ],
            ),
        ),
        (
            "substring",
            FunctionSig::new_builtin(
                FunctionCategory::Text,
                "substring(text, start, end?)",
                "substring",
                ParamShape::new(
                    vec![
                        p("text", Ty::String),
                        p("start", Ty::Number),
                        opt("end", Ty::Number),
                    ],
                    vec![],
                    vec![],
                ),
                Ty::String,
                vec![],
            ),
        ),
        (
            "concat",
            FunctionSig::new_builtin(
                FunctionCategory::Text,
                "concat(lists1, lists2, ...)",
                "concat",
                ParamShape::new(
                    vec![p("lists1", Ty::List(Box::new(Ty::Generic(t0))))],
                    vec![p("listsN", Ty::List(Box::new(Ty::Generic(t0))))],
                    vec![],
                ),
                Ty::List(Box::new(Ty::Generic(t0))),
                vec![GenericParam {
                    id: t0,
                    kind: GenericParamKind::Plain,
                }],
            ),
        ),
        (
            "round",
            FunctionSig::new_builtin(
                FunctionCategory::Number,
                "round(value, places?)",
                "round",
                ParamShape::new(
                    vec![p("value", Ty::Number), opt("places", Ty::Number)],
                    vec![],
                    vec![],
                ),
                Ty::Number,
                vec![],
            ),
        ),
        (
            "min",
            FunctionSig::new_builtin(
                FunctionCategory::Number,
                "min(values1, values2, ...)",
                "min",
                ParamShape::new(
                    vec![],
                    vec![p(
                        "values",
                        Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))]),
                    )],
                    vec![],
                ),
                Ty::Number,
                vec![],
            ),
        ),
        (
            "at",
            FunctionSig::new_builtin(
                FunctionCategory::List,
                "at(list, index)",
                "at",
                ParamShape::new(
                    vec![
                        p("list", Ty::List(Box::new(Ty::Generic(t0)))),
                        p("index", Ty::Number),
                    ],
                    vec![],
                    vec![],
                ),
                Ty::Generic(t0),
                vec![GenericParam {
                    id: t0,
                    kind: GenericParamKind::Plain,
                }],
            ),
        ),
        (
            "splice",
            FunctionSig::new_builtin(
                FunctionCategory::List,
                "splice(list, startIndex, deleteCount, ...items)",
                "splice",
                ParamShape::new(
                    vec![
                        p("list", Ty::List(Box::new(Ty::Generic(t0)))),
                        p("startIndex", Ty::Number),
                        p("deleteCount", Ty::Number),
                    ],
                    vec![p("items", Ty::Generic(t0))],
                    vec![],
                )
                .with_repeat_min_groups(0),
                Ty::List(Box::new(Ty::Generic(t0))),
                vec![GenericParam {
                    id: t0,
                    kind: GenericParamKind::Plain,
                }],
            ),
        ),
        (
            "map",
            FunctionSig::new_builtin(
                FunctionCategory::List,
                "map(list, mapper)",
                "map",
                ParamShape::new(
                    vec![
                        p("list", Ty::List(Box::new(Ty::Generic(t0)))),
                        p(
                            "mapper",
                            Ty::Fn {
                                params: vec![(LambdaParam::Current, Ty::Generic(t0))],
                                ret: Box::new(Ty::Generic(t1)),
                            },
                        ),
                    ],
                    vec![],
                    vec![],
                ),
                Ty::List(Box::new(Ty::Generic(t1))),
                vec![
                    GenericParam {
                        id: t0,
                        kind: GenericParamKind::Plain,
                    },
                    GenericParam {
                        id: t1,
                        kind: GenericParamKind::Plain,
                    },
                ],
            ),
        ),
        (
            "filter",
            FunctionSig::new_builtin(
                FunctionCategory::List,
                "filter(list, predicate)",
                "filter",
                ParamShape::new(
                    vec![
                        p("list", Ty::List(Box::new(Ty::Generic(t0)))),
                        p(
                            "predicate",
                            Ty::Fn {
                                params: vec![(LambdaParam::Current, Ty::Generic(t0))],
                                ret: Box::new(Ty::Boolean),
                            },
                        ),
                    ],
                    vec![],
                    vec![],
                ),
                Ty::List(Box::new(Ty::Generic(t0))),
                vec![GenericParam {
                    id: t0,
                    kind: GenericParamKind::Plain,
                }],
            ),
        ),
        (
            "flat",
            FunctionSig::new_builtin(
                FunctionCategory::List,
                "flat(list)",
                "flat",
                ParamShape::new(
                    vec![p("list", Ty::List(Box::new(Ty::Generic(t0))))],
                    vec![],
                    vec![],
                ),
                Ty::List(Box::new(Ty::Generic(t0))),
                vec![GenericParam {
                    id: t0,
                    kind: GenericParamKind::Plain,
                }],
            ),
        ),
    ];

    for (name, expected) in cases {
        assert_eq!(builtin_by_name(name), expected, "mismatch for `{name}`");
    }
}
