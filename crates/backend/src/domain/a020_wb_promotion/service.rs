use super::repository;
use anyhow::Result;
use contracts::domain::a020_wb_promotion::aggregate::WbPromotion;
use uuid::Uuid;

/// Сохранить документ акции с сырым JSON
pub async fn store_document_with_raw(document: WbPromotion, raw_json: &str) -> Result<Uuid> {
    let mut doc = document;

    // Сохраняем raw JSON
    let raw_ref = if !raw_json.is_empty() {
        let fetched_at = chrono::DateTime::parse_from_rfc3339(&doc.source_meta.fetched_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        match crate::shared::data::raw_storage::save_raw_json(
            "WB",
            "WB_Promotion",
            &doc.header.document_no,
            raw_json,
            fetched_at,
        )
        .await
        {
            Ok(ref_id) => ref_id,
            Err(e) => {
                tracing::warn!("Failed to store raw JSON: {}", e);
                String::new()
            }
        }
    } else {
        String::new()
    };

    doc.source_meta.raw_payload_ref = raw_ref;
    doc.before_write();
    doc.validate().map_err(|e| anyhow::anyhow!(e))?;

    let id = repository::upsert_document(&doc).await?;
    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbPromotion>> {
    repository::get_by_id(id).await
}

pub async fn post(id: Uuid) -> Result<()> {
    repository::set_posted(id, true).await
}

pub async fn unpost(id: Uuid) -> Result<()> {
    repository::set_posted(id, false).await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}
