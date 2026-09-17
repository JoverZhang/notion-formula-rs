use std::path::{Path, PathBuf};

use proc_macro2::Span;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

use crate::Error;

pub(crate) struct Block {
    pub key: String,
    pub path: PathBuf,
    pub line: usize,
    pub code: String,
    lines: Vec<usize>,
}

impl Block {
    pub fn source_line(&self, span: Span) -> usize {
        self.lines
            .get(span.start().line.saturating_sub(1))
            .copied()
            .unwrap_or(self.line)
    }

    pub fn error(&self, line: usize, message: impl std::fmt::Display) -> Error {
        Error::new(format!("{}:{line}: {message}", self.path.display()))
    }

    pub fn syntax_error(&self, error: syn::Error) -> Error {
        self.error(self.source_line(error.span()), error)
    }

    pub fn provenance(&self, span: Span) -> String {
        // Absolute input paths must not leak machine-specific prefixes into output.
        let name = if self.path.is_absolute() {
            self.path.file_name().unwrap_or_default().to_string_lossy()
        } else {
            self.path.to_string_lossy()
        };
        format!(
            "{}:{}",
            name.replace(['\n', '\r'], " "),
            self.source_line(span)
        )
    }
}

pub(crate) fn valid_key(key: &str) -> bool {
    key.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && key.split('_').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
        })
}

fn metadata(info: &str) -> Result<Option<&str>, &'static str> {
    let words: Vec<_> = info.split_whitespace().collect();
    if !words
        .iter()
        .any(|word| *word == "header" || word.starts_with("header="))
    {
        return Ok(None);
    }
    if words.first() != Some(&"rust") {
        return Err("a header block must use rust");
    }
    if words.len() != 2 {
        return Err("expected exactly one fence attribute: header=<snake_case_name>");
    }
    let key = words[1]
        .strip_prefix("header=")
        .filter(|key| valid_key(key))
        .ok_or("header must be a snake_case name, not a path or extension")?;
    Ok(Some(key))
}

pub(crate) fn extract(path: &Path, source: &str) -> Result<Vec<Block>, Error> {
    let mut blocks = Vec::new();
    let mut active: Option<(Block, u8, usize, usize)> = None;
    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                let line = line_at(source, range.start);
                let key = metadata(&info).map_err(|message| {
                    Error::new(format!("{}:{line}: {message}", path.display()))
                })?;
                if let Some(key) = key {
                    let opening = source[range.start..].lines().next().unwrap_or_default();
                    let fence_start = opening.find(['~', '\x60']).expect("parser found a fence");
                    let fence = opening.as_bytes()[fence_start];
                    let width = opening.as_bytes()[fence_start..]
                        .iter()
                        .take_while(|&&ch| ch == fence)
                        .count();
                    let content_start = source[range.start..]
                        .find('\n')
                        .map_or(source.len(), |offset| range.start + offset + 1);
                    active = Some((
                        Block {
                            key: key.to_owned(),
                            path: path.to_owned(),
                            line,
                            code: String::new(),
                            lines: Vec::new(),
                        },
                        fence,
                        width,
                        content_start,
                    ));
                }
            }
            Event::Text(text) => {
                if let Some((block, _, _, last_content_end)) = &mut active {
                    let first_line = line_at(source, range.start);
                    for (offset, part) in text.split_inclusive('\n').enumerate() {
                        if block.code.is_empty() || block.code.ends_with('\n') {
                            block.lines.push(first_line + offset);
                        }
                        block.code.push_str(part);
                    }
                    *last_content_end = range.end;
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((block, fence, width, content_end)) = active.take() {
                    let tail = source.get(content_end..range.end).unwrap_or_default();
                    // Text events already consumed any fence-like code. Only the
                    // parser's closing tail may satisfy the explicit-close rule.
                    let closing = tail.trim().trim_start_matches(['>', ' ', '\t']);
                    let count = closing.bytes().take_while(|&ch| ch == fence).count();
                    if count < width || !closing[count..].trim().is_empty() {
                        return Err(
                            block.error(block.line, "header fence must be explicitly closed")
                        );
                    }
                    blocks.push(block);
                }
            }
            _ => {}
        }
    }
    Ok(blocks)
}

fn line_at(source: &str, offset: usize) -> usize {
    source[..offset].bytes().filter(|&ch| ch == b'\n').count() + 1
}
