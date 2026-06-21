//! Сборка «контекста страницы» для LLM-чата.
//!
//! По ключу вкладки (`page_key`) определяет тип страницы и собирает:
//! - универсальную часть (ссылка, тип, идентичность, описание сущности);
//! - для страниц агрегатов — строку объекта (generic SELECT по имени таблицы из
//!   метаданных) и лёгкие смежные подписи по внешним ключам (FK);
//! - для прочих страниц — общее описание.
//!
//! Возвращает `BuiltContext` с полным JSON (для хранения) и компактным текстом
//! (для инъекции в диалог).

use sea_orm::{DatabaseBackend, FromQueryResult, Statement};
use serde_json::{json, Value};

use crate::shared::llm::METADATA_REGISTRY;
use crate::shared::representation;

/// Колонки, которые не несут смысла для LLM-контекста.
const SKIP_COLUMNS: &[&str] = &[
    "is_deleted",
    "is_posted",
    "version",
    "created_at",
    "updated_at",
];

/// Результат сборки контекста.
pub struct BuiltContext {
    pub title: String,
    pub page_type: String,
    pub entity_index: Option<String>,
    pub entity_id: Option<String>,
    pub context_json: Value,
    pub rendered_text: String,
}

struct PageRef {
    page_type: &'static str,
    /// Полный «kind» для representation (например `a012_wb_sales`).
    kind: Option<String>,
    /// Короткий индекс сущности (например `a012`).
    entity_index: Option<String>,
    entity_id: Option<String>,
}

fn first_segment(s: &str) -> &str {
    s.split('_').next().unwrap_or(s)
}

/// Похоже на индекс объекта: буква (a/p/d/u) + цифры (a012, p903, d400, u501).
fn looks_like_index(seg: &str) -> bool {
    let mut chars = seg.chars();
    matches!(chars.next(), Some('a') | Some('p') | Some('d') | Some('u'))
        && seg.len() >= 2
        && seg[1..].chars().all(|c| c.is_ascii_digit())
}

/// Разобрать ключ вкладки в ссылку на страницу. Зеркалит правила tabs/registry.rs.
fn parse_page_key(key: &str) -> PageRef {
    // Дрилдаун-сессии
    if key.starts_with("drilldown__") || key.starts_with("gl_drilldown__") {
        let session = key.splitn(2, "__").nth(1).unwrap_or("").to_string();
        return PageRef {
            page_type: "drilldown",
            kind: None,
            entity_index: None,
            entity_id: Some(session),
        };
    }

    // Детальные страницы: <kind>_details_<id> (иногда _details_id_<id>)
    if let Some(idx) = key.find("_details_") {
        let kind = key[..idx].to_string();
        let mut rest = &key[idx + "_details_".len()..];
        if let Some(stripped) = rest.strip_prefix("id_") {
            rest = stripped;
        }
        let seg0 = first_segment(&kind).to_string();
        let page_type = if seg0.starts_with('p') { "report" } else { "aggregate" };
        return PageRef {
            page_type,
            kind: Some(kind),
            entity_index: Some(seg0),
            entity_id: Some(rest.to_string()),
        };
    }

    // Дашборды d4XX
    let seg0 = first_segment(key);
    if seg0.starts_with('d') && looks_like_index(seg0) {
        return PageRef {
            page_type: "dashboard",
            kind: Some(key.to_string()),
            entity_index: Some(seg0.to_string()),
            entity_id: None,
        };
    }

    // Списки агрегатов/проекций (a0XX / p9XX без _details)
    if (seg0.starts_with('a') || seg0.starts_with('p')) && looks_like_index(seg0) {
        return PageRef {
            page_type: "aggregate_list",
            kind: Some(key.to_string()),
            entity_index: Some(seg0.to_string()),
            entity_id: None,
        };
    }

    PageRef {
        page_type: "other",
        kind: None,
        entity_index: None,
        entity_id: None,
    }
}

/// Прочитать одну строку объекта (generic SELECT) как JSON по имени таблицы и id.
async fn fetch_object_row(table: &str, id: &str) -> Option<Value> {
    // Имя таблицы из метаданных (доверенное), но проверим на всякий случай.
    if table.is_empty()
        || !table
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    let db = crate::shared::data::db::get_connection();
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        format!("SELECT * FROM {} WHERE id = ? LIMIT 1", table),
        [sea_orm::Value::from(id.to_string())],
    );
    Value::find_by_statement(stmt)
        .all(db)
        .await
        .ok()
        .and_then(|mut rows| rows.drain(..).next())
}

/// Обрезать значение поля до разумной длины (без разрыва UTF-8).
fn short_value(v: &Value) -> Option<String> {
    let s = match v {
        Value::Null => return None,
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let truncated: String = s.chars().take(200).collect();
    Some(truncated)
}

/// Собрать пакет контекста по ключу страницы.
pub async fn build_for_page_key(page_key: &str, label: Option<&str>) -> BuiltContext {
    let pr = parse_page_key(page_key);
    let deep_link = format!("?active={}", page_key);
    let label_title = label
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty());

    let mut ctx = json!({
        "page_key": page_key,
        "deep_link": deep_link,
        "page_type": pr.page_type,
    });
    if let Some(ix) = &pr.entity_index {
        ctx["entity_index"] = json!(ix);
    }
    if let Some(id) = &pr.entity_id {
        ctx["entity_id"] = json!(id);
    }

    // Описание сущности и список FK-полей из метаданных.
    let mut entity_name: Option<String> = None;
    let mut entity_desc: Option<String> = None;
    let mut table_name: Option<String> = None;
    let mut fk_fields: Vec<(String, String)> = Vec::new(); // (column, fk_target)
    if let Some(ix) = &pr.entity_index {
        let schema = METADATA_REGISTRY.get_entity_schema(ix);
        if schema.get("error").is_none() {
            entity_name = schema
                .get("name")
                .and_then(|v| v.as_str())
                .map(String::from);
            entity_desc = schema
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from);
            table_name = schema
                .get("table")
                .and_then(|v| v.as_str())
                .map(String::from);
            if let Some(fields) = schema.get("fields").and_then(|v| v.as_array()) {
                for f in fields {
                    if let (Some(col), Some(fk)) = (
                        f.get("column").and_then(|v| v.as_str()),
                        f.get("fk_to").and_then(|v| v.as_str()),
                    ) {
                        fk_fields.push((col.to_string(), fk.to_string()));
                    }
                }
            }
            ctx["entity"] = json!({
                "name": entity_name,
                "description": entity_desc,
                "table": table_name,
            });
        }
    }

    // Идентичность через representation (для детальных страниц).
    let mut identity_label: Option<String> = None;
    if let (Some(kind), Some(id)) = (&pr.kind, &pr.entity_id) {
        if let Some(rep) = representation::resolve(kind, id).await {
            let lbl = representation::to_label(&rep);
            identity_label = Some(lbl.clone());
            ctx["identity"] = json!({
                "title": rep.title,
                "date": rep.date,
                "doc_id": rep.doc_id,
                "label": lbl,
            });
        }
    }

    // Данные объекта (generic SELECT) для детальных страниц.
    let mut object_row: Option<Value> = None;
    if let (Some(table), Some(id)) = (&table_name, &pr.entity_id) {
        object_row = fetch_object_row(table, id).await;
        if let Some(row) = &object_row {
            ctx["object"] = row.clone();
        }
    }

    // Смежные подписи по FK.
    let mut adjacent: Vec<Value> = Vec::new();
    if let Some(row) = &object_row {
        for (col, fk_target) in fk_fields.iter().take(8) {
            let Some(val) = row.get(col).and_then(short_value) else {
                continue;
            };
            // Представление ссылки: kind = имя таблицы FK-цели из метаданных.
            let fk_index = first_segment(fk_target).to_string();
            let ref_kind = {
                let s = METADATA_REGISTRY.get_entity_schema(&fk_index);
                s.get("table").and_then(|v| v.as_str()).map(String::from)
            };
            let label = match ref_kind {
                Some(rk) => representation::resolve(&rk, &val)
                    .await
                    .map(|r| representation::to_label(&r)),
                None => None,
            };
            adjacent.push(json!({
                "field": col,
                "ref": fk_target,
                "value": val,
                "label": label,
            }));
        }
        if !adjacent.is_empty() {
            ctx["adjacent"] = json!(adjacent);
        }
    }

    let title = identity_label
        .clone()
        .or_else(|| label_title.clone())
        .or_else(|| entity_name.clone())
        .unwrap_or_else(|| page_key.to_string());

    // ── Компактный текст для инъекции в диалог ──────────────────────────────
    let mut text = String::new();
    text.push_str(&format!("Страница: {}\n", label_title.as_deref().unwrap_or(&title)));
    text.push_str(&format!("Тип страницы: {}\n", pr.page_type));
    text.push_str(&format!("Ссылка: {}\n", deep_link));
    if let Some(name) = &entity_name {
        let ix = pr.entity_index.as_deref().unwrap_or("");
        text.push_str(&format!("Сущность: {} [{}]\n", name, ix));
    }
    if let Some(desc) = &entity_desc {
        text.push_str(&format!("Описание: {}\n", desc));
    }
    if let Some(lbl) = &identity_label {
        text.push_str(&format!("Объект: {}\n", lbl));
    }
    if let Some(Value::Object(map)) = &object_row {
        text.push_str("Данные объекта:\n");
        let mut shown = 0;
        for (k, v) in map {
            if SKIP_COLUMNS.contains(&k.as_str()) {
                continue;
            }
            let Some(val) = short_value(v) else { continue };
            text.push_str(&format!("  {}: {}\n", k, val));
            shown += 1;
            if shown >= 40 {
                break;
            }
        }
    }
    if !adjacent.is_empty() {
        text.push_str("Связанные объекты:\n");
        for a in &adjacent {
            let field = a.get("field").and_then(|v| v.as_str()).unwrap_or("");
            let label = a
                .get("label")
                .and_then(|v| v.as_str())
                .or_else(|| a.get("value").and_then(|v| v.as_str()))
                .unwrap_or("");
            text.push_str(&format!("  {}: {}\n", field, label));
        }
    }

    BuiltContext {
        title,
        page_type: pr.page_type.to_string(),
        entity_index: pr.entity_index,
        entity_id: pr.entity_id,
        context_json: ctx,
        rendered_text: text,
    }
}
