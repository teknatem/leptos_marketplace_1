//use crate::aggregates::{
//    customers, inventory, invoices, orders, payments, products, purchases, shipments, suppliers,
//};
use crate::layout::center::tabs::Tabs;
use crate::layout::global_context::AppGlobalContext;
use crate::layout::left::navbar::Navbar;
use crate::layout::right::panel::RightPanel;
use crate::layout::Shell;
use leptos::prelude::*;
// Temporarily avoid Router components while migrating to Leptos 0.8

#[component]
fn MainLayout() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    // Initialize router integration. This runs once when the component is created.
    tabs_store.init_router_integration();

    view! {
        <Shell
            left=|| view! { <Navbar /> }.into_any()
            center=|| view! { <Tabs /> }.into_any()
            right=|| view! { <RightPanel /> }.into_any()
        />
    }
}

#[component]
pub fn AppRoutes() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");
    tabs_store.init_router_integration();
    view! { <MainLayout /> }
}
