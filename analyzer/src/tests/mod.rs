#[macro_use]
#[cfg(test)]
mod common;

#[cfg(test)]
mod analysis;
#[cfg(test)]
mod ide;
#[cfg(test)]
mod lexer;
#[cfg(test)]
mod parser;

#[cfg(test)]
pub(crate) mod completion_dsl {
    pub(crate) use super::ide::completion_dsl::*;
}
