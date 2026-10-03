//! Candidate formula state held through an immutable Engine borrow.

include!("formula_draft.h.rs");

struct FormulaDraftInner<'engine> {
    engine: &'engine FormulaEngine,
    state: FormulaDraftState,
}

impl std::fmt::Debug for FormulaDraftInner<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormulaDraftInner")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl<'engine> FormulaDraft<'engine> {
    pub(crate) fn new(engine: &'engine FormulaEngine, definition: FormulaDefinition) -> Self {
        Self {
            inner: FormulaDraftInner {
                engine,
                state: FormulaDraftState {
                    version: DraftVersion(0),
                    definition,
                    output_type: ValueType::Unknown,
                    diagnostics: Vec::new(),
                    tokens: Vec::new(),
                },
            },
        }
    }

    fn state_impl(&self) -> &FormulaDraftState {
        &self.inner.state
    }

    fn help_impl(&self, _cursor: TextOffset, _config: CompletionConfig) -> CursorHelp {
        todo!("candidate help")
    }

    fn quick_fixes_impl(&self, _diagnostic_id: &DiagnosticId) -> Vec<QuickFix> {
        todo!("diagnostic recovery actions")
    }

    fn format_edits_impl(&self) -> Result<FormulaEdit, FormatError> {
        todo!("version-bound formatting edits")
    }

    fn update_expression_impl(
        &mut self,
        _update: ExpressionUpdate,
    ) -> Result<UpdateExpressionResult, UpdateExpressionError> {
        todo!("atomic candidate updates")
    }

    fn into_definition_impl(self) -> FormulaDefinition {
        self.inner.state.definition
    }
}
