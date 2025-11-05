use super::repository;
use anyhow::Result;
use contracts::domain::a011_ozon_fbo_posting::aggregate::OzonFboPosting;
use uuid::Uuid;

pub async fn store_document_with_raw(
    mut document: OzonFboPosting,
    raw_json: &str,
) -> Result<Uuid> {
    let raw_ref = crate::shared::data::raw_storage::save_raw_json(
        "OZON",
        "OZON_FBO_Posting",
        &document.header.document_no,
        raw_json,
        document.source_meta.fetched_at,
    )
    .await?;

    document.source_meta.raw_payload_ref = raw_ref;
    document
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    document.before_write();

    let id = repository::upsert_document(&document).await?;
    
    // Проецируем в Sales Register
    if let Err(e) = crate::projections::p900_mp_sales_register::service::project_ozon_fbo(&document).await {
        tracing::error!("Failed to project OZON FBO document to Sales Register: {}", e);
    }
    
    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<OzonFboPosting>> {
    repository::get_by_id(id).await
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<OzonFboPosting>> {
    repository::get_by_document_no(document_no).await
}

pub async fn list_all() -> Result<Vec<OzonFboPosting>> {
    repository::list_all().await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}

