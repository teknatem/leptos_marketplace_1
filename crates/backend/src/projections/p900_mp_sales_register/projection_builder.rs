use super::repository::SalesRegisterEntry;
use contracts::domain::a010_ozon_fbs_posting::aggregate::OzonFbsPosting;
use contracts::domain::a011_ozon_fbo_posting::aggregate::OzonFboPosting;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::a013_ym_order::aggregate::YmOrder;

/// Конвертировать OZON FBS Posting в записи Sales Register
pub fn from_ozon_fbs(document: &OzonFbsPosting) -> Vec<SalesRegisterEntry> {
    let mut entries = Vec::new();

    for (_idx, line) in document.lines.iter().enumerate() {
        let event_time = document
            .state
            .delivered_at
            .unwrap_or_else(|| document.source_meta.fetched_at);

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
            marketplace_product_ref: None, // TODO: должно заполняться при сопоставлении с a007
            registrator_ref: document.source_meta.raw_payload_ref.clone(),

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

    entries
}

/// Конвертировать OZON FBO Posting в записи Sales Register
pub fn from_ozon_fbo(document: &OzonFboPosting) -> Vec<SalesRegisterEntry> {
    let mut entries = Vec::new();

    for (_idx, line) in document.lines.iter().enumerate() {
        let event_time = document
            .state
            .delivered_at
            .unwrap_or_else(|| document.source_meta.fetched_at);

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
            marketplace_product_ref: None, // TODO: должно заполняться при сопоставлении с a007
            registrator_ref: document.source_meta.raw_payload_ref.clone(),

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

    entries
}

/// Конвертировать WB Sales в запись Sales Register
pub fn from_wb_sales(document: &WbSales) -> SalesRegisterEntry {
    let event_time = document.state.sale_dt;

    SalesRegisterEntry {
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
        marketplace_product_ref: None, // TODO: должно заполняться при сопоставлении с a007
        registrator_ref: document.source_meta.raw_payload_ref.clone(),

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
    }
}

/// Конвертировать YM Order в записи Sales Register
pub fn from_ym_order(document: &YmOrder) -> Vec<SalesRegisterEntry> {
    let mut entries = Vec::new();

    for line in document.lines.iter() {
        let event_time = document
            .state
            .status_changed_at
            .unwrap_or_else(|| document.source_meta.fetched_at);

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
            marketplace_product_ref: None, // TODO: должно заполняться при сопоставлении с a007
            registrator_ref: document.source_meta.raw_payload_ref.clone(),

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

    entries
}
