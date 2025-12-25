use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use crate::projections::p902_ozon_finance_realization::{repository, service};
use super::super::ozon_api_client::OzonFinanceRealizationRow;

pub async fn process_realization_row(
    connection: &ConnectionMP,
    organization_id: &str,
    registrator_ref: &str,
    row: &OzonFinanceRealizationRow,
    currency_code: &str,
    accrual_date: chrono::NaiveDate,
) -> Result<(i32, i32)> {
    let mut inserted = 0;
    let mut updated = 0;
    
    let row_for_json = row.clone();
    let item = &row.item;
    let posting_number = if let Some(ref order) = row.order {
        order.posting_number.clone()
    } else if !item.offer_id.is_empty() {
        item.offer_id.clone()
    } else {
        // Fallback should be handled by caller if possible, but we use what we have
        format!("REPORT-ROW-{}", row.row_number)
    };
    let sku = item.sku.clone();

    // Обрабатываем delivery_commission (продажа)
    if let Some(ref dc) = row.delivery_commission {
        let existing = repository::get_by_id(&posting_number, &sku, "delivery").await?;
        let is_new = existing.is_none();

        let entry = repository::OzonFinanceRealizationEntry {
            posting_number: posting_number.clone(),
            sku: sku.clone(),
            document_type: "OZON_Finance_Realization".to_string(),
            registrator_ref: registrator_ref.to_string(),
            connection_mp_ref: connection.base.id.as_string(),
            organization_ref: organization_id.to_string(),
            posting_ref: None,
            accrual_date,
            operation_date: None,
            delivery_date: None,
            delivery_schema: None,
            delivery_region: None,
            delivery_city: None,
            quantity: dc.quantity,
            price: row.seller_price_per_instance,
            amount: dc.amount,
            commission_amount: Some(dc.commission),
            commission_percent: if dc.amount > 0.0 {
                Some((dc.commission / dc.amount) * 100.0)
            } else {
                None
            },
            services_amount: Some(dc.standard_fee),
            payout_amount: Some(dc.total),
            operation_type: "delivery".to_string(),
            operation_type_name: Some("Доставка".to_string()),
            is_return: false,
            currency_code: Some(currency_code.to_string()),
            payload_version: 1,
            extra: Some(serde_json::to_string(&row_for_json).unwrap_or_default()),
        };

        service::upsert_realization_row(entry).await?;
        if is_new { inserted += 1; } else { updated += 1; }
    }

    // Обрабатываем return_commission (возврат)
    if let Some(ref rc) = row.return_commission {
        let existing = repository::get_by_id(&posting_number, &sku, "return").await?;
        let is_new = existing.is_none();

        let entry = repository::OzonFinanceRealizationEntry {
            posting_number: posting_number.clone(),
            sku: sku.clone(),
            document_type: "OZON_Finance_Realization".to_string(),
            registrator_ref: registrator_ref.to_string(),
            connection_mp_ref: connection.base.id.as_string(),
            organization_ref: organization_id.to_string(),
            posting_ref: None,
            accrual_date,
            operation_date: None,
            delivery_date: None,
            delivery_schema: None,
            delivery_region: None,
            delivery_city: None,
            quantity: -rc.quantity,
            price: row.seller_price_per_instance.map(|p| -p),
            amount: -rc.amount,
            commission_amount: Some(-rc.commission),
            commission_percent: if rc.amount > 0.0 {
                Some((rc.commission / rc.amount) * 100.0)
            } else {
                None
            },
            services_amount: Some(-rc.standard_fee),
            payout_amount: Some(-rc.total),
            operation_type: "return".to_string(),
            operation_type_name: Some("Возврат".to_string()),
            is_return: true,
            currency_code: Some(currency_code.to_string()),
            payload_version: 1,
            extra: Some(serde_json::to_string(&row_for_json).unwrap_or_default()),
        };

        service::upsert_realization_row(entry).await?;
        if is_new { inserted += 1; } else { updated += 1; }
    }

    Ok((inserted, updated))
}

