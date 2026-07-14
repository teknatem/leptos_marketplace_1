use super::repository;
use contracts::domain::a039_mail_message::aggregate::{direction, status, MailMessage};
use uuid::Uuid;

/// Обрезать строку до `max` символов (для темы/выжимки тела).
fn truncate(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        t.chars().take(max).collect::<String>() + "…"
    }
}

fn short() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_string()
}

fn subject_desc(subject: &str) -> String {
    let s = truncate(subject, 200);
    if s.is_empty() {
        "(без темы)".to_string()
    } else {
        s
    }
}

/// Зафиксировать входящее письмо (status=received) и вернуть агрегат.
#[allow(clippy::too_many_arguments)]
pub async fn record_inbound(
    uid: i64,
    message_id_hdr: Option<String>,
    from_addr: &str,
    to_addr: &str,
    subject: &str,
    body_excerpt: &str,
    user_ref: Option<String>,
) -> anyhow::Result<MailMessage> {
    let mut m = MailMessage::new_for_insert(
        format!("MAIL-IN-{uid}-{}", short()),
        subject_desc(subject),
        direction::INBOUND.to_string(),
        status::RECEIVED.to_string(),
    );
    m.imap_uid = Some(uid);
    m.message_id_hdr = message_id_hdr;
    m.from_addr = from_addr.to_string();
    m.to_addr = to_addr.to_string();
    m.subject = subject.to_string();
    m.body_excerpt = truncate(body_excerpt, 500);
    m.user_ref = user_ref;
    m.before_write();
    m.validate().map_err(|e| anyhow::anyhow!(e))?;
    repository::insert(&m).await?;
    Ok(m)
}

/// Зафиксировать исходящий ответ (status=replied) со ссылкой на входящее.
pub async fn record_outbound(
    to_addr: &str,
    subject: &str,
    body_excerpt: &str,
    user_ref: Option<String>,
    in_reply_to_ref: &str,
) -> anyhow::Result<MailMessage> {
    let mut m = MailMessage::new_for_insert(
        format!("MAIL-OUT-{}", short()),
        subject_desc(subject),
        direction::OUTBOUND.to_string(),
        status::REPLIED.to_string(),
    );
    m.to_addr = to_addr.to_string();
    m.subject = subject.to_string();
    m.body_excerpt = truncate(body_excerpt, 500);
    m.user_ref = user_ref;
    m.in_reply_to_ref = Some(in_reply_to_ref.to_string());
    m.before_write();
    m.validate().map_err(|e| anyhow::anyhow!(e))?;
    repository::insert(&m).await?;
    Ok(m)
}

/// Сохранить изменения записи (смена статуса, ссылки на чат/сообщение/артефакт).
pub async fn save(m: &mut MailMessage) -> anyhow::Result<()> {
    m.before_write();
    repository::update(m).await
}

pub async fn find_inbound_by_uid(uid: i64) -> anyhow::Result<Option<MailMessage>> {
    repository::find_inbound_by_uid(uid).await
}

/// Входящие, ожидающие отправки ответа регламентным заданием.
pub async fn list_pending_reply() -> anyhow::Result<Vec<MailMessage>> {
    repository::list_inbound_by_statuses(status::PENDING_REPLY).await
}

// ─── Read API (handler) ──────────────────────────────────────────────────────

pub async fn list_all() -> anyhow::Result<Vec<MailMessage>> {
    repository::list_all().await
}

pub async fn list_paginated(
    limit: u64,
    offset: u64,
    sort_by: &str,
    sort_desc: bool,
) -> anyhow::Result<(Vec<MailMessage>, u64)> {
    repository::list_paginated(limit, offset, sort_by, sort_desc).await
}

pub async fn get_by_id(id: &str) -> anyhow::Result<Option<MailMessage>> {
    repository::find_by_id(id).await
}

pub async fn delete(id: &str) -> anyhow::Result<()> {
    repository::soft_delete(id).await
}
