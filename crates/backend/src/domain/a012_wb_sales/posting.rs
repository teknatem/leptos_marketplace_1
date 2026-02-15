use super::repository;
use anyhow::Result;
use uuid::Uuid;

async fn sync_organization_from_connection(document: &mut contracts::domain::a012_wb_sales::aggregate::WbSales) -> Result<()> {
    let connection_uuid = match Uuid::parse_str(&document.header.connection_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!(
                "Skip organization sync for WB Sales {}: invalid connection_id={}",
                document.base.id.value(),
                document.header.connection_id
            );
            return Ok(());
        }
    };

    let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_uuid).await?;
    let Some(connection) = connection else {
        tracing::warn!(
            "Skip organization sync for WB Sales {}: connection not found, connection_id={}",
            document.base.id.value(),
            document.header.connection_id
        );
        return Ok(());
    };

    let organization_ref = connection.organization_ref.trim().trim_matches('"');
    let organization_uuid = match Uuid::parse_str(organization_ref) {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!(
                "Skip organization sync for WB Sales {}: invalid organization_ref={}",
                document.base.id.value(),
                connection.organization_ref
            );
            return Ok(());
        }
    };

    if crate::domain::a002_organization::service::get_by_id(organization_uuid)
        .await?
        .is_none()
    {
        tracing::warn!(
            "Skip organization sync for WB Sales {}: organization_ref not found={}",
            document.base.id.value(),
            connection.organization_ref
        );
        return Ok(());
    }

    let resolved_org_id = organization_uuid.to_string();

    if document.header.organization_id != resolved_org_id {
        tracing::info!(
            "Sync organization for WB Sales {}: {} -> {}",
            document.base.id.value(),
            document.header.organization_id,
            resolved_org_id
        );
        document.header.organization_id = resolved_org_id;
    }

    Ok(())
}

/// Провести документ (установить is_posted = true и создать проекции)
pub async fn post_document(id: Uuid) -> Result<()> {
    // Загрузить документ
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Синхронизация организации из подключения (если отличается)
    sync_organization_from_connection(&mut document).await?;

    // Автозаполнение ссылок на marketplace_product и nomenclature
    super::service::auto_fill_references(&mut document).await?;

    // Заполнение dealer_price_ut из p906_nomenclature_prices
    super::service::fill_dealer_price(&mut document).await?;

    // Расчёт финансовых полей (план/факт) на основе данных P903
    super::service::calculate_financial_fields(&mut document).await?;

    // Установить флаг is_posted
    document.is_posted = true;
    document.base.metadata.is_posted = document.is_posted;
    document.before_write();

    // Сохранить документ
    repository::upsert_document(&document).await?;

    // Удалить старые проекции (если были)
    crate::projections::p900_mp_sales_register::service::delete_by_registrator(&id.to_string())
        .await?;
    crate::projections::p904_sales_data::repository::delete_by_registrator(&id.to_string()).await?;

    // Создать новые проекции
    crate::projections::p900_mp_sales_register::service::project_wb_sales(&document, id).await?;
    crate::projections::p904_sales_data::service::project_wb_sales(&document, id).await?;

    Ok(())
}

/// Отменить проведение документа (установить is_posted = false и удалить проекции)
pub async fn unpost_document(id: Uuid) -> Result<()> {
    // Загрузить документ
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Снять флаг is_posted
    document.is_posted = false;
    document.base.metadata.is_posted = document.is_posted;
    document.before_write();

    // Сохранить документ
    repository::upsert_document(&document).await?;

    // Удалить проекции
    crate::projections::p900_mp_sales_register::service::delete_by_registrator(&id.to_string())
        .await?;
    crate::projections::p904_sales_data::repository::delete_by_registrator(&id.to_string()).await?;

    Ok(())
}
