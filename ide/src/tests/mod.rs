#[cfg(test)]
mod ide;

#[cfg(test)]
pub(crate) mod completion_dsl {
    pub(crate) use super::ide::completion_dsl::*;
}
