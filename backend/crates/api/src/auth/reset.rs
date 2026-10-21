//! Password-reset token helpers (SPEC-0060 §2.2) — the types are added in step 5; only the U1–U4 unit tests exist yet.

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
        bytes.iter().map(|b| format!("{b:02x}")).collect()
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
        assert_eq!(RESET_TOKEN_TTL, std::time::Duration::from_secs(30 * 60));
    }
}
