use anyhow::Result;
use contracts::domain::a033_wb_day_close::{CompareResponse, RepostResult, SridDiff, WbDayClose};
use contracts::domain::common::AggregateId;
use uuid::Uuid;

use super::{lines_builder, repository};

// ─────────────────────────────────────────────────────────────────────────────
// create_active
// ─────────────────────────────────────────────────────────────────────────────

/// Создаёт новый активный документ в состоянии Draft (строки не заполнены).
/// Если активный документ уже существует — возвращает его id без изменений.
pub async fn create_active(connection_id: &str, business_date: &str) -> Result<Uuid> {
    if let Some(existing) = repository::get_active(connection_id, business_date).await? {
        return Ok(existing.base.id.value());
    }

    let mut doc = WbDayClose::new_active(connection_id.to_string(), business_date.to_string());
    doc.before_write();
    repository::insert(&doc).await?;
    Ok(doc.base.id.value())
}

// ─────────────────────────────────────────────────────────────────────────────
// recalculate
// ─────────────────────────────────────────────────────────────────────────────

/// Пересчитывает строки, проблемы и итоги документа.
/// Идемпотентен при неизменных данных в p903/p913/a012.
pub async fn recalculate(id: Uuid) -> Result<()> {
    let mut doc = load(id).await?;

    let (lines, problems) = lines_builder::build(&doc.connection_id, &doc.business_date).await?;
    doc.set_lines_and_problems(lines, problems);
    doc.before_write();

    repository::update(&doc).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// repost_problematic_a012
// ─────────────────────────────────────────────────────────────────────────────

/// Перепроводит a012_wb_sales, упомянутые в проблемах документа.
/// Опционально — только для проблем с указанными кодами.
/// По завершении автоматически пересчитывает документ.
pub async fn repost_problematic_a012(
    id: Uuid,
    only_problem_codes: &[String],
) -> Result<RepostResult> {
    let doc = load(id).await?;

    // Собираем уникальные a012_ids из проблем (фильтрация по кодам если нужно)
    let target_ids: std::collections::HashSet<String> = doc
        .problems
        .iter()
        .filter(|p| only_problem_codes.is_empty() || only_problem_codes.contains(&p.code))
        .flat_map(|p| p.a012_ids.iter().cloned())
        .collect();

    let total = target_ids.len();
    let mut success = 0usize;
    let mut failed = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for a012_id_str in &target_ids {
        match Uuid::parse_str(a012_id_str) {
            Ok(a012_uuid) => {
                match crate::domain::a012_wb_sales::posting::post_document(a012_uuid).await {
                    Ok(()) => success += 1,
                    Err(e) => {
                        failed += 1;
                        errors.push(format!("{}: {}", a012_id_str, e));
                        tracing::warn!("Failed to repost a012 {}: {}", a012_id_str, e);
                    }
                }
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("Invalid UUID '{}': {}", a012_id_str, e));
            }
        }
    }

    // Пересчёт после репоста
    recalculate(id).await?;

    Ok(RepostResult {
        total,
        success,
        failed,
        errors,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// archive_and_recreate
// ─────────────────────────────────────────────────────────────────────────────

/// Архивирует текущий активный документ, создаёт новый и сразу пересчитывает его.
/// Операция атомарна (одна транзакция БД).
pub async fn archive_and_recreate(id: Uuid, reason: Option<String>) -> Result<Uuid> {
    let mut old = load(id).await?;

    if old.is_archived {
        return Err(anyhow::anyhow!("Document {} is already archived", id));
    }

    old.mark_archived(reason.clone());

    let mut new_doc = WbDayClose::new_active(old.connection_id.clone(), old.business_date.clone());
    new_doc.replaces_id = Some(old.base.id.as_string());
    new_doc.before_write();

    let new_id = new_doc.base.id.value();

    repository::archive_and_insert_new(&old, &new_doc).await?;

    // Пересчёт нового документа
    recalculate(new_id).await?;

    Ok(new_id)
}

// ─────────────────────────────────────────────────────────────────────────────
// compare
// ─────────────────────────────────────────────────────────────────────────────

pub async fn compare(active_id: Uuid, archived_id: Uuid) -> Result<CompareResponse> {
    let active = load(active_id).await?;
    let archived = load(archived_id).await?;

    // Индексируем строки по srid
    let active_map: std::collections::HashMap<String, f64> = active
        .lines
        .iter()
        .map(|l| (l.srid.clone(), l.result))
        .collect();
    let archived_map: std::collections::HashMap<String, f64> = archived
        .lines
        .iter()
        .map(|l| (l.srid.clone(), l.result))
        .collect();

    let added_srids: Vec<String> = active_map
        .keys()
        .filter(|k| !archived_map.contains_key(*k))
        .cloned()
        .collect();

    let removed_srids: Vec<String> = archived_map
        .keys()
        .filter(|k| !active_map.contains_key(*k))
        .cloned()
        .collect();

    let mut changed_srids: Vec<SridDiff> = active_map
        .iter()
        .filter_map(|(srid, &active_result)| {
            archived_map.get(srid).and_then(|&archived_result| {
                if (active_result - archived_result).abs() > 0.001 {
                    Some(SridDiff {
                        srid: srid.clone(),
                        active_result,
                        archived_result,
                        delta: active_result - archived_result,
                    })
                } else {
                    None
                }
            })
        })
        .collect();
    changed_srids.sort_by(|a, b| {
        b.delta
            .abs()
            .partial_cmp(&a.delta.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(CompareResponse {
        active_date: active.business_date,
        archived_date: Some(archived.business_date),
        active_totals: active.totals,
        archived_totals: archived.totals,
        added_srids,
        removed_srids,
        changed_srids,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

pub async fn get_by_id(id: Uuid) -> Result<Option<WbDayClose>> {
    repository::get_by_id(id).await
}

pub async fn list_paginated(query: repository::ListQuery) -> Result<Vec<WbDayClose>> {
    repository::list_paginated(query).await
}

pub async fn list_by_day(connection_id: &str, business_date: &str) -> Result<Vec<WbDayClose>> {
    repository::list_by_day(connection_id, business_date).await
}

async fn load(id: Uuid) -> Result<WbDayClose> {
    repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("WbDayClose not found: {}", id))
}
