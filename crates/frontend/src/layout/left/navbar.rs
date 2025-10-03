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
        ("connection_1c", "1C Connections"),
    ];

    view! {
        <nav class="main-nav-bar">
            <>
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
            </>
        </nav>
    }
}
