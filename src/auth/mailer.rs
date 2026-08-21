//! Outbound email for security notifications (LINKS-27).
//!
//! Minimal on purpose: one message type, built with `format!` because this
//! crate has no template engine. Delivery is gated on `MailConfig::configured`,
//! so an unconfigured deployment logs the message instead of sending it and a
//! login never depends on mail working.

use chrono::Utc;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::MailConfig;
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

fn smtp_transport(mail: &MailConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>, AppError> {
    let host = mail
        .smtp_host
        .clone()
        .ok_or_else(|| AppError::Configuration("SMTP host is missing".into()))?;
    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host);
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

    if !mail.configured() {
        tracing::warn!(
            delivery_mode = "log",
            email = %email,
            subject,
            "New sign-in location email NOT sent: delivery in log mode. Set SMTP_HOST and SMTP_FROM_EMAIL to enable SMTP."
        );
        tracing::info!(
            delivery_mode = "log",
            email = %email,
            body = %body,
            "New sign-in location email body (log mode)"
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
        .map_err(|error| {
            AppError::Internal(format!("New sign-in location email build failed: {error}"))
        })?;

    match smtp_transport(mail)?.send(message).await {
        Ok(_) => {
            tracing::info!(
                delivery_mode = "smtp",
                delivered = true,
                email = %email,
                "New sign-in location email delivered"
            );
            Ok(())
        }
        Err(error) => {
            tracing::error!(
                delivery_mode = "smtp",
                delivered = false,
                email = %email,
                error = %error,
                "New sign-in location email delivery failed"
            );
            Err(AppError::ExternalService(format!(
                "New sign-in location email delivery failed: {error}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
