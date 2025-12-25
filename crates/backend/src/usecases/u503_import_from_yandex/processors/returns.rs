use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use crate::domain::a016_ym_returns;
use contracts::domain::a016_ym_returns::aggregate::{
    YmReturn, YmReturnDecision, YmReturnHeader, YmReturnLine, YmReturnSourceMeta,
    YmReturnState,
};
use super::super::yandex_api_client::YmReturnItem;
use super::order::parse_ym_date;

pub async fn process_return(
    connection: &ConnectionMP,
    organization_id: &str,
    return_item: &YmReturnItem,
) -> Result<bool> {
    let return_id = return_item.id;
    let return_id_str = return_id.to_string();

    // Check if exists
    let existing = a016_ym_returns::service::get_by_return_id(return_id).await?;
    let is_new = existing.is_none();

    // Map lines
    let lines: Vec<YmReturnLine> = return_item
        .items
        .iter()
        .map(|item| {
            let decisions: Vec<YmReturnDecision> = item
                .decisions
                .iter()
                .map(|d| YmReturnDecision {
                    decision_type: d.decision_type.clone().unwrap_or_default(),
                    amount: d.amount.as_ref().and_then(|a| a.value),
                    currency: d.amount.as_ref().and_then(|a| a.currency_id.clone()),
                    partner_compensation_amount: d
                        .partner_compensation_amount
                        .as_ref()
                        .and_then(|a| a.value),
                    comment: d.comment.clone(),
                })
                .collect();

            let photos: Vec<String> =
                item.photos.iter().filter_map(|p| p.url.clone()).collect();

            YmReturnLine {
                item_id: item.market_sku.unwrap_or(0),
                shop_sku: item.shop_sku.clone().unwrap_or_default(),
                offer_id: item.offer_id.clone().unwrap_or_default(),
                name: item.offer_name.clone().unwrap_or_default(),
                count: item.count,
                price: item.price,
                return_reason: item.return_reason.clone(),
                decisions,
                photos,
            }
        })
        .collect();

    if lines.is_empty() {
        anyhow::bail!("Return {} has no items", return_id_str);
    }

    // Parse dates
    let created_at_source = return_item
        .created_at
        .as_ref()
        .and_then(|s| parse_ym_date(s));
    let updated_at_source = return_item
        .updated_at
        .as_ref()
        .and_then(|s| parse_ym_date(s));

    // Create aggregate
    let header = YmReturnHeader {
        return_id,
        order_id: return_item.order_id,
        connection_id: connection.base.id.as_string(),
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
        campaign_id: connection
            .supplier_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        return_type: return_item.return_type.clone().unwrap_or_default(),
        amount: return_item.amount.as_ref().and_then(|a| a.value),
        currency: return_item
            .amount
            .as_ref()
            .and_then(|a| a.currency_id.clone()),
    };

    let state = YmReturnState {
        refund_status: return_item.refund_status.clone().unwrap_or_default(),
        created_at_source,
        updated_at_source,
        refund_date: None,
    };

    let source_meta = YmReturnSourceMeta {
        raw_payload_ref: String::new(),
        fetched_at: chrono::Utc::now(),
        document_version: 1,
    };

    let return_type_display = return_item.return_type.clone().unwrap_or_default();
    let document = YmReturn::new_for_insert(
        return_id_str.clone(),
        format!(
            "YM {} {} (Order {})",
            if return_type_display == "UNREDEEMED" {
                "Невыкуп"
            } else {
                "Возврат"
            },
            return_id_str,
            return_item.order_id
        ),
        header,
        lines,
        state,
        source_meta,
        true,
    );

    let raw_json = serde_json::to_string(&return_item)?;
    a016_ym_returns::service::store_document_with_raw(document, &raw_json).await?;
    
    Ok(is_new)
}

