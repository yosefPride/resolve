use mongodb::bson::Regex;

/// Escapes regex metacharacters so user-supplied text matches literally when
/// embedded in a MongoDB `$regex` query. Without this, input like `a(b` would
/// be an invalid pattern (query error) or let a caller inject regex syntax.
pub fn escape_regex(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(
            ch,
            '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$'
        ) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

/// Builds a case-insensitive substring match for a literal `term`, for use as a
/// MongoDB field filter (e.g. `doc! { "name": substring_regex(term) }`). The
/// term is escaped, so it always matches literally rather than as a pattern.
pub fn substring_regex(term: &str) -> Regex {
    Regex {
        pattern: escape_regex(term),
        options: "i".to_string(),
    }
}
