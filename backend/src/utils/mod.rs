use mongodb::bson::Regex;
use mongodb::bson::oid::ObjectId;

/// Result type shared by every repository whose only failure mode is "the
/// database broke". ApiError already maps `mongodb::error::Error` to Internal
/// (errors/api_error.rs), so those repositories need no error enum of their
/// own; repositories with domain-specific failures (duplicate email/member/
/// link) keep a small enum and map through [`is_duplicate_key`].
pub type RepoResult<T> = Result<T, mongodb::error::Error>;

/// Inserts `value` and returns the ObjectId Mongo assigned it, so callers can
/// hand back the stored document as `T { id: Some(id), ..value }`.
pub async fn insert_id<T: serde::Serialize + Send + Sync>(
    collection: &mongodb::Collection<T>,
    value: &T,
) -> RepoResult<ObjectId> {
    let result = collection.insert_one(value).await?;
    Ok(result
        .inserted_id
        .as_object_id()
        .expect("insert_one always returns an ObjectId"))
}

/// True when `err` is MongoDB's duplicate-key rejection (code 11000) from a
/// unique index. It surfaces as a WriteError on plain inserts but as a
/// CommandError on find_one_and_update (findAndModify is a command, not a
/// plain write), so both shapes are checked.
pub fn is_duplicate_key(err: &mongodb::error::Error) -> bool {
    use mongodb::error::{ErrorKind, WriteFailure};
    match err.kind.as_ref() {
        ErrorKind::Write(WriteFailure::WriteError(e)) => e.code == 11000,
        ErrorKind::Command(e) => e.code == 11000,
        _ => false,
    }
}

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
