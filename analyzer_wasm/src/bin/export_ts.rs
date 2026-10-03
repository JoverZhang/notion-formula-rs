use std::fs;
use std::path::PathBuf;

use analyzer_wasm::dto::engine;
use analyzer_wasm::dto::v1::{
    AnalyzeResult, AnalyzerConfig, ApplyResult, CodeAction, CompletionItem, CompletionItemKind,
    CompletionResult, Diagnostic, DiagnosticKind, DisplaySegment, HelpResult, Property,
    SignatureHelp, SignatureItem, Span, TextEdit, Token, Ty,
};
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/vite/src/analyzer/generated/wasm_dto.ts");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut out = String::new();
    out.push_str("/* eslint-disable */\n");
    out.push_str("/* prettier-ignore */\n");
    out.push_str("// AUTO-GENERATED: `cargo run -p analyzer_wasm --bin export_ts`\n\n");

    for decl in [
        Ty::decl(),
        Property::decl(),
        AnalyzerConfig::decl(),
        Span::decl(),
        TextEdit::decl(),
        CodeAction::decl(),
        DiagnosticKind::decl(),
        Diagnostic::decl(),
        Token::decl(),
        AnalyzeResult::decl(),
        ApplyResult::decl(),
        DisplaySegment::decl(),
        SignatureItem::decl(),
        SignatureHelp::decl(),
        CompletionItemKind::decl(),
        CompletionItem::decl(),
        CompletionResult::decl(),
        HelpResult::decl(),
    ] {
        let decl = export_decl(decl);
        out.push_str(&decl);
        if !decl.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
    }

    fs::write(out_path, out)?;
    export_engine()?;
    Ok(())
}

fn export_engine() -> Result<(), Box<dyn std::error::Error>> {
    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/vite/src/engine/generated/wasm_dto.ts");
    fs::create_dir_all(out_path.parent().expect("generated DTO has a parent"))?;
    let mut out = String::from(
        "/* eslint-disable */\n/* prettier-ignore */\n// AUTO-GENERATED: `cargo run -p analyzer_wasm --bin export_ts`\n\n",
    );
    for decl in [
        format!("type PropertyId = {};", <engine::PropertyId>::inline()),
        format!("type RowId = {};", <engine::RowId>::inline()),
        format!("type DraftVersion = {};", <engine::DraftVersion>::inline()),
        format!("type DiagnosticId = {};", <engine::DiagnosticId>::inline()),
        format!("type TextOffset = {};", <engine::TextOffset>::inline()),
        Span::decl(),
        TextEdit::decl(),
        Token::decl(),
        DisplaySegment::decl(),
        SignatureItem::decl(),
        engine::SignatureHelp::decl(),
        CompletionItemKind::decl(),
        CompletionItem::decl(),
        engine::CompletionResult::decl(),
        engine::ValueType::decl(),
        engine::FormulaSchema::decl(),
        engine::PropertyDefinition::decl(),
        engine::FormulaDefinition::decl(),
        engine::FormulaEngineState::decl(),
        engine::PropertyState::decl(),
        engine::FormulaState::decl(),
        engine::FormulaStatus::decl(),
        engine::FormulaEngineChangeResult::decl(),
        engine::Value::decl(),
        engine::ColumnData::<f64>::decl(),
        engine::Column::decl(),
        engine::ColumnKind::decl(),
        engine::RuntimeContext::decl(),
        engine::EvaluateInput::decl(),
        engine::EvaluateResult::decl(),
        engine::FormulaOutput::decl(),
        engine::RowError::decl(),
        engine::RuntimeError::decl(),
        engine::FormulaEvaluationError::decl(),
        engine::FormulaEngineInitError::decl(),
        engine::EngineChangeError::decl(),
        engine::CreateDraftError::decl(),
        engine::EvaluateInputError::decl(),
        engine::CompletionConfig::decl(),
        engine::FormulaDraftState::decl(),
        engine::ExpressionDiagnostic::decl(),
        engine::CursorHelp::decl(),
        engine::FormulaEdit::decl(),
        engine::ExpressionUpdate::decl(),
        engine::QuickFix::decl(),
        engine::UpdateExpressionResult::decl(),
        engine::UpdateExpressionError::decl(),
        engine::InvalidDtoPayload::decl(),
        engine::DraftClosedPayload::decl(),
        engine::ActiveDraftsPayload::decl(),
        engine::EngineInitPayload::decl(),
        engine::EngineChangePayload::decl(),
        engine::CreateDraftPayload::decl(),
        engine::EvaluateInputPayload::decl(),
        engine::UpdateExpressionPayload::decl(),
        engine::FormulaClientErrorData::decl(),
    ] {
        out.push_str(&export_decl(decl));
        out.push_str("\n\n");
    }
    fs::write(out_path, out)?;
    Ok(())
}

fn export_decl(mut decl: String) -> String {
    let trimmed = decl.trim_start();
    if trimmed.starts_with("export ") {
        return decl;
    }

    if trimmed.starts_with("type ")
        || trimmed.starts_with("interface ")
        || trimmed.starts_with("enum ")
        || trimmed.starts_with("declare ")
    {
        decl.insert_str(decl.len() - trimmed.len(), "export ");
    }

    decl
}
