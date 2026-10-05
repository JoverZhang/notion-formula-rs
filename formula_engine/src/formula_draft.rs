//! Candidate formula state held through an immutable Engine borrow.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use analyzer::analysis::Context;
use analyzer::{Diagnostic, DiagnosticCode, DiagnosticKind, TokenKind};

use crate::formula_engine::{from_analyzer_type, visit_property_references};

include!("formula_draft.h.rs");

// Assign once when a diagnostic enters a snapshot, so repeated reads are stable
// and another Draft or a later revision cannot accidentally reuse the same ID.
static NEXT_DIAGNOSTIC_ID: AtomicU64 = AtomicU64::new(0);

struct FormulaDraftInner<'engine> {
    engine: DraftEngine<'engine>,
    analysis: DraftAnalysis,
}

// Owned sessions need to keep a draft and its immutable engine alive together.
// Both ownership forms run the same analysis; Arc::get_mut also keeps callers
// from changing the saved engine while an owned draft exists.
enum DraftEngine<'engine> {
    Borrowed(&'engine FormulaEngine),
    Shared(Arc<FormulaEngine>),
}

impl DraftEngine<'_> {
    fn engine(&self) -> &FormulaEngine {
        match self {
            Self::Borrowed(engine) => engine,
            Self::Shared(engine) => engine,
        }
    }
}

struct DraftAnalysis {
    state: FormulaDraftState,
    context: Context,
    quick_fixes: HashMap<DiagnosticId, Vec<QuickFix>>,
}

impl std::fmt::Debug for FormulaDraftInner<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormulaDraftInner")
            .field("state", &self.analysis.state)
            .finish_non_exhaustive()
    }
}

impl<'engine> FormulaDraft<'engine> {
    pub(crate) fn new(engine: &'engine FormulaEngine, definition: FormulaDefinition) -> Self {
        Self {
            inner: FormulaDraftInner {
                engine: DraftEngine::Borrowed(engine),
                analysis: DraftAnalysis::new(engine, definition, DraftVersion(0)),
            },
        }
    }

    fn state_impl(&self) -> &FormulaDraftState {
        &self.inner.analysis.state
    }

    fn help_impl(&self, cursor: TextOffset, config: CompletionConfig) -> CursorHelp {
        let source = &self.state().definition.expression;
        // Help has no coordinate error return; keep malformed caller coordinates
        // within source boundaries before passing them to the IDE helpers.
        let mut cursor = cursor.0.min(source.len());
        while !source.is_char_boundary(cursor) {
            cursor -= 1;
        }
        let help = ide::help(source, cursor, &self.inner.analysis.context, config);
        CursorHelp {
            base_version: self.state().version,
            completion: help.completion,
            signature_help: help.signature_help,
        }
    }

    fn quick_fixes_impl(&self, diagnostic_id: &DiagnosticId) -> Vec<QuickFix> {
        self.inner
            .analysis
            .quick_fixes
            .get(diagnostic_id)
            .cloned()
            .unwrap_or_default()
    }

    fn format_edits_impl(&self) -> Result<FormulaEdit, FormatError> {
        let source = &self.state().definition.expression;
        let formatted = ide::format(source, 0).map_err(|_| FormatError)?;
        let end = u32::try_from(source.len()).map_err(|_| FormatError)?;
        Ok(FormulaEdit {
            base_version: self.state().version,
            edits: vec![TextEdit {
                range: Span { start: 0, end },
                new_text: formatted.source,
            }],
        })
    }

    fn update_expression_impl(
        &mut self,
        update: ExpressionUpdate,
    ) -> Result<UpdateExpressionResult, UpdateExpressionError> {
        let current = self.state();
        let (expression, cursor) = match update {
            ExpressionUpdate::Replace(expression) => {
                let cursor = TextOffset(expression.len());
                if expression == current.definition.expression {
                    return Ok(UpdateExpressionResult {
                        state: current.clone(),
                        cursor,
                    });
                }
                (expression, cursor)
            }
            ExpressionUpdate::Edits { edit, cursor } => {
                if edit.base_version != current.version {
                    return Err(UpdateExpressionError::VersionMismatch);
                }
                let cursor =
                    u32::try_from(cursor.0).map_err(|_| UpdateExpressionError::InvalidCursor)?;
                let applied = ide::apply_edits(&current.definition.expression, edit.edits, cursor)
                    .map_err(map_edit_error)?;
                (applied.source, TextOffset(applied.cursor as usize))
            }
        };

        let definition = FormulaDefinition {
            id: current.definition.id.clone(),
            expression,
        };
        let version = DraftVersion(current.version.0 + 1);
        let analysis = DraftAnalysis::new(self.inner.engine.engine(), definition, version);
        self.inner.analysis = analysis;
        Ok(UpdateExpressionResult {
            state: self.state().clone(),
            cursor,
        })
    }

    fn into_definition_impl(self) -> FormulaDefinition {
        self.inner.analysis.state.definition
    }
}

impl FormulaDraft<'static> {
    /// Internal ownership bridge for wrappers that store Engine and Draft handles.
    #[doc(hidden)]
    pub fn from_shared_engine(
        engine: Arc<FormulaEngine>,
        definition: FormulaDefinition,
    ) -> Result<Self, crate::CreateDraftError> {
        if definition.id.0.is_empty() {
            return Err(crate::CreateDraftError::EmptyId);
        }
        let analysis = DraftAnalysis::new(&engine, definition, DraftVersion(0));
        Ok(Self {
            inner: FormulaDraftInner {
                engine: DraftEngine::Shared(engine),
                analysis,
            },
        })
    }
}

impl DraftAnalysis {
    fn new(engine: &FormulaEngine, definition: FormulaDefinition, version: DraftVersion) -> Self {
        let context = engine.draft_context(&definition);
        let mut syntax = analyzer::analyze_syntax(&definition.expression);
        let (output_type, semantic_diagnostics) =
            analyzer::analysis::analyze_expr(&mut syntax.expr, &context);
        syntax.diagnostics.extend(semantic_diagnostics);

        // The analyzer uses disabled properties for inference and completion;
        // Engine readiness and dependency cycles need diagnostics of their own.
        let mut property_references = Vec::new();
        visit_property_references(&syntax.expr, &mut |name, id_span, span| {
            if complete_property_call(&syntax.tokens, span)
                && !syntax.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == DiagnosticCode::LexError
                        && diagnostic.span.start < span.end
                        && diagnostic.span.end > span.start
                })
            {
                property_references.push(PropertyReference {
                    property_id: PropertyId(name.into()),
                    span,
                    id_span,
                });
            }
            if let Some(reason) = context
                .properties
                .iter()
                .find(|property| property.name == name)
                .and_then(|property| property.disabled_reason.as_ref())
            {
                syntax.diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Error,
                    code: DiagnosticCode::SemanticError,
                    message: reason.clone(),
                    span: id_span,
                    labels: Vec::new(),
                    notes: Vec::new(),
                    actions: Vec::new(),
                });
            }
        });

        let mut quick_fixes = HashMap::new();
        let diagnostics = syntax
            .diagnostics
            .into_iter()
            .map(|diagnostic| {
                let serial = NEXT_DIAGNOSTIC_ID.fetch_add(1, Ordering::Relaxed);
                let id = DiagnosticId(format!("draft-diagnostic-{serial}"));
                if !diagnostic.actions.is_empty() {
                    let fixes = diagnostic
                        .actions
                        .into_iter()
                        .map(|action| QuickFix {
                            title: action.title,
                            edit: FormulaEdit {
                                base_version: version,
                                edits: action.edits,
                            },
                        })
                        .collect();
                    quick_fixes.insert(id.clone(), fixes);
                }
                ExpressionDiagnostic {
                    id,
                    span: diagnostic.span,
                    message: diagnostic.message,
                }
            })
            .collect();
        Self {
            state: FormulaDraftState {
                version,
                definition,
                output_type: from_analyzer_type(&output_type),
                diagnostics,
                tokens: syntax.tokens,
                property_references,
            },
            context,
            quick_fixes,
        }
    }
}

// Parser recovery can retain a one-literal Call after malformed or missing
// delimiters. Only expose complete calls as editable property chips.
fn complete_property_call(tokens: &[Token], span: Span) -> bool {
    let start = tokens.partition_point(|token| token.span.start < span.start);
    let mut kinds = tokens[start..]
        .iter()
        .take_while(|token| token.span.start < span.end)
        .map(|token| &token.kind)
        .filter(|kind| !matches!(kind, TokenKind::DocComment(..) | TokenKind::Newline));
    matches!(kinds.next(), Some(TokenKind::Ident(_)))
        && matches!(kinds.next(), Some(TokenKind::OpenParen))
        && matches!(kinds.next(), Some(TokenKind::Literal(_)))
        && matches!(kinds.next(), Some(TokenKind::CloseParen))
        && kinds.next().is_none()
}

fn map_edit_error(error: ide::IdeError) -> UpdateExpressionError {
    match error {
        ide::IdeError::InvalidCursor => UpdateExpressionError::InvalidCursor,
        ide::IdeError::InvalidEditRange => UpdateExpressionError::InvalidEditRange,
        ide::IdeError::OverlappingEdits => UpdateExpressionError::OverlappingEdits,
        ide::IdeError::FormatError => unreachable!("applying edits does not format source"),
    }
}
