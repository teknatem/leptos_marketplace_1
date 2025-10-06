// use crate::aggregates::{
//     customers::main_table::CustomersMainTable, products::main_table::ProductsMainTable,
// };
// This is the component
use crate::domain::a001_connection_1c::ui::list::Connection1CList;
use crate::domain::a002_organization::ui::list::OrganizationList;
use crate::layout::center::tabs::tab::Tab as TabComponent;
use crate::layout::global_context::{AppGlobalContext, Tab as TabData};
use crate::usecases::u501_import_from_ut::ImportWidget;
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
        Some(key) if key == "u501_import_from_ut" => view! { <ImportWidget /> }.into_any(),
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
