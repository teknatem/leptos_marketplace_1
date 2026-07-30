//! Доступ к БД для тикетов (sys_ticket, sys_ticket_comment, sys_ticket_attachment).
//! Только CRUD, без бизнес-логики.

use contracts::system::tickets::{TicketAttachmentDto, TicketCommentDto, TicketDto};
use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement, Value};

use crate::shared::data::db::get_connection;

fn stmt(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Sqlite, sql, values)
}

fn ticket_from_row(row: &sea_orm::QueryResult) -> Result<TicketDto, DbErr> {
    let tags_raw: String = row.try_get("", "tags").unwrap_or_default();
    let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
    Ok(TicketDto {
        id: row.try_get("", "id")?,
        code: row.try_get("", "code")?,
        title: row.try_get("", "title")?,
        description: row.try_get("", "description")?,
        ticket_type: row.try_get::<String>("", "ticket_type")?.as_str().into(),
        status: row.try_get::<String>("", "status")?.as_str().into(),
        priority: row.try_get::<String>("", "priority")?.as_str().into(),
        deadline: row.try_get("", "deadline")?,
        author_user_id: row.try_get("", "author_user_id")?,
        author_username: row.try_get("", "author_username").unwrap_or(None),
        assignee_user_id: row.try_get("", "assignee_user_id")?,
        assignee_username: row.try_get("", "assignee_username").unwrap_or(None),
        tags,
        origin: row.try_get("", "origin")?,
        context_page_key: row.try_get("", "context_page_key")?,
        context_json: row.try_get("", "context_json")?,
        bitrix_task_id: row.try_get("", "bitrix_task_id")?,
        bitrix_synced_at: row.try_get("", "bitrix_synced_at")?,
        bitrix_received_at: row.try_get("", "bitrix_received_at").unwrap_or(None),
        created_at: row.try_get("", "created_at")?,
        updated_at: row.try_get("", "updated_at")?,
        comment_count: row.try_get("", "comment_count").unwrap_or(0),
        attachment_count: row.try_get("", "attachment_count").unwrap_or(0),
    })
}

const TICKET_SELECT: &str = "
    SELECT t.*,
           au.username AS author_username,
           su.username AS assignee_username,
           (SELECT COUNT(*) FROM sys_ticket_comment c
             WHERE c.ticket_id = t.id AND c.is_deleted = 0) AS comment_count,
           (SELECT COUNT(*) FROM sys_ticket_attachment a
             WHERE a.ticket_id = t.id) AS attachment_count
      FROM sys_ticket t
      LEFT JOIN sys_users au ON au.id = t.author_user_id
      LEFT JOIN sys_users su ON su.id = t.assignee_user_id
";

pub struct TicketListFilter {
    /// Ограничение по автору (для не-админов)
    pub author_user_id: Option<String>,
    pub status: Option<String>,
    pub ticket_type: Option<String>,
    /// Поиск по коду/заголовку
    pub search: Option<String>,
    pub limit: u64,
    pub offset: u64,
}

pub async fn list(filter: &TicketListFilter) -> Result<(Vec<TicketDto>, u64), DbErr> {
    let mut conditions = vec!["t.is_deleted = 0".to_string()];
    let mut values: Vec<Value> = Vec::new();

    if let Some(author) = &filter.author_user_id {
        conditions.push("t.author_user_id = ?".to_string());
        values.push(author.clone().into());
    }
    if let Some(status) = &filter.status {
        conditions.push("t.status = ?".to_string());
        values.push(status.clone().into());
    }
    if let Some(ticket_type) = &filter.ticket_type {
        conditions.push("t.ticket_type = ?".to_string());
        values.push(ticket_type.clone().into());
    }
    if let Some(search) = &filter.search {
        conditions.push("(t.code LIKE ? OR t.title LIKE ?)".to_string());
        let pattern = format!("%{}%", search);
        values.push(pattern.clone().into());
        values.push(pattern.into());
    }

    let where_clause = conditions.join(" AND ");

    let count_sql = format!(
        "SELECT COUNT(*) AS cnt FROM sys_ticket t WHERE {}",
        where_clause
    );
    let count_row = get_connection()
        .query_one(stmt(&count_sql, values.clone()))
        .await?;
    let total: i64 = count_row
        .and_then(|r| r.try_get::<i64>("", "cnt").ok())
        .unwrap_or(0);

    let list_sql = format!(
        "{} WHERE {} ORDER BY t.updated_at DESC LIMIT ? OFFSET ?",
        TICKET_SELECT, where_clause
    );
    values.push((filter.limit as i64).into());
    values.push((filter.offset as i64).into());

    let rows = get_connection().query_all(stmt(&list_sql, values)).await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in &rows {
        items.push(ticket_from_row(row)?);
    }
    Ok((items, total as u64))
}

pub async fn get_by_id(id: &str) -> Result<Option<TicketDto>, DbErr> {
    let sql = format!("{} WHERE t.id = ? AND t.is_deleted = 0", TICKET_SELECT);
    let row = get_connection()
        .query_one(stmt(&sql, vec![id.to_string().into()]))
        .await?;
    row.as_ref().map(ticket_from_row).transpose()
}

pub async fn list_for_bitrix_sync() -> Result<Vec<TicketDto>, DbErr> {
    let sql = format!(
        "{} WHERE t.is_deleted = 0 ORDER BY t.created_at ASC",
        TICKET_SELECT
    );
    let rows = get_connection().query_all(stmt(&sql, vec![])).await?;
    rows.iter().map(ticket_from_row).collect()
}

pub async fn mark_bitrix_sent(id: &str, task_id: &str, sent_at: &str) -> Result<(), DbErr> {
    get_connection()
        .execute(stmt(
            "UPDATE sys_ticket
                SET bitrix_task_id = ?, bitrix_synced_at = ?
              WHERE id = ?",
            vec![
                task_id.to_string().into(),
                sent_at.to_string().into(),
                id.to_string().into(),
            ],
        ))
        .await?;
    Ok(())
}

pub async fn apply_bitrix_status(id: &str, status: &str, received_at: &str) -> Result<bool, DbErr> {
    let result = get_connection()
        .execute(stmt(
            "UPDATE sys_ticket
                SET status = ?, bitrix_received_at = ?
              WHERE id = ? AND is_deleted = 0",
            vec![
                status.to_string().into(),
                received_at.to_string().into(),
                id.to_string().into(),
            ],
        ))
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Следующий порядковый номер для кода тикета "T-NNNNNN"
pub async fn next_code_number() -> Result<i64, DbErr> {
    let row = get_connection()
        .query_one(stmt(
            "SELECT COALESCE(MAX(CAST(SUBSTR(code, 3) AS INTEGER)), 0) + 1 AS n \
             FROM sys_ticket WHERE code LIKE 'T-%'",
            vec![],
        ))
        .await?;
    Ok(row
        .and_then(|r| r.try_get::<i64>("", "n").ok())
        .unwrap_or(1))
}

pub async fn insert(ticket: &TicketDto) -> Result<(), DbErr> {
    let tags = serde_json::to_string(&ticket.tags).unwrap_or_else(|_| "[]".to_string());
    get_connection()
        .execute(stmt(
            "INSERT INTO sys_ticket (
                id, code, title, description, ticket_type, status, priority,
                deadline, author_user_id, assignee_user_id, tags, origin,
                context_page_key, context_json, bitrix_task_id, bitrix_synced_at,
                sync_status, created_at, updated_at, is_deleted
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, ?, ?, 0)",
            vec![
                ticket.id.clone().into(),
                ticket.code.clone().into(),
                ticket.title.clone().into(),
                ticket.description.clone().into(),
                ticket.ticket_type.as_str().into(),
                ticket.status.as_str().into(),
                ticket.priority.as_str().into(),
                ticket.deadline.clone().into(),
                ticket.author_user_id.clone().into(),
                ticket.assignee_user_id.clone().into(),
                tags.into(),
                ticket.origin.clone().into(),
                ticket.context_page_key.clone().into(),
                ticket.context_json.clone().into(),
                ticket.created_at.clone().into(),
                ticket.updated_at.clone().into(),
            ],
        ))
        .await?;
    Ok(())
}

pub async fn update(ticket: &TicketDto) -> Result<(), DbErr> {
    let tags = serde_json::to_string(&ticket.tags).unwrap_or_else(|_| "[]".to_string());
    get_connection()
        .execute(stmt(
            "UPDATE sys_ticket SET
                title = ?, description = ?, ticket_type = ?, status = ?,
                priority = ?, deadline = ?, assignee_user_id = ?, tags = ?,
                updated_at = ?
             WHERE id = ?",
            vec![
                ticket.title.clone().into(),
                ticket.description.clone().into(),
                ticket.ticket_type.as_str().into(),
                ticket.status.as_str().into(),
                ticket.priority.as_str().into(),
                ticket.deadline.clone().into(),
                ticket.assignee_user_id.clone().into(),
                tags.into(),
                ticket.updated_at.clone().into(),
                ticket.id.clone().into(),
            ],
        ))
        .await?;
    Ok(())
}

pub async fn soft_delete(id: &str, deleted_at: &str) -> Result<(), DbErr> {
    get_connection()
        .execute(stmt(
            "UPDATE sys_ticket SET is_deleted = 1, updated_at = ? WHERE id = ?",
            vec![deleted_at.to_string().into(), id.to_string().into()],
        ))
        .await?;
    Ok(())
}

pub async fn touch(id: &str, updated_at: &str) -> Result<(), DbErr> {
    get_connection()
        .execute(stmt(
            "UPDATE sys_ticket SET updated_at = ? WHERE id = ?",
            vec![updated_at.to_string().into(), id.to_string().into()],
        ))
        .await?;
    Ok(())
}

// ============================================================================
// Комментарии
// ============================================================================

fn comment_from_row(row: &sea_orm::QueryResult) -> Result<TicketCommentDto, DbErr> {
    Ok(TicketCommentDto {
        id: row.try_get("", "id")?,
        ticket_id: row.try_get("", "ticket_id")?,
        author_user_id: row.try_get("", "author_user_id")?,
        author_username: row.try_get("", "author_username").unwrap_or(None),
        body: row.try_get("", "body")?,
        origin: row.try_get("", "origin")?,
        bitrix_comment_id: row.try_get("", "bitrix_comment_id").unwrap_or(None),
        created_at: row.try_get("", "created_at")?,
        attachments: Vec::new(),
    })
}

pub async fn list_comments(ticket_id: &str) -> Result<Vec<TicketCommentDto>, DbErr> {
    let rows = get_connection()
        .query_all(stmt(
            "SELECT c.*, COALESCE(u.username, c.external_author_name) AS author_username
               FROM sys_ticket_comment c
               LEFT JOIN sys_users u ON u.id = c.author_user_id
              WHERE c.ticket_id = ? AND c.is_deleted = 0
              ORDER BY c.created_at ASC",
            vec![ticket_id.to_string().into()],
        ))
        .await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in &rows {
        items.push(comment_from_row(row)?);
    }
    Ok(items)
}

pub async fn get_comment(id: &str) -> Result<Option<TicketCommentDto>, DbErr> {
    let row = get_connection()
        .query_one(stmt(
            "SELECT c.*, COALESCE(u.username, c.external_author_name) AS author_username
               FROM sys_ticket_comment c
               LEFT JOIN sys_users u ON u.id = c.author_user_id
              WHERE c.id = ? AND c.is_deleted = 0",
            vec![id.to_string().into()],
        ))
        .await?;
    row.as_ref().map(comment_from_row).transpose()
}

pub async fn insert_comment(comment: &TicketCommentDto) -> Result<(), DbErr> {
    get_connection()
        .execute(stmt(
            "INSERT INTO sys_ticket_comment (
                id, ticket_id, author_user_id, body, origin, bitrix_comment_id,
                created_at, is_deleted
             ) VALUES (?, ?, ?, ?, ?, ?, ?, 0)",
            vec![
                comment.id.clone().into(),
                comment.ticket_id.clone().into(),
                comment.author_user_id.clone().into(),
                comment.body.clone().into(),
                comment.origin.clone().into(),
                comment.bitrix_comment_id.clone().into(),
                comment.created_at.clone().into(),
            ],
        ))
        .await?;
    Ok(())
}

pub async fn list_comments_pending_bitrix(ticket_id: &str) -> Result<Vec<TicketCommentDto>, DbErr> {
    let rows = get_connection()
        .query_all(stmt(
            "SELECT c.*, COALESCE(u.username, c.external_author_name) AS author_username
               FROM sys_ticket_comment c
               LEFT JOIN sys_users u ON u.id = c.author_user_id
              WHERE c.ticket_id = ?
                AND c.is_deleted = 0
                AND c.origin <> 'bitrix'
                AND c.bitrix_comment_id IS NULL
              ORDER BY c.created_at ASC",
            vec![ticket_id.to_string().into()],
        ))
        .await?;
    rows.iter().map(comment_from_row).collect()
}

pub async fn mark_comment_bitrix_sent(id: &str, message_id: &str) -> Result<(), DbErr> {
    get_connection()
        .execute(stmt(
            "UPDATE sys_ticket_comment SET bitrix_comment_id = ? WHERE id = ?",
            vec![message_id.to_string().into(), id.to_string().into()],
        ))
        .await?;
    Ok(())
}

pub async fn last_bitrix_message_id(ticket_id: &str) -> Result<Option<i64>, DbErr> {
    let row = get_connection()
        .query_one(stmt(
            "SELECT MAX(CAST(bitrix_comment_id AS INTEGER)) AS last_id
               FROM sys_ticket_comment
              WHERE ticket_id = ? AND bitrix_comment_id IS NOT NULL",
            vec![ticket_id.to_string().into()],
        ))
        .await?;
    Ok(row.and_then(|r| r.try_get::<Option<i64>>("", "last_id").ok().flatten()))
}

pub async fn insert_bitrix_comment(
    ticket_id: &str,
    message_id: &str,
    author_id: &str,
    author_name: Option<&str>,
    body: &str,
    created_at: &str,
) -> Result<bool, DbErr> {
    let result = get_connection()
        .execute(stmt(
            "INSERT OR IGNORE INTO sys_ticket_comment (
                id, ticket_id, author_user_id, external_author_name, body,
                origin, bitrix_comment_id, created_at, is_deleted
             ) VALUES (?, ?, ?, ?, ?, 'bitrix', ?, ?, 0)",
            vec![
                uuid::Uuid::new_v4().to_string().into(),
                ticket_id.to_string().into(),
                format!("bitrix:{author_id}").into(),
                author_name.map(str::to_string).into(),
                body.to_string().into(),
                message_id.to_string().into(),
                created_at.to_string().into(),
            ],
        ))
        .await?;
    Ok(result.rows_affected() > 0)
}

// ============================================================================
// Вложения
// ============================================================================

fn attachment_from_row(row: &sea_orm::QueryResult) -> Result<TicketAttachmentDto, DbErr> {
    Ok(TicketAttachmentDto {
        id: row.try_get("", "id")?,
        ticket_id: row.try_get("", "ticket_id")?,
        comment_id: row.try_get("", "comment_id")?,
        s3_file_id: row.try_get("", "s3_file_id")?,
        filename: row.try_get("", "filename")?,
        content_type: row.try_get("", "content_type")?,
        file_size: row.try_get("", "file_size")?,
        created_at: row.try_get("", "created_at")?,
    })
}

pub async fn list_attachments(ticket_id: &str) -> Result<Vec<TicketAttachmentDto>, DbErr> {
    let rows = get_connection()
        .query_all(stmt(
            "SELECT * FROM sys_ticket_attachment WHERE ticket_id = ? ORDER BY created_at ASC",
            vec![ticket_id.to_string().into()],
        ))
        .await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in &rows {
        items.push(attachment_from_row(row)?);
    }
    Ok(items)
}

pub async fn get_attachment(id: &str) -> Result<Option<TicketAttachmentDto>, DbErr> {
    let row = get_connection()
        .query_one(stmt(
            "SELECT * FROM sys_ticket_attachment WHERE id = ?",
            vec![id.to_string().into()],
        ))
        .await?;
    row.as_ref().map(attachment_from_row).transpose()
}

pub async fn insert_attachment(attachment: &TicketAttachmentDto) -> Result<(), DbErr> {
    get_connection()
        .execute(stmt(
            "INSERT INTO sys_ticket_attachment (
                id, ticket_id, comment_id, s3_file_id, filename,
                content_type, file_size, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                attachment.id.clone().into(),
                attachment.ticket_id.clone().into(),
                attachment.comment_id.clone().into(),
                attachment.s3_file_id.clone().into(),
                attachment.filename.clone().into(),
                attachment.content_type.clone().into(),
                attachment.file_size.into(),
                attachment.created_at.clone().into(),
            ],
        ))
        .await?;
    Ok(())
}

pub async fn delete_attachment(id: &str) -> Result<(), DbErr> {
    get_connection()
        .execute(stmt(
            "DELETE FROM sys_ticket_attachment WHERE id = ?",
            vec![id.to_string().into()],
        ))
        .await?;
    Ok(())
}
