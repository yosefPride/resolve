// use bcrypt::{DEFAULT_COST, hash, verify};

// pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
//     hash(password, DEFAULT_COST)
// }

// pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
//     verify(password, hash)
// }

use bcrypt::{BcryptError, hash, verify};

// Explicitly define target cost.
// If server becomes slow, change back to DEFAULT_COST.
const WORK_FACTOR: u32 = 12;
// The work factor is an integer specifying the exponent 2^cost
// that determines how many iterations of the hashing algorithm must run,
// directly controlling the time and CPU power required to compute the password hash.
// 12 is the standard.

pub fn hash_password(password: &str) -> Result<String, BcryptError> {
    hash(password, WORK_FACTOR)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, BcryptError> {
    verify(password, hash)
}
