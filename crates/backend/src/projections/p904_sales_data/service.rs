use super::{projection_builder, repository};
use anyhow::Result;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use uuid::Uuid;

/// Проецировать WB Sales в Sales Data (P904)
pub async fn project_wb_sales(document: &WbSales, document_id: Uuid) -> Result<()> {
    let entries = projection_builder::from_wb_sales_lines(document, &document_id.to_string()).await?;
    
    for entry in entries {
        repository::upsert_entry(&entry).await?;
    }

    tracing::info!(
        "Projected WB Sales document {} into Sales Data P904",
        document.header.document_no
    );

    Ok(())
}


pub async fn list(limit: Option<u64>) -> Result<Vec<repository::Model>> {
    repository::list(limit).await
}

pub async fn list_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    connection_mp_ref: Option<String>,
    limit: Option<u64>,
) -> Result<Vec<repository::ModelWithCabinet>> {
    repository::list_with_filters(date_from, date_to, connection_mp_ref, limit).await
}

