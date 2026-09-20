//! Generate project-relative source files from marked Markdown code blocks.

mod config;
mod dispatcher;
mod error;
mod markdown;
mod output;
mod preprocessor;

pub use dispatcher::Dispatcher;
pub use error::Error;
pub use preprocessor::{BlockSource, Context, Preprocessor};
