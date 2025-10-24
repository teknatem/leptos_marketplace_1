// use crate::aggregates::{
//     customers::main_table::CustomersMainTable, products::main_table::ProductsMainTable,
// };
// This is the component
use crate::domain::a001_connection_1c::ui::list::Connection1CList;
use crate::domain::a002_organization::ui::list::OrganizationList;
use crate::domain::a005_marketplace::ui::list::MarketplaceList;
use crate::domain::a006_connection_mp::ui::list::ConnectionMPList;
use crate::domain::a007_marketplace_product::ui::list::MarketplaceProductList;
use crate::domain::a008_marketplace_sales::ui::list::MarketplaceSalesList;
use crate::domain::a009_ozon_returns::ui::list::OzonReturnsList;
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
