//! Owned synchronous WASM session. The Worker owns scheduling and hides handles;
//! the session owns lifetime enforcement and delegates every domain operation.

use std::collections::HashMap;
use std::sync::Arc;

use formula_engine as rust;
use serde::{Serialize, de::DeserializeOwned};

use crate::converter::Converter;
use crate::dto::engine as dto;

include!("engine_session.h.rs");

const SERIALIZER: serde_wasm_bindgen::Serializer = serde_wasm_bindgen::Serializer::new()
    .serialize_large_number_types_as_bigints(true)
    .serialize_missing_as_null(true);

struct FormulaEngineSessionInner {
    engine: Option<Arc<rust::FormulaEngine>>,
    drafts: HashMap<u32, rust::FormulaDraft<'static>>,
    next_handle: u64,
}

impl Drop for FormulaEngineSessionInner {
    fn drop(&mut self) {
        self.close();
    }
}

impl FormulaEngineSessionInner {
    fn new(schema: rust::FormulaSchema) -> Result<Self, rust::FormulaEngineInitError> {
        Ok(Self {
            engine: Some(Arc::new(rust::FormulaEngine::new(schema)?)),
            drafts: HashMap::new(),
            next_handle: 1,
        })
    }

    fn engine(&self) -> Result<&rust::FormulaEngine, Box<dto::FormulaClientErrorData>> {
        self.engine.as_deref().ok_or_else(engine_closed)
    }

    fn engine_mut(&mut self) -> Result<&mut rust::FormulaEngine, Box<dto::FormulaClientErrorData>> {
        let engine = self.engine.as_mut().ok_or_else(engine_closed)?;
        let count = self.drafts.len();
        // Arc's uniqueness is the ownership guard, including any additional
        // owned Draft references; no mutation can invalidate Draft analysis.
        Arc::get_mut(engine).ok_or_else(|| {
            Box::new(dto::FormulaClientErrorData::ActiveDrafts {
                message: "Close or consume all drafts before changing saved definitions".into(),
                payload: dto::ActiveDraftsPayload {
                    count: count as u32,
                },
            })
        })
    }

    fn draft(
        &self,
        handle: u32,
    ) -> Result<&rust::FormulaDraft<'static>, Box<dto::FormulaClientErrorData>> {
        self.engine()?;
        self.drafts.get(&handle).ok_or_else(|| draft_closed(handle))
    }

    fn draft_mut(
        &mut self,
        handle: u32,
    ) -> Result<&mut rust::FormulaDraft<'static>, Box<dto::FormulaClientErrorData>> {
        self.engine()?;
        self.drafts
            .get_mut(&handle)
            .ok_or_else(|| draft_closed(handle))
    }

    fn create_draft(
        &mut self,
        definition: rust::FormulaDefinition,
    ) -> Result<u32, Box<dto::FormulaClientErrorData>> {
        let engine = self.engine.as_ref().ok_or_else(engine_closed)?;
        let handle = u32::try_from(self.next_handle).map_err(|_| {
            Box::new(dto::FormulaClientErrorData::SerializeError {
                message: "Draft handle limit reached".into(),
                payload: (),
            })
        })?;
        let draft = rust::FormulaDraft::from_shared_engine(Arc::clone(engine), definition)
            .map_err(|error| {
                Box::new(dto::FormulaClientErrorData::CreateDraft {
                    message: "Invalid draft definition".into(),
                    payload: dto::CreateDraftPayload {
                        error: error.into(),
                    },
                })
            })?;
        self.next_handle += 1;
        self.drafts.insert(handle, draft);
        Ok(handle)
    }

    fn take_draft(
        &mut self,
        handle: u32,
    ) -> Result<rust::FormulaDraft<'static>, Box<dto::FormulaClientErrorData>> {
        self.engine()?;
        self.drafts
            .remove(&handle)
            .ok_or_else(|| draft_closed(handle))
    }

    fn close(&mut self) {
        self.drafts.clear();
        self.engine = None;
    }
}

impl FormulaEngineSession {
    fn new_impl(schema: JsValue) -> Result<Self, JsValue> {
        let schema: dto::FormulaSchema = from_value(schema, "new")?;
        let inner = FormulaEngineSessionInner::new(schema.into()).map_err(|error| {
            error_value(dto::FormulaClientErrorData::EngineInit {
                message: "Invalid engine schema".into(),
                payload: dto::EngineInitPayload {
                    error: error.into(),
                },
            })
        })?;
        Ok(Self { inner })
    }

    fn get_property_impl(&self, id: String) -> Result<JsValue, JsValue> {
        let property = self
            .inner
            .engine()
            .map_err(error_value)?
            .property(&id.into());
        to_value(&property.map(dto::PropertyState::from))
    }

    fn get_properties_impl(&self) -> Result<JsValue, JsValue> {
        let properties: Vec<dto::PropertyState> = self
            .inner
            .engine()
            .map_err(error_value)?
            .properties()
            .into_iter()
            .map(Into::into)
            .collect();
        to_value(&properties)
    }

    fn get_state_impl(&self) -> Result<JsValue, JsValue> {
        let state: dto::FormulaEngineState =
            self.inner.engine().map_err(error_value)?.state().into();
        to_value(&state)
    }

    fn upsert_impl(&mut self, property: JsValue) -> Result<JsValue, JsValue> {
        let engine = self.inner.engine_mut().map_err(error_value)?;
        let property: dto::PropertyDefinition = from_value(property, "upsert")?;
        let change: dto::FormulaEngineChangeResult = engine
            .upsert(property.into())
            .map_err(|error| {
                error_value(dto::FormulaClientErrorData::EngineChange {
                    message: "Invalid property definition".into(),
                    payload: dto::EngineChangePayload {
                        error: error.into(),
                    },
                })
            })?
            .into();
        to_value(&change)
    }

    fn remove_impl(&mut self, id: String) -> Result<JsValue, JsValue> {
        let engine = self.inner.engine_mut().map_err(error_value)?;
        to_value(
            &engine
                .remove(&id.into())
                .map(dto::FormulaEngineChangeResult::from),
        )
    }

    fn evaluate_impl(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let engine = self.inner.engine().map_err(error_value)?;
        let input: dto::EvaluateInput = from_value(input, "evaluate")?;
        let result: dto::EvaluateResult = engine
            .evaluate(&input.into())
            .map_err(|error| {
                error_value(dto::FormulaClientErrorData::EvaluateInput {
                    message: "Invalid evaluation input".into(),
                    payload: dto::EvaluateInputPayload {
                        error: error.into(),
                    },
                })
            })?
            .into();
        to_value(&result)
    }

    fn create_draft_impl(&mut self, formula: JsValue) -> Result<u32, JsValue> {
        self.inner.engine().map_err(error_value)?;
        let formula: dto::FormulaDefinition = from_value(formula, "create_draft")?;
        self.inner.create_draft(formula.into()).map_err(error_value)
    }

    fn draft_state_impl(&self, handle: u32) -> Result<JsValue, JsValue> {
        let draft = self.inner.draft(handle).map_err(error_value)?;
        to_value(&Converter::draft_state(draft.state()))
    }

    fn draft_help_impl(
        &self,
        handle: u32,
        cursor: JsValue,
        config: JsValue,
    ) -> Result<JsValue, JsValue> {
        let draft = self.inner.draft(handle).map_err(error_value)?;
        let cursor: u32 = from_value(cursor, "draft_help")?;
        let config: dto::CompletionConfig = from_value(config, "draft_help")?;
        let source = &draft.state().definition.expression;
        let cursor = rust::TextOffset(Converter::utf16_to_8_offset(source, cursor as usize));
        let help = draft.help(
            cursor,
            rust::CompletionConfig {
                preferred_limit: config.preferred_limit as usize,
            },
        );
        to_value(&Converter::cursor_help(source, help))
    }

    fn draft_quick_fixes_impl(
        &self,
        handle: u32,
        diagnostic_id: String,
    ) -> Result<JsValue, JsValue> {
        let draft = self.inner.draft(handle).map_err(error_value)?;
        let source = &draft.state().definition.expression;
        let fixes: Vec<dto::QuickFix> = draft
            .quick_fixes(&rust::DiagnosticId(diagnostic_id))
            .into_iter()
            .map(|fix| dto::QuickFix {
                title: fix.title,
                edit: Converter::formula_edit(source, fix.edit),
            })
            .collect();
        to_value(&fixes)
    }

    fn draft_format_edits_impl(&self, handle: u32) -> Result<JsValue, JsValue> {
        let draft = self.inner.draft(handle).map_err(error_value)?;
        let edit = draft.format_edits().map_err(|_| {
            error_value(dto::FormulaClientErrorData::FormatError {
                message: "Expression cannot be formatted".into(),
                payload: (),
            })
        })?;
        to_value(&Converter::formula_edit(
            &draft.state().definition.expression,
            edit,
        ))
    }

    fn draft_update_expression_impl(
        &mut self,
        handle: u32,
        update: JsValue,
    ) -> Result<JsValue, JsValue> {
        let draft = self.inner.draft_mut(handle).map_err(error_value)?;
        let update: dto::ExpressionUpdate = from_value(update, "draft_update_expression")?;
        let update = Converter::expression_update(draft.state(), update).map_err(update_error)?;
        let result = draft.update_expression(update).map_err(update_error)?;
        to_value(&dto::UpdateExpressionResult {
            cursor: Converter::utf8_to_16_offset(
                &result.state.definition.expression,
                result.cursor.0,
            ),
            state: Converter::draft_state(&result.state),
        })
    }

    fn draft_into_definition_impl(&mut self, handle: u32) -> Result<JsValue, JsValue> {
        let draft = self.inner.take_draft(handle).map_err(error_value)?;
        let definition: dto::FormulaDefinition = draft.into_definition().into();
        to_value(&definition)
    }

    fn draft_close_impl(&mut self, handle: u32) {
        self.inner.drafts.remove(&handle);
    }

    fn close_impl(&mut self) {
        self.inner.close();
    }
}

fn engine_closed() -> Box<dto::FormulaClientErrorData> {
    Box::new(dto::FormulaClientErrorData::EngineClosed {
        message: "Engine is closed".into(),
        payload: (),
    })
}

fn draft_closed(handle: u32) -> Box<dto::FormulaClientErrorData> {
    Box::new(dto::FormulaClientErrorData::DraftClosed {
        message: "Draft is closed or consumed".into(),
        payload: dto::DraftClosedPayload { handle },
    })
}

fn update_error(error: rust::UpdateExpressionError) -> JsValue {
    error_value(dto::FormulaClientErrorData::UpdateExpression {
        message: "Invalid expression update".into(),
        payload: dto::UpdateExpressionPayload {
            error: error.into(),
        },
    })
}

fn error_value<T: Serialize>(error: T) -> JsValue {
    // Every error field has an infallible scalar/array DTO representation.
    error
        .serialize(&SERIALIZER)
        .expect("error DTO must serialize")
}

fn from_value<T: DeserializeOwned>(value: JsValue, operation: &str) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value).map_err(|_| {
        error_value(dto::FormulaClientErrorData::InvalidDto {
            message: "Invalid DTO".into(),
            payload: dto::InvalidDtoPayload {
                operation: operation.into(),
            },
        })
    })
}

fn to_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value.serialize(&SERIALIZER).map_err(|_| {
        error_value(dto::FormulaClientErrorData::SerializeError {
            message: "DTO serialization failed".into(),
            payload: (),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(id: &str, expression: &str) -> rust::FormulaDefinition {
        rust::FormulaDefinition {
            id: id.into(),
            expression: expression.into(),
        }
    }

    #[test]
    fn owned_drafts_guard_mutations_and_are_consumed_or_discarded() {
        let mut session =
            FormulaEngineSessionInner::new(rust::FormulaSchema { properties: vec![] }).unwrap();
        let first = session.create_draft(definition("first", "1")).unwrap();
        let second = session.create_draft(definition("second", "2")).unwrap();
        assert!(matches!(
            session.engine_mut(),
            Err(error) if matches!(*error, dto::FormulaClientErrorData::ActiveDrafts {
                payload: dto::ActiveDraftsPayload { count: 2 }, .. })
        ));
        let completed = session.take_draft(first).unwrap().into_definition();
        assert_eq!(completed, definition("first", "1"));
        assert!(matches!(
            session.draft(first),
            Err(error) if matches!(*error, dto::FormulaClientErrorData::DraftClosed { .. })
        ));
        assert!(session.engine_mut().is_err());
        session.drafts.remove(&second);
        session
            .engine_mut()
            .unwrap()
            .upsert(rust::PropertyDefinition::Formula(completed))
            .unwrap();
        assert!(
            session
                .engine()
                .unwrap()
                .property(&"first".into())
                .is_some()
        );
        let later = session.create_draft(definition("first", "3")).unwrap();
        assert_ne!(later, first);
    }

    #[test]
    fn engine_close_releases_drafts_and_is_idempotent() {
        let mut session =
            FormulaEngineSessionInner::new(rust::FormulaSchema { properties: vec![] }).unwrap();
        let handle = session.create_draft(definition("formula", "1")).unwrap();
        let engine = Arc::downgrade(session.engine.as_ref().unwrap());
        session.close();
        session.close();
        assert!(engine.upgrade().is_none());
        assert!(session.drafts.is_empty());
        assert!(matches!(
            session.draft(handle),
            Err(error) if matches!(*error, dto::FormulaClientErrorData::EngineClosed { .. })
        ));
        assert!(matches!(
            session.engine_mut(),
            Err(error) if matches!(*error, dto::FormulaClientErrorData::EngineClosed { .. })
        ));
    }
}
