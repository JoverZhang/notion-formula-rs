use builtin_fn::{
    ArgumentObservation, ArgumentTypeStatus, CallSignatureInput, ParamRef, ShapeValidity, Ty,
    builtin_functions, resolve_call_signature,
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
        Ty::List(Box::new(Ty::Union(vec![Ty::Number, Ty::String])))
    );
    assert_eq!(resolved.projection[0].logical_param, ParamRef::Head(0));
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
    assert!(matches!(partial.validity, ShapeValidity::Invalid(_)));
    assert_eq!(partial.projection.len(), 5);
    assert_eq!(partial.projection[4].logical_param, ParamRef::Tail(0));

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
