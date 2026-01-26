// Aggregate handlers (a001-a019)
pub mod a001_connection_1c;
pub mod a002_organization;
pub mod a003_counterparty;
pub mod a004_nomenclature;
pub mod a005_marketplace;
pub mod a006_connection_mp;
pub mod a007_marketplace_product;
pub mod a008_marketplace_sales;
pub mod a009_ozon_returns;
pub mod a010_ozon_fbs_posting;
pub mod a011_ozon_fbo_posting;
pub mod a012_wb_sales;
pub mod a013_ym_order;
pub mod a014_ozon_transactions;
pub mod a015_wb_orders;
pub mod a016_ym_returns;
pub mod a017_llm_agent;
pub mod a018_llm_chat;
pub mod a019_llm_artifact;

// Projection handlers (p900-p906)
pub mod p900_mp_sales_register;
pub mod p901_nomenclature_barcodes;
pub mod p902_ozon_finance_realization;
pub mod p903_wb_finance_report;
pub mod p904_sales_data;
pub mod p905_wb_commission_history;
pub mod p906_nomenclature_prices;

// Dashboard handlers (d400-d401)
pub mod d400_monthly_summary;
pub mod d401_wb_finance;

// UseCase handlers
pub mod usecases;
