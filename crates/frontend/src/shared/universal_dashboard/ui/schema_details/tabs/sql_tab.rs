//! SQL tab - display generated SQL query

use super::super::view_model::SchemaDetailsVm;
use crate::shared::components::sql_viewer::SqlViewer;
use leptos::prelude::*;
use thaw::*;

/// SQL tab component
#[component]
pub fn SqlTab(vm: SchemaDetailsVm) -> impl IntoView {
    let generated_sql = vm.generated_sql;

    view! {
        <Flex vertical=true gap=FlexGap::Small>
            <div>
                <div style="color: var(--color-text-secondary);">
                    "Сгенерированный SQL запрос для схемы со всеми доступными полями"
                </div>
            </div>

            <SqlViewer sql=generated_sql />
        </Flex>
    }
}
