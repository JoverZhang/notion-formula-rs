use builtin_fn::{GenericParamKind, ParamShape, Ty, builtins_functions};

fn shape(name: &str) -> ParamShape {
    builtins_functions()
        .into_iter()
        .find(|signature| signature.name == name)
        .unwrap_or_else(|| panic!("missing builtin `{name}`"))
        .params
}

#[test]
fn bounded_shape_representatives_lower_from_the_catalog() {
    let flat = shape("flat");
    assert_eq!(
        (flat.head.len(), flat.repeat.len(), flat.tail.len()),
        (1, 0, 0)
    );
    assert!(matches!(flat.head[0].ty, Ty::List(_)));

    let concat = shape("concat");
    assert_eq!(
        (concat.head.len(), concat.repeat.len(), concat.tail.len()),
        (0, 1, 0)
    );
    assert_eq!(concat.repeat_min_groups, 2);
    assert_eq!(concat.repeat[0].name, "lists");

    let splice = shape("splice");
    assert_eq!(
        (splice.head.len(), splice.repeat.len(), splice.tail.len()),
        (3, 1, 0)
    );
    assert_eq!(splice.repeat_min_groups, 0);
    assert_eq!(splice.repeat[0].name, "items");

    let ifs = builtins_functions()
        .into_iter()
        .find(|signature| signature.name == "ifs")
        .expect("ifs declaration");
    assert_eq!(
        (
            ifs.params.head.len(),
            ifs.params.repeat.len(),
            ifs.params.tail.len(),
        ),
        (0, 2, 1)
    );
    assert_eq!(ifs.params.repeat_min_groups, 1);
    assert_eq!(ifs.params.repeat[0].name, "condition");
    assert_eq!(ifs.params.repeat[1].name, "value");
    assert_eq!(ifs.generics[0].kind, GenericParamKind::Variant);
}
