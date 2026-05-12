use crate::domain::a032_wb_returns_claims::ui::details::view_model::WbReturnsClaimsDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use gloo_net::http::Request;
use leptos::prelude::*;
use serde::Deserialize;
use wasm_bindgen_futures::spawn_local;

#[derive(Deserialize)]
struct OrderIdDto {
    pub id: String,
}

async fn resolve_order_uuid(srid: &str) -> Option<String> {
    let url = format!(
        "{}/api/a015/wb-orders/search-by-srid?srid={}",
        api_base(),
        srid
    );
    let response = Request::get(&url).send().await.ok()?;
    if !response.ok() {
        return None;
    }
    let orders: Vec<OrderIdDto> = response.json().await.ok()?;
    orders.into_iter().next().map(|o| o.id)
}

#[component]
pub fn LinksTab(vm: WbReturnsClaimsDetailsVm) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        <div class="tab-content tab-content--data">
            <div class="card card--animated" data-nav-id="a032_wb_returns_claims_details_links">
                <div class="card__body">
                    <h3 class="details-section__title">"Связанные данные"</h3>
                    {move || {
                        let Some(d) = vm.item.get() else {
                            return view! { <div>"Нет данных"</div> }.into_any();
                        };

                        let srid = d.srid.clone();

                        view! {
                            <div class="links-list">
                                {srid.map(|s| {
                                    let s_for_nav = s.clone();
                                    let s_label = s.clone();
                                    let ts = tabs_store;
                                    view! {
                                        <div class="form__group">
                                            <span class="form__label">"Заказ WB (srid)"</span>
                                            <a
                                                href="#"
                                                class="table__link"
                                                title="Открыть заказ WB"
                                                on:click=move |e| {
                                                    e.prevent_default();
                                                    let srid_clone = s_for_nav.clone();
                                                    let ts_clone = ts;
                                                    spawn_local(async move {
                                                        if let Some(uuid) = resolve_order_uuid(&srid_clone).await {
                                                            ts_clone.open_tab(
                                                                &format!("a015_wb_orders_details_{}", uuid),
                                                                &format!("WB Order {}", &srid_clone[..srid_clone.len().min(16)]),
                                                            );
                                                        }
                                                    });
                                                }
                                            >
                                                {s_label}
                                            </a>
                                        </div>
                                    }
                                })}
                                <div class="form__group">
                                    <span class="form__label">"nmId (WB)"</span>
                                    <span class="form__value">{d.nm_id}</span>
                                </div>
                            </div>
                        }
                        .into_any()
                    }}
                </div>
            </div>
        </div>
    }
}
