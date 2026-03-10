use chrono::{Datelike, Duration, NaiveDate, Utc};
use leptos::prelude::*;
use thaw::*;

fn month_range(year: i32, month: u32) -> (String, String) {
    let start = NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid date");
    let end = last_day_of_month(year, month);
    (
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
    )
}

fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .map(|d| d - Duration::days(1))
            .expect("Invalid date")
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .map(|d| d - Duration::days(1))
            .expect("Invalid date")
    }
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn is_full_month_range(date_from: &str, date_to: &str) -> bool {
    let Some(from) = parse_date(date_from) else {
        return false;
    };
    let Some(to) = parse_date(date_to) else {
        return false;
    };
    from.year() == to.year()
        && from.month() == to.month()
        && from.day() == 1
        && to == last_day_of_month(to.year(), to.month())
}

fn month_name_ru(month: u32) -> &'static str {
    match month {
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
        _ => "",
    }
}

fn month_label(date_from: &str, date_to: &str) -> String {
    if !is_full_month_range(date_from, date_to) {
        return format!("{date_from} — {date_to}");
    }

    parse_date(date_from)
        .map(|date| format!("{} {:02}", month_name_ru(date.month()), date.year() % 100))
        .unwrap_or_else(|| format!("{date_from} — {date_to}"))
}

fn anchor_date(date_from: &str, date_to: &str) -> NaiveDate {
    parse_date(date_to)
        .or_else(|| parse_date(date_from))
        .unwrap_or_else(|| Utc::now().date_naive())
}

#[component]
pub fn DateRangePickerSmart(
    #[prop(into)]
    date_from: Signal<String>,
    #[prop(into)]
    date_to: Signal<String>,
    on_change: Callback<(String, String)>,
    #[prop(optional)]
    label: Option<String>,
) -> impl IntoView {
    let show_month_picker = RwSignal::new(false);
    let show_date_picker = RwSignal::new(false);
    let selected_month = RwSignal::new(Utc::now().date_naive().month().to_string());
    let selected_year = RwSignal::new(Utc::now().date_naive().year().to_string());
    let selected_date_to = RwSignal::new(Utc::now().date_naive().format("%Y-%m-%d").to_string());

    let on_change_init = on_change.clone();
    Effect::new(move |_| {
        if date_from.get().is_empty() && date_to.get().is_empty() {
            let now = Utc::now().date_naive();
            let (start, end) = month_range(now.year(), now.month());
            on_change_init.run((start, end));
        }
    });

    let is_month_mode = Signal::derive(move || {
        let from = date_from.get();
        let to = date_to.get();
        is_full_month_range(&from, &to)
    });

    let month_mode_label = Signal::derive(move || {
        let from = date_from.get();
        let to = date_to.get();
        month_label(&from, &to)
    });

    let on_from_change = {
        let on_change = on_change.clone();
        move |new_from: String| {
            let current_to = date_to.get_untracked();
            on_change.run((new_from, current_to));
        }
    };

    let on_to_change = {
        let on_change = on_change.clone();
        move |new_to: String| {
            let current_from = date_from.get_untracked();
            on_change.run((current_from, new_to));
        }
    };

    let on_current_month = {
        let on_change = on_change.clone();
        move |_| {
            let now = Utc::now().date_naive();
            let (start, end) = month_range(now.year(), now.month());
            on_change.run((start, end));
        }
    };

    let on_previous_month = {
        let on_change = on_change.clone();
        move |_| {
            let current_from = date_from.get_untracked();
            if let Ok(d) = NaiveDate::parse_from_str(&current_from, "%Y-%m-%d") {
                let (year, month) = if d.month() == 1 {
                    (d.year() - 1, 12)
                } else {
                    (d.year(), d.month() - 1)
                };
                let (start, end) = month_range(year, month);
                on_change.run((start, end));
            }
        }
    };

    let on_next_month = {
        let on_change = on_change.clone();
        move |_| {
            let current_from = date_from.get_untracked();
            if let Ok(d) = NaiveDate::parse_from_str(&current_from, "%Y-%m-%d") {
                let (year, month) = if d.month() == 12 {
                    (d.year() + 1, 1)
                } else {
                    (d.year(), d.month() + 1)
                };
                let (start, end) = month_range(year, month);
                on_change.run((start, end));
            }
        }
    };

    let sync_dialog_state = move || {
        let anchor = anchor_date(&date_from.get_untracked(), &date_to.get_untracked());
        selected_month.set(anchor.month().to_string());
        selected_year.set(anchor.year().to_string());
        selected_date_to.set(anchor.format("%Y-%m-%d").to_string());
    };

    let on_open_month_picker = move |_| {
        sync_dialog_state();
        show_month_picker.set(true);
    };

    let on_apply_month = {
        let on_change = on_change.clone();
        move |_| {
            let year_str = selected_year.get();
            let month_str = selected_month.get();
            if let (Ok(year), Ok(month)) = (year_str.parse::<i32>(), month_str.parse::<u32>()) {
                let (start, end) = month_range(year, month);
                on_change.run((start, end));
            }
            show_month_picker.set(false);
        }
    };

    let on_open_date_picker = move |_| {
        let year = selected_year.get().parse::<i32>().ok();
        let month = selected_month.get().parse::<u32>().ok();
        if let (Some(year), Some(month)) = (year, month) {
            selected_date_to.set(last_day_of_month(year, month).format("%Y-%m-%d").to_string());
        }
        show_month_picker.set(false);
        show_date_picker.set(true);
    };

    let on_apply_date_to = {
        let on_change = on_change.clone();
        move |_| {
            let selected = selected_date_to.get();
            if let Some(date_to_value) = parse_date(&selected) {
                let date_from_value =
                    NaiveDate::from_ymd_opt(date_to_value.year(), date_to_value.month(), 1)
                        .expect("Invalid date");
                on_change.run((
                    date_from_value.format("%Y-%m-%d").to_string(),
                    date_to_value.format("%Y-%m-%d").to_string(),
                ));
            }
            show_date_picker.set(false);
        }
    };

    let on_select_month = move |month: u32| {
        selected_month.set(month.to_string());
    };

    let on_current_year = move |_| {
        selected_year.set(Utc::now().date_naive().year().to_string());
    };

    let on_previous_year = move |_| {
        selected_year.set((Utc::now().date_naive().year() - 1).to_string());
    };

    view! {
        <Flex vertical=true gap=FlexGap::Small style="max-width: 450px; width: 100%;">
            {label.map(|l| view! { <Label>{l}</Label> })}

            <Flex
                class=Signal::derive(move || {
                    if is_month_mode.get() {
                        "date-range-picker date-range-picker--smart date-range-picker--month".to_string()
                    } else {
                        "date-range-picker date-range-picker--smart".to_string()
                    }
                })
                align=FlexAlign::Center
                gap=FlexGap::Small
            >
                <Show
                    when=move || is_month_mode.get()
                    fallback=move || {
                        view! {
                            <>
                                <input
                                    type="date"
                                    class="date-range-picker__input"
                                    prop:value=date_from
                                    on:input=move |ev| on_from_change(event_target_value(&ev))
                                />

                                <div>"—"</div>

                                <input
                                    type="date"
                                    class="date-range-picker__input"
                                    prop:value=date_to
                                    on:input=move |ev| on_to_change(event_target_value(&ev))
                                />
                            </>
                        }
                    }
                >
                    <div class="date-range-picker__month-summary">
                        <span class="date-range-picker__month-label">{move || month_mode_label.get()}</span>
                    </div>
                </Show>

                <div class="drp-nav-buttons">
                    <button
                        class="drp-icon-btn"
                        title="-1 месяц"
                        on:click=move |_| on_previous_month(())
                    >
                        <div class="drp-btn-icon">
                            <svg width="10" height="12" viewBox="0 0 10 12" fill="none">
                                <path d="M7 1L2 6l5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                            </svg>
                            <span>"M"</span>
                        </div>
                    </button>

                    <button
                        class="drp-icon-btn"
                        title="Текущий месяц"
                        on:click=move |_| on_current_month(())
                    >
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                            <circle cx="7" cy="7" r="3.5" stroke="currentColor" stroke-width="1.5"/>
                            <circle cx="7" cy="7" r="1.5" fill="currentColor"/>
                        </svg>
                    </button>

                    <button
                        class="drp-icon-btn"
                        title="+1 месяц"
                        on:click=move |_| on_next_month(())
                    >
                        <div class="drp-btn-icon">
                            <span>"M"</span>
                            <svg width="10" height="12" viewBox="0 0 10 12" fill="none">
                                <path d="M3 1l5 5-5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                            </svg>
                        </div>
                    </button>

                    <button
                        class="drp-icon-btn"
                        title="Выбрать период"
                        on:click=move |_| on_open_month_picker(())
                    >
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                            <rect x="1.5" y="3" width="11" height="9.5" rx="1.5" stroke="currentColor" stroke-width="1.3"/>
                            <path d="M1.5 6.5h11" stroke="currentColor" stroke-width="1.3"/>
                            <path d="M4.5 1.5v2M9.5 1.5v2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
                        </svg>
                    </button>
                </div>
            </Flex>
        </Flex>

        <Dialog open=show_month_picker>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Выберите месяц и год"</DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=FlexGap::Large>
                            <div>
                                <div style="margin-bottom: 12px; font-weight: 500;">"Месяц:"</div>
                                <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px;">
                                    {
                                        let months = vec![
                                            (1, "Январь"), (2, "Февраль"), (3, "Март"),
                                            (4, "Апрель"), (5, "Май"), (6, "Июнь"),
                                            (7, "Июль"), (8, "Август"), (9, "Сентябрь"),
                                            (10, "Октябрь"), (11, "Ноябрь"), (12, "Декабрь"),
                                        ];
                                        months.into_iter().map(|(month_num, month_name)| {
                                            let is_selected = move || selected_month.get() == month_num.to_string();
                                            view! {
                                                <Button
                                                    appearance=move || {
                                                        if is_selected() { ButtonAppearance::Primary }
                                                        else { ButtonAppearance::Subtle }
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
                            appearance=ButtonAppearance::Subtle
                            on_click=move |_| on_open_date_picker(())
                        >
                            "Выбрать дату"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=move |_| on_apply_month(())
                        >
                            "Применить"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Subtle
                            on_click=move |_| show_month_picker.set(false)
                        >
                            "Отмена"
                        </Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>

        <Dialog open=show_date_picker>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Выберите дату окончания периода"</DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=FlexGap::Small>
                            <Label>"Дата окончания (Date_to)"</Label>
                            <input
                                type="date"
                                class="date-range-picker__input date-range-picker__dialog-date-input"
                                prop:value=selected_date_to
                                on:input=move |ev| selected_date_to.set(event_target_value(&ev))
                            />
                        </Flex>
                    </DialogContent>
                    <DialogActions>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=move |_| on_apply_date_to(())
                        >
                            "Применить"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Subtle
                            on_click=move |_| show_date_picker.set(false)
                        >
                            "Отмена"
                        </Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}
