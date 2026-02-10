use std::fs;
use std::path::PathBuf;

use analyzer_wasm::dto::v1::{
    AnalyzeResult, CompletionItemKind, CompletionItemView, CompletionOutputView,
    DiagnosticKindView, DiagnosticView, DisplaySegmentView, LineColView, SignatureHelpView,
    SignatureItemView, Span, SpanView, TextEditView, TokenView,
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
        Span::decl(),
        SpanView::decl(),
        LineColView::decl(),
        DiagnosticKindView::decl(),
        DiagnosticView::decl(),
        TokenView::decl(),
        AnalyzeResult::decl(),
        TextEditView::decl(),
        DisplaySegmentView::decl(),
        SignatureItemView::decl(),
        SignatureHelpView::decl(),
        CompletionItemKind::decl(),
        CompletionItemView::decl(),
        CompletionOutputView::decl(),
    ] {
        let decl = export_decl(decl);
        out.push_str(&decl);
        if !decl.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
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
