/// Decode string content while retaining invalid escapes for parser recovery.
/// The validity flag lets tokens expose only lexically valid values.
pub(crate) fn decode_string_content(s: &str) -> (String, bool) {
    let mut out = String::with_capacity(s.len());
    let mut valid = true;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    valid = false;
                    out.push('\\');
                    out.push(other);
                }
                None => {
                    valid = false;
                    out.push('\\');
                }
            }
        } else {
            valid &= c != '"';
            out.push(c);
        }
    }
    (out, valid)
}
