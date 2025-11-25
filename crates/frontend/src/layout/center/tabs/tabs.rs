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
use crate::domain::a009_ozon_returns::ui::list::OzonReturnsList;
use crate::domain::a009_ozon_returns::ui::details::OzonReturnsDetail;
use crate::domain::a010_ozon_fbs_posting::ui::list::OzonFbsPostingList;
use crate::domain::a011_ozon_fbo_posting::ui::list::OzonFboPostingList;
use crate::domain::a012_wb_sales::ui::list::WbSalesList;
use crate::domain::a012_wb_sales::ui::details::WbSalesDetail;
use crate::domain::a013_ym_order::ui::list::YmOrderList;
use crate::domain::a015_wb_orders::ui::list::WbOrdersList;
use crate::domain::a014_ozon_transactions::ui::list::OzonTransactionsList;
use crate::projections::p900_mp_sales_register::ui::list::SalesRegisterList;
use crate::projections::p901_nomenclature_barcodes::ui::list::BarcodesList;
use crate::projections::p902_ozon_finance_realization::ui::list::OzonFinanceRealizationList;
use crate::projections::p903_wb_finance_report::ui::list::WbFinanceReportList;
use crate::projections::p904_sales_data::ui::list::SalesDataList;
use crate::projections::p905_wb_commission_history::ui::list::CommissionHistoryList;
use crate::projections::p905_wb_commission_history::ui::details::CommissionHistoryDetails;
use crate::layout::center::tabs::tab::Tab as TabComponent;
use crate::layout::global_context::{AppGlobalContext, Tab as TabData};
use crate::usecases::u501_import_from_ut;
use crate::usecases::u502_import_from_ozon;
use crate::usecases::u503_import_from_yandex;
use crate::usecases::u504_import_from_wildberries;
use crate::usecases::u505_match_nomenclature;
use crate::usecases::u506_import_from_lemanapro;
use leptos::prelude::*;

#[component]
pub fn Tabs() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    let opened = move || tabs_store.opened.get();
    let active_key = move || tabs_store.active.get();

    let render_content = move || match active_key() {
        Some(key) if key == "a001_connection_1c" => view! { <Connection1CList /> }.into_any(),
        Some(key) if key == "a002_organization" => view! { <OrganizationList /> }.into_any(),
        Some(key) if key == "a003_counterparty" => {
            view! { <crate::domain::a003_counterparty::ui::tree::CounterpartyTree /> }.into_any()
        }
        Some(key) if key == "a004_nomenclature" => {
            view! { <crate::domain::a004_nomenclature::ui::tree::NomenclatureTree /> }.into_any()
        }
        Some(key) if key == "a004_nomenclature_list" => {
            view! { <NomenclatureList /> }.into_any()
        }
        Some(key) if key == "a005_marketplace" => view! { <MarketplaceList /> }.into_any(),
        Some(key) if key == "a006_connection_mp" => view! { <ConnectionMPList /> }.into_any(),
        Some(key) if key == "a007_marketplace_product" => {
            view! { <MarketplaceProductList /> }.into_any()
        }
        Some(key) if key == "a008_marketplace_sales" => {
            view! { <MarketplaceSalesList /> }.into_any()
        }
        Some(key) if key == "a009_ozon_returns" => {
            view! { <OzonReturnsList /> }.into_any()
        }
        Some(key) if key.starts_with("a009_ozon_returns_detail_") => {
            let id = key.strip_prefix("a009_ozon_returns_detail_").unwrap().to_string();
            let tabs_store_clone = tabs_store.clone();
            let key_clone = key.clone();
            view! {
                <OzonReturnsDetail
                    id=id
                    on_close=Callback::new(move |_| {
                        tabs_store_clone.close_tab(&key_clone);
                    })
                />
            }.into_any()
        }
        Some(key) if key == "a010_ozon_fbs_posting" => {
            view! { <OzonFbsPostingList /> }.into_any()
        }
        Some(key) if key == "a011_ozon_fbo_posting" => {
            view! { <OzonFboPostingList /> }.into_any()
        }
        Some(key) if key == "a015_wb_orders" => {
            view! { <WbOrdersList /> }.into_any()
        }
        Some(key) if key == "a012_wb_sales" => {
            view! { <WbSalesList /> }.into_any()
        }
        Some(key) if key.starts_with("a012_wb_sales_detail_") => {
            let id = key.strip_prefix("a012_wb_sales_detail_").unwrap().to_string();
            let tabs_store_clone = tabs_store.clone();
            let key_clone = key.clone();
            view! {
                <WbSalesDetail
                    id=id
                    on_close=Callback::new(move |_| {
                        tabs_store_clone.close_tab(&key_clone);
                    })
                />
            }.into_any()
        }
        Some(key) if key == "a013_ym_order" => {
            view! { <YmOrderList /> }.into_any()
        }
        Some(key) if key == "a014_ozon_transactions" => {
            view! { <OzonTransactionsList /> }.into_any()
        }
        Some(key) if key == "u501_import_from_ut" => {
            view! { <u501_import_from_ut::ImportWidget /> }.into_any()
        }
        Some(key) if key == "u502_import_from_ozon" => {
            view! { <u502_import_from_ozon::ImportWidget /> }.into_any()
        }
        Some(key) if key == "u503_import_from_yandex" => {
            view! { <u503_import_from_yandex::ImportWidget /> }.into_any()
        }
        Some(key) if key == "u504_import_from_wildberries" => {
            view! { <u504_import_from_wildberries::ImportWidget /> }.into_any()
        }
        Some(key) if key == "u505_match_nomenclature" => {
            view! { <u505_match_nomenclature::MatchNomenclatureView /> }.into_any()
        }
        Some(key) if key == "u506_import_from_lemanapro" => {
            view! { <u506_import_from_lemanapro::ImportWidget /> }.into_any()
        }
        Some(key) if key == "p900_sales_register" => {
            view! { <SalesRegisterList /> }.into_any()
        }
        Some(key) if key == "p901_barcodes" => {
            view! { <BarcodesList /> }.into_any()
        }
        Some(key) if key == "p902_ozon_finance_realization" => {
            view! { <OzonFinanceRealizationList /> }.into_any()
        }
        Some(key) if key == "p903_wb_finance_report" => {
            view! { <WbFinanceReportList /> }.into_any()
        }
        Some(key) if key == "p904_sales_data" => {
            view! { <SalesDataList /> }.into_any()
        }
        Some(key) if key == "p905_commission_history" => {
            view! { <CommissionHistoryList /> }.into_any()
        }
        Some(key) if key.starts_with("p905-commission/") => {
            view! { <CommissionHistoryDetails id=key.strip_prefix("p905-commission/").unwrap().to_string() /> }.into_any()
        }
        Some(key) if key == "p905-commission-new" => {
            view! { <CommissionHistoryDetails /> }.into_any()
        }
        Some(_) => view! { <div class="placeholder">{"Not implemented yet"}</div> }.into_any(),
        None => view! { <div class="placeholder">{"Select a tab from the left navbar"}</div> }
            .into_any(),
    };

    view! {
        <div class="tabs-container">
            <div class="tabs-bar">
                {move || opened().into_iter().map(|tab| {
                    view! { <TabComponent tab=tab /> }
                }).collect_view()}
            </div>
            <div class="tab-content">
                {render_content}
            </div>
        </div>
    }
}

pub fn create_tabs() -> Vec<TabData> {
    vec![]
}
