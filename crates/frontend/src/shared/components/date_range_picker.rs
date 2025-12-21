use chrono::{Datelike, Duration, NaiveDate, Utc};
use leptos::prelude::*;
use thaw::*;

/// DateRangePicker component - переиспользуемый компонент для выбора периода дат
/// Включает 2 поля ввода дат и 3 кнопки быстрого выбора (текущий месяц, предыдущий месяц, произвольный)
/// Стилизован в соответствии с Thaw UI
#[component]
pub fn DateRangePicker(
    /// Значение даты "от" в формате yyyy-mm-dd
    #[prop(into)]
    date_from: Signal<String>,

    /// Значение даты "до" в формате yyyy-mm-dd
    #[prop(into)]
    date_to: Signal<String>,

    /// Callback при изменении диапазона дат (from, to)
    on_change: Callback<(String, String)>,

    /// Опциональная метка для компонента
    #[prop(optional)]
    label: Option<String>,
) -> impl IntoView {
    // State для Dialog выбора произвольного периода
    let show_picker = RwSignal::new(false);
    let selected_month = RwSignal::new(Utc::now().date_naive().month().to_string());
    let selected_year = RwSignal::new(Utc::now().date_naive().year().to_string());

    // Обработчик изменения даты "от"
    let on_from_change = {
        let on_change = on_change.clone();
        move |new_from: String| {
            let current_to = date_to.get_untracked();
            on_change.run((new_from, current_to));
        }
    };

    // Обработчик изменения даты "до"
    let on_to_change = move |new_to: String| {
        let current_from = date_from.get_untracked();
        on_change.run((current_from, new_to));
    };

    // Установить текущий месяц
    let on_current_month = {
        let on_change = on_change.clone();
        move |_| {
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

            on_change.run((
                month_start.format("%Y-%m-%d").to_string(),
                month_end.format("%Y-%m-%d").to_string(),
            ));
        }
    };

    // Установить предыдущий месяц
    let on_previous_month = {
        let on_change = on_change.clone();
        move |_| {
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

            on_change.run((
                month_start.format("%Y-%m-%d").to_string(),
                month_end.format("%Y-%m-%d").to_string(),
            ));
        }
    };

    // Открыть Dialog выбора произвольного периода
    let on_open_picker = move |_| {
        show_picker.set(true);
    };

    // Применить произвольный выбранный месяц/год
    let on_apply_custom = {
        let on_change = on_change.clone();
        move |_| {
            let year_str = selected_year.get();
            let month_str = selected_month.get();

            if let (Ok(year), Ok(month)) = (year_str.parse::<i32>(), month_str.parse::<u32>()) {
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

                on_change.run((
                    month_start.format("%Y-%m-%d").to_string(),
                    month_end.format("%Y-%m-%d").to_string(),
                ));
            }
            show_picker.set(false);
        }
    };

    view! {

    <style>
        ".date-range-picker-compact .thaw-button--small { width: 32px; min-width: 32px; height: 32px;}"
    </style>

        <Flex vertical=true gap=FlexGap::Small>
            // Label на отдельной строке
            {label.map(|l| view! {
                <Label>{l}</Label>
            })}

            // Даты и кнопки на второй строке
            <Flex align=FlexAlign::Center gap=FlexGap::Small style="border: 1px solid var(--colorNeutralStroke1, #d1d1d1); border-radius: var(--borderRadiusMedium, 4px); background: var(--colorNeutralBackground1, #fff); min-height: 32px; height: 32px;">
                // Поле даты "от"
                <input
                    type="date"
                    prop:value=date_from
                    on:input=move |ev| {
                        on_from_change(event_target_value(&ev));
                    }
                    style="
                        padding: 0px 12px;
                        font-size: 0.875rem;
                        border: none;
                        background: var(--colorNeutralBackground6, #fff);
                        color: var(--colorNeutralForeground1, #242424);
                        width: 130px;
                        transition: border-color 0.15s ease;
                    "
                />

                <div>"—"</div>

                // Поле даты "до"
                <input
                    type="date"
                    prop:value=date_to
                    on:input=move |ev| {
                        on_to_change(event_target_value(&ev));
                    }
                    style="
                        padding: 0px 12px;
                        font-size: 0.875rem;
                        border: none;
                        background: var(--colorNeutralBackground6, #fff);
                        color: var(--colorNeutralForeground1, #242424);
                        width: 130px;
                        transition: border-color 0.15s ease;
                    "
                />
                <div class="date-range-picker-compact">
                <ButtonGroup>
                    // Кнопка "Предыдущий месяц"
                    <Button
                        size=ButtonSize::Small
                        appearance=ButtonAppearance::Subtle
                        on_click=move |_| on_previous_month(())
                    >
                        "-1M"
                    </Button>

                    // Кнопка "Текущий месяц"
                    <Button
                        size=ButtonSize::Small
                        appearance=ButtonAppearance::Subtle
                        on_click=move |_| on_current_month(())
                    >
                        "0M"
                    </Button>

                    // Кнопка "Произвольный период"
                    <Button
                        size=ButtonSize::Small
                        appearance=ButtonAppearance::Subtle
                        on_click=on_open_picker
                    >
                        "⋯"
                    </Button>
                </ButtonGroup>
                </div>
            </Flex>
        </Flex>

        // Dialog для выбора произвольного периода
        <Dialog open=show_picker>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Выберите месяц и год"</DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=FlexGap::Large>
                            <div>
                                <div style="margin-bottom: 8px; font-weight: 500;">"Месяц:"</div>
                                <Select value=selected_month>
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
                                </Select>
                            </div>

                            <div>
                                <div style="margin-bottom: 8px; font-weight: 500;">"Год:"</div>
                                <Input
                                    input_type=InputType::Number
                                    value=selected_year
                                />
                            </div>
                        </Flex>
                    </DialogContent>
                    <DialogActions>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=move |_| on_apply_custom(())
                        >
                            "Применить"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Subtle
                            on_click=move |_| show_picker.set(false)
                        >
                            "Отмена"
                        </Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}
