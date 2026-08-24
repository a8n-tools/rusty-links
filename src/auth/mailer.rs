//! Outbound email for security notifications (LINKS-27, LINKS-35).
//!
//! Minimal on purpose: const bodies interpolated with `replace`, because this
//! crate has no template engine. Delivery is gated on `MailConfig::configured`,
//! so an unconfigured deployment logs the message instead of sending it.
//!
//! The LINKS-27 alert is fire-and-forget: it must never fail a login. The
//! LINKS-35 approval mail is not: it runs on the login hot path and a delivery
//! failure propagates, so the sign-in fails closed instead of completing
//! ungated. That difference is the caller's, not this module's.

use chrono::Utc;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::AsyncSmtpTransportBuilder;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::auth::login_approval::Hold;
use crate::config::{MailConfig, SmtpTlsMode};
use crate::error::AppError;

/// Plain-text body of the new-sign-in-location alert.
const NEW_SIGNIN_LOCATION_BODY: &str = "\
New sign-in to your Rusty Links account

A sign-in to your Rusty Links account was detected from a country you have not used before.

Country: {country}
When: {timestamp}
IP address: {ip_address}
Device: {device}

If this was you, no action is needed.

If you do not recognise this sign-in, someone else may have access to your account. Sign in and change your Rusty Links password now, then review your active sessions.
";

/// Plain-text body of the sign-in approval request (LINKS-35).
const LOGIN_APPROVAL_BODY: &str = "\
Approve the sign-in to your Rusty Links account

A sign-in to your Rusty Links account came {reason}, so it is being held and no session was issued.

Country: {country}
When: {timestamp}
IP address: {ip_address}
Device: {device}

If this was you, approve it here and then sign in again:

{approval_url}

The link works once and expires in {expires_in_minutes} minutes.

If this was not you, do nothing. The sign-in never completes without your approval. Change your Rusty Links password anyway, because whoever tried it had it.
";

/// Select the lettre constructor for the configured TLS mode (LINKS-37).
///
/// `relay`/`starttls_relay` each set the default port for their mode, so the
/// caller's explicit `SMTP_PORT` stays an override applied afterwards.
fn transport_builder(mode: SmtpTlsMode, host: &str) -> Result<AsyncSmtpTransportBuilder, AppError> {
    match mode {
        SmtpTlsMode::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(host).map_err(|error| {
            AppError::Configuration(format!("SMTP TLS transport setup failed: {error}"))
        }),
        SmtpTlsMode::Starttls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .map_err(|error| {
                AppError::Configuration(format!("SMTP STARTTLS transport setup failed: {error}"))
            }),
        SmtpTlsMode::None => {
            tracing::warn!(
                smtp_host = %host,
                "SMTP_TLS=none: mail is sent over an UNENCRYPTED connection. Use this only for a trusted local relay."
            );
            Ok(AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(
                host,
            ))
        }
    }
}

fn smtp_transport(mail: &MailConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>, AppError> {
    let host = mail
        .smtp_host
        .clone()
        .ok_or_else(|| AppError::Configuration("SMTP host is missing".into()))?;
    let mut builder = transport_builder(mail.smtp_tls, &host)?;
    if let Some(port) = mail.smtp_port {
        builder = builder.port(port);
    }
    if let (Some(username), Some(password)) =
        (mail.smtp_username.clone(), mail.smtp_password.clone())
    {
        builder = builder.credentials(Credentials::new(username, password));
    }
    Ok(builder.build())
}

/// Send one security notification, or log it when SMTP is unconfigured.
///
/// `kind` names the message in the logs. Returns `Ok` without sending in log
/// mode, so a caller that must not fail on a missing mail server does not, and
/// the body (including any link it carries) still reaches the log.
async fn deliver(
    mail: &MailConfig,
    email: &str,
    kind: &str,
    subject: String,
    body: String,
) -> Result<(), AppError> {
    if !mail.configured() {
        tracing::warn!(
            delivery_mode = "log",
            email = %email,
            kind,
            subject,
            "Security email NOT sent: delivery in log mode. Set SMTP_HOST and SMTP_FROM_EMAIL to enable SMTP."
        );
        tracing::info!(
            delivery_mode = "log",
            email = %email,
            kind,
            body = %body,
            "Security email body (log mode)"
        );
        return Ok(());
    }

    let from_email = mail
        .smtp_from_email
        .clone()
        .ok_or_else(|| AppError::Configuration("SMTP sender email is missing".into()))?;
    let from_mailbox = Mailbox::new(
        mail.smtp_from_name.clone(),
        from_email.parse().map_err(|error| {
            AppError::Configuration(format!("Invalid SMTP from address: {error}"))
        })?,
    );
    let to_mailbox = email
        .parse()
        .map_err(|error| AppError::Internal(format!("Invalid recipient address: {error}")))?;
    let message = Message::builder()
        .from(from_mailbox)
        .to(Mailbox::new(None, to_mailbox))
        .subject(subject)
        .body(body)
        .map_err(|error| AppError::Internal(format!("{kind} email build failed: {error}")))?;

    match smtp_transport(mail)?.send(message).await {
        Ok(_) => {
            tracing::info!(
                delivery_mode = "smtp",
                delivered = true,
                email = %email,
                kind,
                "Security email delivered"
            );
            Ok(())
        }
        Err(error) => {
            tracing::error!(
                delivery_mode = "smtp",
                delivered = false,
                email = %email,
                kind,
                error = %error,
                "Security email delivery failed"
            );
            Err(AppError::ExternalService(format!(
                "{kind} email delivery failed: {error}"
            )))
        }
    }
}

/// Email the user that their account was signed in to from a new country.
///
/// Returns `Ok` without sending when SMTP is unconfigured (log mode), so the
/// caller cannot fail a login on a missing mail server.
pub async fn send_new_signin_location_email(
    mail: &MailConfig,
    email: &str,
    country: &str,
    ip: &str,
    device: Option<&str>,
) -> Result<(), AppError> {
    let subject = format!("New sign-in to your Rusty Links account from {country}");
    let body = NEW_SIGNIN_LOCATION_BODY
        .replace("{country}", country)
        .replace("{timestamp}", &Utc::now().to_rfc3339())
        .replace("{ip_address}", ip)
        .replace("{device}", device.unwrap_or("unknown"));

    deliver(mail, email, "New sign-in location", subject, body).await
}

/// Email the user the single-use link that approves a held sign-in (LINKS-35).
///
/// Unlike the alert above this runs on the login hot path, so a delivery
/// failure propagates and the sign-in fails closed rather than completing
/// ungated. In log mode nothing is sent, and the link is only in the log: a
/// deployment turning the gate on must configure SMTP first.
pub async fn send_login_approval_email(
    mail: &MailConfig,
    email: &str,
    hold: &Hold,
    ip: &str,
    device: Option<&str>,
    approval_url: &str,
    expires_in_minutes: i64,
) -> Result<(), AppError> {
    // A device-only hold in a deployment with no geoblock edge resolves no
    // country, so the subject names one only when there is one to name.
    let subject = match hold.country.as_deref() {
        Some(country) => format!("Approve the sign-in to your Rusty Links account from {country}"),
        None => "Approve the sign-in to your Rusty Links account".to_string(),
    };
    let body = LOGIN_APPROVAL_BODY
        .replace("{reason}", hold.reason.summary())
        .replace("{country}", hold.country.as_deref().unwrap_or("unknown"))
        .replace("{timestamp}", &Utc::now().to_rfc3339())
        .replace("{ip_address}", ip)
        .replace("{device}", device.unwrap_or("unknown"))
        .replace("{approval_url}", approval_url)
        .replace("{expires_in_minutes}", &expires_in_minutes.to_string());

    deliver(mail, email, "Sign-in approval", subject, body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::login_approval::HoldReason;

    // Log mode is the unconfigured default: no SMTP host or sender means the
    // alert is logged, never sent, and never errors a login.
    #[tokio::test]
    async fn unconfigured_mail_logs_instead_of_sending() {
        let mail = MailConfig::default();
        assert!(!mail.configured());
        let result =
            send_new_signin_location_email(&mail, "user@example.com", "DE", "203.0.113.7", None)
                .await;
        assert!(result.is_ok());
    }

    // Both a host and a sender are required before delivery is attempted.
    #[test]
    fn configured_requires_host_and_sender() {
        let mut mail = MailConfig {
            smtp_host: Some("smtp.example.com".into()),
            ..MailConfig::default()
        };
        assert!(!mail.configured());
        mail.smtp_from_email = Some("alerts@example.com".into());
        assert!(mail.configured());
    }

    // lettre exposes no getter for a transport's TLS setting, so the mode ->
    // constructor mapping is asserted through the builder's Debug output.
    fn builder_debug(mode: SmtpTlsMode) -> String {
        let builder = transport_builder(mode, "smtp.example.com").expect("builder should be built");
        format!("{builder:?}")
    }

    #[test]
    fn tls_mode_selects_implicit_tls_relay() {
        let debug = builder_debug(SmtpTlsMode::Tls);
        assert!(debug.contains("tls: Wrapper"), "{debug}");
        assert!(debug.contains("port: 465"), "{debug}");
    }

    #[test]
    fn starttls_mode_selects_starttls_relay() {
        let debug = builder_debug(SmtpTlsMode::Starttls);
        assert!(debug.contains("tls: Required"), "{debug}");
        assert!(debug.contains("port: 587"), "{debug}");
    }

    // The plaintext escape hatch stays reachable for a trusted local relay.
    #[test]
    fn none_mode_selects_plaintext_builder() {
        let debug = builder_debug(SmtpTlsMode::None);
        assert!(debug.contains("tls: None"), "{debug}");
        assert!(debug.contains("port: 25"), "{debug}");
    }

    // An unconfigured deployment gets an encrypted transport, not plaintext.
    #[test]
    fn default_mail_config_transport_is_encrypted() {
        let mail = MailConfig::default();
        assert_eq!(mail.smtp_tls, SmtpTlsMode::Starttls);
        assert!(!builder_debug(mail.smtp_tls).contains("tls: None"));
    }

    // SMTP_PORT stays an override on top of the port the mode supplies.
    #[test]
    fn explicit_port_overrides_the_mode_default() {
        let builder = transport_builder(SmtpTlsMode::Starttls, "smtp.example.com")
            .expect("builder should be built")
            .port(2525);
        let debug = format!("{builder:?}");
        assert!(debug.contains("port: 2525"), "{debug}");
        assert!(debug.contains("tls: Required"), "{debug}");
    }

    // The body carries the country, IP, and device the alert is about.
    #[test]
    fn body_interpolates_every_placeholder() {
        let body = NEW_SIGNIN_LOCATION_BODY
            .replace("{country}", "DE")
            .replace("{timestamp}", "2026-08-21T00:00:00Z")
            .replace("{ip_address}", "203.0.113.7")
            .replace("{device}", "Firefox");
        assert!(body.contains("Country: DE"));
        assert!(body.contains("IP address: 203.0.113.7"));
        assert!(body.contains("Device: Firefox"));
        assert!(!body.contains('{'));
    }

    // The approval mail carries the link and its lifetime, and leaves no
    // placeholder behind: a half-interpolated link is an unapprovable sign-in.
    // Every hold reason is checked, because a reason with no text of its own
    // would leave `{reason}` in the body (LINKS-45).
    #[test]
    fn approval_body_interpolates_every_placeholder() {
        for reason in [
            HoldReason::NewCountry,
            HoldReason::NewDevice,
            HoldReason::NewCountryAndDevice,
        ] {
            let body = LOGIN_APPROVAL_BODY
                .replace("{reason}", reason.summary())
                .replace("{country}", "DE")
                .replace("{timestamp}", "2026-08-21T00:00:00Z")
                .replace("{ip_address}", "203.0.113.7")
                .replace("{device}", "Firefox")
                .replace(
                    "{approval_url}",
                    "https://links.example.com/auth/approve-login?token=abc",
                )
                .replace("{expires_in_minutes}", "15");
            assert!(body.contains("Country: DE"));
            assert!(body.contains(reason.summary()), "{reason:?}");
            assert!(body.contains("https://links.example.com/auth/approve-login?token=abc"));
            assert!(body.contains("expires in 15 minutes"));
            assert!(!body.contains('{'), "{reason:?} left a placeholder behind");
        }
    }

    // A device-only hold in a deployment that resolves no country still reads
    // as a complete message: the country line says "unknown" rather than
    // leaving the placeholder or an empty field.
    #[test]
    fn approval_body_names_no_country_as_unknown() {
        let hold = Hold {
            reason: HoldReason::NewDevice,
            country: None,
        };
        let body = LOGIN_APPROVAL_BODY
            .replace("{reason}", hold.reason.summary())
            .replace("{country}", hold.country.as_deref().unwrap_or("unknown"));
        assert!(body.contains("Country: unknown"));
    }

    // Log mode never errors, so an unconfigured deployment sees the link in the
    // log rather than a failed request with nothing to show for it.
    #[tokio::test]
    async fn unconfigured_mail_logs_the_approval_link_instead_of_sending() {
        let result = send_login_approval_email(
            &MailConfig::default(),
            "user@example.com",
            &Hold {
                reason: HoldReason::NewCountry,
                country: Some("DE".to_string()),
            },
            "203.0.113.7",
            None,
            "https://links.example.com/auth/approve-login?token=abc",
            15,
        )
        .await;
        assert!(result.is_ok());

        // A device-only hold in a deployment that resolves no country still
        // sends: the subject simply names no country.
        let device_only = send_login_approval_email(
            &MailConfig::default(),
            "user@example.com",
            &Hold {
                reason: HoldReason::NewDevice,
                country: None,
            },
            "203.0.113.7",
            None,
            "https://links.example.com/auth/approve-login?token=abc",
            15,
        )
        .await;
        assert!(device_only.is_ok());
    }
}
