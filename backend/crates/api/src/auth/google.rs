//! Google ID-token verification (R-0033).

use std::sync::Arc;

use async_trait::async_trait;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

/// Verified claims extracted from a Google ID token.
#[derive(Debug, Clone)]
pub struct GoogleClaims {
    pub email: String,
}

/// Verifies Google ID tokens — swappable for tests (R-0033 AC8).
#[async_trait]
pub trait GoogleIdTokenVerifier: Send + Sync {
    async fn verify(&self, id_token: &str, audience: &str) -> Result<GoogleClaims, ()>;
}

/// Production verifier: fetches Google's JWKS and validates RS256 tokens.
pub struct LiveGoogleVerifier {
    client: reqwest::Client,
}

impl Default for LiveGoogleVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveGoogleVerifier {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl GoogleIdTokenVerifier for LiveGoogleVerifier {
    async fn verify(&self, id_token: &str, audience: &str) -> Result<GoogleClaims, ()> {
        verify_with_jwks(&self.client, id_token, audience).await
    }
}

/// Static RSA public key verifier for integration tests.
pub struct StaticGoogleVerifier {
    decoding_key: DecodingKey,
}

impl StaticGoogleVerifier {
    /// Builds a verifier from an RSA public key PEM (integration tests only).
    ///
    /// # Errors
    /// Returns an error when `pem` is not a valid RSA public key.
    pub fn from_rsa_pem(pem: &[u8]) -> Result<Self, jsonwebtoken::errors::Error> {
        Ok(Self {
            decoding_key: DecodingKey::from_rsa_pem(pem)?,
        })
    }
}

#[async_trait]
impl GoogleIdTokenVerifier for StaticGoogleVerifier {
    async fn verify(&self, id_token: &str, audience: &str) -> Result<GoogleClaims, ()> {
        verify_with_key(id_token, audience, &self.decoding_key)
    }
}

/// Always fails — used when Google auth is not configured.
pub struct DisabledGoogleVerifier;

#[async_trait]
impl GoogleIdTokenVerifier for DisabledGoogleVerifier {
    async fn verify(&self, _id_token: &str, _audience: &str) -> Result<GoogleClaims, ()> {
        Err(())
    }
}

#[derive(Debug, Deserialize)]
struct GoogleJwtClaims {
    email: String,
    email_verified: Option<bool>,
    aud: String,
}

async fn verify_with_jwks(
    client: &reqwest::Client,
    id_token: &str,
    audience: &str,
) -> Result<GoogleClaims, ()> {
    let header = decode_header(id_token).map_err(|_| ())?;
    let kid = header.kid.ok_or(())?;
    let body = client
        .get("https://www.googleapis.com/oauth2/v3/certs")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|_| ())?
        .text()
        .await
        .map_err(|_| ())?;
    let jwks: serde_json::Value = serde_json::from_str(&body).map_err(|_| ())?;
    let keys = jwks["keys"].as_array().ok_or(())?;
    let key = keys
        .iter()
        .find(|k| k["kid"].as_str() == Some(&kid))
        .ok_or(())?;
    let n = key["n"].as_str().ok_or(())?;
    let e = key["e"].as_str().ok_or(())?;
    let decoding_key = DecodingKey::from_rsa_components(n, e).map_err(|_| ())?;
    verify_with_key(id_token, audience, &decoding_key)
}

fn verify_with_key(id_token: &str, audience: &str, key: &DecodingKey) -> Result<GoogleClaims, ()> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[audience]);
    validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);
    validation.validate_exp = true;
    let token = decode::<GoogleJwtClaims>(id_token, key, &validation).map_err(|_| ())?;
    if token.claims.email_verified == Some(false) {
        return Err(());
    }
    if token.claims.aud != audience {
        return Err(());
    }
    Ok(GoogleClaims {
        email: token.claims.email,
    })
}

#[derive(Debug, Deserialize)]
pub(crate) struct GoogleLoginRequest {
    pub id_token: String,
}

/// Google OAuth settings carried on [`AppState`].
#[derive(Clone)]
pub struct GoogleAuthSettings {
    pub audience: Option<Arc<str>>,
    pub verifier: Arc<dyn GoogleIdTokenVerifier>,
}

impl Default for GoogleAuthSettings {
    fn default() -> Self {
        Self {
            audience: None,
            verifier: Arc::new(DisabledGoogleVerifier),
        }
    }
}
