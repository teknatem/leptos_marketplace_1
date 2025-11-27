use anyhow::Result;
use chrono::Utc;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::a014_ozon_transactions::aggregate::OzonTransactions;
use uuid::Uuid;

use super::repository::Model;

pub async fn from_wb_sales_lines(document: &WbSales, document_id: &str) -> Result<Vec<Model>> {
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    let entry = Model {
        id,
        registrator_ref: document_id.to_string(),
        registrator_type: "WB_Sales".to_string(),
        date: document.state.sale_dt.to_rfc3339(),
        connection_mp_ref: document.header.connection_id.clone(),
        nomenclature_ref: document.nomenclature_ref.clone().unwrap_or_default(),
        marketplace_product_ref: document.marketplace_product_ref.clone().unwrap_or_default(),

        // Sums - map from WbSalesLine
        customer_in: 0.0,
        customer_out: 0.0,
        coinvest_in: 0.0,
        commission_out: 0.0,
        acquiring_out: document.line.payment_sale_amount.unwrap_or(0.0),
        penalty_out: 0.0,
        logistics_out: 0.0,
        seller_out: 0.0,
        price_full: document.line.total_price.unwrap_or(0.0),
        price_list: document.line.price_list.unwrap_or(0.0),
        price_return: 0.0,
        commission_percent: 0.0,
        coinvest_persent: document.line.spp.unwrap_or(0.0),
        total: 0.0,

        document_no: document.header.document_no.clone(),
        article: document.line.supplier_article.clone(),
        posted_at: now,
    };

    Ok(vec![entry])
}

/// Конвертировать OZON Transactions в записи Sales Data (P904)
pub async fn from_ozon_transactions(
    document: &OzonTransactions,
    document_id: &str,
) -> Result<Vec<Model>> {
    let mut entries = Vec::new();
    let now = Utc::now().to_rfc3339();

    // Если нет items, ничего не создаем
    if document.items.is_empty() {
        tracing::warn!(
            "A014 document {} has no items, skipping P904 projection",
            document.header.operation_id
        );
        return Ok(entries);
    }

    // Пытаемся найти документ отгрузки по posting_number
    let posting_number = &document.posting.posting_number;

    // Сначала пробуем найти A010 (FBS)
    let posting_fbs =
        crate::domain::a010_ozon_fbs_posting::service::get_by_document_no(posting_number).await?;

    // Если не нашли, пробуем A011 (FBO)
    let posting_fbo = if posting_fbs.is_none() {
        crate::domain::a011_ozon_fbo_posting::service::get_by_document_no(posting_number).await?
    } else {
        None
    };

    // Если не нашли ни FBS, ни FBO - логируем и создаем записи с минимальными данными
    if posting_fbs.is_none() && posting_fbo.is_none() {
        tracing::warn!(
            "A014 document {}: posting {} not found in A010/A011, creating basic P904 entries",
            document.header.operation_id,
            posting_number
        );

        // Создаем базовые записи без детализации
        let accruals_per_item = if !document.items.is_empty() {
            document.header.accruals_for_sale / document.items.len() as f64
        } else {
            0.0
        };

        for item in &document.items {
            let sku_str = item.sku.to_string();

            // Найти/создать a007_marketplace_product
            let marketplace_product_ref =
                crate::domain::a007_marketplace_product::service::find_or_create_for_sale(
                    crate::domain::a007_marketplace_product::service::FindOrCreateParams {
                        marketplace_ref: document.header.marketplace_id.clone(),
                        connection_mp_ref: document.header.connection_id.clone(),
                        marketplace_sku: sku_str.clone(),
                        barcode: None,
                        title: item.name.clone(),
                    },
                )
                .await?;

            // Получить nomenclature_ref из a007
            let nomenclature_ref = if let Some(product) =
                crate::domain::a007_marketplace_product::service::get_by_id(marketplace_product_ref)
                    .await?
            {
                product.nomenclature_ref.unwrap_or_default()
            } else {
                String::new()
            };

            let entry = Model {
                id: Uuid::new_v4().to_string(),
                registrator_ref: document_id.to_string(),
                registrator_type: "OZON_Transactions".to_string(),
                date: document.header.operation_date.clone(),
                connection_mp_ref: document.header.connection_id.clone(),
                nomenclature_ref,
                marketplace_product_ref: marketplace_product_ref.to_string(),

                // Sums - только customer_in из accruals_for_sale
                customer_in: accruals_per_item,
                customer_out: 0.0,
                coinvest_in: 0.0,
                commission_out: 0.0,
                acquiring_out: 0.0,
                penalty_out: 0.0,
                logistics_out: 0.0,
                seller_out: 0.0,
                price_full: 0.0,
                price_list: 0.0,
                price_return: 0.0,
                commission_percent: 0.0,
                coinvest_persent: 0.0,
                total: 0.0,

                document_no: posting_number.clone(),
                article: sku_str,
                posted_at: now.clone(),
            };
            entries.push(entry);
        }

        return Ok(entries);
    }

    // Работаем с найденным постингом (FBS или FBO)
    if let Some(fbs_doc) = posting_fbs {
        // FBS документ
        let total_amount: f64 = fbs_doc
            .lines
            .iter()
            .map(|l| l.amount_line.unwrap_or(0.0))
            .sum();

        for line in &fbs_doc.lines {
            // Вычисляем пропорциональную долю accruals_for_sale
            let proportion = if total_amount > 0.0 {
                line.amount_line.unwrap_or(0.0) / total_amount
            } else {
                1.0 / fbs_doc.lines.len() as f64
            };
            let customer_in = document.header.accruals_for_sale * proportion;

            // Найти/создать a007_marketplace_product
            let marketplace_product_ref =
                crate::domain::a007_marketplace_product::service::find_or_create_for_sale(
                    crate::domain::a007_marketplace_product::service::FindOrCreateParams {
                        marketplace_ref: document.header.marketplace_id.clone(),
                        connection_mp_ref: document.header.connection_id.clone(),
                        marketplace_sku: line.offer_id.clone(),
                        barcode: line.barcode.clone(),
                        title: line.name.clone(),
                    },
                )
                .await?;

            // Получить nomenclature_ref из a007
            let nomenclature_ref = if let Some(product) =
                crate::domain::a007_marketplace_product::service::get_by_id(marketplace_product_ref)
                    .await?
            {
                product.nomenclature_ref.unwrap_or_default()
            } else {
                String::new()
            };

            let entry = Model {
                id: Uuid::new_v4().to_string(),
                registrator_ref: document_id.to_string(),
                registrator_type: "OZON_Transactions".to_string(),
                date: document.header.operation_date.clone(),
                connection_mp_ref: document.header.connection_id.clone(),
                nomenclature_ref,
                marketplace_product_ref: marketplace_product_ref.to_string(),

                // Sums
                customer_in,
                customer_out: 0.0,
                coinvest_in: 0.0,
                commission_out: 0.0,
                acquiring_out: 0.0,
                penalty_out: 0.0,
                logistics_out: 0.0,
                seller_out: 0.0,
                price_full: line.price_list.unwrap_or(0.0) * line.qty as f64,
                price_list: line.price_list.unwrap_or(0.0),
                price_return: 0.0,
                commission_percent: 0.0,
                coinvest_persent: 0.0,
                total: 0.0,

                document_no: posting_number.clone(),
                article: line.offer_id.clone(),
                posted_at: now.clone(),
            };
            entries.push(entry);
        }
    } else if let Some(fbo_doc) = posting_fbo {
        // FBO документ
        let total_amount: f64 = fbo_doc
            .lines
            .iter()
            .map(|l| l.amount_line.unwrap_or(0.0))
            .sum();

        for line in &fbo_doc.lines {
            // Вычисляем пропорциональную долю accruals_for_sale
            let proportion = if total_amount > 0.0 {
                line.amount_line.unwrap_or(0.0) / total_amount
            } else {
                1.0 / fbo_doc.lines.len() as f64
            };
            let customer_in = document.header.accruals_for_sale * proportion;

            // Найти/создать a007_marketplace_product
            let marketplace_product_ref =
                crate::domain::a007_marketplace_product::service::find_or_create_for_sale(
                    crate::domain::a007_marketplace_product::service::FindOrCreateParams {
                        marketplace_ref: document.header.marketplace_id.clone(),
                        connection_mp_ref: document.header.connection_id.clone(),
                        marketplace_sku: line.offer_id.clone(),
                        barcode: line.barcode.clone(),
                        title: line.name.clone(),
                    },
                )
                .await?;

            // Получить nomenclature_ref из a007
            let nomenclature_ref = if let Some(product) =
                crate::domain::a007_marketplace_product::service::get_by_id(marketplace_product_ref)
                    .await?
            {
                product.nomenclature_ref.unwrap_or_default()
            } else {
                String::new()
            };

            let entry = Model {
                id: Uuid::new_v4().to_string(),
                registrator_ref: document_id.to_string(),
                registrator_type: "OZON_Transactions".to_string(),
                date: document.header.operation_date.clone(),
                connection_mp_ref: document.header.connection_id.clone(),
                nomenclature_ref,
                marketplace_product_ref: marketplace_product_ref.to_string(),

                // Sums
                customer_in,
                customer_out: 0.0,
                coinvest_in: 0.0,
                commission_out: 0.0,
                acquiring_out: 0.0,
                penalty_out: 0.0,
                logistics_out: 0.0,
                seller_out: 0.0,
                price_full: line.price_list.unwrap_or(0.0) * line.qty as f64,
                price_list: line.price_list.unwrap_or(0.0),
                price_return: 0.0,
                commission_percent: 0.0,
                coinvest_persent: 0.0,
                total: 0.0,

                document_no: posting_number.clone(),
                article: line.offer_id.clone(),
                posted_at: now.clone(),
            };
            entries.push(entry);
        }
    }

    tracing::info!(
        "Created {} P904 entries from A014 document {}",
        entries.len(),
        document.header.operation_id
    );

    Ok(entries)
}
