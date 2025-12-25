use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use crate::domain::{a007_marketplace_product, a008_marketplace_sales};
use std::collections::HashMap;

/// Обработать одну строку продажи (item) из финансовой операции
pub async fn process_sale_item(
    connection: &ConnectionMP,
    organization_id: &str,
    sku_to_product_id: &mut HashMap<String, String>,
    accrual_date: chrono::NaiveDate,
    operation_type: &str,
    key: &str,
    qty: i32,
    revenue: f64,
) -> Result<bool> {
    // Получаем или создаем product_id в a007
    let product_id = if let Some(pid) = sku_to_product_id.get(key) {
        pid.clone()
    } else {
        let existing =
            a007_marketplace_product::repository::get_by_connection_and_sku(
                &connection.base.id.as_string(),
                key,
            )
            .await?;
        let pid = if let Some(mp) = existing {
            mp.to_string_id()
        } else {
            let mut new = contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct::new_for_insert(
                key.to_string(),
                key.to_string(),
                connection.marketplace_id.clone(),
                connection.base.id.as_string(),
                key.to_string(),
                None,
                key.to_string(),
                None,
                None,
                None,
                Some(chrono::Utc::now()),
                None,
                Some("auto-created from finance operation".to_string()),
            );
            // Автоматический поиск номенклатуры по артикулу
            let _ = a007_marketplace_product::service::search_and_set_nomenclature(
                &mut new,
            )
            .await;
            let id = a007_marketplace_product::repository::insert(&new).await?;
            id.to_string()
        };
        sku_to_product_id.insert(key.to_string(), pid.clone());
        pid
    };

    // Читаем существующую запись по ключу (включая operation_type)
    let existing = a008_marketplace_sales::repository::get_by_key(
        &connection.base.id.as_string(),
        &product_id,
        accrual_date,
        operation_type,
    )
    .await?;

    if let Some(mut sale) = existing {
        sale.quantity += qty;
        sale.revenue += revenue;
        sale.before_write();
        a008_marketplace_sales::repository::update(&sale).await?;
        Ok(false) // Updated
    } else {
        let dto = contracts::domain::a008_marketplace_sales::aggregate::MarketplaceSalesDto {
            id: None,
            code: None,
            description: format!("{} {}", operation_type, key),
            connection_id: connection.base.id.as_string(),
            organization_id: organization_id.to_string(),
            marketplace_id: connection.marketplace_id.clone(),
            accrual_date,
            product_id: product_id.clone(),
            quantity: qty,
            revenue,
            operation_type: operation_type.to_string(),
            comment: None,
        };
        let _ = a008_marketplace_sales::service::create(dto).await?;
        Ok(true) // Inserted
    }
}

