use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons;
use leptos::prelude::*;

#[component]
pub fn Navbar() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    fn icon_for(kind: &str) -> AnyView {
        icons::icon(kind)
    }
    let aggregates = vec![
        ("customers", "Customers"),
        ("orders", "Orders"),
        ("products", "Products"),
        ("inventory", "Inventory"),
        ("suppliers", "Suppliers"),
        ("purchases", "Purchases"),
        ("invoices", "Invoices"),
        ("payments", "Payments"),
        ("shipments", "Shipments"),
        ("users", "Users"),
        ("a001_connection_1c", "1C Connections"),
        ("a002_organization", "Организации"),
        ("a003_counterparty", "Контрагенты"),
        ("a004_nomenclature", "Номенклатура"),
    ];

    let usecases = vec![("u501_import_from_ut", "Импорт из УТ 11")];

    view! {
        <nav class="main-nav-bar">
            <>
                <div style="padding: 10px; font-weight: bold; color: #888; font-size: 12px; text-transform: uppercase;">
                    "Справочники"
                </div>
                <ul>
                    {aggregates.into_iter().map(|(key, title)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(key)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </ul>

                <div style="padding: 10px; font-weight: bold; color: #888; font-size: 12px; text-transform: uppercase; margin-top: 20px;">
                    "Операции"
                </div>
                <ul>
                    {usecases.into_iter().map(|(key, title)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for("download")}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            </>
        </nav>
    }
}
