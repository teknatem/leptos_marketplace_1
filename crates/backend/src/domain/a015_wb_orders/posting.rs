use super::repository;
use anyhow::Result;
use uuid::Uuid;

async fn sync_organization_from_connection(document: &mut contracts::domain::a015_wb_orders::aggregate::WbOrders) -> Result<()> {
    let connection_uuid = match Uuid::parse_str(&document.header.connection_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!(
                "Skip organization sync for WB Orders {}: invalid connection_id={}",
                document.base.id.value(),
                document.header.connection_id
            );
            return Ok(());
        }
    };

    let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_uuid).await?;
    let Some(connection) = connection else {
        tracing::warn!(
            "Skip organization sync for WB Orders {}: connection not found, connection_id={}",
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
                "Skip organization sync for WB Orders {}: invalid organization_ref={}",
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
            "Skip organization sync for WB Orders {}: organization_ref not found={}",
            document.base.id.value(),
            connection.organization_ref
        );
        return Ok(());
    }

    let resolved_org_id = organization_uuid.to_string();

    if document.header.organization_id != resolved_org_id {
        tracing::info!(
            "Sync organization for WB Orders {}: {} -> {}",
            document.base.id.value(),
            document.header.organization_id,
            resolved_org_id
        );
        document.header.organization_id = resolved_org_id;
    }

    Ok(())
}

/// Провести документ (установить is_posted = true)
/// Для Orders пока нет проекций, только устанавливаем флаг
pub async fn post_document(id: Uuid) -> Result<()> {
    // Загрузить документ
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Синхронизация организации из подключения (если отличается)
    sync_organization_from_connection(&mut document).await?;

    // Автозаполнение ссылок на marketplace_product и nomenclature
    super::service::auto_fill_references(&mut document).await?;

    // При каждом проведении принудительно пересчитываем base_nomenclature_ref
    // по текущему nomenclature_ref.
    super::service::refill_base_nomenclature_ref(&mut document).await?;

    // Заполнение dealer_price_ut из p906_nomenclature_prices (только при Post)
    super::service::fill_dealer_price(&mut document).await?;

    // Расчет margin_pro после dealer_price_ut
    super::service::calculate_margin_pro(&mut document).await?;

    // Установить флаг is_posted
    document.is_posted = true;
    document.base.metadata.is_posted = document.is_posted;
    document.before_write();

    // Сохранить документ
    repository::upsert_document(&document).await?;

    // TODO: Если в будущем нужны проекции для Orders, добавить их здесь
    tracing::info!("Posted WB Orders document: {}", id);

    Ok(())
}

/// Отменить проведение документа (установить is_posted = false)
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

    // TODO: Если в будущем нужны проекции для Orders, удалить их здесь
    tracing::info!("Unposted WB Orders document: {}", id);

    Ok(())
}

