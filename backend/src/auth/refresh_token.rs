use rand::Rng;
use sha2::{Digest, Sha256};

pub const REFRESH_TOKEN_TTL_DAYS: i64 = 30;
pub const REFRESH_TOKEN_COOKIE: &str = "refresh_token";

// 32 bytes of CSPRNG output (256 bits) — already high enough entropy that,
// unlike passwords, the token can be hashed with a fast general-purpose hash
// (SHA-256) rather than a deliberately slow one like bcrypt.
const TOKEN_BYTES: usize = 32;

/// Generates a new opaque refresh token. Returns `(raw, hash)`: `raw` is handed
/// to the client and never stored; `hash` is what gets persisted, so a leaked
/// database can't be used to mint sessions.
pub fn generate() -> (String, String) {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::rng().fill(&mut bytes);
    let raw = encode_hex(&bytes);
    let hash = hash_token(&raw);
    (raw, hash)
}

pub fn hash_token(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    encode_hex(&hasher.finalize())
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
