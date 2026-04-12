use super::repository;
use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};
use contracts::domain::a028_missing_cost_registry::aggregate::{
    MissingCostRegistry, MissingCostRegistryLine, MissingCostRegistryUpdateDto,
};
use std::collections::BTreeSet;
use uuid::Uuid;

pub use repository::{MissingCostRegistryListQuery, MissingCostRegistryListResult};

const A028_REGISTRATOR_TYPE: &str = "a028_missing_cost_registry";

fn normalize_month_start(target_date: &str) -> Result<String> {
    let date_part = target_date.split('T').next().unwrap_or(target_date);
    let date = NaiveDate::parse_from_str(date_part, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid target_date '{}': {}", target_date, e))?;
    let month_start = NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
        .ok_or_else(|| anyhow::anyhow!("Invalid month start for {}", target_date))?;
    Ok(month_start.format("%Y-%m-%d").to_string())
}

async fn get_or_create_monthly_document(document_date: &str) -> Result<MissingCostRegistry> {
    if let Some(doc) = repository::get_by_document_date(document_date).await? {
        return Ok(doc);
    }

    let mut draft = MissingCostRegistry::new_monthly(document_date.to_string());
    draft.before_write();

    match repository::insert_document(&draft).await {
        Ok(()) => Ok(draft),
        Err(error) => {
            if let Some(existing) = repository::get_by_document_date(document_date).await? {
                Ok(existing)
            } else {
                Err(error)
            }
        }
    }
}

pub async fn get_by_id(id: Uuid) -> Result<Option<MissingCostRegistry>> {
    repository::get_by_id(id).await
}

pub async fn list_paginated(
    query: MissingCostRegistryListQuery,
) -> Result<MissingCostRegistryListResult> {
    repository::list_sql(query).await
}

pub async fn update_document(id: Uuid, dto: MissingCostRegistryUpdateDto) -> Result<()> {
    let mut doc = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    doc.update_from_dto(&dto);
    doc.validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    doc.before_write();
    repository::update_document(&doc).await?;

    if doc.base.metadata.is_posted {
        crate::projections::p912_nomenclature_costs::service::project_missing_cost_registry(&doc)
            .await?;
    }

    Ok(())
}

pub async fn post_document(id: Uuid) -> Result<()> {
    let mut doc = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    doc.validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    doc.base.metadata.is_posted = true;
    doc.before_write();
    repository::update_document(&doc).await?;
    crate::projections::p912_nomenclature_costs::service::project_missing_cost_registry(&doc)
        .await?;
    Ok(())
}

pub async fn unpost_document(id: Uuid) -> Result<()> {
    let mut doc = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    doc.base.metadata.is_posted = false;
    doc.before_write();
    repository::update_document(&doc).await?;
    crate::projections::p912_nomenclature_costs::service::remove_by_registrator(
        A028_REGISTRATOR_TYPE,
        &id.to_string(),
    )
    .await?;
    Ok(())
}

pub async fn ensure_missing_cost_entries(
    target_date: &str,
    nomenclature_refs: &[String],
) -> Result<()> {
    let month_start = normalize_month_start(target_date)?;
    let missing_refs: Vec<String> = nomenclature_refs
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    if missing_refs.is_empty() {
        return Ok(());
    }

    for _ in 0..5 {
        let mut doc = get_or_create_monthly_document(&month_start).await?;
        let expected_version = doc.base.metadata.version;
        let mut lines = doc.parse_lines();
        let existing_refs: BTreeSet<String> = lines
            .iter()
            .map(|line| line.nomenclature_ref.clone())
            .collect();

        let mut changed = false;
        for nomenclature_ref in &missing_refs {
            if existing_refs.contains(nomenclature_ref) {
                continue;
            }
            lines.push(MissingCostRegistryLine {
                nomenclature_ref: nomenclature_ref.clone(),
                cost: None,
                comment: None,
                detected_at: Utc::now().to_rfc3339(),
            });
            changed = true;
        }

        if !changed {
            return Ok(());
        }

        doc.set_lines(lines);
        doc.validate()
            .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
        doc.before_write();

        if repository::update_document_if_version(&doc, expected_version).await? {
            return Ok(());
        }
    }

    Err(anyhow::anyhow!(
        "Failed to append missing cost entries after optimistic retries"
    ))
}
