//! LLM-инструменты работы с почтой: приём (IMAP) и отправка (SMTP).
//!
//! Реализация транспорта — в `crate::shared::mail`. Здесь только определения
//! инструментов для LLM и диспетчер `execute_mail_tool`.

use super::types::ToolDefinition;
use crate::shared::mail::extract_addr;
use serde_json::{json, Value};
use std::collections::HashSet;

/// Имена инструментов почтового навыка (для маршрутизации в `execute_tool_call`).
pub const MAIL_TOOL_NAMES: &[&str] = &["list_emails", "read_email", "send_email"];

/// Определения почтовых инструментов для передачи LLM.
pub fn mail_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_emails".into(),
            description: "Получить список последних писем во входящих (INBOX). \
                          Возвращает uid, отправителя, тему, дату и признак прочитанного — \
                          без тела. Показываются только письма от зарегистрированных \
                          пользователей системы (поле hidden — сколько прочих скрыто). \
                          Используй, чтобы найти нужное письмо, затем read_email(uid)."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Сколько последних писем вернуть (1–100, по умолчанию 20).",
                        "minimum": 1,
                        "maximum": 100
                    }
                }
            }),
        },
        ToolDefinition {
            name: "read_email".into(),
            description: "Получить полное письмо по uid (из list_emails): отправитель, \
                          получатель, тема, дата и текстовое тело."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "uid": {
                        "type": "integer",
                        "description": "UID письма из результата list_emails."
                    }
                },
                "required": ["uid"]
            }),
        },
        ToolDefinition {
            name: "send_email".into(),
            description: "Отправить письмо от лица почтового ящика системы. \
                          Письмо уходит немедленно — сформулируй тему и текст аккуратно. \
                          Отправка возможна ТОЛЬКО на email зарегистрированного активного \
                          пользователя системы; на прочие адреса — отказ. Действует лимит \
                          на число писем за интервал."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "to": {
                        "type": "string",
                        "description": "Адрес получателя, например 'user@example.com' или 'Имя <user@example.com>'."
                    },
                    "subject": { "type": "string", "description": "Тема письма." },
                    "body": { "type": "string", "description": "Текст письма (plain text)." }
                },
                "required": ["to", "subject", "body"]
            }),
        },
    ]
}

/// Загрузить email'ы активных пользователей (нормализованные) как множество.
/// Почта работает как закрытый канал: приём и отправка разрешены только на эти адреса.
async fn user_email_set() -> anyhow::Result<HashSet<String>> {
    let emails = crate::system::users::repository::list_active_emails().await?;
    Ok(emails.into_iter().collect())
}

/// Выполнить почтовый tool call. Возвращает JSON-результат.
pub async fn execute_mail_tool(name: &str, arguments_json: &str) -> Value {
    let args: Value = serde_json::from_str(arguments_json).unwrap_or_else(|_| json!({}));

    // Единый источник разрешённых адресов — email'ы активных пользователей.
    let users = match user_email_set().await {
        Ok(set) => set,
        Err(e) => {
            return json!({ "ok": false, "error": format!("Не удалось получить список пользователей: {e}") })
        }
    };

    match name {
        "list_emails" => {
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
            match crate::shared::mail::list_inbox(limit).await {
                Ok(items) => {
                    let total = items.len();
                    // Показываем только письма от зарегистрированных пользователей.
                    let visible: Vec<_> = items
                        .into_iter()
                        .filter(|h| users.contains(&extract_addr(&h.from)))
                        .collect();
                    let hidden = total - visible.len();
                    json!({
                        "ok": true,
                        "count": visible.len(),
                        "hidden": hidden,
                        "emails": visible,
                        "hint": if hidden > 0 {
                            "Скрыты письма не от зарегистрированных пользователей. Открой нужное через read_email(uid)."
                        } else {
                            "Открой письмо целиком через read_email(uid)."
                        }
                    })
                }
                Err(e) => {
                    json!({ "ok": false, "error": format!("Не удалось получить письма: {e}") })
                }
            }
        }
        "read_email" => {
            let uid = match args.get("uid").and_then(Value::as_u64) {
                Some(u) => u as u32,
                None => {
                    return json!({ "ok": false, "error": "Не указан обязательный параметр uid." })
                }
            };
            match crate::shared::mail::read_email(uid).await {
                Ok(email) => {
                    if !users.contains(&extract_addr(&email.from)) {
                        return json!({
                            "ok": false,
                            "error": format!(
                                "Письмо от '{}' не связано с зарегистрированным пользователем — чтение недоступно.",
                                email.from
                            )
                        });
                    }
                    json!({ "ok": true, "email": email })
                }
                Err(e) => {
                    json!({ "ok": false, "error": format!("Не удалось прочитать письмо: {e}") })
                }
            }
        }
        "send_email" => {
            let to = args.get("to").and_then(Value::as_str).unwrap_or("").trim();
            let subject = args.get("subject").and_then(Value::as_str).unwrap_or("");
            let body = args.get("body").and_then(Value::as_str).unwrap_or("");
            if to.is_empty() {
                return json!({ "ok": false, "error": "Не указан адрес получателя (to)." });
            }
            if subject.trim().is_empty() && body.trim().is_empty() {
                return json!({ "ok": false, "error": "Письмо пустое: нужны subject и/или body." });
            }
            // Получатель должен быть зарегистрированным пользователем.
            let to_addr = extract_addr(to);
            if !users.contains(&to_addr) {
                return json!({
                    "ok": false,
                    "error": format!(
                        "Адрес '{to_addr}' не принадлежит ни одному активному пользователю. \
                         Отправка возможна только зарегистрированным пользователям."
                    )
                });
            }
            // Rate-limit: не превышаем лимит отправки за окно.
            if let Err(e) = crate::shared::mail::check_and_record_send() {
                return json!({ "ok": false, "error": e.to_string() });
            }
            match crate::shared::mail::send_email(to, subject, body).await {
                Ok(()) => json!({
                    "ok": true,
                    "sent_to": to,
                    "recipient_email": to_addr,
                    "subject": subject,
                    "hint": "Письмо отправлено."
                }),
                Err(e) => {
                    json!({ "ok": false, "error": format!("Не удалось отправить письмо: {e}") })
                }
            }
        }
        other => {
            json!({ "ok": false, "error": format!("Неизвестный почтовый инструмент: {other}") })
        }
    }
}
