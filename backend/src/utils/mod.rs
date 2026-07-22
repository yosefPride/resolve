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

/// Levenshtein (edit) distance between two strings, for typo-tolerant matching
/// where an exact/substring match found nothing (docs/api.md, ticket title
/// search). Not Mongo-native, so callers run this in-process over an
/// already-fetched, already-scoped result set.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let len_b = b.len();

    let mut prev: Vec<usize> = (0..=len_b).collect();
    let mut curr = vec![0usize; len_b + 1];

    for (i, &ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[len_b]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_distance_identical_strings_is_zero() {
        assert_eq!(levenshtein_distance("login", "login"), 0);
    }

    #[test]
    fn levenshtein_distance_counts_single_substitution() {
        assert_eq!(levenshtein_distance("login", "logn"), 1);
    }

    #[test]
    fn levenshtein_distance_counts_insertions_and_deletions() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn levenshtein_distance_against_empty_string_is_length() {
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", ""), 3);
    }
}
