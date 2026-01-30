//! SQL Tab - displays generated SQL query

use leptos::prelude::*;
use contracts::shared::pivot::GenerateSqlResponse;
use crate::shared::pivot::ui::SqlViewer;

#[component]
pub fn SqlTab(
    #[prop(into)] generated_sql: Signal<Option<GenerateSqlResponse>>,
) -> impl IntoView {
    view! {
        <div class="sql-tab">
            <SqlViewer sql=generated_sql />
        </div>
    }
}
