//! Tab content registry - единственный источник правды для маппинга tab.key → View
//!
//! Этот модуль содержит функцию `render_tab_content`, которая по ключу таба
//! возвращает соответствующий View. Все tab keys собраны здесь в одном месте.

use crate::dashboards::MetadataDashboard;
use crate::dashboards::{D401WbFinanceDashboard, MonthlySummaryDashboard};
use crate::data_view::ui::{DataViewDetail, DataViewList, FilterRegistryPage};
use crate::domain::a001_connection_1c::ui::list::Connection1CList;
use crate::domain::a002_organization::ui::details::OrganizationDetails;
use crate::domain::a002_organization::ui::list::OrganizationList;
use crate::domain::a004_nomenclature::ui::list::NomenclatureList;
use crate::domain::a005_marketplace::ui::details::MarketplaceDetails;
use crate::domain::a005_marketplace::ui::list::MarketplaceList;
use crate::domain::a006_connection_mp::ui::{ConnectionMPDetail, ConnectionMPList};
use crate::domain::a007_marketplace_product::ui::details::MarketplaceProductDetails;
use crate::domain::a007_marketplace_product::ui::list::MarketplaceProductList;
use crate::domain::a008_marketplace_sales::ui::list::MarketplaceSalesList;
use crate::domain::a009_ozon_returns::ui::details::OzonReturnsDetail;
use crate::domain::a009_ozon_returns::ui::list::OzonReturnsList;
use crate::domain::a010_ozon_fbs_posting::ui::list::OzonFbsPostingList;
use crate::domain::a011_ozon_fbo_posting::ui::list::OzonFboPostingList;
use crate::domain::a012_wb_sales::ui::details::WbSalesDetail;
use crate::domain::a012_wb_sales::ui::list::WbSalesList;
use crate::domain::a013_ym_order::ui::details::YmOrderDetail;
use crate::domain::a013_ym_order::ui::list::YmOrderList;
use crate::domain::a014_ozon_transactions::ui::details::OzonTransactionsDetail;
use crate::domain::a014_ozon_transactions::ui::list::OzonTransactionsList;
use crate::domain::a015_wb_orders::ui::details::WbOrdersDetails;
use crate::domain::a015_wb_orders::ui::list::WbOrdersList;
use crate::domain::a016_ym_returns::ui::details::YmReturnDetail;
use crate::domain::a016_ym_returns::ui::list::YmReturnsList;
use crate::domain::a017_llm_agent::ui::list::LlmAgentList;
use crate::domain::a018_llm_chat::ui::details::LlmChatDetails;
use crate::domain::a018_llm_chat::ui::list::LlmChatList;
use crate::domain::a019_llm_artifact::ui::details::LlmArtifactDetails;
use crate::domain::a019_llm_artifact::ui::list::LlmArtifactList;
use crate::domain::a020_wb_promotion::ui::details::WbPromotionDetail;
use crate::domain::a020_wb_promotion::ui::list::WbPromotionList;
use crate::domain::a021_production_output::ui::details::ProductionOutputDetail;
use crate::domain::a021_production_output::ui::list::ProductionOutputList;
use crate::domain::a022_kit_variant::ui::details::KitVariantDetail;
use crate::domain::a022_kit_variant::ui::list::KitVariantList;
use crate::domain::a023_purchase_of_goods::ui::details::PurchaseOfGoodsDetail;
use crate::domain::a023_purchase_of_goods::ui::list::PurchaseOfGoodsList;
use crate::domain::a024_bi_indicator::ui::details::BiIndicatorDetails;
use crate::domain::a024_bi_indicator::ui::list::BiIndicatorList;
use crate::domain::a025_bi_dashboard::ui::dashboard::BiDashboardView;
use crate::domain::a025_bi_dashboard::ui::details::BiDashboardDetails;
use crate::domain::a025_bi_dashboard::ui::list::BiDashboardList;
use crate::domain::a026_wb_advert_daily::ui::details::WbAdvertDailyDetail;
use crate::domain::a026_wb_advert_daily::ui::list::WbAdvertDailyList;
use crate::domain::general_ledger::{GeneralLedgerDetailsPage, GeneralLedgerPage};
use crate::layout::global_context::AppGlobalContext;
use crate::projections::p900_mp_sales_register::ui::list::SalesRegisterList;
use crate::projections::p901_nomenclature_barcodes::ui::list::BarcodesList;
use crate::projections::p902_ozon_finance_realization::ui::list::OzonFinanceRealizationList;
use crate::projections::p903_wb_finance_report::ui::details::WbFinanceReportDetail;
use crate::projections::p903_wb_finance_report::ui::list::WbFinanceReportList;
use crate::projections::p904_sales_data::ui::list::SalesDataList;
use crate::projections::p905_wb_commission_history::ui::details::CommissionHistoryDetails;
use crate::projections::p905_wb_commission_history::ui::list::CommissionHistoryList;
use crate::projections::p906_nomenclature_prices::ui::list::NomenclaturePricesList;
use crate::projections::p907_ym_payment_report::ui::details::YmPaymentReportDetail;
use crate::projections::p907_ym_payment_report::ui::list::YmPaymentReportList;
use crate::projections::p908_wb_goods_prices::WbGoodsPricesList;
use crate::projections::p909_mp_order_line_turnovers::ui::details::MpOrderLineTurnoverDetail;
use crate::projections::p909_mp_order_line_turnovers::ui::list::MpOrderLineTurnoversList;
use crate::projections::p910_mp_unlinked_turnovers::ui::details::MpUnlinkedTurnoverDetail;
use crate::projections::p910_mp_unlinked_turnovers::ui::list::MpUnlinkedTurnoversList;
use crate::projections::p911_wb_advert_by_items::ui::details::WbAdvertByItemDetail;
use crate::projections::p911_wb_advert_by_items::ui::list::WbAdvertByItemsList;
use crate::shared::drilldown_report::DrilldownReportPage;
use crate::shared::universal_dashboard::{SchemaBrowser, UniversalDashboard};
use crate::system::pages::thaw_test::ThawTestPage;
use crate::system::tasks::ui::details::ScheduledTaskDetails;
use crate::system::tasks::ui::list::ScheduledTaskList;
use crate::system::users::ui::details::{CreateUserPage, UserDetailsPage};
use crate::system::users::ui::list::UsersListPage;
use crate::usecases::u501_import_from_ut;
use crate::usecases::u502_import_from_ozon;
use crate::usecases::u503_import_from_yandex;
use crate::usecases::u504_import_from_wildberries;
use crate::usecases::u505_match_nomenclature;
use crate::usecases::u506_import_from_lemanapro;
use crate::usecases::u507_import_from_erp;
use crate::usecases::u508_repost_documents;
use leptos::logging::log;
use leptos::prelude::*;

/// Рендерит контент таба по его ключу.
///
/// # Arguments
/// * `key` - уникальный ключ таба (например "a001_connection_1c", "u501_import_from_ut")
/// * `tabs_store` - контекст для закрытия таба (используется в detail-views с on_close)
///
/// # Returns
/// AnyView с содержимым таба или placeholder для неизвестных ключей
pub fn render_tab_content(key: &str, tabs_store: AppGlobalContext) -> AnyView {
    let key_for_close = key.to_string();

    match key {
        // ═══════════════════════════════════════════════════════════════════
        // Domain Aggregates (a001-a016)
        // ═══════════════════════════════════════════════════════════════════

        // a001: 1C Connections
        "a001_connection_1c" => {
            log!("✅ Creating Connection1CList");
            view! { <Connection1CList /> }.into_any()
        }

        // a002: Organizations
        "a002_organization" => view! { <OrganizationList /> }.into_any(),
        k if k.starts_with("a002_organization_details_") => {
            let id = Some(
                k.strip_prefix("a002_organization_details_")
                    .unwrap()
                    .to_string(),
            );
            view! {
                <OrganizationDetails
                    id=id
                    on_saved=std::rc::Rc::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                    on_cancel=std::rc::Rc::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a003: Counterparties (tree view)
        "a003_counterparty" => {
            view! { <crate::domain::a003_counterparty::ui::tree::CounterpartyTree /> }.into_any()
        }

        // a004: Nomenclature
        "a004_nomenclature" => {
            view! { <crate::domain::a004_nomenclature::ui::tree::NomenclatureTree /> }.into_any()
        }
        "a004_nomenclature_list" => view! { <NomenclatureList /> }.into_any(),
        k if k.starts_with("a004_nomenclature_details_") => {
            let id = k
                .strip_prefix("a004_nomenclature_details_")
                .unwrap()
                .to_string();
            view! {
                <crate::domain::a004_nomenclature::ui::details::NomenclatureDetails
                    id=Some(id)
                    on_saved=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                    on_cancel=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a005: Marketplaces
        "a005_marketplace" => view! { <MarketplaceList /> }.into_any(),
        k if k.starts_with("a005_marketplace_details_") => {
            let id = Some(
                k.strip_prefix("a005_marketplace_details_")
                    .unwrap()
                    .to_string(),
            );
            view! {
                <MarketplaceDetails
                    id=id
                    on_saved=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                    on_cancel=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a006: Marketplace Connections
        "a006_connection_mp" => view! { <ConnectionMPList /> }.into_any(),
        "a006_connection_mp_details" => view! {
            <ConnectionMPDetail
                id=None
                on_close=Callback::new({
                    let key_for_close = key_for_close.clone();
                    move |_| {
                        tabs_store.close_tab(&key_for_close);
                    }
                })
            />
        }
        .into_any(),
        k if k.starts_with("a006_connection_mp_details_") => {
            let id = Some(
                k.strip_prefix("a006_connection_mp_details_")
                    .unwrap()
                    .to_string(),
            );

            view! {
                <ConnectionMPDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a007: Marketplace Products
        "a007_marketplace_product" => view! { <MarketplaceProductList /> }.into_any(),
        k if k.starts_with("a007_marketplace_product_details_") => {
            let id = k
                .strip_prefix("a007_marketplace_product_details_")
                .unwrap()
                .to_string();
            view! {
                <MarketplaceProductDetails
                    id=Some(id)
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }
        "a007_marketplace_product_new" => view! {
            <MarketplaceProductDetails
                id=None
                on_close=Callback::new({
                    let key_for_close = key_for_close.clone();
                    move |_| {
                        tabs_store.close_tab(&key_for_close);
                    }
                })
            />
        }
        .into_any(),

        // a008: Marketplace Sales
        "a008_marketplace_sales" => view! { <MarketplaceSalesList /> }.into_any(),

        // a009: Ozon Returns
        "a009_ozon_returns" => view! { <OzonReturnsList /> }.into_any(),
        k if k.starts_with("a009_ozon_returns_details_") => {
            let id = k
                .strip_prefix("a009_ozon_returns_details_")
                .unwrap()
                .to_string();
            log!("✅ Creating OzonReturnsDetail with id: {}", id);
            view! {
                <OzonReturnsDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a010: Ozon FBS Postings
        "a010_ozon_fbs_posting" => view! { <OzonFbsPostingList /> }.into_any(),

        // a011: Ozon FBO Postings
        "a011_ozon_fbo_posting" => view! { <OzonFboPostingList /> }.into_any(),

        // a012: Wildberries Sales
        "a012_wb_sales" => view! { <WbSalesList /> }.into_any(),
        k if k.starts_with("a012_wb_sales_details_") => {
            let id = k
                .strip_prefix("a012_wb_sales_details_")
                .unwrap()
                .to_string();
            log!("✅ Creating WbSalesDetail with id: {}", id);
            view! {
                <WbSalesDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a013: Yandex Market Orders
        "a013_ym_order" => view! { <YmOrderList /> }.into_any(),
        k if k.starts_with("a013_ym_order_details_") => {
            let id = k
                .strip_prefix("a013_ym_order_details_")
                .unwrap()
                .to_string();
            log!("✅ Creating YmOrderDetail with id: {}", id);
            view! {
                <YmOrderDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a014: Ozon Transactions
        "a014_ozon_transactions" => view! { <OzonTransactionsList /> }.into_any(),
        k if k.starts_with("a014_ozon_transactions_details_") => {
            let id = k
                .strip_prefix("a014_ozon_transactions_details_")
                .unwrap()
                .to_string();
            log!("✅ Creating OzonTransactionsDetail with id: {}", id);
            view! {
                <OzonTransactionsDetail
                    transaction_id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a015: Wildberries Orders
        "a015_wb_orders" => view! { <WbOrdersList /> }.into_any(),
        k if k.starts_with("a015_wb_orders_details_") => {
            let id = k
                .strip_prefix("a015_wb_orders_details_")
                .unwrap()
                .to_string();
            log!("✅ Creating WbOrdersDetails with id: {}", id);
            view! {
                <WbOrdersDetails
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        "a026_wb_advert_daily" => view! { <WbAdvertDailyList /> }.into_any(),
        k if k.starts_with("a026_wb_advert_daily_details_") => {
            let id = k
                .strip_prefix("a026_wb_advert_daily_details_")
                .unwrap()
                .to_string();
            view! {
                <WbAdvertDailyDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a016: Yandex Market Returns
        "a016_ym_returns" => view! { <YmReturnsList /> }.into_any(),
        k if k.starts_with("a016_ym_returns_details_") => {
            let id = k
                .strip_prefix("a016_ym_returns_details_")
                .unwrap()
                .to_string();
            log!("✅ Creating YmReturnDetail with id: {}", id);
            view! {
                <YmReturnDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a017: LLM Agents
        "a017_llm_agent" => view! { <LlmAgentList /> }.into_any(),

        // a018: LLM Chat
        "a018_llm_chat" => view! { <LlmChatList /> }.into_any(),
        k if k.starts_with("a018_llm_chat_details_") => {
            let id = k
                .strip_prefix("a018_llm_chat_details_")
                .unwrap()
                .to_string();
            log!("✅ Creating LlmChatDetails with id: {}", id);
            view! {
                <LlmChatDetails
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a019: LLM Artifacts
        "a019_llm_artifact" => view! { <LlmArtifactList /> }.into_any(),
        k if k.starts_with("a019_llm_artifact_details_") => {
            let id = k
                .strip_prefix("a019_llm_artifact_details_")
                .unwrap()
                .to_string();
            log!("✅ Creating LlmArtifactDetails with id: {}", id);
            view! {
                <LlmArtifactDetails
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a020: WB Calendar Promotions
        // a021: Выпуск продукции
        "a021_production_output" => view! { <ProductionOutputList /> }.into_any(),

        // a022: Варианты комплектации
        "a022_kit_variant" => view! { <KitVariantList /> }.into_any(),
        k if k.starts_with("a022_kit_variant_details_") => {
            let id = k
                .strip_prefix("a022_kit_variant_details_")
                .unwrap()
                .to_string();
            view! {
                <KitVariantDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a023: Приобретение товаров и услуг
        "a023_purchase_of_goods" => view! { <PurchaseOfGoodsList /> }.into_any(),
        k if k.starts_with("a023_purchase_of_goods_details_") => {
            let id = k
                .strip_prefix("a023_purchase_of_goods_details_")
                .unwrap()
                .to_string();
            view! {
                <PurchaseOfGoodsDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }
        k if k.starts_with("a021_production_output_details_") => {
            let id = k
                .strip_prefix("a021_production_output_details_")
                .unwrap()
                .to_string();
            view! {
                <ProductionOutputDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a024: BI Indicators
        "a024_bi_indicator" => view! { <BiIndicatorList /> }.into_any(),
        k if k.starts_with("a024_bi_indicator_details_") => {
            let raw_id = k
                .strip_prefix("a024_bi_indicator_details_")
                .unwrap()
                .to_string();
            let id = if raw_id == "new" { None } else { Some(raw_id) };
            view! {
                <BiIndicatorDetails
                    id=id
                    on_saved=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                    on_cancel=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // a025: BI Dashboards
        "a025_bi_dashboard" => view! { <BiDashboardList /> }.into_any(),
        k if k.starts_with("a025_bi_dashboard_details_") => {
            let raw_id = k
                .strip_prefix("a025_bi_dashboard_details_")
                .unwrap()
                .to_string();
            let id = if raw_id == "new" { None } else { Some(raw_id) };
            view! {
                <BiDashboardDetails
                    id=id
                    on_saved=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                    on_cancel=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }
        k if k.starts_with("a025_bi_dashboard_view_") => {
            let id = k
                .strip_prefix("a025_bi_dashboard_view_")
                .unwrap()
                .to_string();
            view! {
                <BiDashboardView id=id />
            }
            .into_any()
        }

        // DataView semantic layer catalog
        "data_view" => view! { <DataViewList /> }.into_any(),
        "filter_registry" => view! { <FilterRegistryPage /> }.into_any(),
        k if k.starts_with("data_view_details_") => {
            let view_id = k.strip_prefix("data_view_details_").unwrap().to_string();
            view! { <DataViewDetail view_id=view_id /> }.into_any()
        }

        "a020_wb_promotion" => view! { <WbPromotionList /> }.into_any(),
        k if k.starts_with("a020_wb_promotion_details_") => {
            let id = k
                .strip_prefix("a020_wb_promotion_details_")
                .unwrap()
                .to_string();
            log!("✅ Creating WbPromotionDetail with id: {}", id);
            view! {
                <WbPromotionDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // ═══════════════════════════════════════════════════════════════════
        // Use Cases (u501-u508)
        // ═══════════════════════════════════════════════════════════════════
        "u501_import_from_ut" => view! { <u501_import_from_ut::ImportWidget /> }.into_any(),
        "u502_import_from_ozon" => view! { <u502_import_from_ozon::ImportWidget /> }.into_any(),
        "u503_import_from_yandex" => view! { <u503_import_from_yandex::ImportWidget /> }.into_any(),
        "u504_import_from_wildberries" => {
            view! { <u504_import_from_wildberries::ImportWidget /> }.into_any()
        }
        "u505_match_nomenclature" => {
            view! { <u505_match_nomenclature::MatchNomenclatureView /> }.into_any()
        }
        "u506_import_from_lemanapro" => {
            view! { <u506_import_from_lemanapro::ImportWidget /> }.into_any()
        }
        "u507_import_from_erp" => view! { <u507_import_from_erp::ImportWidget /> }.into_any(),
        "u508_repost_documents" => {
            view! { <u508_repost_documents::RepostDocumentsWidget /> }.into_any()
        }

        // Журнал операций (general_ledger)
        "general_ledger" => view! { <GeneralLedgerPage /> }.into_any(),
        k if k.starts_with("general_ledger_details_") => {
            let id = k
                .strip_prefix("general_ledger_details_")
                .unwrap()
                .to_string();
            view! {
                <GeneralLedgerDetailsPage
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // ═══════════════════════════════════════════════════════════════════
        // Projections (p900-p906)
        // ═══════════════════════════════════════════════════════════════════
        "p900_sales_register" => view! { <SalesRegisterList /> }.into_any(),
        "p901_barcodes" => view! { <BarcodesList /> }.into_any(),
        "p902_ozon_finance_realization" => view! { <OzonFinanceRealizationList /> }.into_any(),
        "p903_wb_finance_report" => view! { <WbFinanceReportList /> }.into_any(),
        k if k.starts_with("p903_wb_finance_report_details_") => {
            let rest = k
                .strip_prefix("p903_wb_finance_report_details_")
                .unwrap()
                .to_string();
            let Some((rr_dt_encoded, rrd_id_str)) = rest.rsplit_once("__") else {
                log!("⚠️ Bad p903 details tab key: {}", k);
                return view! { <div class="placeholder">{"Bad finance report tab key"}</div> }
                    .into_any();
            };

            let rr_dt = urlencoding::decode(&rr_dt_encoded)
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| rr_dt_encoded.to_string());
            let rrd_id: i64 = rrd_id_str.parse().unwrap_or_default();

            view! {
                <WbFinanceReportDetail
                    rr_dt=rr_dt
                    rrd_id=rrd_id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }
        "p904_sales_data" => {
            log!("✅ Creating SalesDataList");
            view! { <SalesDataList /> }.into_any()
        }
        "p909_mp_order_line_turnovers" => view! { <MpOrderLineTurnoversList /> }.into_any(),
        k if k.starts_with("p909_mp_order_line_turnovers_details_") => {
            let encoded = k
                .strip_prefix("p909_mp_order_line_turnovers_details_")
                .unwrap_or_default();
            let id = urlencoding::decode(encoded)
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| encoded.to_string());
            view! {
                <MpOrderLineTurnoverDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }
        "p910_mp_unlinked_turnovers" => view! { <MpUnlinkedTurnoversList /> }.into_any(),
        k if k.starts_with("p910_mp_unlinked_turnovers_details_") => {
            let encoded = k
                .strip_prefix("p910_mp_unlinked_turnovers_details_")
                .unwrap_or_default();
            let id = urlencoding::decode(encoded)
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| encoded.to_string());
            view! {
                <MpUnlinkedTurnoverDetail
                    id=id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }
        "p911_wb_advert_by_items" => view! { <WbAdvertByItemsList /> }.into_any(),
        k if k.starts_with("p911_wb_advert_by_items_details_") => {
            let encoded = k
                .strip_prefix("p911_wb_advert_by_items_details_")
                .unwrap_or_default();
            let general_ledger_ref = urlencoding::decode(encoded)
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| encoded.to_string());
            view! {
                <WbAdvertByItemDetail
                    general_ledger_ref=general_ledger_ref
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }
        "p905_commission_history" => view! { <CommissionHistoryList /> }.into_any(),
        k if k.starts_with("p905-commission/") => {
            let id = k.strip_prefix("p905-commission/").unwrap().to_string();
            view! { <CommissionHistoryDetails id=id /> }.into_any()
        }
        "p905-commission-new" => view! { <CommissionHistoryDetails /> }.into_any(),
        "p906_nomenclature_prices" => {
            log!("✅ Creating NomenclaturePricesList");
            view! { <NomenclaturePricesList /> }.into_any()
        }
        "p907_ym_payment_report" => {
            log!("✅ Creating YmPaymentReportList");
            view! { <YmPaymentReportList /> }.into_any()
        }
        "p908_wb_goods_prices" => {
            log!("✅ Creating WbGoodsPricesList");
            view! { <WbGoodsPricesList /> }.into_any()
        }
        k if k.starts_with("p907_ym_payment_report_details_") => {
            let encoded = k
                .strip_prefix("p907_ym_payment_report_details_")
                .unwrap_or_default();
            let record_key = urlencoding::decode(encoded)
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| encoded.to_string());
            log!(
                "✅ Creating YmPaymentReportDetail for record_key: {}",
                record_key
            );
            view! {
                <YmPaymentReportDetail
                    record_key=record_key
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // ═══════════════════════════════════════════════════════════════════
        // System (sys_*)
        // ═══════════════════════════════════════════════════════════════════
        "sys_users" => view! { <UsersListPage /> }.into_any(),
        "sys_user_new" => view! {
            <CreateUserPage
                on_close=Callback::new({
                    let key_for_close = key_for_close.clone();
                    move |_| {
                        tabs_store.close_tab(&key_for_close);
                    }
                })
            />
        }
        .into_any(),
        k if k.starts_with("sys_user_details_") => {
            let id = k.strip_prefix("sys_user_details_").unwrap().to_string();
            view! { <UserDetailsPage user_id=id /> }.into_any()
        }
        "sys_roles" => view! { <crate::system::roles::ui::list::RolesListPage /> }.into_any(),
        "sys_roles_matrix" => {
            view! { <crate::system::roles::ui::matrix::RoleMatrixPage /> }.into_any()
        }
        k if k.starts_with("sys_role_details_") => {
            let id = k.strip_prefix("sys_role_details_").unwrap().to_string();
            view! { <crate::system::roles::ui::details::RoleDetailsPage role_id=id /> }.into_any()
        }
        "sys_scheduled_tasks" => view! { <ScheduledTaskList /> }.into_any(),
        "sys_scheduled_task_details" => {
            view! { <ScheduledTaskDetails id="new".to_string() /> }.into_any()
        }
        k if k.starts_with("sys_scheduled_task_details_") => {
            let id = k
                .strip_prefix("sys_scheduled_task_details_")
                .unwrap()
                .to_string();
            view! { <ScheduledTaskDetails id=id /> }.into_any()
        }
        "sys_thaw_test" => {
            log!("✅ Creating ThawTestPage");
            view! { <ThawTestPage /> }.into_any()
        }
        "dom_inspector" => {
            log!("✅ Creating DomValidatorPage");
            view! { <crate::shared::dom_validator::page::DomValidatorPage /> }.into_any()
        }

        // ═══════════════════════════════════════════════════════════════════
        // Dashboards (d400-d401)
        // ═══════════════════════════════════════════════════════════════════
        "d400_monthly_summary" => {
            log!("✅ Creating MonthlySummaryDashboard");
            view! { <MonthlySummaryDashboard /> }.into_any()
        }
        "d401_metadata_dashboard" => {
            log!("✅ Creating MetadataDashboard");
            view! { <MetadataDashboard /> }.into_any()
        }
        "d401_wb_finance" => {
            log!("✅ Creating D401WbFinanceDashboard (legacy)");
            view! { <D401WbFinanceDashboard /> }.into_any()
        }

        // ═══════════════════════════════════════════════════════════════════
        // Features (new pivot system)
        // ═══════════════════════════════════════════════════════════════════
        "universal_dashboard" => {
            log!("✅ Creating UniversalDashboard");
            view! { <UniversalDashboard /> }.into_any()
        }
        "schema_browser" => {
            log!("✅ Creating SchemaBrowser");
            view! { <SchemaBrowser /> }.into_any()
        }
        "all_reports" => {
            log!("✅ Creating AllReportsList");
            view! { <crate::shared::universal_dashboard::AllReportsList /> }.into_any()
        }

        // All Reports Details
        k if k.starts_with("all_reports_details_") => {
            let config_id = k.strip_prefix("all_reports_details_").unwrap().to_string();
            log!("✅ Creating AllReportsDetails for config: {}", config_id);
            view! {
                <crate::shared::universal_dashboard::AllReportsDetails
                    config_id=config_id
                    on_close=Callback::new({
                        let key_for_close = key_for_close.clone();
                        move |_| {
                            tabs_store.close_tab(&key_for_close);
                        }
                    })
                />
            }
            .into_any()
        }

        // Schema Details
        k if k.starts_with("schema_details_") => {
            let schema_id = k.strip_prefix("schema_details_").unwrap().to_string();
            log!("✅ Creating SchemaDetails for schema: {}", schema_id);
            view! {
                <crate::shared::universal_dashboard::ui::schema_details::SchemaDetails
                    schema_id=schema_id
                    on_close=Callback::new(move |_| {
                        tabs_store.close_tab(&key_for_close);
                    })
                />
            }
            .into_any()
        }

        // Universal Dashboard Report (opened from All Reports list)
        // Format: universal_dashboard_report_{uuid}__{schema_id}__{config_id}
        k if k.starts_with("universal_dashboard_report_") => {
            let rest = k.strip_prefix("universal_dashboard_report_").unwrap();
            // Parse: uuid__schema_id__config_id
            let parts: Vec<&str> = rest.split("__").collect();
            if parts.len() == 3 {
                let schema_id = parts[1].to_string();
                let config_id = parts[2].to_string();
                log!(
                    "✅ Creating UniversalDashboard with schema: {}, config: {}",
                    schema_id,
                    config_id
                );
                view! {
                    <UniversalDashboard
                        initial_schema_id=schema_id
                        initial_config_id=config_id
                        on_close=Callback::new({
                            let key_for_close = key_for_close.clone();
                            move |_| {
                                tabs_store.close_tab(&key_for_close);
                            }
                        })
                    />
                }
                .into_any()
            } else {
                log!("⚠️ Bad universal_dashboard_report tab key: {}", k);
                view! { <div class="placeholder">{"Bad report tab key"}</div> }.into_any()
            }
        }

        // ═══════════════════════════════════════════════════════════════════
        // Drilldown Report — manual mode (no session)
        // Tab key: "drilldown__new"
        // ═══════════════════════════════════════════════════════════════════
        k if k == "drilldown__new" => {
            log!("✅ DrilldownReportPage manual mode");
            let key_for_close2 = key_for_close.clone();
            view! {
                <DrilldownReportPage
                    session_id=None
                    on_close=Some(Callback::new(move |_| {
                        tabs_store.close_tab(&key_for_close2);
                    }))
                />
            }
            .into_any()
        }

        // ═══════════════════════════════════════════════════════════════════
        // Drilldown Report (DataView-based, session-stored)
        // Tab key format: "drilldown__{session_id}"
        // Full params are stored in sys_drilldown table on the server.
        // ═══════════════════════════════════════════════════════════════════
        k if k.starts_with("drilldown__") => {
            let session_id = k.strip_prefix("drilldown__").unwrap_or("").to_string();
            log!("✅ DrilldownReportPage session_id={}", session_id);

            let key_for_close2 = key_for_close.clone();
            view! {
                <DrilldownReportPage
                    session_id=Some(session_id)
                    on_close=Some(Callback::new(move |_| {
                        tabs_store.close_tab(&key_for_close2);
                    }))
                />
            }
            .into_any()
        }

        // ═══════════════════════════════════════════════════════════════════
        // Unknown / Fallback
        // ═══════════════════════════════════════════════════════════════════
        _ => {
            log!("⚠️ Unknown tab type: {}", key);
            view! { <div class="placeholder">{"Not implemented yet"}</div> }.into_any()
        }
    }
}
