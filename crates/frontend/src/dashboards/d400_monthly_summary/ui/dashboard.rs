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
        <div class="dashboard-container" style="padding: 16px; height: 100%; display: flex; flex-direction: column;">
            // Header with month selector
            <div class="dashboard-header" style="display: flex; align-items: center; gap: 16px; margin-bottom: 16px; padding-bottom: 16px; border-bottom: 1px solid #e0e0e0;">
                <h2 style="margin: 0; font-size: 1.25rem; color: #333; font-weight: 600;">
                    "Сводка по маркетплейсам"
                </h2>

                <div style="display: flex; align-items: center; gap: 8px; margin-left: auto;">
                    <button
                        on:click=go_prev_month
                        style="padding: 6px 12px; border: 1px solid #ced4da; border-radius: 4px; background: #fff; cursor: pointer; font-size: 0.875rem;"
                        title="Предыдущий месяц"
                    >
                        "←"
                    </button>

                    <div style="display: flex; align-items: center; gap: 8px; padding: 6px 16px; background: #f8f9fa; border-radius: 4px; min-width: 160px; justify-content: center;">
                        <span style="font-weight: 600; color: #333;">
                            {month_name}
                        </span>
                        <span style="color: #666;">
                            {move || selected_year.get()}
                        </span>
                    </div>

                    <button
                        on:click=go_next_month
                        style="padding: 6px 12px; border: 1px solid #ced4da; border-radius: 4px; background: #fff; cursor: pointer; font-size: 0.875rem;"
                        title="Следующий месяц"
                    >
                        "→"
                    </button>

                    <button
                        on:click=go_current_month
                        style="padding: 6px 12px; border: 1px solid #ced4da; border-radius: 4px; background: #e3f2fd; cursor: pointer; font-size: 0.875rem;"
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
                        <div style="display: flex; align-items: center; justify-content: center; padding: 40px; color: #666;">
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
                        <div style="padding: 16px; background: #ffebee; border-radius: 4px; color: #c62828; margin-bottom: 16px;">
                            <strong>"Ошибка: "</strong>
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
                        <div class="dashboard-table-container" style="flex: 1; overflow: auto;">
                            <table class="dashboard-table" style="width: 100%; border-collapse: collapse; font-size: 0.875rem;">
                                <thead>
                                    <tr style="background: #f5f5f5;">
                                        <th style="padding: 12px 16px; text-align: left; border-bottom: 2px solid #e0e0e0; font-weight: 600; color: #333; min-width: 200px;">
                                            "Показатель"
                                        </th>
                                        {marketplaces.iter().map(|mp| {
                                            let mp_display = mp.clone();
                                            view! {
                                                <th style="padding: 12px 16px; text-align: right; border-bottom: 2px solid #e0e0e0; font-weight: 600; color: #333; min-width: 120px;">
                                                    {mp_display}
                                                </th>
                                            }
                                        }).collect_view()}
                                        <th style="padding: 12px 16px; text-align: right; border-bottom: 2px solid #e0e0e0; font-weight: 600; color: #333; background: #e8f5e9; min-width: 140px;">
                                            "Итого"
                                        </th>
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
                        <div style="display: flex; align-items: center; justify-content: center; padding: 40px; color: #666;">
                            "Нет данных для отображения"
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
    let row_style = if is_total {
        "background: #fff; font-weight: 600;"
    } else {
        "background: #fafafa;"
    };

    let name_style = if is_total {
        "padding: 12px 16px; border-bottom: 1px solid #e0e0e0; color: #333;"
    } else {
        "padding: 8px 16px; padding-left: 32px; border-bottom: 1px solid #f0f0f0; color: #666; font-size: 0.8125rem;"
    };

    let value_style = if is_total {
        "padding: 12px 16px; text-align: right; border-bottom: 1px solid #e0e0e0; color: #1976d2; cursor: pointer;"
    } else {
        "padding: 8px 16px; text-align: right; border-bottom: 1px solid #f0f0f0; color: #1976d2; cursor: pointer; font-size: 0.8125rem;"
    };

    let total_value_style = if is_total {
        "padding: 12px 16px; text-align: right; border-bottom: 1px solid #e0e0e0; background: #e8f5e9; font-weight: 700; color: #2e7d32;"
    } else {
        "padding: 8px 16px; text-align: right; border-bottom: 1px solid #f0f0f0; background: #f1f8e9; color: #2e7d32; font-size: 0.8125rem;"
    };

    // Display name with group
    let display_name = if let Some(ref group) = row.group_name {
        format!("└ {}", group)
    } else {
        row.indicator_name.clone()
    };

    let filter = row.drilldown_filter.clone();

    view! {
        <tr style=row_style>
            <td style=name_style>
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
                        style=value_style
                        on:click=move |_| {
                            on_drilldown_clone(filter_clone.clone(), Some(mp_clone.clone()));
                        }
                        title="Нажмите для детализации"
                    >
                        {formatted}
                    </td>
                }
            }).collect_view()}
            <td style=total_value_style>
                {format_number(row.values.get("total").copied().unwrap_or(0.0))}
            </td>
        </tr>
    }
}

