use crate::converter::Converter;
use crate::converter::shared::{diagnostic_view, token_view};
use crate::dto::v1::AnalyzeResult;

impl Converter {
    pub fn analyze_output(source: &str, output: analyzer::AnalyzeResult) -> AnalyzeResult {
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
            output_type: output.output_type.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::converter::Converter;

    #[test]
    fn diagnostics_include_line_col_for_multiline_source() {
        let source = "1 +\n2 *";
        let ctx = analyzer::semantic::Context {
            properties: Vec::new(),
            functions: analyzer::semantic::builtins_functions(),
        };
        let output = analyzer::analyze(source, &ctx);

        let result = Converter::analyze_output(source, output);
        let diag = result
            .diagnostics
            .first()
            .expect("expected diagnostic for incomplete expression");

        assert_eq!(diag.span.start, 6);
        assert_eq!(diag.span.end, 7);
        assert_eq!(diag.line, 2);
        assert_eq!(diag.col, 3);
    }
}
