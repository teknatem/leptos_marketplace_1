use super::{projection_builder, repository};
use anyhow::Result;
use contracts::domain::a010_ozon_fbs_posting::aggregate::OzonFbsPosting;
use contracts::domain::a011_ozon_fbo_posting::aggregate::OzonFboPosting;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::a013_ym_order::aggregate::YmOrder;
use uuid::Uuid;

/// Проецировать OZON FBS Posting в Sales Register
pub async fn project_ozon_fbs(document: &OzonFbsPosting, document_id: Uuid) -> Result<()> {
    let entries = projection_builder::from_ozon_fbs(document, &document_id.to_string()).await?;

    for entry in entries {
        repository::upsert_entry(&entry).await?;
    }

    tracing::info!(
        "Projected OZON FBS document {} into Sales Register ({} lines)",
        document.header.document_no,
        document.lines.len()
    );

    Ok(())
}

/// Проецировать OZON FBO Posting в Sales Register
pub async fn project_ozon_fbo(document: &OzonFboPosting, document_id: Uuid) -> Result<()> {
    let entries = projection_builder::from_ozon_fbo(document, &document_id.to_string()).await?;

    for entry in entries {
        repository::upsert_entry(&entry).await?;
    }

    tracing::info!(
        "Projected OZON FBO document {} into Sales Register ({} lines)",
        document.header.document_no,
        document.lines.len()
    );

    Ok(())
}

/// Проецировать WB Sales в Sales Register
pub async fn project_wb_sales(document: &WbSales, document_id: Uuid) -> Result<()> {
    let entry = projection_builder::from_wb_sales(document, &document_id.to_string()).await?;
    repository::upsert_entry(&entry).await?;

    tracing::info!(
        "Projected WB Sales document {} into Sales Register",
        document.header.document_no
    );

    Ok(())
}

/// Проецировать YM Order в Sales Register
pub async fn project_ym_order(document: &YmOrder, document_id: Uuid) -> Result<()> {
    let entries = projection_builder::from_ym_order(document, &document_id.to_string()).await?;

    for entry in entries {
        repository::upsert_entry(&entry).await?;
    }

    tracing::info!(
        "Projected YM Order {} into Sales Register ({} lines)",
        document.header.document_no,
        document.lines.len()
    );

    Ok(())
}

/// Получить список продаж
pub async fn list_sales(limit: Option<u64>) -> Result<Vec<repository::Model>> {
    repository::list_sales(limit).await
}

/// Получить записи по маркетплейсу
pub async fn get_by_marketplace(
    marketplace: &str,
    limit: Option<u64>,
) -> Result<Vec<repository::Model>> {
    repository::get_by_marketplace(marketplace, limit).await
}

