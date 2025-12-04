// use crate::aggregates::{
//     customers::main_table::CustomersMainTable, products::main_table::ProductsMainTable,
// };
// This is the component
use crate::domain::a001_connection_1c::ui::list::Connection1CList;
use crate::domain::a002_organization::ui::list::OrganizationList;
use crate::domain::a004_nomenclature::ui::list::NomenclatureList;
use crate::domain::a005_marketplace::ui::list::MarketplaceList;
use crate::domain::a006_connection_mp::ui::list::ConnectionMPList;
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
use crate::domain::a015_wb_orders::ui::list::WbOrdersList;
use crate::domain::a016_ym_returns::ui::list::YmReturnsList;
use crate::layout::center::tabs::tab::Tab as TabComponent;
use crate::layout::global_context::{AppGlobalContext, Tab as TabData};
use crate::projections::p900_mp_sales_register::ui::list::SalesRegisterList;
use crate::projections::p901_nomenclature_barcodes::ui::list::BarcodesList;
use crate::projections::p902_ozon_finance_realization::ui::list::OzonFinanceRealizationList;
use crate::projections::p903_wb_finance_report::ui::list::WbFinanceReportList;
use crate::projections::p904_sales_data::ui::list::SalesDataList;
use crate::projections::p905_wb_commission_history::ui::details::CommissionHistoryDetails;
use crate::projections::p905_wb_commission_history::ui::list::CommissionHistoryList;
use crate::dashboards::MonthlySummaryDashboard;
use crate::usecases::u501_import_from_ut;
use crate::usecases::u502_import_from_ozon;
use crate::usecases::u503_import_from_yandex;
use crate::usecases::u504_import_from_wildberries;
use crate::usecases::u505_match_nomenclature;
use crate::usecases::u506_import_from_lemanapro;
use leptos::logging::log;
use leptos::prelude::*;

// Helper component for rendering individual tab content
#[component]
fn TabPage(tab: TabData, tabs_store: AppGlobalContext) -> impl IntoView {
    let tab_key = tab.key.clone();
    let tab_key_for_active_check = tab_key.clone();
    
    // Check if this tab is active - this closure will be reactive
    let is_active = move || {
        let current_active = tabs_store.active.get();
        let active = current_active.as_ref() == Some(&tab_key_for_active_check);
        active
    };
    
    log!("🔨 TabPage CREATED for: '{}' (this should happen once per open)", tab_key);
    
    // Log when component is destroyed
    let tab_key_for_cleanup = tab_key.clone();
    on_cleanup(move || {
        log!("💥 TabPage DESTROYED for: '{}'", tab_key_for_cleanup);
    });
    
    // Render content based on tab key
    let tab_key_for_content = tab_key.clone();
    let content = {
        let key_ref = tab_key_for_content.as_str();
        let tabs_store_for_details = tabs_store.clone();
        let key_for_close = tab_key_for_content.clone();
        
        match key_ref {
            "a001_connection_1c" => {
                log!("✅ Creating Connection1CList");
                view! { <Connection1CList /> }.into_any()
            }
            "a002_organization" => view! { <OrganizationList /> }.into_any(),
            "a003_counterparty" => {
                view! { <crate::domain::a003_counterparty::ui::tree::CounterpartyTree /> }.into_any()
            }
            "a004_nomenclature" => {
                view! { <crate::domain::a004_nomenclature::ui::tree::NomenclatureTree /> }.into_any()
            }
            "a004_nomenclature_list" => {
                view! { <NomenclatureList /> }.into_any()
            }
            "a005_marketplace" => view! { <MarketplaceList /> }.into_any(),
            "a006_connection_mp" => view! { <ConnectionMPList /> }.into_any(),
            "a007_marketplace_product" => {
                view! { <MarketplaceProductList /> }.into_any()
            }
            "a008_marketplace_sales" => {
                view! { <MarketplaceSalesList /> }.into_any()
            }
            "a009_ozon_returns" => {
                view! { <OzonReturnsList /> }.into_any()
            }
            k if k.starts_with("a009_ozon_returns_detail_") => {
                let id = k.strip_prefix("a009_ozon_returns_detail_").unwrap().to_string();
                log!("✅ Creating OzonReturnsDetail with id: {}", id);
                view! {
                    <OzonReturnsDetail
                        id=id
                        on_close=Callback::new(move |_| {
                            tabs_store_for_details.close_tab(&key_for_close);
                        })
                    />
                }.into_any()
            }
            "a010_ozon_fbs_posting" => {
                view! { <OzonFbsPostingList /> }.into_any()
            }
            "a011_ozon_fbo_posting" => {
                view! { <OzonFboPostingList /> }.into_any()
            }
            "a015_wb_orders" => {
                view! { <WbOrdersList /> }.into_any()
            }
            "a012_wb_sales" => {
                view! { <WbSalesList /> }.into_any()
            }
            k if k.starts_with("a012_wb_sales_detail_") => {
                let id = k.strip_prefix("a012_wb_sales_detail_").unwrap().to_string();
                log!("✅ Creating WbSalesDetail with id: {}", id);
                view! {
                    <WbSalesDetail
                        id=id
                        on_close=Callback::new(move |_| {
                            tabs_store_for_details.close_tab(&key_for_close);
                        })
                    />
                }.into_any()
            }
            "a013_ym_order" => {
                view! { <YmOrderList /> }.into_any()
            }
            k if k.starts_with("a013_ym_order_detail_") => {
                let id = k.strip_prefix("a013_ym_order_detail_").unwrap().to_string();
                log!("✅ Creating YmOrderDetail with id: {}", id);
                view! {
                    <YmOrderDetail
                        id=id
                        on_close=Callback::new(move |_| {
                            tabs_store_for_details.close_tab(&key_for_close);
                        })
                    />
                }.into_any()
            }
            "a016_ym_returns" => {
                view! { <YmReturnsList /> }.into_any()
            }
            "a014_ozon_transactions" => {
                view! { <OzonTransactionsList /> }.into_any()
            }
            k if k.starts_with("a014_ozon_transactions_detail_") => {
                let id = k.strip_prefix("a014_ozon_transactions_detail_").unwrap().to_string();
                log!("✅ Creating OzonTransactionsDetail with id: {}", id);
                view! {
                    <OzonTransactionsDetail
                        transaction_id=id
                        on_close=Callback::new(move |_| {
                            tabs_store_for_details.close_tab(&key_for_close);
                        })
                    />
                }.into_any()
            }
            "u501_import_from_ut" => {
                view! { <u501_import_from_ut::ImportWidget /> }.into_any()
            }
            "u502_import_from_ozon" => {
                view! { <u502_import_from_ozon::ImportWidget /> }.into_any()
            }
            "u503_import_from_yandex" => {
                view! { <u503_import_from_yandex::ImportWidget /> }.into_any()
            }
            "u504_import_from_wildberries" => {
                view! { <u504_import_from_wildberries::ImportWidget /> }.into_any()
            }
            "u505_match_nomenclature" => {
                view! { <u505_match_nomenclature::MatchNomenclatureView /> }.into_any()
            }
            "u506_import_from_lemanapro" => {
                view! { <u506_import_from_lemanapro::ImportWidget /> }.into_any()
            }
            "p900_sales_register" => {
                view! { <SalesRegisterList /> }.into_any()
            }
            "p901_barcodes" => {
                view! { <BarcodesList /> }.into_any()
            }
            "p902_ozon_finance_realization" => {
                view! { <OzonFinanceRealizationList /> }.into_any()
            }
            "p903_wb_finance_report" => {
                view! { <WbFinanceReportList /> }.into_any()
            }
            "p904_sales_data" => {
                log!("✅ Creating SalesDataList");
                view! { <SalesDataList /> }.into_any()
            }
            "p905_commission_history" => {
                view! { <CommissionHistoryList /> }.into_any()
            }
            k if k.starts_with("p905-commission/") => {
                view! { <CommissionHistoryDetails id=k.strip_prefix("p905-commission/").unwrap().to_string() /> }.into_any()
            }
            "p905-commission-new" => {
                view! { <CommissionHistoryDetails /> }.into_any()
            }
            // Dashboards
            "d400_monthly_summary" => {
                log!("✅ Creating MonthlySummaryDashboard");
                view! { <MonthlySummaryDashboard /> }.into_any()
            }
            _ => {
                log!("⚠️ Unknown tab type: {}", key_ref);
                view! { <div class="placeholder">{"Not implemented yet"}</div> }.into_any()
            }
        }
    };
    
    view! {
        <div 
            class="tab-page"
            class:hidden=move || !is_active()
            data-tab-key=tab_key
        >
            {content}
        </div>
    }
}

#[component]
pub fn Tabs() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    view! {
        <div class="tabs-container">
            <div class="tabs-bar">
                <For
                    each=move || tabs_store.opened.get()
                    key=|tab| tab.key.clone()
                    children=move |tab| {
                        view! { <TabComponent tab=tab /> }
                    }
                />
            </div>
            <div class="tab-content">
                <For
                    each=move || {
                        let tabs = tabs_store.opened.get();
                        log!("📋 <For> each triggered. Tabs count: {}", tabs.len());
                        for (i, tab) in tabs.iter().enumerate() {
                            log!("  {}. key='{}', title='{}'", i+1, tab.key, tab.title);
                        }
                        tabs
                    }
                    key=|tab| {
                        let key = tab.key.clone();
                        log!("🔑 <For> key function called for: '{}'", key);
                        key
                    }
                    children=move |tab: TabData| {
                        log!("👶 <For> children function called for: '{}'", tab.key);
                        view! {
                            <TabPage tab=tab tabs_store=tabs_store />
                        }
                    }
                />
            </div>
        </div>
    }
}

pub fn create_tabs() -> Vec<TabData> {
    vec![]
}
