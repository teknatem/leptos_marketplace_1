//! Почтовая интеграция: приём (IMAP) и отправка (SMTP).
//!
//! Используется LLM-инструментами (`shared/llm/mail_tools.rs`) для чтения входящих
//! и отправки писем от лица ящика, настроенного в `config.toml` секции `[mail]`.
//!
//! IMAP-клиент (`imap` crate) синхронный — вызывается через `spawn_blocking`.
//! SMTP-клиент (`lettre`) асинхронный.

mod imap_client;
pub mod policy;
mod smtp;

use crate::shared::config::get_mail_config;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Краткая карточка письма для списка входящих (без тела).
#[derive(Debug, Clone, Serialize)]
pub struct EmailHeader {
    pub uid: u32,
    pub from: String,
    pub subject: String,
    pub date: String,
    pub seen: bool,
}

/// Полное письмо: заголовки + текст.
#[derive(Debug, Clone, Serialize)]
pub struct EmailFull {
    pub uid: u32,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub date: String,
    pub body: String,
    /// RFC Message-ID заголовка (для тредирования ответа и идемпотентности).
    pub message_id: Option<String>,
}

/// Список последних `limit` писем из INBOX (новые сверху).
pub async fn list_inbox(limit: usize) -> anyhow::Result<Vec<EmailHeader>> {
    tokio::task::spawn_blocking(move || imap_client::list_inbox(limit))
        .await
        .map_err(|e| anyhow::anyhow!("mail task join error: {e}"))?
}

/// Полное письмо по UID.
pub async fn read_email(uid: u32) -> anyhow::Result<EmailFull> {
    tokio::task::spawn_blocking(move || imap_client::read_email(uid))
        .await
        .map_err(|e| anyhow::anyhow!("mail task join error: {e}"))?
}

/// Отправить письмо. `to` — один адрес (можно "Имя <a@b>").
pub async fn send_email(to: &str, subject: &str, body: &str) -> anyhow::Result<()> {
    smtp::send_email(to, subject, body, None).await
}

/// Отправить письмо как ответ в треде (проставляет In-Reply-To/References по Message-ID).
pub async fn send_email_reply(
    to: &str,
    subject: &str,
    body: &str,
    in_reply_to: Option<&str>,
) -> anyhow::Result<()> {
    smtp::send_email(to, subject, body, in_reply_to).await
}

/// Извлечь «голый» email из строки вида "Имя <user@host>" или "user@host".
/// Возвращает нормализованный (trim + lowercase) адрес.
pub fn extract_addr(raw: &str) -> String {
    let s = raw.trim();
    // Часть внутри последних угловых скобок, если они есть.
    let inner = match (s.rfind('<'), s.rfind('>')) {
        (Some(lt), Some(gt)) if gt > lt + 1 => &s[lt + 1..gt],
        _ => s,
    };
    inner.trim().to_lowercase()
}

// ─── Rate-limit отправки ─────────────────────────────────────────────────────

/// Скользящее окно меток времени успешно инициированных отправок.
static SEND_LOG: Mutex<VecDeque<Instant>> = Mutex::new(VecDeque::new());

/// Проверить лимит отправки и, если не превышен, зафиксировать текущую отправку.
/// Лимит берётся из `[mail]`: не более `send_rate_limit` писем за
/// `send_rate_window_secs` секунд. `send_rate_limit == 0` отключает ограничение.
pub fn check_and_record_send() -> anyhow::Result<()> {
    let cfg = get_mail_config();
    let limit = cfg.send_rate_limit;
    if limit == 0 {
        return Ok(());
    }
    let window = Duration::from_secs(cfg.send_rate_window_secs.max(1));
    let now = Instant::now();

    let mut log = SEND_LOG
        .lock()
        .map_err(|_| anyhow::anyhow!("send rate-limit lock poisoned"))?;

    // Выкинуть метки старше окна (спереди — самые старые).
    while let Some(front) = log.front() {
        if now.duration_since(*front) >= window {
            log.pop_front();
        } else {
            break;
        }
    }

    if log.len() >= limit {
        let wait = window
            .checked_sub(now.duration_since(*log.front().unwrap()))
            .unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Превышен лимит отправки: {limit} писем за {} с. Попробуй снова через ~{} с.",
            window.as_secs(),
            wait.as_secs().max(1)
        ));
    }

    log.push_back(now);
    Ok(())
}
