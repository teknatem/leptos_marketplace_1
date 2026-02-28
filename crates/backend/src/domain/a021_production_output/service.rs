use super::repository;
use anyhow::Result;
use contracts::domain::a021_production_output::aggregate::ProductionOutput;
use contracts::domain::common::AggregateId;
use uuid::Uuid;

pub use repository::{ProductionOutputListQuery, ProductionOutputListResult};

/// Сохранить или обновить документ из API
/// Возвращает (id, is_new)
pub async fn upsert_from_api(doc: &ProductionOutput) -> Result<(String, bool)> {
    let is_new = repository::upsert_document(doc).await?;
    Ok((doc.to_string_id(), is_new))
}

/// Заполнить nomenclature_ref по артикулу (только если поле пустое)
pub async fn fill_nomenclature_ref_if_empty(doc: &mut ProductionOutput) -> Result<bool> {
    if doc.nomenclature_ref.is_some() {
        return Ok(false);
    }

    let article = doc.article.trim().to_string();
    if article.is_empty() {
        return Ok(false);
    }

    let matches =
        crate::domain::a004_nomenclature::repository::find_by_article_ignore_case(&article)
            .await?;

    tracing::info!(
        "fill_nomenclature_ref: article='{}' → found {} match(es) in a004",
        article,
        matches.len()
    );

    // Заполняем только при однозначном совпадении
    if let Some(nom) = matches.into_iter().find(|n| !n.is_folder) {
        let nom_id = nom.base.id.as_string();
        tracing::info!(
            "fill_nomenclature_ref: linked doc '{}' → nomenclature '{}' ({})",
            doc.document_no,
            nom.base.description,
            nom_id
        );
        doc.nomenclature_ref = Some(nom_id.clone());
        repository::update_nomenclature_ref(&doc.to_string_id(), Some(nom_id)).await?;
        return Ok(true);
    }

    tracing::warn!(
        "fill_nomenclature_ref: no match for article='{}' in doc '{}'",
        article,
        doc.document_no
    );
    Ok(false)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<ProductionOutput>> {
    repository::get_by_id(id).await
}

/// Провести документ (is_posted = true)
pub async fn post_document(id: Uuid) -> Result<()> {
    let mut doc = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    fill_nomenclature_ref_if_empty(&mut doc).await?;

    doc.base.metadata.is_posted = true;
    repository::upsert_document(&doc).await?;
    tracing::info!("Posted production output document: {}", id);
    Ok(())
}

/// Отменить проведение документа (is_posted = false)
pub async fn unpost_document(id: Uuid) -> Result<()> {
    let mut doc = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    doc.base.metadata.is_posted = false;
    repository::upsert_document(&doc).await?;
    tracing::info!("Unposted production output document: {}", id);
    Ok(())
}

pub async fn list_paginated(
    query: ProductionOutputListQuery,
) -> Result<ProductionOutputListResult> {
    repository::list_sql(query).await
}
