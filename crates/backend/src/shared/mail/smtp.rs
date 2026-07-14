//! Отправка писем через SMTP (lettre, async).

use crate::shared::config::get_mail_config;
use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// Отправить письмо от лица ящика из `[mail]`.
///
/// `in_reply_to` — Message-ID исходного письма; если задан, проставляются
/// заголовки In-Reply-To/References для склейки треда.
pub async fn send_email(
    to: &str,
    subject: &str,
    body: &str,
    in_reply_to: Option<&str>,
) -> anyhow::Result<()> {
    let cfg = get_mail_config();
    cfg.validate_ready()?;

    // From: используем from_name + sender_address.
    let from_addr = cfg.sender_address();
    let from_mbox: Mailbox = if cfg.from_name.trim().is_empty() {
        from_addr
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid [mail].from_address '{from_addr}': {e}"))?
    } else {
        format!("{} <{}>", cfg.from_name.trim(), from_addr)
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid from mailbox: {e}"))?
    };

    let to_mbox: Mailbox = to
        .trim()
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid recipient '{to}': {e}"))?;

    let mut builder = Message::builder()
        .from(from_mbox)
        .to(to_mbox)
        .subject(subject);

    // Тредирование ответа: In-Reply-To + References по Message-ID исходного письма.
    if let Some(mid) = in_reply_to.map(str::trim).filter(|s| !s.is_empty()) {
        builder = builder
            .in_reply_to(mid.to_string())
            .references(mid.to_string());
    }

    let email = builder
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| anyhow::anyhow!("failed to build message: {e}"))?;

    let creds = Credentials::new(cfg.username.clone(), cfg.password.clone());

    // Порт 587 → STARTTLS, иначе implicit TLS (465).
    let builder = if cfg.smtp_port == 587 {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.smtp_host)
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.smtp_host)
    }
    .map_err(|e| anyhow::anyhow!("smtp relay setup failed: {e}"))?;

    let mailer = builder.port(cfg.smtp_port).credentials(creds).build();

    mailer
        .send(email)
        .await
        .map_err(|e| anyhow::anyhow!("smtp send failed: {e}"))?;

    Ok(())
}
