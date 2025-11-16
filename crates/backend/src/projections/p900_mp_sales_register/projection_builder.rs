use super::repository::SalesRegisterEntry;
use crate::domain::a007_marketplace_product::service::{find_or_create_for_sale, get_by_id, FindOrCreateParams};
use contracts::domain::a009_ozon_returns::aggregate::OzonReturns;
use contracts::domain::a010_ozon_fbs_posting::aggregate::OzonFbsPosting;
use contracts::domain::a011_ozon_fbo_posting::aggregate::OzonFboPosting;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::a013_ym_order::aggregate::YmOrder;
use uuid::Uuid;

/// Helper функция для получения nomenclature_ref из marketplace_product
async fn get_nomenclature_ref(marketplace_product_id: Uuid) -> anyhow::Result<Option<String>> {
    if let Some(mp_product) = get_by_id(marketplace_product_id).await? {
        Ok(mp_product.nomenclature_ref)
    } else {
        Ok(None)
    }
}

/// Конвертировать OZON FBS Posting в записи Sales Register
pub async fn from_ozon_fbs(document: &OzonFbsPosting, document_id: &str) -> anyhow::Result<Vec<SalesRegisterEntry>> {
    let mut entries = Vec::new();

    for (_idx, line) in document.lines.iter().enumerate() {
        let event_time = document
            .state
            .delivered_at
            .unwrap_or_else(|| document.source_meta.fetched_at);

        // Поиск или создание a007
        let marketplace_product_ref = find_or_create_for_sale(FindOrCreateParams {
            marketplace_ref: document.header.marketplace_id.clone(),
            connection_mp_ref: document.header.connection_id.clone(),
            marketplace_sku: line.offer_id.clone(),
            barcode: line.barcode.clone(),
            title: line.name.clone(),
        })
        .await?;

        // Получить nomenclature_ref из a007
        let nomenclature_ref = get_nomenclature_ref(marketplace_product_ref).await?;

        let entry = SalesRegisterEntry {
            // NK
            marketplace: "OZON".to_string(),
            document_no: document.header.document_no.clone(),
            line_id: line.line_id.clone(),

            // Metadata
            scheme: Some("FBS".to_string()),
            document_type: "OZON_FBS_Posting".to_string(),
            document_version: document.source_meta.document_version,

            // References to aggregates
            connection_mp_ref: document.header.connection_id.clone(),
            organization_ref: document.header.organization_id.clone(),
            marketplace_product_ref: Some(marketplace_product_ref.to_string()),
            nomenclature_ref,
            registrator_ref: document_id.to_string(),

            // Timestamps and status
            event_time_source: event_time,
            sale_date: event_time.date_naive(),
            source_updated_at: document.state.updated_at_source,
            status_source: document.state.status_raw.clone(),
            status_norm: document.state.status_norm.clone(),

            // Product identification
            seller_sku: Some(line.offer_id.clone()),
            mp_item_id: line.product_id.to_string(),
            barcode: line.barcode.clone(),
            title: Some(line.name.clone()),

            // Quantities and money
            qty: line.qty,
            price_list: line.price_list,
            discount_total: line.discount_total,
            price_effective: line.price_effective,
            amount_line: line.amount_line,
            currency_code: line.currency_code.clone(),

            // Technical
            payload_version: 1,
            extra: None,
        };
        entries.push(entry);
    }

    Ok(entries)
}

/// Конвертировать OZON FBO Posting в записи Sales Register
pub async fn from_ozon_fbo(document: &OzonFboPosting, document_id: &str) -> anyhow::Result<Vec<SalesRegisterEntry>> {
    let mut entries = Vec::new();

    for (_idx, line) in document.lines.iter().enumerate() {
        let event_time = document
            .state
            .delivered_at
            .unwrap_or_else(|| document.source_meta.fetched_at);

        // Поиск или создание a007
        let marketplace_product_ref = find_or_create_for_sale(FindOrCreateParams {
            marketplace_ref: document.header.marketplace_id.clone(),
            connection_mp_ref: document.header.connection_id.clone(),
            marketplace_sku: line.offer_id.clone(),
            barcode: line.barcode.clone(),
            title: line.name.clone(),
        })
        .await?;

        // Получить nomenclature_ref из a007
        let nomenclature_ref = get_nomenclature_ref(marketplace_product_ref).await?;

        let entry = SalesRegisterEntry {
            // NK
            marketplace: "OZON".to_string(),
            document_no: document.header.document_no.clone(),
            line_id: line.line_id.clone(),

            // Metadata
            scheme: Some("FBO".to_string()),
            document_type: "OZON_FBO_Posting".to_string(),
            document_version: document.source_meta.document_version,

            // References to aggregates
            connection_mp_ref: document.header.connection_id.clone(),
            organization_ref: document.header.organization_id.clone(),
            marketplace_product_ref: Some(marketplace_product_ref.to_string()),
            nomenclature_ref,
            registrator_ref: document_id.to_string(),

            // Timestamps and status
            event_time_source: event_time,
            sale_date: event_time.date_naive(),
            source_updated_at: document.state.updated_at_source,
            status_source: document.state.status_raw.clone(),
            status_norm: document.state.status_norm.clone(),

            // Product identification
            seller_sku: Some(line.offer_id.clone()),
            mp_item_id: line.product_id.to_string(),
            barcode: line.barcode.clone(),
            title: Some(line.name.clone()),

            // Quantities and money
            qty: line.qty,
            price_list: line.price_list,
            discount_total: line.discount_total,
            price_effective: line.price_effective,
            amount_line: line.amount_line,
            currency_code: line.currency_code.clone(),

            // Technical
            payload_version: 1,
            extra: None,
        };
        entries.push(entry);
    }

    Ok(entries)
}

/// Конвертировать WB Sales в запись Sales Register
pub async fn from_wb_sales(document: &WbSales, document_id: &str) -> anyhow::Result<SalesRegisterEntry> {
    let event_time = document.state.sale_dt;

    // Поиск или создание a007
    let marketplace_product_ref = find_or_create_for_sale(FindOrCreateParams {
        marketplace_ref: document.header.marketplace_id.clone(),
        connection_mp_ref: document.header.connection_id.clone(),
        marketplace_sku: document.line.supplier_article.clone(),
        barcode: Some(document.line.barcode.clone()),
        title: document.line.name.clone(),
    })
    .await?;

    // Получить nomenclature_ref из a007
    let nomenclature_ref = get_nomenclature_ref(marketplace_product_ref).await?;

    Ok(SalesRegisterEntry {
        // NK
        marketplace: "WB".to_string(),
        document_no: document.header.document_no.clone(),
        line_id: document.line.line_id.clone(), // В WB line_id совпадает с document_no (srid)

        // Metadata
        scheme: None,
        document_type: "WB_Sales".to_string(),
        document_version: document.source_meta.document_version,

        // References to aggregates
        connection_mp_ref: document.header.connection_id.clone(),
        organization_ref: document.header.organization_id.clone(),
        marketplace_product_ref: Some(marketplace_product_ref.to_string()),
        nomenclature_ref,
        registrator_ref: document_id.to_string(),

        // Timestamps and status
        event_time_source: event_time,
        sale_date: event_time.date_naive(),
        source_updated_at: document.state.last_change_dt,
        status_source: document.state.event_type.clone(),
        status_norm: document.state.status_norm.clone(),

        // Product identification
        seller_sku: Some(document.line.supplier_article.clone()),
        mp_item_id: document.line.nm_id.to_string(),
        barcode: Some(document.line.barcode.clone()),
        title: Some(document.line.name.clone()),

        // Quantities and money
        qty: document.line.qty,
        price_list: document.line.price_list,
        discount_total: document.line.discount_total,
        price_effective: document.line.price_effective,
        amount_line: document.line.amount_line,
        currency_code: document.line.currency_code.clone(),

        // Technical
        payload_version: 1,
        extra: None,
    })
}

/// Конвертировать YM Order в записи Sales Register
/// ВАЖНО: Записи создаются только если заполнена delivery_date!
pub async fn from_ym_order(document: &YmOrder, document_id: &str) -> anyhow::Result<Vec<SalesRegisterEntry>> {
    let mut entries = Vec::new();

    // Проверяем наличие delivery_date - без нее не проецируем
    let delivery_date = match document.state.delivery_date {
        Some(date) => date,
        None => {
            // Нет даты доставки - не является продажей, пропускаем
            return Ok(entries);
        }
    };

    for line in document.lines.iter() {
        // Используем delivery_date как дату события
        let event_time = delivery_date;

        // Поиск или создание a007
        let marketplace_product_ref = find_or_create_for_sale(FindOrCreateParams {
            marketplace_ref: document.header.marketplace_id.clone(),
            connection_mp_ref: document.header.connection_id.clone(),
            marketplace_sku: line.shop_sku.clone(),
            barcode: None, // YM не предоставляет barcode в заказах
            title: line.name.clone(),
        })
        .await?;

        // Получить nomenclature_ref из a007
        let nomenclature_ref = get_nomenclature_ref(marketplace_product_ref).await?;

        let entry = SalesRegisterEntry {
            // NK
            marketplace: "YM".to_string(),
            document_no: document.header.document_no.clone(),
            line_id: line.line_id.clone(),

            // Metadata
            scheme: None,
            document_type: "YM_Order".to_string(),
            document_version: document.source_meta.document_version,

            // References to aggregates
            connection_mp_ref: document.header.connection_id.clone(),
            organization_ref: document.header.organization_id.clone(),
            marketplace_product_ref: Some(marketplace_product_ref.to_string()),
            nomenclature_ref,
            registrator_ref: document_id.to_string(),

            // Timestamps and status
            event_time_source: event_time,
            sale_date: event_time.date_naive(),
            source_updated_at: document.state.updated_at_source,
            status_source: document.state.status_raw.clone(),
            status_norm: document.state.status_norm.clone(),

            // Product identification
            seller_sku: Some(line.shop_sku.clone()),
            mp_item_id: line.shop_sku.clone(),
            barcode: None,
            title: Some(line.name.clone()),

            // Quantities and money
            qty: line.qty,
            price_list: line.price_list,
            discount_total: line.discount_total,
            price_effective: line.price_effective,
            amount_line: line.amount_line,
            currency_code: line.currency_code.clone(),

            // Technical
            payload_version: 1,
            extra: None,
        };
        entries.push(entry);
    }

    Ok(entries)
}

/// Конвертировать OZON Returns (возвраты) в запись Sales Register
/// ВАЖНО: Количество и сумма будут ОТРИЦАТЕЛЬНЫМИ (возврат = минус продажи)
pub async fn from_ozon_returns(document: &OzonReturns, document_id: &str) -> anyhow::Result<SalesRegisterEntry> {
    // Используем return_date как дату события (конвертируем NaiveDate в DateTime)
    let event_time = document
        .return_date
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    // Поиск или создание a007
    let marketplace_product_ref = find_or_create_for_sale(FindOrCreateParams {
        marketplace_ref: document.marketplace_id.clone(),
        connection_mp_ref: document.connection_id.clone(),
        marketplace_sku: document.sku.clone(),
        barcode: None, // A009 не имеет barcode
        title: document.product_name.clone(),
    })
    .await?;

    // Получить nomenclature_ref из a007
    let nomenclature_ref = get_nomenclature_ref(marketplace_product_ref).await?;

    // Вычисляем отрицательные значения для возврата
    let qty_negative = -(document.quantity as f64);
    let amount_negative = -(document.price * document.quantity as f64);

    Ok(SalesRegisterEntry {
        // NK - используем return_id в качестве document_no и line_id
        marketplace: "OZON".to_string(),
        document_no: document.return_id.clone(),
        line_id: document.return_id.clone(),

        // Metadata
        scheme: Some("RETURN".to_string()),
        document_type: "OZON_Returns".to_string(),
        document_version: 1,

        // References to aggregates
        connection_mp_ref: document.connection_id.clone(),
        organization_ref: document.organization_id.clone(),
        marketplace_product_ref: Some(marketplace_product_ref.to_string()),
        nomenclature_ref,
        registrator_ref: document_id.to_string(),

        // Timestamps and status
        event_time_source: event_time,
        sale_date: document.return_date,
        source_updated_at: None,
        status_source: document.return_type.clone(),
        status_norm: "RETURNED".to_string(),

        // Product identification
        seller_sku: Some(document.sku.clone()),
        mp_item_id: document.sku.clone(),
        barcode: None,
        title: Some(document.product_name.clone()),

        // Quantities and money - ОТРИЦАТЕЛЬНЫЕ!
        qty: qty_negative,
        price_list: None,
        discount_total: None,
        price_effective: Some(document.price),
        amount_line: Some(amount_negative),
        currency_code: Some("RUB".to_string()),

        // Technical
        payload_version: 1,
        extra: None,
    })
}
