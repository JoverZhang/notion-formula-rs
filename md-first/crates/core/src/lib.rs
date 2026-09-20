//! Generate project-relative source files from marked Markdown code blocks.

mod config;
mod context;
mod dispatcher;
mod error;
mod markdown;
mod output;

pub use context::{BlockSource, Context, Preprocessor};
pub use dispatcher::Dispatcher;
pub use error::Error;
