use std::cell::RefCell;

use crate::markdown::SourceMap;

include!("preprocessor.h.rs");

struct ContextInner {
    maps: Vec<SourceMap>,
    errors: RefCell<Vec<String>>,
}

impl Context {
    pub(crate) fn new(
        project_root: PathBuf,
        out: PathBuf,
        sources: Vec<BlockSource>,
        maps: Vec<SourceMap>,
    ) -> Self {
        Self {
            project_root,
            out,
            sources,
            inner: ContextInner {
                maps,
                errors: RefCell::new(Vec::new()),
            },
        }
    }

    fn error_impl(&self, block_index: usize, span: Span, message: impl Into<String>) {
        let message = message.into();
        let diagnostic = match self.inner.maps.get(block_index) {
            Some(map) => map.diagnostic(span, message),
            None => format!(
                "{}: preprocessor used invalid block index {block_index}: {message}",
                self.out.display()
            ),
        };
        self.inner.errors.borrow_mut().push(diagnostic);
    }

    pub(crate) fn take_errors(&self) -> Vec<String> {
        std::mem::take(&mut self.inner.errors.borrow_mut())
    }
}
