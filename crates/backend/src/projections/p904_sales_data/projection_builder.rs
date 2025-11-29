use anyhow::Result;
use chrono::Utc;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::a014_ozon_transactions::aggregate::OzonTransactions;
use uuid::Uuid;

use super::repository::Model;

/// Константа эквайринга ПРОДАЖИ (1.9%)
const ACQUIRING_RATE: f64 = 0.019;

/// Константа эквайринга ВОЗВРАТА (0.53%)
const ACQUIRING_RETURN_RATE: f64 = 0.0053;

pub async fn from_wb_sales_lines(document: &WbSales, document_id: &str) -> Result<Vec<Model>> {
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    // Получаем значения из строки документа
    let total_price = document.line.total_price.unwrap_or(0.0);
    let price_list = document.line.price_list.unwrap_or(0.0);
    let finished_price = document.line.finished_price.unwrap_or(0.0);
    let amount_line = document.line.amount_line.unwrap_or(0.0);
    let price_effective = document.line.price_effective.unwrap_or(0.0);
    let spp = document.line.spp.unwrap_or(0.0);

    // Расчёт сумм по формулам:
    // 1. total_price -> price_full
    let price_full = total_price;

    // 2. price_list -> price_list (без изменений)
    // let price_list = price_list;

    // 3. Если price_effective > 0, то customer_in = finished_price, иначе customer_out = finished_price
    let (customer_in, customer_out, price_return) = if price_effective > 0.0 {
        (finished_price, 0.0, 0.0)
    } else {
        (0.0, finished_price, price_list)
    };

    // 4. amount_line - price_effective -> commission_out
    let commission_out = amount_line - price_effective;

    // 5. spp -> coinvest_persent
    let coinvest_persent = spp;

    // 6. commission_out / price_effective * 100 -> commission_percent (округляем до 2 знаков)
    let commission_percent = if finished_price != 0.0 {
        ((commission_out / price_effective * 100.0) * 100.0).round() / 100.0
    } else {
        0.0
    };

    // 7. finished_price * ACQUIRING_RATE * -1 -> acquiring_out (со знаком минус)
    let acquiring_out = if finished_price > 0.0 {
        // ПРОДАЖА
        -(finished_price * ACQUIRING_RATE)
    } else {
        // ВОЗВРАТ
        finished_price * ACQUIRING_RETURN_RATE
    };

    // 8. amount_line - finished_price -> если > 0, то coinvest_in, иначе 0
    let diff = amount_line - finished_price;
    let coinvest_in = if diff > 0.0 && price_effective > 0.0 {
        diff
    } else if diff < 0.0 && price_effective < 0.0 {
        diff
    } else {
        0.0
    };

    // 9. total = amount_line + acquiring_out + commission_out
    // Разобраться
    let discount_spp = price_effective - finished_price;
    let total = amount_line + acquiring_out + commission_out + discount_spp;

    // 10. seller_out = (customer_out + customer_in) - (acquiring_out + coinvest_in + commission_out)
    let seller_out = -(customer_out + customer_in) - (acquiring_out + coinvest_in + commission_out);

    let entry = Model {
        id,
        registrator_ref: document_id.to_string(),
        registrator_type: "WB_Sales".to_string(),
        date: document.state.sale_dt.to_rfc3339(),
        connection_mp_ref: document.header.connection_id.clone(),
        nomenclature_ref: document.nomenclature_ref.clone().unwrap_or_default(),
        marketplace_product_ref: document.marketplace_product_ref.clone().unwrap_or_default(),

        // Sums - рассчитанные по формулам
        customer_in,
        customer_out,
        coinvest_in,
        commission_out,
        acquiring_out,
        penalty_out: 0.0,
        logistics_out: 0.0,
        seller_out,
        price_full,
        price_list,
        price_return,
        commission_percent,
        coinvest_persent,
        total,

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

    // Вычисляем logistics_total как сумму ВСЕХ сервисов
    let logistics_total: f64 = document.services.iter().map(|s| s.price).sum();

    // Получаем значения из header для распределения
    let sale_commission = document.header.sale_commission;
    let amount = document.header.amount;

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

        // Создаем базовые записи - распределяем равномерно по items
        let items_count = document.items.len() as f64;
        let proportion = if items_count > 0.0 {
            1.0 / items_count
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

            // Вычисляем суммы пропорционально
            let customer_in = document.header.accruals_for_sale * proportion;
            let commission_out = sale_commission * proportion;
            let logistics_out = logistics_total * proportion;
            let seller_out = -amount * proportion;
            let total = customer_in + commission_out + logistics_out;

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
                commission_out,
                acquiring_out: 0.0,
                penalty_out: 0.0,
                logistics_out,
                seller_out,
                price_full: 0.0,
                price_list: 0.0,
                price_return: 0.0,
                commission_percent: 0.0,
                coinvest_persent: 0.0,
                total,

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
            // Вычисляем пропорциональную долю
            let proportion = if total_amount > 0.0 {
                line.amount_line.unwrap_or(0.0) / total_amount
            } else {
                1.0 / fbs_doc.lines.len() as f64
            };

            // Вычисляем суммы пропорционально
            let customer_in = document.header.accruals_for_sale * proportion;
            let commission_out = sale_commission * proportion;
            let logistics_out = logistics_total * proportion;
            let seller_out = -amount * proportion;
            let total = customer_in + commission_out + logistics_out;

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
                commission_out,
                acquiring_out: 0.0,
                penalty_out: 0.0,
                logistics_out,
                seller_out,
                price_full: line.price_list.unwrap_or(0.0) * line.qty as f64,
                price_list: line.price_list.unwrap_or(0.0),
                price_return: 0.0,
                commission_percent: 0.0,
                coinvest_persent: 0.0,
                total,

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
            // Вычисляем пропорциональную долю
            let proportion = if total_amount > 0.0 {
                line.amount_line.unwrap_or(0.0) / total_amount
            } else {
                1.0 / fbo_doc.lines.len() as f64
            };

            // Вычисляем суммы пропорционально
            let customer_in = document.header.accruals_for_sale * proportion;
            let commission_out = sale_commission * proportion;
            let logistics_out = logistics_total * proportion;
            let seller_out = -amount * proportion;
            let total = customer_in + commission_out + logistics_out;

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
                commission_out,
                acquiring_out: 0.0,
                penalty_out: 0.0,
                logistics_out,
                seller_out,
                price_full: line.price_list.unwrap_or(0.0) * line.qty as f64,
                price_list: line.price_list.unwrap_or(0.0),
                price_return: 0.0,
                commission_percent: 0.0,
                coinvest_persent: 0.0,
                total,

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
