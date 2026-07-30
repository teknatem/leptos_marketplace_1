use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use contracts::system::tickets::{TicketDto, TicketStatus};
use reqwest::Client;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::time::Duration;

use crate::shared::config::Bitrix24Config;

const TYPE_TAG_PREFIX: &str = "mpi:type:";

#[derive(Debug)]
pub struct RemoteTask {
    pub status: u8,
    pub chat_id: Option<i64>,
}

#[derive(Debug)]
pub struct RemoteMessage {
    pub id: i64,
    pub author_id: i64,
    pub author_name: Option<String>,
    pub text: String,
    pub date: String,
}

pub struct BitrixClient {
    http: Client,
    config: Bitrix24Config,
}

impl BitrixClient {
    pub fn new(config: Bitrix24Config) -> Result<Self> {
        config.validate_ready()?;
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self { http, config })
    }

    fn method_url(&self, method: &str) -> String {
        format!(
            "{}/{}.json",
            self.config.webhook_url.trim().trim_end_matches('/'),
            method
        )
    }

    fn v3_method_url(&self, method: &str) -> Result<String> {
        let base = self.config.webhook_url.trim().trim_end_matches('/');
        let base = if base.contains("/rest/api/") {
            base.to_string()
        } else if base.contains("/rest/") {
            base.replacen("/rest/", "/rest/api/", 1)
        } else {
            return Err(anyhow!("Bitrix24 webhook URL does not contain /rest/"));
        };
        Ok(format!("{base}/{method}.json"))
    }

    async fn post(&self, url: String, payload: Value) -> Result<Value> {
        let response = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .context("Bitrix24 request failed")?;
        let status = response.status();
        let body: Value = response
            .json()
            .await
            .context("Bitrix24 returned invalid JSON")?;
        if !status.is_success() || body.get("error").is_some() {
            let message = body
                .pointer("/error_description")
                .and_then(Value::as_str)
                .or_else(|| body.pointer("/error/message").and_then(Value::as_str))
                .or_else(|| body.get("error").and_then(Value::as_str))
                .unwrap_or("unknown Bitrix24 API error");
            return Err(anyhow!("Bitrix24 API {}: {}", status, message));
        }
        Ok(body)
    }

    async fn legacy_call(&self, method: &str, payload: Value) -> Result<Value> {
        self.post(self.method_url(method), payload).await
    }

    pub async fn find_task_by_code(&self, code: &str) -> Result<Option<String>> {
        let mut start = 0i64;
        for _ in 0..20 {
            let body = self
                .legacy_call(
                    "tasks.task.list",
                    json!({
                        "order": {"ID": "DESC"},
                        "filter": {"GROUP_ID": self.config.group_id},
                        "select": ["ID", "TITLE"],
                        "start": start
                    }),
                )
                .await?;
            let tasks = body
                .pointer("/result/tasks")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow!("Bitrix24 task list has unexpected shape"))?;
            let prefix = format!("{code} —");
            if let Some(id) = tasks.iter().find_map(|task| {
                let title = string_field(task, "title", "TITLE")?;
                if title == code || title.starts_with(&prefix) {
                    string_field(task, "id", "ID")
                } else {
                    None
                }
            }) {
                return Ok(Some(id));
            }
            let Some(next) = body.get("next").and_then(Value::as_i64) else {
                break;
            };
            start = next;
        }
        Ok(None)
    }

    pub async fn create_task(&self, ticket: &TicketDto) -> Result<String> {
        let body = self
            .legacy_call(
                "tasks.task.add",
                json!({"fields": self.task_fields(ticket)}),
            )
            .await?;
        string_at_any(
            &body,
            &[
                "/result/task/id",
                "/result/task/ID",
                "/result/id",
                "/result",
            ],
        )
        .ok_or_else(|| anyhow!("Bitrix24 create response has no task ID"))
    }

    pub async fn update_task(&self, task_id: &str, ticket: &TicketDto) -> Result<()> {
        self.legacy_call(
            "tasks.task.update",
            json!({"taskId": task_id, "fields": self.task_fields(ticket)}),
        )
        .await?;
        Ok(())
    }

    pub async fn get_task(&self, task_id: &str) -> Result<RemoteTask> {
        let body = self
            .legacy_call(
                "tasks.task.get",
                json!({
                    "taskId": task_id,
                    "select": ["ID", "STATUS", "CHAT_ID"]
                }),
            )
            .await?;
        let task = body
            .pointer("/result/task")
            .ok_or_else(|| anyhow!("Bitrix24 task response has unexpected shape"))?;
        let status = string_field(task, "status", "STATUS")
            .and_then(|v| v.parse::<u8>().ok())
            .ok_or_else(|| anyhow!("Bitrix24 task response has no valid status"))?;
        let chat_id = string_field(task, "chatId", "CHAT_ID").and_then(|v| v.parse().ok());
        Ok(RemoteTask { status, chat_id })
    }

    pub async fn send_comment(&self, task_id: &str, text: &str) -> Result<String> {
        let task_id = task_id
            .parse::<i64>()
            .context("Bitrix24 task ID is not numeric")?;
        let body = self
            .post(
                self.v3_method_url("tasks.task.chat.message.send")?,
                json!({"fields": {"taskId": task_id, "text": text}}),
            )
            .await?;
        string_at_any(
            &body,
            &[
                "/result/item/id",
                "/result/item/messageId",
                "/result/messageId",
                "/result/id",
            ],
        )
        .ok_or_else(|| anyhow!("Bitrix24 comment response has no message ID"))
    }

    pub async fn get_messages(
        &self,
        chat_id: i64,
        after_id: Option<i64>,
    ) -> Result<Vec<RemoteMessage>> {
        let mut payload = Map::new();
        payload.insert("DIALOG_ID".into(), json!(format!("chat{chat_id}")));
        payload.insert("LIMIT".into(), json!(50));
        if let Some(id) = after_id {
            payload.insert("FIRST_ID".into(), json!(id));
        }
        let body = self
            .legacy_call("im.dialog.messages.get", Value::Object(payload))
            .await?;
        let result = body
            .get("result")
            .ok_or_else(|| anyhow!("Bitrix24 messages response has unexpected shape"))?;
        let names: HashMap<i64, String> = result
            .get("users")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|user| {
                Some((
                    user.get("id")?.as_i64()?,
                    user.get("name")?.as_str()?.to_string(),
                ))
            })
            .collect();
        let mut messages = Vec::new();
        for value in result
            .get("messages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let id = value.get("id").and_then(Value::as_i64).unwrap_or_default();
            let author_id = value
                .get("author_id")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let text = value
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            let date = value
                .get("date")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if id > 0 && author_id > 0 && !text.is_empty() && !date.is_empty() {
                messages.push(RemoteMessage {
                    id,
                    author_id,
                    author_name: names.get(&author_id).cloned(),
                    text,
                    date,
                });
            }
        }
        messages.sort_by_key(|message| message.id);
        Ok(messages)
    }

    fn task_fields(&self, ticket: &TicketDto) -> Value {
        json!({
            "TITLE": format!("{} — {}", ticket.code, ticket.title),
            "DESCRIPTION": ticket.description,
            "PRIORITY": ticket.priority.bitrix_priority(),
            "DEADLINE": ticket.deadline.as_deref().and_then(deadline_for_bitrix),
            "TAGS": tags_for_bitrix(ticket),
            "STATUS": ticket.status.bitrix_status(),
            "GROUP_ID": self.config.group_id,
            "RESPONSIBLE_ID": self.config.responsible_id
        })
    }
}

pub fn status_from_bitrix(value: u8, current: TicketStatus) -> TicketStatus {
    match value {
        2 => TicketStatus::New,
        3 => TicketStatus::InProgress,
        4 => TicketStatus::Resolved,
        5 if current == TicketStatus::Rejected => TicketStatus::Rejected,
        5 => TicketStatus::Closed,
        6 => TicketStatus::Waiting,
        _ => current,
    }
}

pub fn needs_push(updated_at: &str, synced_at: Option<&str>) -> bool {
    let Some(synced_at) = synced_at else {
        return true;
    };
    match (
        DateTime::parse_from_rfc3339(updated_at),
        DateTime::parse_from_rfc3339(synced_at),
    ) {
        (Ok(updated), Ok(synced)) => updated > synced,
        _ => updated_at > synced_at,
    }
}

fn tags_for_bitrix(ticket: &TicketDto) -> Vec<String> {
    let mut tags = ticket.tags.clone();
    tags.retain(|tag| !tag.starts_with(TYPE_TAG_PREFIX));
    tags.push(format!("{TYPE_TAG_PREFIX}{}", ticket.ticket_type.as_str()));
    tags
}

fn deadline_for_bitrix(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        return None;
    }
    let moscow = FixedOffset::east_opt(3 * 3600)?;
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Some(parsed.with_timezone(&moscow).to_rfc3339());
    }
    for format in ["%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(value, format) {
            return parsed
                .and_local_timezone(moscow)
                .single()
                .map(|date| date.to_rfc3339());
        }
    }
    Some(value.to_string())
}

fn string_field(value: &Value, camel: &str, upper: &str) -> Option<String> {
    value
        .get(camel)
        .or_else(|| value.get(upper))
        .and_then(value_to_string)
}

fn value_to_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_i64().map(|v| v.to_string()))
}

fn string_at_any(value: &Value, pointers: &[&str]) -> Option<String> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(value_to_string))
}

pub fn utc_now() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_statuses_without_losing_rejected() {
        assert_eq!(
            status_from_bitrix(3, TicketStatus::New),
            TicketStatus::InProgress
        );
        assert_eq!(
            status_from_bitrix(5, TicketStatus::Rejected),
            TicketStatus::Rejected
        );
        assert_eq!(
            status_from_bitrix(5, TicketStatus::InProgress),
            TicketStatus::Closed
        );
    }

    #[test]
    fn converts_local_deadline_to_moscow_offset() {
        assert_eq!(
            deadline_for_bitrix("2026-07-30T12:30").as_deref(),
            Some("2026-07-30T12:30:00+03:00")
        );
    }

    #[test]
    fn compares_rfc3339_instants() {
        assert!(needs_push(
            "2026-07-30T10:00:01Z",
            Some("2026-07-30T10:00:00Z")
        ));
        assert!(!needs_push(
            "2026-07-30T10:00:00Z",
            Some("2026-07-30T13:00:01+03:00")
        ));
    }
}
