use super::FunctionSig;

#[macro_use]
mod macros;

mod date;
mod general;
mod list;
mod math;
mod people;
mod special;
mod text;

// Category order is intentionally deterministic, matching the historical order in
// `analysis/functions.rs`: General, Text, Number, Date, People, List, Special.
pub fn builtins_functions() -> Vec<FunctionSig> {
    let mut out = Vec::new();
    out.extend(general::builtins());
    out.extend(text::builtins());
    out.extend(math::builtins());
    out.extend(date::builtins());
    out.extend(people::builtins());
    out.extend(list::builtins());
    out.extend(special::builtins());
    out
}
