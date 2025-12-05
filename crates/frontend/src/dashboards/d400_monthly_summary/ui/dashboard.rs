use crate::dashboards::d400_monthly_summary::api;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::list_utils::format_number;
use chrono::{Datelike, Utc};
use contracts::dashboards::d400_monthly_summary::{DrilldownFilter, IndicatorRow, MonthlySummaryResponse};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Monthly Summary Dashboard component
#[component]
pub fn MonthlySummaryDashboard() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    // Current date for default month selection
    let now = Utc::now().date_naive();
    let (selected_year, set_selected_year) = signal(now.year());
    let (selected_month, set_selected_month) = signal(now.month());

    // Data state
    let (data, set_data) = signal(None::<MonthlySummaryResponse>);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);

    // Load data function
    let load_data = move || {
        let year = selected_year.get();
        let month = selected_month.get();
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            match api::get_monthly_summary(year, month).await {
                Ok(response) => {
                    set_data.set(Some(response));
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // Load data on mount
    Effect::new(move |_| {
        load_data();
    });

    // Month navigation
    let go_prev_month = move |_| {
        let (new_year, new_month) = if selected_month.get() == 1 {
            (selected_year.get() - 1, 12)
        } else {
            (selected_year.get(), selected_month.get() - 1)
        };
        set_selected_year.set(new_year);
        set_selected_month.set(new_month);
        load_data();
    };

    let go_next_month = move |_| {
        let (new_year, new_month) = if selected_month.get() == 12 {
            (selected_year.get() + 1, 1)
        } else {
            (selected_year.get(), selected_month.get() + 1)
        };
        set_selected_year.set(new_year);
        set_selected_month.set(new_month);
        load_data();
    };

    let go_current_month = move |_| {
        let now = Utc::now().date_naive();
        set_selected_year.set(now.year());
        set_selected_month.set(now.month());
        load_data();
    };

    // Handle drilldown click
    let handle_drilldown = {
        let tabs_store = tabs_store.clone();
        move |_filter: DrilldownFilter, _mp: Option<String>| {
            // TODO: In the future, we can pass filters to the component
            // For now, just open the p904_sales_data tab
            tabs_store.open_tab("p904_sales_data", "Sales Data (P904)");
        }
    };

    // Month names in Russian
    let month_name = move || {
        match selected_month.get() {
            1 => "Январь",
            2 => "Февраль",
            3 => "Март",
            4 => "Апрель",
            5 => "Май",
            6 => "Июнь",
            7 => "Июль",
            8 => "Август",
            9 => "Сентябрь",
            10 => "Октябрь",
            11 => "Ноябрь",
            12 => "Декабрь",
            _ => "—",
        }
    };

    view! {
        <div class="d400-dashboard">
            // Header with gradient
            <div class="d400-header">
                <div class="d400-header-content">
                    <h1 class="d400-title">"Сводка по маркетплейсам"</h1>
                    <p class="d400-subtitle">"Ключевые показатели эффективности за период"</p>
                </div>

                <div class="d400-month-selector">
                    <button
                        class="d400-nav-btn"
                        on:click=go_prev_month
                        title="Предыдущий месяц"
                    >
                        "‹"
                    </button>

                    <div class="d400-month-display">
                        <span class="d400-month-name">{month_name}</span>
                        <span class="d400-year">{move || selected_year.get()}</span>
                    </div>

                    <button
                        class="d400-nav-btn"
                        on:click=go_next_month
                        title="Следующий месяц"
                    >
                        "›"
                    </button>

                    <button
                        class="d400-today-btn"
                        on:click=go_current_month
                        title="Текущий месяц"
                    >
                        "Сегодня"
                    </button>
                </div>
            </div>

            // Loading state
            {move || {
                if loading.get() {
                    view! {
                        <div class="d400-loading">
                            <div class="d400-spinner"></div>
                            <span>"Загрузка данных..."</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            // Error state
            {move || {
                if let Some(err) = error.get() {
                    view! {
                        <div class="d400-error">
                            <strong>"⚠ Ошибка: "</strong>
                            {err}
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            // Data table
            {move || {
                let handle_drilldown = handle_drilldown.clone();

                if let Some(response) = data.get() {
                    let marketplaces = response.marketplaces.clone();
                    let rows = response.rows.clone();

                    view! {
                        <div class="d400-table-wrapper">
                            <table class="d400-table">
                                <thead>
                                    <tr>
                                        <th class="d400-th d400-th-indicator">"Показатель"</th>
                                        {marketplaces.iter().map(|mp| {
                                            let mp_display = mp.clone();
                                            let mp_class = format!("d400-th d400-th-mp d400-mp-{}", mp.to_lowercase());
                                            view! {
                                                <th class=mp_class>{mp_display}</th>
                                            }
                                        }).collect_view()}
                                        <th class="d400-th d400-th-total">"ИТОГО"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {rows.iter().map(|row| {
                                        let row_clone = row.clone();
                                        let marketplaces_clone = marketplaces.clone();
                                        let is_total_row = row.level == 0;
                                        let handle_drilldown = handle_drilldown.clone();

                                        view! {
                                            <DashboardRow
                                                row=row_clone
                                                marketplaces=marketplaces_clone
                                                is_total=is_total_row
                                                on_drilldown=handle_drilldown
                                            />
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }.into_any()
                } else if !loading.get() {
                    view! {
                        <div class="d400-empty">
                            <span class="d400-empty-icon">"📊"</span>
                            <span>"Нет данных для отображения"</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}
        </div>
    }
}

/// Dashboard row component
#[component]
fn DashboardRow(
    row: IndicatorRow,
    marketplaces: Vec<String>,
    is_total: bool,
    on_drilldown: impl Fn(DrilldownFilter, Option<String>) + Clone + 'static,
) -> impl IntoView {
    let row_class = if is_total {
        "d400-row d400-row-total"
    } else {
        "d400-row d400-row-detail"
    };

    let name_class = if is_total {
        "d400-td d400-td-name"
    } else {
        "d400-td d400-td-name d400-td-name-indent"
    };

    let value_class = if is_total {
        "d400-td d400-td-value"
    } else {
        "d400-td d400-td-value d400-td-value-detail"
    };

    let total_class = if is_total {
        "d400-td d400-td-total-value"
    } else {
        "d400-td d400-td-total-value d400-td-total-detail"
    };

    // Display name with group
    let display_name = if let Some(ref group) = row.group_name {
        format!("└ {}", group)
    } else {
        row.indicator_name.clone()
    };

    let filter = row.drilldown_filter.clone();

    view! {
        <tr class=row_class>
            <td class=name_class>
                {display_name}
            </td>
            {marketplaces.iter().map(|mp| {
                let value = row.values.get(mp).copied().unwrap_or(0.0);
                let formatted = format_number(value);
                let mp_clone = mp.clone();
                let filter_clone = filter.clone();
                let on_drilldown_clone = on_drilldown.clone();

                view! {
                    <td
                        class=value_class
                        on:click=move |_| {
                            on_drilldown_clone(filter_clone.clone(), Some(mp_clone.clone()));
                        }
                        title="Нажмите для детализации"
                    >
                        {formatted}
                    </td>
                }
            }).collect_view()}
            <td class=total_class>
                {format_number(row.values.get("total").copied().unwrap_or(0.0))}
            </td>
        </tr>
    }
}

