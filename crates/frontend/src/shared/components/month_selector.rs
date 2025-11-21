use chrono::{Datelike, Duration, NaiveDate, Utc};
use leptos::prelude::*;

/// MonthSelector component with quick buttons for date range selection
#[component]
pub fn MonthSelector(
    /// Callback to set date range (from, to) in yyyy-mm-dd format
    on_select: Callback<(String, String)>,
) -> impl IntoView {
    let (show_picker, set_show_picker) = signal(false);
    let (selected_month, set_selected_month) = signal(Utc::now().date_naive().month());
    let (selected_year, set_selected_year) = signal(Utc::now().date_naive().year());

    // Set current month
    let on_current_month = move |_| {
        let now = Utc::now().date_naive();
        let year = now.year();
        let month = now.month();

        let month_start =
            NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid month start date");
        let month_end = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .map(|d| d - Duration::days(1))
                .expect("Invalid month end date")
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
                .map(|d| d - Duration::days(1))
                .expect("Invalid month end date")
        };

        on_select.run((
            month_start.format("%Y-%m-%d").to_string(),
            month_end.format("%Y-%m-%d").to_string(),
        ));
    };

    // Set previous month
    let on_previous_month = move |_| {
        let now = Utc::now().date_naive();
        let (year, month) = if now.month() == 1 {
            (now.year() - 1, 12)
        } else {
            (now.year(), now.month() - 1)
        };

        let month_start =
            NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid month start date");
        let month_end = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .map(|d| d - Duration::days(1))
                .expect("Invalid month end date")
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
                .map(|d| d - Duration::days(1))
                .expect("Invalid month end date")
        };

        on_select.run((
            month_start.format("%Y-%m-%d").to_string(),
            month_end.format("%Y-%m-%d").to_string(),
        ));
    };

    // Open month/year picker
    let on_open_picker = move |_| {
        set_show_picker.set(true);
    };

    let button_style = "width: 32px; height: 32px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.75rem; background: #fff; color: #495057; cursor: pointer; font-weight: 500; transition: all 0.2s ease; display: flex; align-items: center; justify-content: center; padding: 0;";

    view! {
        <div style="display: flex; align-items: center; gap: 4px;">
            <button
                on:click=on_previous_month
                style=button_style
                title="Предыдущий месяц"
            >
                "-1M"
            </button>
            <button
                on:click=on_current_month
                style=button_style
                title="Текущий месяц"
            >
                "0M"
            </button>
            <button
                on:click=on_open_picker
                style=button_style
                title="Выбрать произвольный период"
            >
                "⋯"
            </button>

            {move || {
                if show_picker.get() {
                    view! {
                        <div style="position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000;">
                            <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 4px 12px rgba(0,0,0,0.15); min-width: 300px;">
                                <h3 style="margin: 0 0 16px 0; font-size: 1.1rem; color: #333;">"Выберите месяц и год"</h3>

                                <div style="display: flex; flex-direction: column; gap: 12px;">
                                    <div>
                                        <label style="display: block; margin-bottom: 4px; font-size: 0.875rem; color: #666;">"Месяц:"</label>
                                        <select
                                            prop:value=move || selected_month.get().to_string()
                                            on:change=move |ev| {
                                                if let Ok(month) = event_target_value(&ev).parse::<u32>() {
                                                    set_selected_month.set(month);
                                                }
                                            }
                                            style="width: 100%; padding: 8px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem;"
                                        >
                                            <option value="1">"Январь"</option>
                                            <option value="2">"Февраль"</option>
                                            <option value="3">"Март"</option>
                                            <option value="4">"Апрель"</option>
                                            <option value="5">"Май"</option>
                                            <option value="6">"Июнь"</option>
                                            <option value="7">"Июль"</option>
                                            <option value="8">"Август"</option>
                                            <option value="9">"Сентябрь"</option>
                                            <option value="10">"Октябрь"</option>
                                            <option value="11">"Ноябрь"</option>
                                            <option value="12">"Декабрь"</option>
                                        </select>
                                    </div>

                                    <div>
                                        <label style="display: block; margin-bottom: 4px; font-size: 0.875rem; color: #666;">"Год:"</label>
                                        <input
                                            type="number"
                                            prop:value=move || selected_year.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(year) = event_target_value(&ev).parse::<i32>() {
                                                    set_selected_year.set(year);
                                                }
                                            }
                                            min="2020"
                                            max="2030"
                                            style="width: 100%; padding: 8px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem;"
                                        />
                                    </div>

                                    <div style="display: flex; gap: 8px; margin-top: 8px;">
                                        <button
                                            on:click=move |_| {
                                                let year = selected_year.get();
                                                let month = selected_month.get();

                                                let month_start = NaiveDate::from_ymd_opt(year, month, 1)
                                                    .expect("Invalid month start date");
                                                let month_end = if month == 12 {
                                                    NaiveDate::from_ymd_opt(year + 1, 1, 1)
                                                        .map(|d| d - Duration::days(1))
                                                        .expect("Invalid month end date")
                                                } else {
                                                    NaiveDate::from_ymd_opt(year, month + 1, 1)
                                                        .map(|d| d - Duration::days(1))
                                                        .expect("Invalid month end date")
                                                };

                                                on_select.run((
                                                    month_start.format("%Y-%m-%d").to_string(),
                                                    month_end.format("%Y-%m-%d").to_string(),
                                                ));
                                                set_show_picker.set(false);
                                            }
                                            style="flex: 1; padding: 8px; background: linear-gradient(135deg, #4CAF50, #45a049); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.875rem; font-weight: 500;"
                                        >
                                            "Применить"
                                        </button>
                                        <button
                                            on:click=move |_| set_show_picker.set(false)
                                            style="flex: 1; padding: 8px; background: #6c757d; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.875rem; font-weight: 500;"
                                        >
                                            "Отмена"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}
        </div>
    }
}
