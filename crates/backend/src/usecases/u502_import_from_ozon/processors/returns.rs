use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use crate::domain::a009_ozon_returns;

/// Обработать одну строку возврата
pub async fn process_return_item(
    connection: &ConnectionMP,
    organization_id: &str,
    return_id_str: &str,
    return_date: chrono::NaiveDate,
    return_reason: &str,
    return_type: &str,
    order_id_str: &str,
    order_number: &str,
    posting_number: &str,
    clearing_id_str: &Option<String>,
    return_clearing_id_str: &Option<String>,
    sku_str: &str,
    product_name: &str,
    price: f64,
    quantity: i32,
    display_name: &str,
) -> Result<bool> {
    // Проверяем существует ли возврат по ключу (connection_id, return_id, sku)
    let existing = a009_ozon_returns::repository::get_by_return_key(
        &connection.base.id.as_string(),
        return_id_str,
        sku_str,
    )
    .await?;

    if let Some(mut ozon_return) = existing {
        // Обновляем существующий возврат
        ozon_return.sku = sku_str.to_string();
        ozon_return.product_name = product_name.to_string();
        ozon_return.price = price;
        ozon_return.quantity = quantity;
        ozon_return.return_reason_name = return_reason.to_string();
        ozon_return.return_type = return_type.to_string();
        ozon_return.return_date = return_date;
        ozon_return.order_id = order_id_str.to_string();
        ozon_return.order_number = order_number.to_string();
        ozon_return.posting_number = posting_number.to_string();
        ozon_return.clearing_id = clearing_id_str.clone();
        ozon_return.return_clearing_id = return_clearing_id_str.clone();
        ozon_return.before_write();
        a009_ozon_returns::repository::update(&ozon_return).await?;
        Ok(false) // Updated
    } else {
        // Создаем новый возврат
        let dto = contracts::domain::a009_ozon_returns::aggregate::OzonReturnsDto {
            id: None,
            code: None,
            description: display_name.to_string(),
            connection_id: connection.base.id.as_string(),
            organization_id: organization_id.to_string(),
            marketplace_id: connection.marketplace_id.clone(),
            return_id: return_id_str.to_string(),
            return_date,
            return_reason_name: return_reason.to_string(),
            return_type: return_type.to_string(),
            order_id: order_id_str.to_string(),
            order_number: order_number.to_string(),
            sku: sku_str.to_string(),
            product_name: product_name.to_string(),
            price,
            quantity,
            posting_number: posting_number.to_string(),
            clearing_id: clearing_id_str.clone(),
            return_clearing_id: return_clearing_id_str.clone(),
            comment: None,
        };
        let _ = a009_ozon_returns::service::create(dto).await?;
        Ok(true) // Inserted
    }
}

