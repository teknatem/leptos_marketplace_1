use super::super::wildberries_api_client::{WbCalendarPromotion, WbCalendarPromotionDetail};
use crate::domain::a020_wb_promotion;
use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a020_wb_promotion::aggregate::{
    WbPromotion, WbPromotionData, WbPromotionHeader, WbPromotionNomenclature,
    WbPromotionRanging, WbPromotionSourceMeta,
};

pub async fn process_promotion(
    connection: &ConnectionMP,
    organization_id: &str,
    promotion: &WbCalendarPromotion,
    nm_ids: Vec<i64>,
    details: Option<&WbCalendarPromotionDetail>,
) -> Result<bool> {
    use contracts::domain::common::AggregateId;

    let promotion_id = promotion.id;
    let document_no = format!("PROMO-{}", promotion_id);

    let existing = a020_wb_promotion::repository::get_by_promotion_id_and_connection(
        promotion_id,
        &connection.base.id.as_string(),
    )
    .await?;
    let is_new = existing.is_none();

    let header = WbPromotionHeader {
        document_no: document_no.clone(),
        connection_id: connection.base.id.as_string(),
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
    };

    // Базовые поля из list, обогащённые полями из details
    let data = WbPromotionData {
        promotion_id,
        name: details
            .and_then(|d| d.name.clone())
            .or_else(|| promotion.name.clone())
            .unwrap_or_else(|| format!("Акция {}", promotion_id)),
        description: details
            .and_then(|d| d.description.clone())
            .or_else(|| promotion.description.clone()),
        advantages: details
            .map(|d| d.advantages.clone())
            .unwrap_or_default(),
        start_date_time: details
            .and_then(|d| d.start_date_time.clone())
            .or_else(|| promotion.start_date_time.clone())
            .unwrap_or_default(),
        end_date_time: details
            .and_then(|d| d.end_date_time.clone())
            .or_else(|| promotion.end_date_time.clone())
            .unwrap_or_default(),
        promotion_type: details
            .and_then(|d| d.promotion_type.clone())
            .or_else(|| promotion.promotion_type.clone()),
        exception_products_count: details
            .and_then(|d| d.exception_products_count)
            .or(promotion.exception_products_count),
        in_promo_action_total: details
            .and_then(|d| d.in_promo_action_total)
            .or(promotion.in_promo_action_total),
        in_promo_action_leftovers: details.and_then(|d| d.in_promo_action_leftovers),
        not_in_promo_action_leftovers: details.and_then(|d| d.not_in_promo_action_leftovers),
        not_in_promo_action_total: details.and_then(|d| d.not_in_promo_action_total),
        participation_percentage: details.and_then(|d| d.participation_percentage),
        ranging: details
            .map(|d| {
                d.ranging
                    .iter()
                    .map(|r| WbPromotionRanging {
                        condition: r.condition.clone(),
                        participation_rate: r.participation_rate,
                        boost: r.boost,
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };

    let nomenclatures: Vec<WbPromotionNomenclature> = nm_ids
        .into_iter()
        .map(|nm_id| WbPromotionNomenclature { nm_id })
        .collect();

    let source_meta = WbPromotionSourceMeta {
        raw_payload_ref: String::new(),
        fetched_at: chrono::Utc::now().to_rfc3339(),
    };

    let document = WbPromotion::new_for_insert(header, data, nomenclatures, source_meta);

    // Сохраняем raw JSON из details (богаче, чем из list)
    let raw_json = if let Some(d) = details {
        serde_json::to_string(d)?
    } else {
        serde_json::to_string(promotion)?
    };

    tracing::debug!(
        "Processing WB promotion: id={}, name={:?}, document_no={}, has_details={}",
        promotion_id,
        promotion.name,
        document_no,
        details.is_some()
    );

    match a020_wb_promotion::service::store_document_with_raw(document, &raw_json).await {
        Ok(_) => {
            if is_new {
                tracing::debug!("Created new WB promotion: id={}", promotion_id);
            } else {
                tracing::debug!("Updated existing WB promotion: id={}", promotion_id);
            }
            Ok(is_new)
        }
        Err(e) => {
            tracing::error!(
                "Failed to store WB promotion - promotion_id: {}, error: {}",
                promotion_id,
                e
            );
            Err(e)
        }
    }
}
