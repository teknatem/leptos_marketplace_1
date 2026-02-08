//! Test tab - run schema validation test

use super::super::super::schema_browser::ValidationCard;
use super::super::view_model::SchemaDetailsVm;
use leptos::prelude::*;
use thaw::*;

/// Test tab component
#[component]
pub fn TestTab(vm: SchemaDetailsVm) -> impl IntoView {
    let test_result = vm.test_result;
    let testing = vm.testing;

    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <div>
                <div style="font-size: var(--font-size-lg); font-weight: 600; color: var(--color-text-primary); margin-bottom: var(--spacing-sm);">
                    "Тестирование схемы"
                </div>
                <div style="color: var(--color-text-secondary);">
                    "Запустите тест для проверки корректности схемы и доступности данных"
                </div>
            </div>

            <div>
                <Button
                    appearance=ButtonAppearance::Primary
                    size=ButtonSize::Medium
                    on_click=move |_| vm.run_test()
                    disabled=move || testing.get()
                    loading=move || testing.get()
                >
                    "Запустить тест"
                </Button>
            </div>

            {move || {
                test_result.get().map(|result| {
                    view! {
                        <ValidationCard result=result />
                    }
                })
            }}

            {move || {
                (test_result.get().is_none() && !testing.get()).then(|| view! {
                    <div style="padding: var(--spacing-xl); text-align: center; color: var(--color-text-secondary); background: var(--color-neutral-50); border-radius: var(--radius-md); border: 1px dashed var(--color-neutral-200);">
                        <p>"Нажмите кнопку для запуска теста"</p>
                    </div>
                })
            }}
        </Flex>
    }
}
