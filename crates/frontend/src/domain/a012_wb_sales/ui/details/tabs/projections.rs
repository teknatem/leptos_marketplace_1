//! Projections tab - p900 and p904 projection data

use super::super::view_model::WbSalesDetailsVm;
use leptos::prelude::*;
use thaw::*;

/// Projections tab component - displays p900/p904 projection data
#[component]
pub fn ProjectionsTab(vm: WbSalesDetailsVm) -> impl IntoView {
    view! {
        {move || {
            if vm.projections_loading.get() {
                return view! {
                    <Card>
                        <Flex gap=FlexGap::Small style="align-items: left; justify-content: center; padding: var(--spacing-xl);">
                            <Spinner />
                            <span>"Загрузка проекций..."</span>
                        </Flex>
                    </Card>
                }.into_any();
            }

            if let Some(proj_data) = vm.projections.get() {
                let p900_len = proj_data["p900_sales_register"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0);
                let p904_len = proj_data["p904_sales_data"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0);
                let pretty = serde_json::to_string_pretty(&proj_data)
                    .unwrap_or_else(|_| proj_data.to_string());

                view! {
                    <div style="display: grid; grid-template-columns: 800px; gap: var(--spacing-md); align-items: start; justify-items: start;">
                    <Card>
                        <h4 class="details-section__title">"Проекции"</h4>

                        // Summary badges
                        <Flex gap=FlexGap::Medium style="margin-bottom: var(--spacing-md);">
                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                                {format!("p900_sales_register: {}", p900_len)}
                            </Badge>
                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                                {format!("p904_sales_data: {}", p904_len)}
                            </Badge>
                        </Flex>

                        // JSON content
                        <pre style="margin: 0; max-height: 70vh; overflow: auto; font-size: var(--font-size-sm); background: var(--color-bg-secondary); padding: var(--spacing-md); border-radius: var(--radius-sm);">
                            {pretty}
                        </pre>
                    </Card>
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
