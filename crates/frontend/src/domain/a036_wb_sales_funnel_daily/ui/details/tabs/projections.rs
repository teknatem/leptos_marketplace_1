//! Projections tab — движения воронки p916, порождённые документом a036 (маркетинговая
//! стадия: переходы/корзина/заказы-воронки), как JSON, соответствующий записанным данным.

use super::super::view_model::WbSalesFunnelDailyDetailsVm;
use crate::shared::json_viewer::widget::JsonViewer;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn ProjectionsTab(vm: WbSalesFunnelDailyDetailsVm) -> impl IntoView {
    let projections = vm.projections;
    let projections_loading = vm.projections_loading;

    view! {
        {move || {
            if projections_loading.get() {
                return view! {
                    <Card>
                        <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-xl);">
                            <Spinner />
                            <span>"Загрузка проекций..."</span>
                        </Flex>
                    </Card>
                }.into_any();
            }

            if let Some(proj) = projections.get() {
                let p916_len = proj["p916_mp_sales_funnel_turnovers"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0);
                let pretty = serde_json::to_string_pretty(&proj)
                    .unwrap_or_else(|_| proj.to_string());

                view! {
                    <div style="display: grid; grid-template-columns: 100%; gap: var(--spacing-md); align-items: start; justify-items: start;">
                        <Flex gap=FlexGap::Medium style="margin-bottom: var(--spacing-md);">
                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>
                                {format!("p916_mp_sales_funnel_turnovers: {}", p916_len)}
                            </Badge>
                        </Flex>

                        <JsonViewer
                            json_content=pretty
                            title="Проекции (p916)".to_string()
                        />
                    </div>
                }.into_any()
            } else {
                view! {
                    <Card>
                        <h4 class="details-section__title">"Проекции"</h4>
                        <div style="color: var(--color-text-secondary);">
                            "Нет данных проекций"
                        </div>
                    </Card>
                }.into_any()
            }
        }}
    }
}
