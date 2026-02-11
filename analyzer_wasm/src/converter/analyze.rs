use analyzer::{Diagnostic, ParseOutput};

use crate::converter::shared::{diagnostic_view, token_view};
use crate::dto::v1::AnalyzeResult;

pub(crate) fn analyze_output(
    source: &str,
    output: ParseOutput,
    output_type: String,
) -> AnalyzeResult {
    let source_map = analyzer::SourceMap::new(source);

    let diagnostics = output
        .diagnostics
        .iter()
        .map(|d| diagnostic_view(source, &source_map, d))
        .collect();

    let tokens = output
        .tokens
        .iter()
        .filter(|t| !t.is_trivia())
        .map(|t| token_view(source, t))
        .collect();

    AnalyzeResult {
        diagnostics,
        tokens,
        output_type,
    }
}

pub(crate) fn analyze_error(source: &str, diag: &Diagnostic) -> AnalyzeResult {
    let source_map = analyzer::SourceMap::new(source);

    AnalyzeResult {
        diagnostics: vec![diagnostic_view(source, &source_map, diag)],
        tokens: Vec::new(),
        output_type: analyzer::semantic::Ty::Unknown.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::analyze_output;

    #[test]
    fn diagnostics_include_line_col_for_multiline_source() {
        let source = "1 +\n2 *";
        let output = analyzer::analyze(source).expect("expected ParseOutput");

        let result = analyze_output(source, output, "unknown".to_string());
        let diag = result
            .diagnostics
            .first()
            .expect("expected diagnostic for incomplete expression");

        assert_eq!(diag.span.range.start, 6);
        assert_eq!(diag.span.range.end, 7);
        assert_eq!(diag.line, 2);
        assert_eq!(diag.col, 3);
    }
}
