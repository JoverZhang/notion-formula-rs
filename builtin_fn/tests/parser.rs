use builtin_fn::{
    BuiltinSigParseErrorKind, BuiltinSigParser, FunctionCategory, GenericId, GenericKindRegistry,
    GenericParam, GenericParamKind, LambdaParam, ParamShape, ParamSig, Ty, default_parser,
};

fn parse(text: &str) -> builtin_fn::FunctionSig {
    default_parser()
        .parse(FunctionCategory::General, text)
        .unwrap()
}

fn p(name: &str, ty: Ty) -> ParamSig {
    ParamSig {
        name: name.to_string(),
        ty,
        optional: false,
    }
}

#[test]
fn parses_basic_signature() {
    let sig = default_parser()
        .parse(FunctionCategory::Number, "pi() -> number")
        .unwrap();

    assert_eq!(sig.name, "pi");
    assert_eq!(sig.detail, "pi()");
    assert_eq!(sig.params, ParamShape::new(vec![], vec![], vec![]));
    assert_eq!(sig.ret, Ty::Number);
}

#[test]
fn parses_optional_param() {
    let sig = default_parser()
        .parse(
            FunctionCategory::Number,
            "round(value: number, places?: number) -> number",
        )
        .unwrap();

    assert_eq!(
        sig.params,
        ParamShape::new(
            vec![
                p("value", Ty::Number),
                ParamSig {
                    name: "places".into(),
                    ty: Ty::Number,
                    optional: true,
                },
            ],
            vec![],
            vec![],
        )
    );
}

#[test]
fn parses_generics_and_lambda_types() {
    let sig = default_parser()
        .parse(
            FunctionCategory::List,
            "map<T: Plain, U: Plain>(list: T[], mapper: (current: T) -> U) -> U[]",
        )
        .unwrap();

    assert_eq!(
        sig.generics,
        vec![
            GenericParam {
                id: GenericId(0),
                kind: GenericParamKind::Plain,
            },
            GenericParam {
                id: GenericId(1),
                kind: GenericParamKind::Plain,
            },
        ]
    );
    assert_eq!(
        sig.params,
        ParamShape::new(
            vec![
                p("list", Ty::List(Box::new(Ty::Generic(GenericId(0))))),
                p(
                    "mapper",
                    Ty::Fn {
                        params: vec![(LambdaParam::Current, Ty::Generic(GenericId(0)))],
                        ret: Box::new(Ty::Generic(GenericId(1))),
                    },
                ),
            ],
            vec![],
            vec![],
        )
    );
    assert_eq!(sig.ret, Ty::List(Box::new(Ty::Generic(GenericId(1)))));
}

#[test]
fn parses_grouped_union_list_type() {
    let sig = parse("demo<T: Plain>(value: (number | string)[]) -> T");
    assert_eq!(
        sig.params.head[0].ty,
        Ty::List(Box::new(Ty::Union(vec![Ty::Number, Ty::String])))
    );
}

#[test]
fn parses_identifier_binder() {
    let sig = default_parser()
        .parse(
            FunctionCategory::General,
            "let<T: Plain, U: Plain>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U",
        )
        .unwrap();

    assert_eq!(
        sig.params.head[0].ty,
        Ty::Ident(Box::new(Ty::Generic(GenericId(0))))
    );
    assert_eq!(
        sig.params.head[2].ty,
        Ty::Fn {
            params: vec![(
                LambdaParam::ParamRef("ident".into()),
                Ty::Generic(GenericId(0))
            )],
            ret: Box::new(Ty::Generic(GenericId(1))),
        }
    );
}

#[test]
fn parses_repeat_group_signature() {
    let sig = default_parser()
        .parse(
            FunctionCategory::General,
            "ifs<T: Variant>(condition1: boolean, value1: () -> T, ..., else: () -> T) -> T",
        )
        .unwrap();

    assert_eq!(sig.params.head, Vec::<ParamSig>::new());
    assert_eq!(sig.params.repeat.len(), 2);
    assert_eq!(sig.params.tail.len(), 1);
    assert_eq!(sig.detail, "ifs(condition1, value1, ..., else)");
}

#[test]
fn parses_rest_param_signature() {
    let sig = default_parser()
        .parse(
            FunctionCategory::List,
            "splice<T: Plain>(list: T[], startIndex: number, deleteCount: number, ...items: T[]) -> T[]",
        )
        .unwrap();

    assert_eq!(sig.params.head.len(), 3);
    assert_eq!(sig.params.repeat_min_groups, 0);
    assert_eq!(
        sig.params.repeat,
        vec![p("items", Ty::Generic(GenericId(0)))]
    );
    assert_eq!(
        sig.detail,
        "splice(list, startIndex, deleteCount, ...items)"
    );
}

#[test]
fn supports_generic_kind_aliases() {
    let mut registry = GenericKindRegistry::with_builtin_kinds();
    registry.register("Flat", GenericParamKind::Plain);
    let parser = BuiltinSigParser::new(registry);

    let sig = parser
        .parse(FunctionCategory::List, "flat<T: Flat>(list: T[]) -> T[]")
        .unwrap();

    assert_eq!(sig.generics[0].kind, GenericParamKind::Plain);
}

#[test]
fn lowers_any_to_hidden_plain_generic() {
    let sig = default_parser()
        .parse(FunctionCategory::General, "format(value: any) -> string")
        .unwrap();

    assert_eq!(
        sig.generics,
        vec![GenericParam {
            id: GenericId(0),
            kind: GenericParamKind::Plain,
        }]
    );
    assert_eq!(sig.params.head[0].ty, Ty::Generic(GenericId(0)));
}

#[test]
fn missing_arrow_is_reported() {
    let err = default_parser()
        .parse(FunctionCategory::Number, "pi() number")
        .unwrap_err();

    assert_eq!(err.kind, BuiltinSigParseErrorKind::MissingArrow);
}

#[test]
fn duplicate_generic_name_is_reported() {
    let err = default_parser()
        .parse(
            FunctionCategory::List,
            "map<T: Plain, T: Plain>(list: T[]) -> T[]",
        )
        .unwrap_err();

    assert_eq!(
        err.kind,
        BuiltinSigParseErrorKind::DuplicateGenericName { name: "T".into() }
    );
}

#[test]
fn unknown_generic_reference_is_reported() {
    let err = default_parser()
        .parse(FunctionCategory::List, "map(list: T[]) -> T[]")
        .unwrap_err();

    assert_eq!(
        err.kind,
        BuiltinSigParseErrorKind::UnknownGenericReference { name: "T".into() }
    );
}

#[test]
fn invalid_repeat_group_placement_is_reported() {
    let err = default_parser()
        .parse(
            FunctionCategory::List,
            "bad(..., value: number, ...items: number[]) -> number",
        )
        .unwrap_err();

    assert_eq!(
        err.kind,
        BuiltinSigParseErrorKind::InvalidRepeatGroupPlacement
    );
}

#[test]
fn rest_param_requires_list_type() {
    let err = default_parser()
        .parse(FunctionCategory::List, "bad(...items: number) -> number")
        .unwrap_err();

    assert_eq!(err.kind, BuiltinSigParseErrorKind::RestParamMustUseListType);
}
