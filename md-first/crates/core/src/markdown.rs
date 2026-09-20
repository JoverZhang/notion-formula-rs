use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use proc_macro2::Span;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

use crate::{BlockSource, Error, config};

pub(crate) struct Block {
    pub out: PathBuf,
    pub source: BlockSource,
    pub code: String,
    pub map: SourceMap,
}

struct Segment {
    code: Range<usize>,
    original: Range<usize>,
}

pub(crate) struct SourceMap {
    document: PathBuf,
    original: Arc<str>,
    segments: Vec<Segment>,
    fallback: usize,
}

impl SourceMap {
    pub fn diagnostic(&self, span: Span, message: impl std::fmt::Display) -> String {
        let offset = span.byte_range().start;
        let original = self
            .segments
            .iter()
            .rev()
            .find(|segment| segment.code.start <= offset)
            .map_or(self.fallback, |segment| {
                segment.original.start + (offset - segment.code.start).min(segment.original.len())
            });
        located(&self.document, &self.original, original, message)
    }

    fn append(&mut self, code: &mut String, text: &str, range: Range<usize>) {
        let mut original_start = range.start;
        for part in text.split_inclusive('\n') {
            let remaining = &self.original[original_start..range.end];
            let original_end = original_start
                + remaining
                    .find('\n')
                    .map_or(remaining.len(), |index| index + 1);
            let raw = self.original[original_start..original_end].trim_end_matches(['\r', '\n']);
            let content = part.trim_end_matches('\n');
            // Container prefixes, CRLF and partially expanded indentation can
            // differ from the text events. Map their common suffix exactly.
            let suffix = raw
                .bytes()
                .rev()
                .zip(content.bytes().rev())
                .take_while(|(a, b)| a == b)
                .count();
            let generated_prefix = content.len() - suffix;
            if generated_prefix > 0 {
                self.segments.push(Segment {
                    code: code.len()..code.len() + generated_prefix,
                    original: original_start..original_start,
                });
            }
            self.segments.push(Segment {
                code: code.len() + generated_prefix..code.len() + part.len(),
                original: original_start + raw.len() - suffix..original_end,
            });
            code.push_str(part);
            original_start = original_end;
        }
    }
}

pub(crate) fn extract(path: &Path, source: Arc<str>) -> Result<Vec<Block>, Error> {
    let mut blocks = Vec::new();
    let mut active: Option<(Block, u8, usize, usize)> = None;
    for (event, range) in Parser::new(&source).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                let Some(out) = metadata(&info)
                    .map_err(|message| Error::new(located(path, &source, range.start, message)))?
                else {
                    continue;
                };
                let opening = source[range.start..].lines().next().unwrap_or_default();
                let fence_start = opening.find(['~', '`']).expect("parser found a fence");
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
                        out,
                        source: BlockSource {
                            document: path.to_owned(),
                            start_line: line_at(&source, content_start),
                        },
                        code: String::new(),
                        map: SourceMap {
                            document: path.to_owned(),
                            original: Arc::clone(&source),
                            segments: Vec::new(),
                            fallback: content_start,
                        },
                    },
                    fence,
                    width,
                    content_start,
                ));
            }
            Event::Text(text) => {
                if let Some((block, _, _, end)) = &mut active {
                    block.map.append(&mut block.code, &text, range.clone());
                    *end = range.end;
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((block, fence, width, end)) = active.take() {
                    let tail = source.get(end..range.end).unwrap_or_default();
                    let closing = tail.trim().trim_start_matches(['>', ' ', '\t']);
                    let count = closing.bytes().take_while(|&ch| ch == fence).count();
                    if count < width || !closing[count..].trim().is_empty() {
                        return Err(Error::new(located(
                            path,
                            &source,
                            end,
                            "out fence must be explicitly closed",
                        )));
                    }
                    blocks.push(block);
                }
            }
            _ => {}
        }
    }
    Ok(blocks)
}

fn metadata(info: &str) -> Result<Option<PathBuf>, &'static str> {
    let words: Vec<_> = info.split_whitespace().collect();
    if !words
        .iter()
        .any(|word| *word == "out" || word.starts_with("out="))
    {
        return Ok(None);
    }
    if words.len() != 2 || words[0].contains('=') {
        return Err("expected a language and out=<project-relative-path>");
    }
    let out = words[1]
        .strip_prefix("out=")
        .ok_or("expected out=<project-relative-path>")?;
    config::relative_path(out).map(Some)
}

fn line_at(source: &str, offset: usize) -> usize {
    source[..offset].bytes().filter(|&ch| ch == b'\n').count() + 1
}

fn located(path: &Path, source: &str, offset: usize, message: impl std::fmt::Display) -> String {
    let prefix = &source.as_bytes()[..offset.min(source.len())];
    let line = prefix.iter().filter(|&&ch| ch == b'\n').count() + 1;
    let column = prefix
        .iter()
        .rposition(|&ch| ch == b'\n')
        .map_or(prefix.len() + 1, |index| prefix.len() - index);
    format!("{}:{line}:{column}: {message}", path.display())
}
