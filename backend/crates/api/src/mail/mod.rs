//! Outgoing mail (R-0060 / SPEC-0060 §2.3): one seam, a Cloudflare Email
//! Service implementation, and the doubles the tests and an unconfigured
//! deployment use.
//!
//! Shaped like `auth::google`: the trait is what handlers depend on, the
//! production implementation talks to the network, and `AppState` carries it
//! inside [`MailSettings`].

use std::{
    fmt,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Notify;

/// One message, already rendered.
#[derive(Debug, Clone)]
pub struct Message {
    /// Recipient address.
    pub to: String,
    /// Subject line.
    pub subject: String,
    /// Plain-text body.
    pub text: String,
    /// HTML body; the same content as `text`.
    pub html: String,
}

/// Why a send failed. No variant carries the recipient or the body, so logging
/// one leaks neither (R-0060 AC6).
#[derive(Debug, thiserror::Error)]
pub enum MailError {
    /// Network failure, timeout, 429 or 5xx — worth retrying later.
    #[error("mail transport error")]
    Transport,
    /// Any other 4xx — the provider refused the message.
    #[error("mail rejected by provider")]
    Rejected,
    /// No provider is configured in this deployment.
    #[error("mailer not configured")]
    NotConfigured,
}

/// Sends one message.
#[async_trait]
pub trait Mailer: Send + Sync {
    /// # Errors
    /// Returns a [`MailError`] when the message was not accepted for delivery.
    async fn send(&self, message: Message) -> Result<(), MailError>;
}

/// What `AppState` carries: the mailer and the web origin its links point at.
/// Bundled so a configured mailer never exists without its link origin
/// (SPEC-0060 architect finding 9).
#[derive(Clone)]
pub struct MailSettings {
    /// Where messages go.
    pub mailer: Arc<dyn Mailer>,
    /// Origin of the web app the emailed links open, without a trailing `/`.
    pub public_web_url: Arc<str>,
}

/// Sender address used when `MAIL_FROM` is unset (R-0060 OQ-3).
pub const DEFAULT_FROM_ADDRESS: &str = "no-reply@goosethropic.systems";
/// Sender display name used when `MAIL_FROM_NAME` is unset (R-0060 OQ-3).
pub const DEFAULT_FROM_NAME: &str = "Goose Fitness Physics";

/// The local Flutter web dev server — only ever the origin of links an
/// unconfigured mailer never sends.
const DEFAULT_PUBLIC_WEB_URL: &str = "http://localhost:9500";

impl Default for MailSettings {
    fn default() -> Self {
        Self {
            mailer: Arc::new(UnconfiguredMailer),
            public_web_url: Arc::from(DEFAULT_PUBLIC_WEB_URL),
        }
    }
}

impl MailSettings {
    /// Reads `CF_ACCOUNT_ID`, `CF_EMAIL_API_TOKEN`, `PUBLIC_WEB_URL`,
    /// `MAIL_FROM` (the sender address) and `MAIL_FROM_NAME`. Returns `None`
    /// unless the first three are all set and non-empty; the caller then falls
    /// back to [`MailSettings::default`].
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        let account_id = var("CF_ACCOUNT_ID")?;
        let token = var("CF_EMAIL_API_TOKEN")?;
        let public_web_url = var("PUBLIC_WEB_URL")?;
        let from = Sender {
            address: var("MAIL_FROM").unwrap_or_else(|| DEFAULT_FROM_ADDRESS.to_string()),
            name: var("MAIL_FROM_NAME").unwrap_or_else(|| DEFAULT_FROM_NAME.to_string()),
        };
        Some(Self {
            mailer: Arc::new(CloudflareMailer::new(&account_id, &token, from)),
            public_web_url: Arc::from(public_web_url.trim_end_matches('/')),
        })
    }
}

/// Cloudflare Email Service over its REST API.
///
/// `POST https://api.cloudflare.com/client/v4/accounts/{account_id}/email/sending/send`
/// with a bearer token and `{from, to, subject, text, html}`
/// (developers.cloudflare.com/email-service/api/send-emails/rest-api/).
pub struct CloudflareMailer {
    client: reqwest::Client,
    account_id: Arc<str>,
    token: Arc<str>,
    from: Sender,
}

/// A sender with a display name. The REST API takes it as
/// `{"address", "name"}`; the `"Name <address>"` string is the SMTP header
/// form, which the REST API rejects
/// (developers.cloudflare.com/email-service/examples/email-sending/recipients/).
#[derive(Debug, Clone)]
pub struct Sender {
    /// The verified address on the sending domain.
    pub address: String,
    /// What the recipient's mail client shows.
    pub name: String,
}

/// An unresponsive provider must not pin the send task forever (SPEC-0060
/// architect finding 7); `auth::google` uses 5 s for the same reason.
const SEND_TIMEOUT: Duration = Duration::from_secs(10);

impl CloudflareMailer {
    /// Builds no connection until the first send.
    #[must_use]
    pub fn new(account_id: &str, token: &str, from: Sender) -> Self {
        Self {
            client: reqwest::Client::new(),
            account_id: Arc::from(account_id),
            token: Arc::from(token),
            from,
        }
    }
}

/// The JSON body Cloudflare's send endpoint takes for one message.
fn request_body(from: &Sender, message: &Message) -> serde_json::Value {
    json!({
        "from": { "address": from.address, "name": from.name },
        "to": message.to,
        "subject": message.subject,
        "text": message.text,
        "html": message.html,
    })
}

/// Prints the account only: the token never reaches a log line.
impl fmt::Debug for CloudflareMailer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloudflareMailer")
            .field("account_id", &self.account_id)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Mailer for CloudflareMailer {
    async fn send(&self, message: Message) -> Result<(), MailError> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/email/sending/send",
            self.account_id
        );
        let body = request_body(&self.from, &message);
        let response = self
            .client
            .post(url)
            .bearer_auth(&*self.token)
            .timeout(SEND_TIMEOUT)
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|_| MailError::Transport)?;

        let status = response.status();
        if status.as_u16() == 429 || status.is_server_error() {
            return Err(MailError::Transport);
        }
        if !status.is_success() {
            return Err(MailError::Rejected);
        }
        // Cloudflare's envelope: a 2xx still says `success: false` when the
        // message was refused.
        let text = response.text().await.map_err(|_| MailError::Transport)?;
        let envelope: serde_json::Value =
            serde_json::from_str(&text).map_err(|_| MailError::Transport)?;
        if envelope["success"].as_bool() == Some(true) {
            Ok(())
        } else {
            Err(MailError::Rejected)
        }
    }
}

/// Every send fails with [`MailError::NotConfigured`]. What a deployment
/// without Email Service credentials gets, so the forgot endpoint still
/// answers 202 and the failure is logged (SPEC-0060 §2.3).
#[derive(Debug, Default)]
pub struct UnconfiguredMailer;

#[async_trait]
impl Mailer for UnconfiguredMailer {
    async fn send(&self, _message: Message) -> Result<(), MailError> {
        Err(MailError::NotConfigured)
    }
}

/// Test double: records every message and lets a test wait for the spawned
/// send to land instead of racing it (SPEC-0060 architect finding 2).
#[derive(Default)]
pub struct RecordingMailer {
    sent: Mutex<Vec<Message>>,
    notify: Notify,
}

impl RecordingMailer {
    /// A snapshot of what was recorded so far.
    #[must_use]
    pub fn sent(&self) -> Vec<Message> {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Resolves `true` once at least `n` messages are recorded, `false` if
    /// `timeout` passes first.
    pub async fn wait_for(&self, n: usize, timeout: Duration) -> bool {
        tokio::time::timeout(timeout, async {
            loop {
                // Register interest before checking, so a send between the
                // check and the await still wakes us.
                let notified = self.notify.notified();
                if self.count() >= n {
                    return;
                }
                notified.await;
            }
        })
        .await
        .is_ok()
    }

    fn count(&self) -> usize {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

#[async_trait]
impl Mailer for RecordingMailer {
    async fn send(&self, message: Message) -> Result<(), MailError> {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(message);
        self.notify.notify_waiters();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message() -> Message {
        Message {
            to: "a@b.com".into(),
            subject: "s".into(),
            text: "t".into(),
            html: "h".into(),
        }
    }

    #[tokio::test]
    async fn recording_mailer_wait_for_resolves_after_a_send() {
        let mailer = Arc::new(RecordingMailer::default());
        let sender = mailer.clone();
        tokio::spawn(async move { sender.send(message()).await });

        assert!(mailer.wait_for(1, Duration::from_secs(2)).await);
        assert_eq!(mailer.sent().len(), 1);
    }

    #[tokio::test]
    async fn recording_mailer_wait_for_times_out_when_nothing_is_sent() {
        let mailer = RecordingMailer::default();
        assert!(!mailer.wait_for(1, Duration::from_millis(20)).await);
    }

    #[tokio::test]
    async fn unconfigured_mailer_always_fails() {
        let result = UnconfiguredMailer.send(message()).await;
        assert!(matches!(result, Err(MailError::NotConfigured)));
    }

    fn sender() -> Sender {
        Sender {
            address: DEFAULT_FROM_ADDRESS.into(),
            name: DEFAULT_FROM_NAME.into(),
        }
    }

    /// Regression: the first live send used the SMTP `"Name <address>"`
    /// string for `from`, which the REST API does not take, so no reset
    /// email was delivered. The named sender is an object.
    #[test]
    fn request_body_sends_the_named_sender_as_an_object() {
        let body = request_body(&sender(), &message());

        assert_eq!(
            body["from"],
            json!({ "address": "no-reply@goosethropic.systems", "name": "Goose Fitness Physics" })
        );
        assert_eq!(body["to"], "a@b.com");
        assert_eq!(body["subject"], "s");
        assert_eq!(body["text"], "t");
        assert_eq!(body["html"], "h");
    }

    #[test]
    fn cloudflare_mailer_debug_never_prints_the_token() {
        let mailer = CloudflareMailer::new("acct-1", "secret-token-xyz", sender());
        let debug = format!("{mailer:?}");
        assert!(debug.contains("acct-1"));
        assert!(!debug.contains("secret-token-xyz"));
    }
}
