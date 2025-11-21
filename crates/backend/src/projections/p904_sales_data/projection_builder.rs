use anyhow::Result;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use uuid::Uuid;
use chrono::Utc;

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
        
        // Sums - initially 0 as requested
        customer_in: 0.0,
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
        
        document_no: document.header.document_no.clone(),
        article: document.line.supplier_article.clone(),
        posted_at: now,
    };
    
    Ok(vec![entry])
}
