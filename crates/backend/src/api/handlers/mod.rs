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
pub mod a020_wb_promotion;
pub mod a021_production_output;
pub mod a022_kit_variant;
pub mod a023_purchase_of_goods;
pub mod a024_bi_indicator;
pub mod a025_bi_dashboard;
pub mod a026_wb_advert_daily;
pub mod a027_wb_documents;
pub mod a028_missing_cost_registry;
pub mod a029_wb_supply;
pub mod a030_wb_advert_campaign;

// Projection handlers (p900-p908)
pub mod p900_mp_sales_register;
pub mod p901_nomenclature_barcodes;
pub mod p902_ozon_finance_realization;
pub mod p903_wb_finance_report;
pub mod p904_sales_data;
pub mod p905_wb_commission_history;
pub mod p906_nomenclature_prices;
pub mod p907_ym_payment_report;
pub mod p908_wb_goods_prices;
pub mod p909_mp_order_line_turnovers;
pub mod p910_mp_unlinked_turnovers;
pub mod p911_wb_advert_by_items;
pub mod p912_nomenclature_costs;

// DataView semantic layer handlers
pub mod data_view;

// Drilldown session store
pub mod sys_drilldown;

// System journal (общий журнал операций)
pub mod general_ledger;

// Dashboard handlers (d400)
pub mod d400_monthly_summary;

// Data scheme handlers (ds01-ds02)
pub mod ds01_wb_finance_report;
pub mod ds02_mp_sales_register;

// UseCase handlers
pub mod usecases;

// External integration API (1C, etc.)
pub mod ext_1c_wb_supply;

// Debug endpoints (dev only)
pub mod debug;
pub mod llm_knowledge;
