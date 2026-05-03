pub mod config_helpers;
pub mod u501_import_ut;
pub mod u502_import_ozon;
pub mod u503_import_yandex;

// WB atomic task managers
pub mod task001_wb_orders_fbs_polling;
pub mod task002_wb_orders_stats_hourly;
pub mod task003_wb_products;
pub mod task004_wb_sales;
pub mod task005_wb_supplies;
pub mod task006_wb_finance;
pub mod task007_wb_commissions;
pub mod task008_wb_prices;
pub mod task009_wb_promotions;
pub mod task010_wb_documents;
pub mod task011_wb_advert;
pub mod task012_wb_advert_campaigns;

pub use u501_import_ut::U501ImportUtManager;
pub use u502_import_ozon::U502ImportOzonManager;
pub use u503_import_yandex::U503ImportYandexManager;

pub use task001_wb_orders_fbs_polling::Task001WbOrdersFbsPollingManager;
pub use task002_wb_orders_stats_hourly::Task002WbOrdersStatsHourlyManager;
pub use task003_wb_products::Task003WbProductsManager;
pub use task004_wb_sales::Task004WbSalesManager;
pub use task005_wb_supplies::Task005WbSuppliesManager;
pub use task006_wb_finance::Task006WbFinanceManager;
pub use task007_wb_commissions::Task007WbCommissionsManager;
pub use task008_wb_prices::Task008WbPricesManager;
pub use task009_wb_promotions::Task009WbPromotionsManager;
pub use task010_wb_documents::Task010WbDocumentsManager;
pub use task011_wb_advert::Task011WbAdvertManager;
pub use task012_wb_advert_campaigns::Task012WbAdvertCampaignsManager;
