use super::repository;
use anyhow::Result;
use uuid::Uuid;

async fn sync_organization_from_connection(
    document: &mut contracts::domain::a015_wb_orders::aggregate::WbOrders,
) -> Result<()> {
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

pub async fn post_document(id: Uuid) -> Result<()> {
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    sync_organization_from_connection(&mut document).await?;
    super::service::auto_fill_references(&mut document).await?;
    super::service::refill_base_nomenclature_ref(&mut document).await?;
    super::service::fill_dealer_price_resolved(&mut document).await?;
    super::service::calculate_margin_pro(&mut document).await?;

    document.is_posted = true;
    document.base.metadata.is_posted = document.is_posted;
    document.before_write();

    repository::upsert_document(&document).await?;

    let registrator_ref = format!("a015:{}", id);
    crate::projections::p909_mp_order_line_turnovers::service::remove_by_registrator_ref(
        &registrator_ref,
    )
    .await?;
    crate::projections::p909_mp_order_line_turnovers::service::project_wb_order(&document, id)
        .await?;
    tracing::info!("Posted WB Orders document: {}", id);

    Ok(())
}

pub async fn unpost_document(id: Uuid) -> Result<()> {
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    document.is_posted = false;
    document.base.metadata.is_posted = document.is_posted;
    document.before_write();

    repository::upsert_document(&document).await?;

    crate::projections::p909_mp_order_line_turnovers::service::remove_by_registrator_ref(&format!(
        "a015:{}",
        id
    ))
    .await?;
    tracing::info!("Unposted WB Orders document: {}", id);

    Ok(())
}
