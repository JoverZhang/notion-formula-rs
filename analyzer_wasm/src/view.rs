use analyzer::{Diagnostic, DiagnosticKind, ParseOutput, SourceMap, Span, Token, TokenKind};

use crate::dto::v1::{
    AnalyzeResult, CompletionItemKind, CompletionItemView, CompletionOutputView,
    DiagnosticKindView, DiagnosticView, SignatureHelpView, SpanView, TextEditView, TokenView,
    Utf16Span,
};
use crate::offsets::byte_offset_to_utf16_offset;
use crate::span::byte_span_to_utf16_span;
use crate::text_edit::apply_text_edits_bytes;

pub struct ViewCtx<'a> {
    source: &'a str,
    sm: SourceMap<'a>,
}

impl<'a> ViewCtx<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            sm: SourceMap::new(source),
        }
    }

    pub fn analyze_output(&self, output: ParseOutput) -> AnalyzeResult {
        let diagnostics = output.diagnostics.iter().map(|d| self.diag(d)).collect();

        let tokens = output
            .tokens
            .iter()
            .filter(|t| !t.is_trivia())
            .map(|t| self.token(t))
            .collect();

        let formatted = analyzer::format_expr(&output.expr, self.source, &output.tokens);

        AnalyzeResult {
            diagnostics,
            tokens,
            formatted,
        }
    }

    pub fn analyze_error(&self, diag: &Diagnostic) -> AnalyzeResult {
        AnalyzeResult {
            diagnostics: vec![self.diag(diag)],
            tokens: Vec::new(),
            formatted: String::new(),
        }
    }

    pub fn invalid_context_diag(&self) -> DiagnosticView {
        let diag = Diagnostic {
            kind: DiagnosticKind::Error,
            message: "Invalid context JSON".into(),
            span: Span { start: 0, end: 0 },
            labels: vec![],
            notes: vec![],
        };
        self.diag(&diag)
    }

    pub fn completion_output(&self, output: &analyzer::CompletionOutput) -> CompletionOutputView {
        let replace = self.simple_span(output.replace);
        let signature_help = output.signature_help.as_ref().map(|sig| SignatureHelpView {
            label: sig.label.clone(),
            params: sig.params.clone(),
            active_param: sig.active_param,
        });

        let items = output
            .items
            .iter()
            .map(|item| self.completion_item(output, item))
            .collect();

        CompletionOutputView {
            items,
            replace,
            signature_help,
        }
    }

    fn completion_item(
        &self,
        output: &analyzer::CompletionOutput,
        item: &analyzer::CompletionItem,
    ) -> CompletionItemView {
        let primary_edit_view = item.primary_edit.as_ref().map(|edit| TextEditView {
            range: self.simple_span(edit.range),
            new_text: edit.new_text.clone(),
        });

        let additional_edits = item
            .additional_edits
            .iter()
            .map(|edit| TextEditView {
                range: self.simple_span(edit.range),
                new_text: edit.new_text.clone(),
            })
            .collect::<Vec<_>>();

        let cursor_utf16 = item.primary_edit.as_ref().map(|primary_edit| {
            let mut edits = Vec::with_capacity(1 + item.additional_edits.len());
            edits.push(primary_edit.clone());
            edits.extend(item.additional_edits.iter().cloned());
            let updated = apply_text_edits_bytes(self.source, &edits);

            let cursor_byte = item.cursor.unwrap_or_else(|| {
                output
                    .replace
                    .start
                    .saturating_add(primary_edit.new_text.len() as u32)
            });
            let cursor_byte = usize::min(cursor_byte as usize, updated.len());
            byte_offset_to_utf16_offset(&updated, cursor_byte)
        });

        CompletionItemView {
            label: item.label.clone(),
            kind: completion_kind_view(item.kind),
            insert_text: item.insert_text.clone(),
            primary_edit: primary_edit_view,
            cursor: cursor_utf16,
            additional_edits,
            detail: item.detail.clone(),
            is_disabled: item.is_disabled,
            disabled_reason: item.disabled_reason.clone(),
        }
    }

    fn diag(&self, diag: &Diagnostic) -> DiagnosticView {
        DiagnosticView {
            kind: diagnostic_kind_view(&diag.kind),
            message: diag.message.clone(),
            span: self.span(diag.span),
        }
    }

    fn token(&self, token: &Token) -> TokenView {
        let start = token.span.start as usize;
        let end = token.span.end as usize;
        let text = self.source.get(start..end).unwrap_or("").to_string();

        TokenView {
            kind: token_kind_string(&token.kind).to_string(),
            text,
            span: self.span(token.span),
        }
    }

    fn span(&self, span: Span) -> SpanView {
        let range = byte_span_to_utf16_span(self.source, span);
        let (line, col) = self.sm.line_col(span.start);

        SpanView {
            range,
            line: line as u32,
            col: col as u32,
        }
    }

    fn simple_span(&self, span: Span) -> Utf16Span {
        byte_span_to_utf16_span(self.source, span)
    }
}

fn diagnostic_kind_view(kind: &DiagnosticKind) -> DiagnosticKindView {
    match kind {
        DiagnosticKind::Error => DiagnosticKindView::Error,
    }
}

fn completion_kind_view(kind: analyzer::CompletionKind) -> CompletionItemKind {
    use analyzer::CompletionKind::*;
    match kind {
        Function => CompletionItemKind::Function,
        Builtin => CompletionItemKind::Builtin,
        Property => CompletionItemKind::Property,
        Operator => CompletionItemKind::Operator,
    }
}

fn token_kind_string(kind: &TokenKind) -> &'static str {
    use TokenKind::*;
    use analyzer::LitKind;

    match kind {
        Lt => "Lt",
        Le => "Le",
        EqEq => "EqEq",
        Ne => "Ne",
        Ge => "Ge",
        Gt => "Gt",
        AndAnd => "AndAnd",
        OrOr => "OrOr",
        Bang => "Bang",
        Plus => "Plus",
        Minus => "Minus",
        Star => "Star",
        Slash => "Slash",
        Percent => "Percent",
        Caret => "Caret",
        Dot => "Dot",
        Comma => "Comma",
        Colon => "Colon",
        Pound => "Pound",
        Question => "Question",
        OpenParen => "OpenParen",
        CloseParen => "CloseParen",
        Literal(lit) => match lit.kind {
            LitKind::Bool => "Bool",
            LitKind::Number => "Number",
            LitKind::String => "String",
        },
        Ident(_) => "Ident",
        DocComment(..) => "DocComment",
        LineComment(_) => "LineComment",
        BlockComment(_) => "BlockComment",
        Newline => "Newline",
        Eof => "Eof",
    }
}
