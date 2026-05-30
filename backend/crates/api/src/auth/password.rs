//! argon2id password hashing.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Hash a plaintext password using argon2id with default parameters and a
/// fresh per-password salt. Returns the PHC string (`$argon2id$v=19$…`).
///
/// # Errors
/// Returns an error if the argon2 hashing routine fails.
//
// `argon2::password_hash::Error` does not implement `std::error::Error`, so it
// can't flow through `eyre`'s `wrap_err`/`?`; we map it to an `eyre::Report`
// via its `Display`.
pub(crate) fn hash(plain: &str) -> eyre::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    let hash = argon
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| eyre::eyre!("argon2 hash: {e}"))?
        .to_string();
    Ok(hash)
}

/// Verify a plaintext password against a stored PHC string.
/// Returns `Ok(())` on match, `Err` on mismatch or malformed hash.
///
/// # Errors
/// Returns an error if the hash is malformed or the password does not match.
pub(crate) fn verify(plain: &str, phc: &str) -> eyre::Result<()> {
    let parsed = PasswordHash::new(phc).map_err(|e| eyre::eyre!("parse PHC hash: {e}"))?;
    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .map_err(|e| eyre::eyre!("argon2 verify: {e}"))?;
    Ok(())
}
