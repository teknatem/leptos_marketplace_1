//! JSON tab - raw WB payload

use super::super::view_model::WbOrdersDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::json_viewer::widget::JsonViewer;
use leptos::prelude::*;
use thaw::*;

/// Карточка одного raw-payload (Statistics или Marketplace API).
#[component]
fn RawJsonCard(
    nav_id: &'static str,
    delay_ms: u32,
    title: &'static str,
    endpoint: &'static str,
    description: &'static str,
    loading: Signal<bool>,
    json: Signal<Option<String>>,
    /// Текст «нет данных» (Marketplace отсутствует у FBW / отменённых FBS).
    empty_note: &'static str,
) -> impl IntoView {
    view! {
        <CardAnimated delay_ms=delay_ms nav_id=nav_id style="padding: var(--spacing-sm);">
            <h4 class="details-section__title">{title}</h4>
            <div style="margin-bottom: var(--spacing-sm); color: var(--color-text-secondary);">
                <code>{endpoint}</code>" — "{description}
            </div>
            {move || {
                if loading.get() {
                    view! {
                        <Flex gap=FlexGap::Small style="align-items: center; padding: var(--spacing-lg);">
                            <Spinner />
                            <span>"Загрузка JSON..."</span>
                        </Flex>
                    }
                    .into_any()
                } else if let Some(json) = json.get() {
                    view! {
                        <div style="max-height: calc(100vh - 360px); overflow: auto;">
                            <JsonViewer json_content=json title=title.to_string() />
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <div style="color: var(--color-text-secondary);">{empty_note}</div>
                    }
                    .into_any()
                }
            }}
        </CardAnimated>
    }
}

#[component]
pub fn JsonTab(vm: WbOrdersDetailsVm) -> impl IntoView {
    let stats_loading = Signal::derive({
        let vm = vm.clone();
        move || vm.raw_json_loading.get()
    });
    let stats_json = Signal::derive({
        let vm = vm.clone();
        move || vm.raw_json.get()
    });
    let mp_loading = Signal::derive({
        let vm = vm.clone();
        move || vm.marketplace_raw_json_loading.get()
    });
    let mp_json = Signal::derive({
        let vm = vm.clone();
        move || vm.marketplace_raw_json.get()
    });

    view! {
        <div class="detail-grid">
            <RawJsonCard
                nav_id="a015_wb_orders_details_json_statistics"
                delay_ms=0
                title="Statistics API"
                endpoint="/api/v1/supplier/orders"
                description="Основной ответ WB по документу (суммы в рублях)."
                loading=stats_loading
                json=stats_json
                empty_note="JSON данные не загружены"
            />
            <RawJsonCard
                nav_id="a015_wb_orders_details_json_marketplace"
                delay_ms=40
                title="Marketplace API"
                endpoint="/api/v3/orders"
                description="Сборочное задание FBS (валюта продажи в currencyCode). Отсутствует у FBW и отменённых FBS."
                loading=mp_loading
                json=mp_json
                empty_note="Marketplace API payload отсутствует для этого заказа"
            />
        </div>
    }
}
