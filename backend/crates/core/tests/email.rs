//! Unit tests for the `fitai_core::Email` value-object — the single
//! email-normalization authority (SPEC-0002 §2.4, §3.5).
//!
//! Authored by the qa agent during R-0002 step 3 (test planning), BEFORE the
//! `fitai-core` crate exists. Pre-implementation red state = compile failure
//! (the crate / type are absent). Implementation step 5 makes these green.
//!
//! Coverage:
//!   - SAC2 (AC2): malformed email is rejected at the domain boundary, which
//!     is what lets the handler return 400 `validation`/`email`.
//!   - The normalization invariant (trim + lowercase) that guarantees the
//!     handler-write path and the login-lookup path can never disagree on the
//!     stored email (SPEC-0002 §2.4).

#![allow(clippy::unwrap_used, clippy::panic)]

use fitai_core::{Email, EmailParseError};

#[test]
fn parse_accepts_a_well_formed_address() {
    let email = Email::parse("alice@example.com").unwrap();
    assert_eq!(email.as_str(), "alice@example.com");
}

#[test]
fn parse_trims_surrounding_whitespace() {
    let email = Email::parse("  alice@example.com  ").unwrap();
    assert_eq!(email.as_str(), "alice@example.com");
}

#[test]
fn parse_lowercases_for_canonical_storage() {
    let email = Email::parse("Alice@Example.COM").unwrap();
    assert_eq!(
        email.as_str(),
        "alice@example.com",
        "Email must be the single normalization authority so the write path and \
         login-lookup path can never disagree on the stored value"
    );
}

#[test]
fn parse_is_idempotent() {
    let once = Email::parse("Alice@Example.com").unwrap();
    let twice = Email::parse(once.as_str()).unwrap();
    assert_eq!(once, twice);
}

#[test]
fn parse_rejects_missing_at_sign() {
    assert!(matches!(Email::parse("not-an-email"), Err(EmailParseError)));
}

#[test]
fn parse_rejects_empty_local_part() {
    assert!(matches!(Email::parse("@example.com"), Err(EmailParseError)));
}

#[test]
fn parse_rejects_empty_domain() {
    assert!(matches!(Email::parse("alice@"), Err(EmailParseError)));
}

#[test]
fn parse_rejects_domain_without_dot() {
    assert!(matches!(
        Email::parse("alice@localhost"),
        Err(EmailParseError)
    ));
}

#[test]
fn parse_rejects_empty_string() {
    assert!(matches!(Email::parse(""), Err(EmailParseError)));
}

#[test]
fn parse_rejects_whitespace_only() {
    assert!(matches!(Email::parse("   "), Err(EmailParseError)));
}
