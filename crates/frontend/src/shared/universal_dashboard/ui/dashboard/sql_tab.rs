//! SQL Tab - displays generated SQL query

use crate::shared::components::sql_viewer::SqlViewer;
use contracts::shared::universal_dashboard::GenerateSqlResponse;
use leptos::prelude::*;
use thaw::Card;

#[component]
pub fn SqlTab(#[prop(into)] generated_sql: Signal<Option<GenerateSqlResponse>>) -> impl IntoView {
    view! {
        <div class="sql-tab">
        <Card>
                <SqlViewer sql=generated_sql />
        </Card>
        </div>
    }
}
