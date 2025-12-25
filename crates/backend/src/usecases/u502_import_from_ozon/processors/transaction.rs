use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use crate::domain::a014_ozon_transactions;
use contracts::domain::a014_ozon_transactions::aggregate::{
    OzonTransactions, OzonTransactionsHeader, OzonTransactionsItem,
    OzonTransactionsPosting, OzonTransactionsService, OzonTransactionsSourceMeta,
};
use super::super::ozon_api_client::OzonTransactionOperation;

pub async fn process_transaction(
    connection: &ConnectionMP,
    organization_id: &str,
    operation: &OzonTransactionOperation,
) -> Result<bool> {
    let code = format!("OZON-TXN-{}", operation.operation_id);
    let description = format!(
        "{} - {}",
        operation.operation_type_name, operation.posting.posting_number
    );

    // Собираем header
    let header = OzonTransactionsHeader {
        operation_id: operation.operation_id,
        operation_type: operation.operation_type.clone(),
        operation_date: operation.operation_date.clone(),
        operation_type_name: operation.operation_type_name.clone(),
        delivery_charge: operation.delivery_charge,
        return_delivery_charge: operation.return_delivery_charge,
        accruals_for_sale: operation.accruals_for_sale,
        sale_commission: operation.sale_commission,
        amount: operation.amount,
        transaction_type: operation.transaction_type.clone(),
        connection_id: connection.base.id.as_string(),
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
    };

    // Собираем posting
    let posting = OzonTransactionsPosting {
        delivery_schema: operation.posting.delivery_schema.clone(),
        order_date: operation.posting.order_date.clone(),
        posting_number: operation.posting.posting_number.clone(),
        warehouse_id: operation.posting.warehouse_id,
    };

    // Собираем items
    let items: Vec<OzonTransactionsItem> = operation
        .items
        .iter()
        .map(|item| OzonTransactionsItem {
            name: item.name.clone(),
            sku: item.sku,
            price: None,
            ratio: None,
            marketplace_product_ref: None,
            nomenclature_ref: None,
        })
        .collect();

    // Собираем services
    let services: Vec<OzonTransactionsService> = operation
        .services
        .iter()
        .map(|service| OzonTransactionsService {
            name: service.name.clone(),
            price: service.price,
        })
        .collect();

    // Source meta
    let source_meta = OzonTransactionsSourceMeta {
        raw_payload_ref: format!("ozon_txn_{}", operation.operation_id),
        fetched_at: chrono::Utc::now(),
        document_version: 1,
    };

    // Создаем агрегат
    let aggregate = OzonTransactions::new_for_insert(
        code,
        description,
        header,
        posting,
        items,
        services,
        source_meta,
        false, // is_posted = false по умолчанию
    );

    // Upsert по operation_id
    let existing = a014_ozon_transactions::repository::get_by_operation_id(
        aggregate.header.operation_id,
    )
    .await?;
    
    let is_new = existing.is_none();
    a014_ozon_transactions::repository::upsert_by_operation_id(&aggregate).await?;
    
    Ok(is_new)
}

