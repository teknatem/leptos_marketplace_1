pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::close_page_button::ClosePageButton;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use chrono::{Datelike, Utc};
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use thaw::*;

#[derive(Clone, Debug)]
struct CabinetOption {
    id: String,
    label: String,
}

async fn load_cabinet_options() -> Vec<CabinetOption> {
    let url = format!("{}/api/connection_mp", api_base());
    let Ok(resp) = Request::get(&url).send().await else {
        return vec![];
    };
    if !resp.ok() {
        return vec![];
    }
    let Ok(data) = resp.json::<Vec<ConnectionMP>>().await else {
        return vec![];
    };
    let mut options: Vec<CabinetOption> = data
        .into_iter()
        .map(|c| {
            let label = if c.base.description.trim().is_empty() {
                c.base.code.clone()
            } else {
                c.base.description.clone()
            };
            CabinetOption {
                id: c.base.id.as_string(),
                label,
            }
        })
        .collect();
    options.sort_by(|a, b| a.label.cmp(&b.label));
    options
}

pub fn format_date(iso_date: &str) -> String {
    if let Some(date_part) = iso_date.split('T').next() {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                return format!("{}.{}.{}", day, month, year);
            }
        }
    }
    iso_date.to_string()
}

pub fn format_money(value: f64) -> String {
    if value == 0.0 {
        return "—".to_string();
    }
    let formatted = format!("{:.2}", value);
    let parts: Vec<&str> = formatted.split('.').collect();
    let integer_part = parts[0];
    let decimal_part = parts.get(1).copied().unwrap_or("00");
    let mut result = String::new();
    let chars: Vec<char> = integer_part.chars().rev().collect();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && i % 3 == 0 && *ch != '-' {
            result.push('\u{00A0}');
        }
        result.push(*ch);
    }
    format!(
        "{}.{}",
        result.chars().rev().collect::<String>(),
        decimal_part
    )
}

/// DTO для списка документов.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WbDayCloseListDto {
    pub id: String,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "businessDate")]
    pub business_date: String,
    #[serde(rename = "isArchived")]
    pub is_archived: bool,
    pub archived_at: Option<String>,
    pub archived_reason: Option<String>,
    pub last_recalculated_at: Option<String>,
    pub snapshot_hash: String,
    pub lines_count: i64,
    pub problems_block: i64,
    pub problems_warn: i64,
    pub problems_info: i64,
    #[serde(default)]
    pub problem_lines: i64,
    pub result: f64,
    pub margin_diff: f64,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

async fn fetch_list(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    include_archived: bool,
) -> Result<Vec<WbDayCloseListDto>, String> {
    let mut url = format!("{}/api/a033/wb-day-close", api_base());
    let mut params: Vec<String> = Vec::new();
    if !connection_id.is_empty() {
        params.push(format!("connection_id={}", connection_id));
    }
    if !date_from.is_empty() {
        params.push(format!("date_from={}", date_from));
    }
    if !date_to.is_empty() {
        params.push(format!("date_to={}", date_to));
    }
    if include_archived {
        params.push("include_archived=true".to_string());
    }
    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.ok() {
        return Err(format!("Server error: {}", resp.status()));
    }
    resp.json::<Vec<WbDayCloseListDto>>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}

#[component]
pub fn WbDayCloseList() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    let now = Utc::now().date_naive();
    let (year, month) = (now.year(), now.month());
    let month_start = chrono::NaiveDate::from_ymd_opt(year, month, 1).expect("start");
    let month_end = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .map(|d| d - chrono::Duration::days(1))
    .expect("end");

    let filter_connection_id = RwSignal::new(String::new());
    let filter_date_from = RwSignal::new(month_start.format("%Y-%m-%d").to_string());
    let filter_date_to = RwSignal::new(month_end.format("%Y-%m-%d").to_string());
    let show_archived = RwSignal::new(false);
    let cabinets = RwSignal::new(Vec::<CabinetOption>::new());

    let load_list = {
        let state = state;
        move || {
            let cid = filter_connection_id.get_untracked();
            let df = filter_date_from.get_untracked();
            let dt = filter_date_to.get_untracked();
            let archived = show_archived.get_untracked();
            set_loading.set(true);
            set_error.set(None);
            spawn_local(async move {
                match fetch_list(&cid, &df, &dt, archived).await {
                    Ok(items) => {
                        state.update(|s| {
                            s.items = items;
                            s.is_loaded = true;
                        });
                        set_loading.set(false);
                    }
                    Err(e) => {
                        set_error.set(Some(e));
                        set_loading.set(false);
                    }
                }
            });
        }
    };

    let load_list_clone = load_list.clone();
    Effect::new(move |_| {
        spawn_local(async move {
            cabinets.set(load_cabinet_options().await);
        });
        load_list_clone();
    });

    let open_details = {
        let tabs_store = tabs_store.clone();
        move |id: String, date: String| {
            let key = format!("a033_wb_day_close_details_{}", id);
            let label = format!("Закрытие {}", date);
            tabs_store.open_tab(&key, &label);
        }
    };

    view! {
        <PageFrame page_id="a033_wb_day_close" category="list">

            // ── Заголовок ─────────────────────────────────────────────
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="margin-bottom: 16px;">
                <h1 style="font-size: 24px; font-weight: bold;">"Закрытие дня WB"</h1>
                <Space>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click={
                            let tabs = tabs_store.clone();
                            move |_| tabs.open_tab("a033_wb_day_close_new", "Новый документ")
                        }
                    >
                        {icon("plus")} " Создать"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click={
                            let ll = load_list.clone();
                            move |_| ll()
                        }
                        disabled=Signal::derive(move || loading.get())
                    >
                        {icon("refresh-cw")} " Обновить"
                    </Button>
                    <ClosePageButton />
                </Space>
            </Flex>

            // ── Фильтры ───────────────────────────────────────────────
            <div style="display: flex; flex-wrap: wrap; align-items: flex-end; gap: 12px; margin-bottom: 12px;">
                <div style="min-width: 420px;">
                    <DateRangePicker
                        date_from=Signal::derive(move || filter_date_from.get())
                        date_to=Signal::derive(move || filter_date_to.get())
                        on_change=Callback::new(move |(from, to): (String, String)| {
                            filter_date_from.set(from);
                            filter_date_to.set(to);
                        })
                        label="Период:".to_string()
                    />
                </div>
                <div style="width: 280px;">
                    <Flex vertical=true gap=FlexGap::Small>
                        <Label>"Кабинет:"</Label>
                        <Select value=filter_connection_id>
                            <option value="">"Все кабинеты"</option>
                            {move || cabinets.get().into_iter().map(|cab| {
                                view! {
                                    <option value=cab.id.clone()>{cab.label}</option>
                                }
                            }).collect_view()}
                        </Select>
                    </Flex>
                </div>
                <div style="display: flex; align-items: flex-end; gap: 8px;">
                    <Flex vertical=true gap=FlexGap::Small>
                        <Label>" "</Label>
                        <Checkbox checked=show_archived label="Архивные" />
                    </Flex>
                </div>
                <div style="display: flex; align-items: flex-end;">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click={
                            let ll = load_list.clone();
                            move |_| ll()
                        }
                        disabled=Signal::derive(move || loading.get())
                    >
                        {icon("search")} " Найти"
                    </Button>
                </div>
            </div>


            // ── Ошибка загрузки ───────────────────────────────────────
            {move || error.get().map(|e| view! {
                <div style="padding: 12px; background: var(--color-error-50, #fef2f2); border: 1px solid var(--color-error-100, #fee2e2); border-radius: 8px; display: flex; align-items: center; gap: 8px; margin-bottom: 12px;">
                    <span style="color: var(--color-error, #dc2626); font-size: 18px;">"⚠"</span>
                    <span style="color: var(--color-error, #dc2626);">{e}</span>
                </div>
            })}

            // ── Таблица ───────────────────────────────────────────────
            {move || if loading.get() {
                view! {
                    <div style="padding: 16px; text-align: center;">
                        <Spinner /> " Загрузка..."
                    </div>
                }.into_any()
            } else if state.get().items.is_empty() && state.get().is_loaded {
                view! {
                    <div style="padding: 16px; text-align: center; color: var(--colorNeutralForeground3, #888);">
                        "Нет данных"
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}

            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell>"Дата"</TableHeaderCell>
                        <TableHeaderCell>"Кабинет"</TableHeaderCell>
                        <TableHeaderCell>"Строк"</TableHeaderCell>
                        <TableHeaderCell>"Блок."</TableHeaderCell>
                        <TableHeaderCell>"Пред."</TableHeaderCell>
                        <TableHeaderCell>"Результат"</TableHeaderCell>
                        <TableHeaderCell>"Маржа"</TableHeaderCell>
                        <TableHeaderCell>"Статус"</TableHeaderCell>
                        <TableHeaderCell>"Пересчитан"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || {
                        let rows = state.get().items;
                        let cab_map: std::collections::HashMap<String, String> = cabinets
                            .get()
                            .into_iter()
                            .map(|c| (c.id, c.label))
                            .collect();
                        rows.into_iter().map(|item| {
                            let id = item.id.clone();
                            let date = item.business_date.clone();
                            let open = open_details.clone();
                            let cab_label = cab_map
                                .get(&item.connection_id)
                                .cloned()
                                .unwrap_or_else(|| item.connection_id[..item.connection_id.len().min(16)].to_string());
                            view! {
                                <TableRow on:click=move |_| open(id.clone(), date.clone())>
                                    <TableCell>
                                        <TableCellLayout>
                                            <strong>{format_date(&item.business_date)}</strong>
                                        </TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>
                                            {cab_label}
                                        </TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>
                                            {item.lines_count}
                                        </TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>
                                            {if item.problems_block > 0 {
                                                view! {
                                                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Danger>
                                                        {item.problems_block}
                                                    </Badge>
                                                }.into_any()
                                            } else {
                                                view! { <span>"—"</span> }.into_any()
                                            }}
                                        </TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>
                                            {if item.problems_warn > 0 {
                                                view! {
                                                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Warning>
                                                        {item.problems_warn}
                                                    </Badge>
                                                }.into_any()
                                            } else {
                                                view! { <span>"—"</span> }.into_any()
                                            }}
                                        </TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>
                                            {format_money(item.result)}
                                        </TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>
                                            {format_money(item.margin_diff)}
                                        </TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>
                                            {if item.is_archived {
                                                view! {
                                                    <Badge appearance=BadgeAppearance::Outline color=BadgeColor::Subtle>
                                                        "Архив"
                                                    </Badge>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Success>
                                                        "Активен"
                                                    </Badge>
                                                }.into_any()
                                            }}
                                        </TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>
                                            {item.last_recalculated_at
                                                .as_deref()
                                                .map(format_date)
                                                .unwrap_or_else(|| "—".to_string())}
                                        </TableCellLayout>
                                    </TableCell>
                                </TableRow>
                            }
                        }).collect_view().into_any()
                    }}
                </TableBody>
            </Table>

        </PageFrame>
    }
}
