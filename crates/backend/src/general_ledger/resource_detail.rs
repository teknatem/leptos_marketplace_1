//! Получение detail-строк для одной GL-проводки и сверка с её amount.

use anyhow::Result;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

use contracts::general_ledger::resource_detail::{
    GlResourceDetailIntegrity, GlResourceDetailResponse, GlResourceDetailTotals,
};

use super::detail_links::{
    descriptor_for_resource_table, GlDetailLinkDescriptor, GlDetailLinkKind,
};
use crate::projections::p903_wb_finance_report::repository as p903;
use crate::projections::p909_mp_order_line_turnovers::repository as p909;
use crate::projections::p910_mp_unlinked_turnovers::repository as p910;
use crate::projections::p911_wb_advert_by_items::repository as p911;
use crate::projections::p913_wb_advert_order_attr::repository as p913;
use crate::shared::data::db::get_connection;

const MATCH_TOLERANCE: f64 = 0.01;
const MISMATCH_SAMPLE_LIMIT: usize = 5;

fn conn() -> &'static sea_orm::DatabaseConnection {
    get_connection()
}

pub async fn get_resource_details(gl_id: &str) -> Result<GlResourceDetailResponse> {
    let gl = super::repository::get_by_id(gl_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("GL entry {gl_id} not found"))?;

    let resource_table = gl.resource_table.clone();
    let resource_field = gl.resource_field.clone();
    let resource_sign = gl.resource_sign;
    let gl_amount = gl.amount;

    let descriptor = descriptor_for_resource_table(&resource_table);

    let (rows, error) = match descriptor {
        Some(descriptor) => match fetch_rows(descriptor, &gl).await {
            Ok(rows) => (rows, None),
            Err(err) => (Vec::new(), Some(err.to_string())),
        },
        None => (
            Vec::new(),
            Some(format!(
                "Resource table '{resource_table}' is not registered in detail_links"
            )),
        ),
    };

    let integrity = compute_integrity(&rows, &gl.id, descriptor);

    let sum_resource = rows
        .iter()
        .map(|row| {
            row.get(&resource_field)
                .and_then(JsonValue::as_f64)
                .unwrap_or(0.0)
        })
        .sum::<f64>();
    let sum_signed = sum_resource * f64::from(resource_sign);
    let delta = sum_signed - gl_amount;
    let row_count = rows.len();
    let sum_ok = row_count >= 1 && delta.abs() <= MATCH_TOLERANCE;
    let is_match = sum_ok && integrity.is_ok;

    Ok(GlResourceDetailResponse {
        gl_id: gl.id,
        resource_table,
        resource_field,
        resource_sign,
        supported: descriptor.is_some(),
        rows,
        totals: GlResourceDetailTotals {
            row_count,
            sum_resource,
            sum_signed,
            gl_amount,
            delta,
            is_match,
        },
        integrity,
        error,
    })
}

/// Классифицирует каждую detail-строку по полю `general_ledger_ref`.
///
/// `ExternalLinked` (p903) не имеет колонки `general_ledger_ref` — связь
/// идёт через `gl.registrator_ref → detail.id`. Для таких таблиц
/// integrity-проверка проходит автоматически (если строки нашлись).
fn compute_integrity(
    rows: &[JsonValue],
    gl_id: &str,
    descriptor: Option<&GlDetailLinkDescriptor>,
) -> GlResourceDetailIntegrity {
    if rows.is_empty() {
        return GlResourceDetailIntegrity::default();
    }

    let is_projection_linked = matches!(
        descriptor.map(|d| d.kind),
        Some(GlDetailLinkKind::ProjectionLinked)
    );
    if !is_projection_linked {
        return GlResourceDetailIntegrity {
            matched_count: rows.len(),
            missing_count: 0,
            mismatched_count: 0,
            mismatched_refs_sample: Vec::new(),
            is_ok: true,
        };
    }

    let mut matched = 0usize;
    let mut missing = 0usize;
    let mut mismatched = 0usize;
    let mut sample: BTreeMap<String, ()> = BTreeMap::new();

    for row in rows {
        match row.get("general_ledger_ref") {
            None | Some(JsonValue::Null) => missing += 1,
            Some(JsonValue::String(s)) if s.is_empty() => missing += 1,
            Some(JsonValue::String(s)) if s == gl_id => matched += 1,
            Some(JsonValue::String(s)) => {
                mismatched += 1;
                if sample.len() < MISMATCH_SAMPLE_LIMIT {
                    sample.insert(s.clone(), ());
                }
            }
            Some(other) => {
                mismatched += 1;
                if sample.len() < MISMATCH_SAMPLE_LIMIT {
                    sample.insert(other.to_string(), ());
                }
            }
        }
    }

    GlResourceDetailIntegrity {
        matched_count: matched,
        missing_count: missing,
        mismatched_count: mismatched,
        mismatched_refs_sample: sample.into_keys().collect(),
        is_ok: matched == rows.len(),
    }
}

async fn fetch_rows(
    descriptor: &GlDetailLinkDescriptor,
    gl: &super::repository::Model,
) -> Result<Vec<JsonValue>> {
    match descriptor.kind {
        GlDetailLinkKind::ProjectionLinked => match descriptor.detail_table {
            "p909_mp_order_line_turnovers" => fetch_p909(gl).await,
            "p910_mp_unlinked_turnovers" => fetch_p910(gl).await,
            "p911_wb_advert_by_items" => fetch_p911(gl).await,
            "p913_wb_advert_order_attr" => fetch_p913(gl).await,
            other => Err(anyhow::anyhow!(
                "ProjectionLinked table '{other}' has no detail loader"
            )),
        },
        GlDetailLinkKind::ExternalLinked => match descriptor.detail_table {
            "p903_wb_finance_report" => fetch_p903(&gl.registrator_ref).await,
            other => Err(anyhow::anyhow!(
                "ExternalLinked table '{other}' has no detail loader"
            )),
        },
    }
}

/// Загружает кандидатов: строки, которые либо уже указывают на эту GL-проводку
/// (`general_ledger_ref = gl.id`), либо имеют тот же (registrator_type, registrator_ref,
/// turnover_code), но с `general_ledger_ref IS NULL` — это сломанные/недоназначенные
/// строки, которые `compute_integrity` пометит как ошибку целостности.
async fn fetch_p909(gl: &super::repository::Model) -> Result<Vec<JsonValue>> {
    let mut rows = p909::Entity::find()
        .filter(p909::Column::GeneralLedgerRef.eq(gl.id.clone()))
        .into_json()
        .all(conn())
        .await?;
    let orphans = p909::Entity::find()
        .filter(p909::Column::RegistratorType.eq(gl.registrator_type.clone()))
        .filter(p909::Column::RegistratorRef.eq(gl.registrator_ref.clone()))
        .filter(p909::Column::TurnoverCode.eq(gl.turnover_code.clone()))
        .filter(p909::Column::GeneralLedgerRef.is_null())
        .into_json()
        .all(conn())
        .await?;
    rows.extend(orphans);
    Ok(rows)
}

async fn fetch_p910(gl: &super::repository::Model) -> Result<Vec<JsonValue>> {
    let mut rows = p910::Entity::find()
        .filter(p910::Column::GeneralLedgerRef.eq(gl.id.clone()))
        .into_json()
        .all(conn())
        .await?;
    let orphans = p910::Entity::find()
        .filter(p910::Column::RegistratorType.eq(gl.registrator_type.clone()))
        .filter(p910::Column::RegistratorRef.eq(gl.registrator_ref.clone()))
        .filter(p910::Column::TurnoverCode.eq(gl.turnover_code.clone()))
        .filter(p910::Column::GeneralLedgerRef.is_null())
        .into_json()
        .all(conn())
        .await?;
    rows.extend(orphans);
    Ok(rows)
}

async fn fetch_p911(gl: &super::repository::Model) -> Result<Vec<JsonValue>> {
    let mut rows = p911::Entity::find()
        .filter(p911::Column::GeneralLedgerRef.eq(gl.id.clone()))
        .into_json()
        .all(conn())
        .await?;
    let orphans = p911::Entity::find()
        .filter(p911::Column::RegistratorType.eq(gl.registrator_type.clone()))
        .filter(p911::Column::RegistratorRef.eq(gl.registrator_ref.clone()))
        .filter(p911::Column::TurnoverCode.eq(gl.turnover_code.clone()))
        .filter(p911::Column::GeneralLedgerRef.is_null())
        .into_json()
        .all(conn())
        .await?;
    rows.extend(orphans);
    Ok(rows)
}

async fn fetch_p913(gl: &super::repository::Model) -> Result<Vec<JsonValue>> {
    let mut rows = p913::Entity::find()
        .filter(p913::Column::GeneralLedgerRef.eq(gl.id.clone()))
        .into_json()
        .all(conn())
        .await?;
    let orphans = p913::Entity::find()
        .filter(p913::Column::RegistratorType.eq(gl.registrator_type.clone()))
        .filter(p913::Column::RegistratorRef.eq(gl.registrator_ref.clone()))
        .filter(p913::Column::TurnoverCode.eq(gl.turnover_code.clone()))
        .filter(p913::Column::GeneralLedgerRef.is_null())
        .into_json()
        .all(conn())
        .await?;
    rows.extend(orphans);
    Ok(rows)
}

async fn fetch_p903(registrator_ref: &str) -> Result<Vec<JsonValue>> {
    if !registrator_ref.starts_with("p903:") {
        let row = p903::Entity::find_by_id(registrator_ref.to_string())
            .into_json()
            .one(conn())
            .await?;
        return Ok(row.into_iter().collect());
    }

    let parts: Vec<&str> = registrator_ref.splitn(3, ':').collect();
    if parts.len() == 2 {
        let rows = p903::Entity::find()
            .filter(p903::Column::SourceRowRef.eq(registrator_ref))
            .into_json()
            .all(conn())
            .await?;
        return Ok(rows);
    }

    if parts.len() == 3 {
        let rr_dt = parts[1].to_string();
        let rrd_id: i64 = parts[2].parse().map_err(|err| {
            anyhow::anyhow!("Invalid rrd_id in registrator_ref '{registrator_ref}': {err}")
        })?;
        let rows = p903::Entity::find()
            .filter(p903::Column::RrDt.eq(rr_dt))
            .filter(p903::Column::RrdId.eq(rrd_id))
            .into_json()
            .all(conn())
            .await?;
        return Ok(rows);
    }

    Ok(Vec::new())
}
