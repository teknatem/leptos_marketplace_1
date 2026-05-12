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

fn format_date(iso_date: &str) -> String {
    if let Some(date_part) = iso_date.split('T').next() {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                return format!("{}.{}.{}", day, month, year);
            }
        }
    }
    iso_date.to_string()
}

fn status_label(status: Option<i32>) -> &'static str {
    match status {
        Some(1) => "Открыта",
        Some(2) => "На рассмотрении",
        Some(3) => "Одобрена",
        Some(4) => "Отклонена",
        Some(5) => "Закрыта",
        _ => "Неизвестен",
    }
}

#[component]
pub fn GeneralTab(vm: WbReturnsClaimsDetailsVm) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    move || {
        let Some(d) = vm.item.get() else {
            return view! { <div>"Нет данных"</div> }.into_any();
        };

        view! {
            <div class="detail-grid">
                <div class="detail-grid__col">
                    <div class="card card--animated" data-nav-id="a032_wb_returns_claims_details_claim_info">
                        <div class="card__body">
                            <h3 class="details-section__title">"Заявка"</h3>
                            <div class="form__group">
                                <span class="form__label">"ID заявки WB"</span>
                                <span class="form__value form__value--mono">{d.claim_id.clone()}</span>
                            </div>
                            <div class="form__group">
                                <span class="form__label">"Статус"</span>
                                <span class="form__value">
                                    {format!("{} ({})", status_label(d.status), d.status.map(|v| v.to_string()).unwrap_or_default())}
                                </span>
                            </div>
                            {d.status_ex.map(|v| view! {
                                <div class="form__group">
                                    <span class="form__label">"Статус (расширенный)"</span>
                                    <span class="form__value">{v}</span>
                                </div>
                            })}
                            {d.claim_type.map(|v| view! {
                                <div class="form__group">
                                    <span class="form__label">"Тип заявки"</span>
                                    <span class="form__value">{v}</span>
                                </div>
                            })}
                            <div class="form__group">
                                <span class="form__label">"Архив"</span>
                                <span class="form__value">{if d.is_archive { "Да" } else { "Нет" }}</span>
                            </div>
                            <div class="form__group">
                                <span class="form__label">"Дата создания заявки"</span>
                                <span class="form__value">{format_date(&d.dt)}</span>
                            </div>
                            {d.dt_update.as_deref().map(|v| view! {
                                <div class="form__group">
                                    <span class="form__label">"Дата обновления"</span>
                                    <span class="form__value">{format_date(v)}</span>
                                </div>
                            })}
                            {d.delivery_dt.as_deref().map(|v| view! {
                                <div class="form__group">
                                    <span class="form__label">"Дата доставки возврата"</span>
                                    <span class="form__value">{format_date(v)}</span>
                                </div>
                            })}
                        </div>
                    </div>

                    <div class="card card--animated" data-nav-id="a032_wb_returns_claims_details_comments">
                        <div class="card__body">
                            <h3 class="details-section__title">"Комментарии"</h3>
                            <div class="form__group">
                                <span class="form__label">"Комментарий покупателя"</span>
                                <span class="form__value">{d.user_comment.clone().unwrap_or_else(|| "—".to_string())}</span>
                            </div>
                            <div class="form__group">
                                <span class="form__label">"Комментарий WB"</span>
                                <span class="form__value">{d.wb_comment.clone().unwrap_or_else(|| "—".to_string())}</span>
                            </div>
                            {d.actions.as_deref().map(|v| view! {
                                <div class="form__group">
                                    <span class="form__label">"Доступные действия"</span>
                                    <span class="form__value form__value--mono">{v.to_string()}</span>
                                </div>
                            })}
                        </div>
                    </div>
                </div>

                <div class="detail-grid__col">
                    <div class="card card--animated" data-nav-id="a032_wb_returns_claims_details_product">
                        <div class="card__body">
                            <h3 class="details-section__title">"Товар"</h3>
                            <div class="form__group">
                                <span class="form__label">"nmId (WB)"</span>
                                <span class="form__value">{d.nm_id}</span>
                            </div>
                            <div class="form__group">
                                <span class="form__label">"Наименование товара"</span>
                                <span class="form__value">{d.imt_name.clone().unwrap_or_else(|| "—".to_string())}</span>
                            </div>
                            {d.price.map(|p| view! {
                                <div class="form__group">
                                    <span class="form__label">"Цена"</span>
                                    <span class="form__value">{format!("{:.2} {}", p, d.currency_code.clone().unwrap_or_default())}</span>
                                </div>
                            })}
                            {d.srid.as_deref().map(|v| {
                                let srid = v.to_string();
                                let srid_for_nav = srid.clone();
                                let srid_label = srid.clone();
                                let ts = tabs_store;
                                view! {
                                    <div class="form__group">
                                        <span class="form__label">"srid заказа"</span>
                                        <a
                                            href="#"
                                            class="table__link form__value--mono"
                                            title="Открыть заказ WB"
                                            on:click=move |e| {
                                                e.prevent_default();
                                                let srid_clone = srid_for_nav.clone();
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
                                            {srid_label.clone()}
                                        </a>
                                    </div>
                                }
                            })}
                            {d.order_dt.as_deref().map(|v| view! {
                                <div class="form__group">
                                    <span class="form__label">"Дата заказа"</span>
                                    <span class="form__value">{format_date(v)}</span>
                                </div>
                            })}
                            {d.origin_id_info.as_deref().map(|v| view! {
                                <div class="form__group">
                                    <span class="form__label">"IMEI / ID"</span>
                                    <span class="form__value form__value--mono">{v.to_string()}</span>
                                </div>
                            })}
                        </div>
                    </div>

                    <div class="card card--animated" data-nav-id="a032_wb_returns_claims_details_meta">
                        <div class="card__body">
                            <h3 class="details-section__title">"Метаданные"</h3>
                            <div class="form__group">
                                <span class="form__label">"Создан"</span>
                                <span class="form__value">{format_date(&d.metadata.created_at)}</span>
                            </div>
                            <div class="form__group">
                                <span class="form__label">"Обновлён"</span>
                                <span class="form__value">{format_date(&d.metadata.updated_at)}</span>
                            </div>
                            <div class="form__group">
                                <span class="form__label">"Версия"</span>
                                <span class="form__value">{d.metadata.version}</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        }.into_any()
    }
}
