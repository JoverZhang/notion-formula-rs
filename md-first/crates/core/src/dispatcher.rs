use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use crate::{Context, config, markdown, output};

include!("dispatcher.h.rs");

#[derive(Default)]
struct DispatcherInner {
    preprocessors: BTreeMap<String, Box<dyn Preprocessor>>,
}

impl Dispatcher {
    fn new_impl() -> Self {
        Self::default()
    }

    fn register_impl(
        &mut self,
        suffix: impl Into<String>,
        preprocessor: impl Preprocessor + 'static,
    ) -> Result<(), Error> {
        let suffix = suffix.into();
        if suffix.is_empty() {
            return Err(Error::new("preprocessor suffix must not be empty"));
        }
        if self.inner.preprocessors.contains_key(&suffix) {
            return Err(Error::new(format!(
                "duplicate preprocessor suffix: {suffix}"
            )));
        }
        self.inner
            .preprocessors
            .insert(suffix, Box::new(preprocessor));
        Ok(())
    }

    fn generate_impl(&self, project_root: impl AsRef<Path>) -> Result<Vec<PathBuf>, Error> {
        let (root, files) = self.prepare(project_root.as_ref())?;
        output::write(&root, &files)?;
        Ok(files.into_keys().collect())
    }

    fn check_impl(&self, project_root: impl AsRef<Path>) -> Result<(), Error> {
        let (root, files) = self.prepare(project_root.as_ref())?;
        output::check(&root, &files)
    }

    fn prepare(&self, root: &Path) -> Result<(PathBuf, BTreeMap<PathBuf, String>), Error> {
        let root = fs::canonicalize(root).map_err(|error| Error::io(root, error))?;
        let mut groups: BTreeMap<PathBuf, Vec<markdown::Block>> = BTreeMap::new();
        for document in config::sources(&root)? {
            let path = root.join(&document);
            let source: Arc<str> = fs::read_to_string(&path)
                .map_err(|error| Error::io(&path, error))?
                .into();
            for block in markdown::extract(&document, source)? {
                groups.entry(block.out.clone()).or_default().push(block);
            }
        }

        let mut files = BTreeMap::new();
        let mut errors = Vec::new();
        for (out, blocks) in groups {
            let name = out.to_string_lossy();
            let processor = self
                .inner
                .preprocessors
                .iter()
                .filter(|(suffix, _)| name.ends_with(suffix.as_str()))
                .max_by_key(|(suffix, _)| suffix.len());
            let Some((_, processor)) = processor else {
                let source = &blocks[0].source;
                errors.push(format!(
                    "{}:{}: no preprocessor registered for {}",
                    source.document.display(),
                    source.start_line,
                    out.display()
                ));
                continue;
            };
            let mut tokens = Vec::new();
            let mut sources = Vec::new();
            let mut maps = Vec::new();
            let mut invalid = false;
            for block in blocks {
                match block.code.parse() {
                    Ok(stream) => tokens.push(stream),
                    Err(error) => {
                        let error: proc_macro2::LexError = error;
                        errors.push(block.map.diagnostic(error.span(), error));
                        invalid = true;
                    }
                }
                sources.push(block.source);
                maps.push(block.map);
            }
            if invalid {
                continue;
            }
            let context = Context::new(root.clone(), out.clone(), sources, maps);
            let result = processor.preprocess(tokens, &context);
            let diagnostics = context.take_errors();
            if diagnostics.is_empty() {
                files.insert(out, output::render(result, &context.sources));
            } else {
                errors.extend(diagnostics);
            }
        }
        if errors.is_empty() {
            Ok((root, files))
        } else {
            Err(Error(errors))
        }
    }
}
