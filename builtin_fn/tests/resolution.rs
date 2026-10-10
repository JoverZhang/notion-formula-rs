use builtin_fn::{
    ArgumentObservation, ArgumentTypeStatus, CallShapeError, CallSignatureInput, ParamRef,
    ShapeValidity, Ty, builtin_functions, resolve_call_signature,
};

fn function(name: &str) -> builtin_fn::FunctionSig {
    builtin_fn::builtins_functions()
        .into_iter()
        .find(|signature| signature.name == name)
        .unwrap_or_else(|| panic!("missing builtin `{name}`"))
}

#[test]
fn flat_resolver_refines_only_the_dynamic_return_type() {
    let signature = function("flat");
    let nested = Ty::List(Box::new(Ty::List(Box::new(Ty::Union(vec![
        Ty::Number,
        Ty::List(Box::new(Ty::String)),
    ])))));
    let resolved = resolve_call_signature(
        &signature,
        CallSignatureInput {
            arguments: &[ArgumentObservation::Typed(nested)],
        },
    );

    assert_eq!(resolved.validity, ShapeValidity::Valid);
    assert_eq!(
        resolved.return_ty,
        Ty::List(Box::new(Ty::Union(vec![
            Ty::Number,
            Ty::List(Box::new(Ty::String)),
        ])))
    );
    assert_eq!(resolved.projection[0].logical_param, ParamRef::Head(0));
}

#[test]
fn empty_return_type_depends_on_arity() {
    let signature = function("empty");
    assert!(matches!(signature.ret, Ty::Generic(_)));

    for (arguments, expected) in [
        (vec![], Ty::Unknown),
        (vec![ArgumentObservation::Typed(Ty::Number)], Ty::Boolean),
        (vec![ArgumentObservation::Typed(Ty::Unknown)], Ty::Boolean),
        (vec![ArgumentObservation::Empty], Ty::Boolean),
    ] {
        let resolved = resolve_call_signature(
            &signature,
            CallSignatureInput {
                arguments: &arguments,
            },
        );
        assert_eq!(resolved.validity, ShapeValidity::Valid);
        assert_eq!(resolved.return_ty, expected);
    }
}

#[test]
fn empty_extra_argument_uses_recovery_without_accepting_the_shape() {
    let resolved = resolve_call_signature(
        &function("empty"),
        CallSignatureInput {
            arguments: &[
                ArgumentObservation::Typed(Ty::Number),
                ArgumentObservation::Typed(Ty::String),
            ],
        },
    );

    assert_eq!(
        resolved.validity,
        ShapeValidity::Invalid(CallShapeError::TooMany {
            maximum: 1,
            actual: 2,
        })
    );
    assert_eq!(resolved.return_ty, Ty::Number);
    assert_eq!(
        resolved.arguments[1].type_status,
        ArgumentTypeStatus::Unmapped
    );
}

#[test]
fn flat_resolves_union_list_arguments_and_preserves_unknown() {
    fn list(ty: Ty) -> Ty {
        Ty::List(Box::new(ty))
    }

    let signature = function("flat");
    for (argument, expected) in [
        (
            Ty::Union(vec![list(list(Ty::Number)), list(list(list(Ty::String)))]),
            list(Ty::Union(vec![Ty::Number, list(Ty::String)])),
        ),
        (Ty::Unknown, list(Ty::Unknown)),
        (list(Ty::Unknown), list(Ty::Unknown)),
        (
            Ty::Union(vec![list(list(Ty::Number)), Ty::Unknown]),
            list(Ty::Union(vec![Ty::Number, Ty::Unknown])),
        ),
        (
            list(Ty::Union(vec![Ty::Null, list(Ty::Number)])),
            list(Ty::Union(vec![Ty::Null, Ty::Number])),
        ),
    ] {
        let resolved = resolve_call_signature(
            &signature,
            CallSignatureInput {
                arguments: &[ArgumentObservation::Typed(argument)],
            },
        );
        assert_eq!(resolved.validity, ShapeValidity::Valid);
        assert_eq!(resolved.return_ty, expected);
        assert!(!matches!(
            resolved.arguments[0].type_status,
            ArgumentTypeStatus::Mismatch { .. }
        ));
    }
}

#[test]
fn flat_return_refinement_does_not_hide_invalid_arguments() {
    let signature = function("flat");
    let missing = resolve_call_signature(&signature, CallSignatureInput { arguments: &[] });
    assert_eq!(
        missing.validity,
        ShapeValidity::Invalid(CallShapeError::TooFew {
            minimum: 1,
            actual: 0,
        })
    );
    assert_eq!(missing.return_ty, Ty::List(Box::new(Ty::Unknown)));

    let mismatch = resolve_call_signature(
        &signature,
        CallSignatureInput {
            arguments: &[ArgumentObservation::Typed(Ty::Union(vec![
                Ty::Number,
                Ty::List(Box::new(Ty::List(Box::new(Ty::String)))),
            ]))],
        },
    );
    assert!(matches!(
        mismatch.arguments[0].type_status,
        ArgumentTypeStatus::Mismatch { .. }
    ));
    assert_eq!(mismatch.return_ty, Ty::List(Box::new(Ty::String)));

    let extra = resolve_call_signature(
        &signature,
        CallSignatureInput {
            arguments: &[
                ArgumentObservation::Typed(Ty::List(Box::new(Ty::Number))),
                ArgumentObservation::Typed(Ty::String),
            ],
        },
    );
    assert_eq!(
        extra.validity,
        ShapeValidity::Invalid(CallShapeError::TooMany {
            maximum: 1,
            actual: 2,
        })
    );
}

#[test]
fn concat_incomplete_projection_and_generic_binding_share_one_resolution() {
    let signature = function("concat");
    let incomplete = resolve_call_signature(
        &signature,
        CallSignatureInput {
            arguments: &[ArgumentObservation::Empty],
        },
    );
    assert!(matches!(incomplete.validity, ShapeValidity::Invalid(_)));
    assert_eq!(incomplete.projection.len(), 2);
    assert_eq!(incomplete.projection[0].repeat_group, Some(1));
    assert_eq!(incomplete.projection[1].repeat_group, Some(2));

    let resolved = resolve_call_signature(
        &signature,
        CallSignatureInput {
            arguments: &[
                ArgumentObservation::Typed(Ty::List(Box::new(Ty::Number))),
                ArgumentObservation::Typed(Ty::List(Box::new(Ty::String))),
            ],
        },
    );
    assert_eq!(resolved.validity, ShapeValidity::Valid);
    assert_eq!(
        resolved.return_ty,
        Ty::List(Box::new(Ty::Union(vec![Ty::Number, Ty::String])))
    );
}

#[test]
fn splice_projects_zero_or_more_groups_after_its_head() {
    let signature = function("splice");
    let no_items = resolve_call_signature(
        &signature,
        CallSignatureInput {
            arguments: &[
                ArgumentObservation::Typed(Ty::List(Box::new(Ty::String))),
                ArgumentObservation::Typed(Ty::Number),
                ArgumentObservation::Typed(Ty::Number),
            ],
        },
    );
    assert_eq!(no_items.validity, ShapeValidity::Valid);
    assert_eq!(no_items.projection.len(), 3);

    let with_item = resolve_call_signature(
        &signature,
        CallSignatureInput {
            arguments: &[
                ArgumentObservation::Typed(Ty::List(Box::new(Ty::String))),
                ArgumentObservation::Typed(Ty::Number),
                ArgumentObservation::Typed(Ty::Number),
                ArgumentObservation::Typed(Ty::String),
            ],
        },
    );
    assert_eq!(with_item.validity, ShapeValidity::Valid);
    assert_eq!(with_item.projection[3].logical_param, ParamRef::Repeat(0));
    assert_eq!(with_item.projection[3].repeat_group, Some(1));
}

#[test]
fn ifs_partial_and_final_snapshots_support_staged_lambda_inference() {
    let signature = function("ifs");
    let partial = resolve_call_signature(
        &signature,
        CallSignatureInput {
            arguments: &[
                ArgumentObservation::Typed(Ty::Boolean),
                ArgumentObservation::Empty,
                ArgumentObservation::Typed(Ty::Boolean),
                ArgumentObservation::Empty,
            ],
        },
    );
    assert_eq!(partial.validity, ShapeValidity::Valid);
    assert_eq!(partial.projection.len(), 4);
    assert_eq!(partial.projection[3].logical_param, ParamRef::Repeat(1));

    let resolved = resolve_call_signature(
        &signature,
        CallSignatureInput {
            arguments: &[
                ArgumentObservation::Typed(Ty::Boolean),
                ArgumentObservation::Typed(Ty::String),
                ArgumentObservation::Typed(Ty::Boolean),
                ArgumentObservation::Typed(Ty::Number),
                ArgumentObservation::Typed(Ty::Boolean),
            ],
        },
    );
    assert_eq!(resolved.validity, ShapeValidity::Valid);
    assert_eq!(
        resolved.return_ty,
        Ty::Union(vec![Ty::Boolean, Ty::Number, Ty::String])
    );
}

#[test]
fn type_status_distinguishes_empty_unknown_mismatch_and_unmapped() {
    let signature = function("substring");
    let resolved = resolve_call_signature(
        &signature,
        CallSignatureInput {
            arguments: &[
                ArgumentObservation::Typed(Ty::Number),
                ArgumentObservation::Typed(Ty::Unknown),
                ArgumentObservation::Empty,
                ArgumentObservation::Typed(Ty::String),
            ],
        },
    );

    assert!(matches!(resolved.validity, ShapeValidity::Invalid(_)));
    assert!(matches!(
        resolved.arguments[0].type_status,
        ArgumentTypeStatus::Mismatch { actual: Ty::Number }
    ));
    assert_eq!(
        resolved.arguments[1].type_status,
        ArgumentTypeStatus::Indeterminate
    );
    assert_eq!(
        resolved.arguments[2].type_status,
        ArgumentTypeStatus::Indeterminate
    );
    assert_eq!(
        resolved.arguments[3].type_status,
        ArgumentTypeStatus::Unmapped
    );
}

#[test]
fn synthetic_case_of_covers_head_repeat_and_tail_projection() {
    let category = builtin_functions! {
        category: General;

        caseOf<T, U: Variant>(
            subject: T,
            repeat(min = 1) {
                candidate: T,
                result: () -> U,
            },
            otherwise: () -> U,
        ) -> U;
    };
    let signature = category.entries[0].implementation.as_ref().unwrap();
    let resolved = resolve_call_signature(
        signature,
        CallSignatureInput {
            arguments: &[
                ArgumentObservation::Typed(Ty::Number),
                ArgumentObservation::Typed(Ty::Number),
                ArgumentObservation::Typed(Ty::String),
                ArgumentObservation::Typed(Ty::Boolean),
            ],
        },
    );

    assert_eq!(resolved.validity, ShapeValidity::Valid);
    assert_eq!(resolved.projection[0].logical_param, ParamRef::Head(0));
    assert_eq!(resolved.projection[1].logical_param, ParamRef::Repeat(0));
    assert_eq!(resolved.projection[2].logical_param, ParamRef::Repeat(1));
    assert_eq!(resolved.projection[3].logical_param, ParamRef::Tail(0));
    assert_eq!(resolved.return_ty, Ty::Union(vec![Ty::Boolean, Ty::String]));
}
