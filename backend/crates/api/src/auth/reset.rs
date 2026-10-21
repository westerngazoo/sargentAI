//! Password-reset token and email (R-0060 / SPEC-0060 §2.2, §2.6). Pure: no
//! database, no network, so everything here is unit-tested in place.

use std::{
    fmt::{self, Write as _},
    time::Duration,
};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

use crate::mail::Message;

/// How long an emailed link stays valid (R-0060 AC2).
pub(crate) const RESET_TOKEN_TTL: Duration = Duration::from_mins(30);

/// Random bytes behind a token: 256 bits (R-0060 AC2).
const TOKEN_BYTES: usize = 32;

/// The plaintext token. It exists only in memory and in the emailed link;
/// the database stores its [`ResetTokenHash`].
pub(crate) struct ResetToken(String);

/// SHA-256 of the token string, which is what `password_reset_tokens` stores
/// and looks up by.
pub(crate) struct ResetTokenHash(Vec<u8>);

impl ResetToken {
    /// 32 bytes from the OS CSPRNG, hex-encoded: 64 lowercase characters.
    pub(crate) fn generate() -> Self {
        let mut bytes = [0u8; TOKEN_BYTES];
        OsRng.fill_bytes(&mut bytes);
        Self(hex(&bytes))
    }

    /// Accepts exactly 64 lowercase hex characters, the only shape
    /// [`ResetToken::generate`] produces. Anything else is `None`, which the
    /// reset endpoint treats as an unknown token rather than a 400.
    pub(crate) fn parse(s: &str) -> Option<Self> {
        let well_formed =
            s.len() == TOKEN_BYTES * 2 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        well_formed.then(|| Self(s.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// SHA-256 over the token string's bytes, the exact string the link carries.
    pub(crate) fn hash(&self) -> ResetTokenHash {
        ResetTokenHash(Sha256::digest(self.0.as_bytes()).to_vec())
    }
}

/// Never prints the token, so no `tracing` field can leak it.
impl fmt::Debug for ResetToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ResetToken(..)")
    }
}

impl ResetTokenHash {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// SHA-256 of a rate-limit key such as `email:a@b.com` (SPEC-0060 §2.1), so
/// the ledger stores no address.
pub(crate) fn ledger_key(kind: &str, value: &str) -> Vec<u8> {
    Sha256::digest(format!("{kind}:{value}").as_bytes()).to_vec()
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            // Writing to a `String` cannot fail.
            let _ = write!(out, "{b:02x}");
            out
        })
}

const SUBJECT: &str = "Reset your Goose Fitness Physics password";

/// The reset email (SPEC-0060 §2.6). The token rides in the URL fragment,
/// which browsers never send to a server or in `Referer`.
pub(crate) fn message(public_web_url: &str, to: &str, token: &ResetToken) -> Message {
    let link = format!("{public_web_url}/#/reset-password?token={}", token.as_str());
    let text = format!(
        "Someone asked to reset the password for this account. If it was you, \
         open this link within 30 minutes:\n\n{link}\n\n\
         If it wasn't you, ignore this message; your password has not changed.\n"
    );
    let html = format!(
        "<p>Someone asked to reset the password for this account. If it was you, \
         open this link within 30 minutes:</p>\
         <p><a href=\"{link}\">{link}</a></p>\
         <p>If it wasn't you, ignore this message; your password has not changed.</p>"
    );
    Message {
        to: to.to_owned(),
        subject: SUBJECT.to_owned(),
        text,
        html,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed 64-hex token and the SHA-256 of its ASCII bytes, computed
    /// outside this crate (`printf '%s' <token> | shasum -a 256`). Pins the
    /// hashing contract T4 checks against the database: the digest is over
    /// the hex string the link carries, not over the 32 decoded bytes.
    const KNOWN_TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const KNOWN_TOKEN_SHA256: &str =
        "a8ae6e6ee929abea3afcfc5258c8ccd6f85273e0d4626d26c7279f3250f77c8e";

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
    }

    /// U1 → AC2: `generate()` yields 64 lowercase hex characters, and two
    /// calls differ.
    #[test]
    fn u1_generate_yields_64_lowercase_hex_and_is_random() {
        let first = ResetToken::generate();
        let second = ResetToken::generate();

        for token in [&first, &second] {
            assert_eq!(token.as_str().len(), 64);
            assert!(
                token
                    .as_str()
                    .bytes()
                    .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
                "token must be lowercase hex, got {}",
                token.as_str()
            );
        }
        assert_ne!(first.as_str(), second.as_str(), "two tokens must differ");
    }

    /// U2 → AC2, AC3: `parse` accepts exactly 64 lowercase hex characters
    /// and rejects 63, 65, uppercase, and non-hex.
    #[test]
    fn u2_parse_accepts_only_64_lowercase_hex() {
        let parsed = ResetToken::parse(KNOWN_TOKEN).expect("a well-formed token parses");
        assert_eq!(parsed.as_str(), KNOWN_TOKEN);

        assert!(ResetToken::parse(&KNOWN_TOKEN[..63]).is_none(), "63 chars");
        assert!(
            ResetToken::parse(&format!("{KNOWN_TOKEN}0")).is_none(),
            "65 chars"
        );
        assert!(
            ResetToken::parse(&KNOWN_TOKEN.to_uppercase()).is_none(),
            "uppercase"
        );
        let non_hex = format!("{}g", &KNOWN_TOKEN[..63]);
        assert!(ResetToken::parse(&non_hex).is_none(), "non-hex");
        assert!(ResetToken::parse("").is_none(), "empty");
    }

    /// U3 → AC2: `hash` is stable across calls and equals the independently
    /// computed SHA-256 of the token's bytes.
    #[test]
    fn u3_hash_is_sha256_of_the_token_bytes_and_stable() {
        let token = ResetToken::parse(KNOWN_TOKEN).unwrap();

        let first = token.hash();
        let second = token.hash();

        assert_eq!(first.as_bytes(), second.as_bytes(), "hash must be stable");
        assert_eq!(first.as_bytes().len(), 32, "SHA-256 is 32 bytes");
        assert_eq!(hex(first.as_bytes()), KNOWN_TOKEN_SHA256);
    }

    /// U4 → AC2: the `Debug` output never contains the token.
    #[test]
    fn u4_debug_output_does_not_contain_the_token() {
        let token = ResetToken::parse(KNOWN_TOKEN).unwrap();

        let debug = format!("{token:?}");

        assert!(
            !debug.contains(KNOWN_TOKEN),
            "Debug must not print the token, got {debug}"
        );
        assert!(
            !debug.contains(&KNOWN_TOKEN[..16]),
            "Debug must not print a prefix of the token either, got {debug}"
        );
        assert_eq!(debug, "ResetToken(..)");
    }

    /// AC2: the TTL constant is the 30 minutes R-0060 fixes.
    #[test]
    fn reset_token_ttl_is_thirty_minutes() {
        assert_eq!(RESET_TOKEN_TTL, std::time::Duration::from_mins(30));
    }
}
