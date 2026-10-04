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
    let mut check = false;
    for argument in std::env::args_os().skip(1) {
        if argument == "--check" && !check {
            check = true;
        } else {
            return Err("usage: export_ts [--check]".into());
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let outputs = [
        (
            root.join("../examples/vite/src/analyzer/generated/wasm_dto.ts"),
            analyzer_dto(),
        ),
        (
            root.join("../packages/notion-formula/src/generated/wasm_dto.ts"),
            engine_dto(),
        ),
    ];
    if check {
        return check_outputs(&outputs);
    }
    for (path, contents) in outputs {
        fs::create_dir_all(path.parent().expect("generated DTO has a parent"))?;
        fs::write(path, contents)?;
    }
    Ok(())
}

fn check_outputs(outputs: &[(PathBuf, String)]) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();
    for (path, expected) in outputs {
        match fs::read(path) {
            Ok(actual) if actual == expected.as_bytes() => {}
            Ok(_) => errors.push(format!("Generated DTO is stale: {}", path.display())),
            Err(error) => errors.push(format!(
                "Cannot read generated DTO {}: {error}",
                path.display()
            )),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

fn analyzer_dto() -> String {
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

    out
}

fn engine_dto() -> String {
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
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_reports_both_stale_outputs_and_never_writes_or_recreates_files() {
        let root = std::env::temp_dir().join(format!("wasm-dto-check-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let outputs = [
            (root.join("legacy.ts"), "legacy generated\n".to_string()),
            (root.join("engine.ts"), "engine generated\n".to_string()),
        ];
        fs::write(&outputs[0].0, "legacy changed\n").unwrap();
        fs::write(&outputs[1].0, [0xff]).unwrap();
        let error = check_outputs(&outputs).unwrap_err().to_string();
        assert!(error.contains("legacy.ts"));
        assert!(error.contains("engine.ts"));
        assert_eq!(fs::read(&outputs[0].0).unwrap(), b"legacy changed\n");
        assert_eq!(fs::read(&outputs[1].0).unwrap(), [0xff]);

        for (path, contents) in &outputs {
            fs::write(path, contents).unwrap();
        }
        check_outputs(&outputs).unwrap();
        fs::remove_file(&outputs[1].0).unwrap();
        let error = check_outputs(&outputs).unwrap_err().to_string();
        assert!(error.contains("engine.ts"));
        assert!(!outputs[1].0.exists());
        assert_eq!(fs::read(&outputs[0].0).unwrap(), outputs[0].1.as_bytes());
        fs::remove_dir_all(root).unwrap();
    }
}
