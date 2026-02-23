use super::super::wildberries_api_client::WbGoodsPriceRow;
use crate::projections::p908_wb_goods_prices::repository::{self, WbGoodsPriceEntry};
use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;

const ZERO_UUID: &str = "00000000-0000-0000-0000-000000000000";

pub async fn process_goods_price(
    connection: &ConnectionMP,
    row: &WbGoodsPriceRow,
) -> Result<()> {
    let connection_mp_ref = connection.base.id.as_string();

    // Extract price from first size (most products have one size)
    let (price, discounted_price) = row
        .sizes
        .first()
        .map(|s| (s.price, s.discounted_price))
        .unwrap_or((None, None));

    let sizes_json = serde_json::to_string(&row.sizes).unwrap_or_else(|_| "[]".to_string());

    // Enrichment: resolve ext_nomenklature_ref and dealer_price_ut
    let (ext_nomenklature_ref, dealer_price_ut, margin_pro) =
        enrich(row.vendor_code.as_deref(), discounted_price).await;

    let entry = WbGoodsPriceEntry {
        nm_id: row.nm_id,
        connection_mp_ref,
        vendor_code: row.vendor_code.clone(),
        discount: row.discount,
        editable_size_price: row.editable_size_price,
        price,
        discounted_price,
        sizes_json,
        ext_nomenklature_ref,
        dealer_price_ut,
        margin_pro,
    };

    repository::upsert_entry(&entry).await?;
    Ok(())
}

/// Resolve enrichment fields for a single goods-price row:
/// 1. Find nomenclature by vendor_code (article)
/// 2. Determine ext_nomenklature_ref (base_ref if valid, else own id)
/// 3. Lookup dealer_price_ut via p906
/// 4. Calculate margin_pro
async fn enrich(
    vendor_code: Option<&str>,
    discounted_price: Option<f64>,
) -> (Option<String>, Option<f64>, Option<f64>) {
    let Some(article) = vendor_code else {
        return (None, None, None);
    };
    let article = article.trim();
    if article.is_empty() {
        return (None, None, None);
    }

    // 1. Look up nomenclature by article
    let nomenclatures = match crate::domain::a004_nomenclature::repository::find_by_article(article).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("p908 enrich: find_by_article({}) failed: {}", article, e);
            return (None, None, None);
        }
    };

    let Some(nom) = nomenclatures.into_iter().next() else {
        return (None, None, None);
    };

    // 2. Resolve ext_nomenklature_ref
    let nom_id = nom.base.id.as_string();
    let ext_ref = match &nom.base_nomenclature_ref {
        Some(base_ref) if !base_ref.is_empty() && base_ref != ZERO_UUID => base_ref.clone(),
        _ => nom_id,
    };

    // 3. Lookup dealer_price_ut by ext_ref using today's date
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let dealer_price = lookup_dealer_price(&ext_ref, &today).await;

    // 4. Calculate margin_pro
    let margin = match (discounted_price, dealer_price) {
        (Some(dp), Some(dealer)) if dealer > 0.0 && dp > 0.0 => {
            Some((dp - dealer) / dealer * 100.0)
        }
        _ => None,
    };

    (Some(ext_ref), dealer_price, margin)
}

/// Lookup price from p906: try price for date, fallback to last nonzero
async fn lookup_dealer_price(ext_ref: &str, date: &str) -> Option<f64> {
    // 1. Price on date
    let price_on_date =
        crate::projections::p906_nomenclature_prices::repository::get_price_for_date(ext_ref, date)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("p908 enrich: get_price_for_date({}) failed: {}", ext_ref, e);
                None
            });

    if let Some(p) = price_on_date {
        if p > 0.0 {
            return Some(p);
        }
    }

    // 2. Fallback: last nonzero price
    crate::projections::p906_nomenclature_prices::repository::get_last_nonzero_price(ext_ref)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("p908 enrich: get_last_nonzero_price({}) failed: {}", ext_ref, e);
            None
        })
}
