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

    // Установить предыдущий месяц (вычитает от текущего выбранного периода)
    let on_previous_month = {
        let on_change = on_change.clone();
        move |_| {
            // Берем текущую дату "от" и вычитаем от нее месяц
            let current_from = date_from.get_untracked();

            // Парсим текущую дату
            if let Ok(current_date) = NaiveDate::parse_from_str(&current_from, "%Y-%m-%d") {
                let (year, month) = if current_date.month() == 1 {
                    (current_date.year() - 1, 12)
                } else {
                    (current_date.year(), current_date.month() - 1)
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

    // Обработчик выбора месяца по кнопке
    let on_select_month = move |month: u32| {
        selected_month.set(month.to_string());
    };

    // Установить текущий год
    let on_current_year = move |_| {
        let current_year = Utc::now().date_naive().year();
        selected_year.set(current_year.to_string());
    };

    // Установить предыдущий год
    let on_previous_year = move |_| {
        let previous_year = Utc::now().date_naive().year() - 1;
        selected_year.set(previous_year.to_string());
    };

    view! {

    <style>
        ".date-range-picker-compact .thaw-button--small { width: 32px; min-width: 32px; height: 30px;}"
        "
        /* Match Thaw Input visuals (bottom stroke differs) */
        .date-range-picker {
            box-sizing: border-box;
            border: 1px solid var(--colorNeutralStroke1, #d1d1d1);
            border-bottom-color: var(--colorNeutralStrokeAccessible, var(--colorNeutralStroke2, rgba(0, 0, 0, 0.25)));
            border-radius: var(--borderRadiusMedium, 4px);
            background: var(--colorNeutralBackground1, #fff);
            min-height: 32px;
            height: 32px;
            box-shadow: none;
        }

        .date-range-picker:hover {
            border-color: var(--colorNeutralStroke1Hover, var(--colorNeutralStroke1, #d1d1d1));
            border-bottom-color: var(--colorNeutralStrokeAccessibleHover, var(--colorNeutralStrokeAccessible, var(--colorNeutralStroke2, rgba(0, 0, 0, 0.25))));
        }

        .date-range-picker:focus-within {
            border-color: var(--colorBrandStroke1, var(--color-primary, #3b82f6));
            box-shadow:
                0 0 0 2px var(--colorBrandStroke2, rgba(59, 130, 246, 0.20)),
                inset 0 -1px 0 var(--colorBrandStroke1, var(--color-primary, #3b82f6));
        }

        /* Inner date inputs: mimic Thaw Input inner field */
        .date-range-picker input[type=\"date\"] {
            /*height: 100%;*/
            box-sizing: border-box;
            background: transparent;
            border-radius: 0;
            cursor: pointer;
        }

        /* Calendar icon (Chromium/WebKit) */
        .date-range-picker input[type=\"date\"]::-webkit-calendar-picker-indicator {
            cursor: pointer;
        }

        .date-range-picker input[type=\"date\"]:focus {
            outline: none;
        }
        "
    </style>

        <Flex vertical=true gap=FlexGap::Small>
            // Label на отдельной строке
            {label.map(|l| view! {
                <Label>{l}</Label>
            })}

            // Даты и кнопки на второй строке
            <Flex class="date-range-picker" align=FlexAlign::Center gap=FlexGap::Small>
                // Поле даты "от"
                <input
                    type="date"
                    prop:value=date_from
                    on:input=move |ev| {
                        on_from_change(event_target_value(&ev));
                    }
                    style="
                        margin-top: 4px;
                        margin-bottom: 4px;                    
                        padding: 0px 12px;
                        margin-left: 4px;
                        font-size: 0.875rem;
                        border: none;
                        border-radius: var(--borderRadiusMedium, 4px);
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
                        margin-top: 4px;
                        margin-bottom: 4px;                    
                        padding: 0px 12px;
                        font-size: 0.875rem;
                        border: none;
                        border-radius: var(--borderRadiusMedium, 4px);
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
                            // Секция выбора месяца - сетка 4x3
                            <div>
                                <div style="margin-bottom: 12px; font-weight: 500;">"Месяц:"</div>
                                <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px;">
                                    {
                                        let months = vec![
                                            (1, "Янв"), (2, "Фев"), (3, "Мар"), (4, "Апр"),
                                            (5, "Май"), (6, "Июн"), (7, "Июл"), (8, "Авг"),
                                            (9, "Сен"), (10, "Окт"), (11, "Ноя"), (12, "Дек"),
                                        ];

                                        months.into_iter().map(|(month_num, month_name)| {
                                            let is_selected = move || selected_month.get() == month_num.to_string();
                                            view! {
                                                <Button
                                                    size=ButtonSize::Small
                                                    appearance=move || {
                                                        if is_selected() {
                                                            ButtonAppearance::Primary
                                                        } else {
                                                            ButtonAppearance::Subtle
                                                        }
                                                    }
                                                    on_click=move |_| on_select_month(month_num)
                                                    attr:style="width: 100%;"
                                                >
                                                    {month_name}
                                                </Button>
                                            }
                                        }).collect_view()
                                    }
                                </div>
                            </div>

                            // Секция выбора года
                            <div>
                                <div style="margin-bottom: 12px; font-weight: 500;">"Год:"</div>
                                <Flex gap=FlexGap::Small vertical=false align=FlexAlign::Center>
                                    <Button
                                        size=ButtonSize::Small
                                        appearance=ButtonAppearance::Subtle
                                        on_click=on_previous_year
                                    >
                                        {(Utc::now().date_naive().year() - 1).to_string()}
                                    </Button>
                                    <Button
                                        size=ButtonSize::Small
                                        appearance=ButtonAppearance::Subtle
                                        on_click=on_current_year
                                    >
                                        {Utc::now().date_naive().year().to_string()}
                                    </Button>
                                    <Input
                                        input_type=InputType::Number
                                        value=selected_year
                                        attr:style="flex: 1;"
                                    />
                                </Flex>
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
