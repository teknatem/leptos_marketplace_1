pub mod state;

use super::details::OzonFboPostingDetail;
use self::state::create_state;
use crate::shared::api_utils::api_base;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCellMoney, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use crate::shared::modal_stack::ModalStackService;
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::init_column_resize;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use thaw::*;

fn format_date(iso_date: &str) -> String {
    if let Some(date_part) = iso_date.split('T').next() {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                return format!("{}.{}.{}", day, month, year);
            }
        }
    }
    iso_date.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OzonFboPostingDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub document_no: String,
    pub status_norm: String,
    pub substatus_raw: Option<String>,
    pub created_at_source: Option<String>,
    pub total_amount: f64,
    pub line_count: usize,
    pub is_posted: bool,
}

impl Sortable for OzonFboPostingDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "document_no" => self
                .document_no
                .to_lowercase()
                .cmp(&other.document_no.to_lowercase()),
            "status_norm" => self
                .status_norm
                .to_lowercase()
                .cmp(&other.status_norm.to_lowercase()),
            "substatus_raw" => match (&self.substatus_raw, &other.substatus_raw) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "created_at_source" => match (&self.created_at_source, &other.created_at_source) {
                (Some(a), Some(b)) => a.cmp(b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "total_amount" => self
                .total_amount
                .partial_cmp(&other.total_amount)
                .unwrap_or(Ordering::Equal),
            "line_count" => self.line_count.cmp(&other.line_count),
            "description" => self
                .description
                .to_lowercase()
                .cmp(&other.description.to_lowercase()),
            "is_posted" => self.is_posted.cmp(&other.is_posted),
            _ => Ordering::Equal,
        }
    }
}

const TABLE_ID: &str = "a011-ozon-fbo-posting-table";
const COLUMN_WIDTHS_KEY: &str = "a011_ozon_fbo_posting_column_widths";

#[component]
pub fn OzonFboPostingList() -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);
    let (posting_in_progress, set_posting_in_progress) = signal(false);
    let (current_operation, set_current_operation) = signal::<Option<(usize, usize)>>(None);
    let (detail_reload_trigger, set_detail_reload_trigger) = signal::<u32>(0);

    let all_rows: RwSignal<Vec<OzonFboPostingDto>> = RwSignal::new(Vec::new());
    let status_filter = RwSignal::new(String::new());
    let date_from = RwSignal::new(String::new());
    let date_to = RwSignal::new(String::new());

    let refresh_view = move || {
        let source = all_rows.get_untracked();
        let df = date_from.get_untracked();
        let dt = date_to.get_untracked();
        let status = status_filter.get_untracked();
        let field = state.with_untracked(|s| s.sort_field.clone());
        let ascending = state.with_untracked(|s| s.sort_ascending);
        let page_size = state.with_untracked(|s| s.page_size);
        let page = state.with_untracked(|s| s.page);

        let mut filtered: Vec<OzonFboPostingDto> = source
            .into_iter()
            .filter(|item| {
                if !df.is_empty() || !dt.is_empty() {
                    if let Some(ref d) = item.created_at_source {
                        let date_part = d.split('T').next().unwrap_or(d.as_str());
                        if !df.is_empty() && date_part < df.as_str() { return false; }
                        if !dt.is_empty() && date_part > dt.as_str() { return false; }
                    } else {
                        return false;
                    }
                }
                if !status.is_empty() && item.status_norm != status { return false; }
                true
            })
            .collect();

        filtered.sort_by(|a, b| {
            let cmp = a.compare_by_field(b, &field);
            if ascending { cmp } else { cmp.reverse() }
        });

        let total = filtered.len();
        let total_pages = if total == 0 { 0 } else { (total + page_size - 1) / page_size };
        let page = page.min(if total_pages == 0 { 0 } else { total_pages - 1 });
        let start = page * page_size;
        let end = (start + page_size).min(total);
        let page_items = if start < total { filtered[start..end].to_vec() } else { vec![] };

        state.update(|s| {
            s.items = page_items;
            s.total_count = total;
            s.total_pages = total_pages;
            s.page = page;
        });
    };

    let load_postings = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            let url = format!("{}/api/a011/ozon-fbo-posting", api_base());
            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        match response.text().await {
                            Ok(text) => {
                                match serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                                    Ok(data) => {
                                        let items: Vec<OzonFboPostingDto> = data
                                            .into_iter()
                                            .enumerate()
                                            .filter_map(|(idx, v)| {
                                                let status_norm = v
                                                    .get("state")
                                                    .and_then(|s| s.get("status_norm"))
                                                    .and_then(|d| d.as_str())
                                                    .map(|s| s.to_string())
                                                    .unwrap_or_else(|| "UNKNOWN".to_string());
                                                let substatus_raw = v
                                                    .get("state")
                                                    .and_then(|s| s.get("substatus_raw"))
                                                    .and_then(|d| d.as_str())
                                                    .map(|s| s.to_string());
                                                let created_at_source = v
                                                    .get("state")
                                                    .and_then(|s| s.get("created_at"))
                                                    .and_then(|d| d.as_str())
                                                    .map(|s| s.to_string());
                                                let lines = v.get("lines")?.as_array()?;
                                                let line_count = lines.len();
                                                let total_amount: f64 = lines
                                                    .iter()
                                                    .filter_map(|line| line.get("amount_line")?.as_f64())
                                                    .sum();
                                                let is_posted = v
                                                    .get("is_posted")
                                                    .and_then(|p| p.as_bool())
                                                    .unwrap_or(false);
                                                let result = Some(OzonFboPostingDto {
                                                    id: v.get("id")?.as_str()?.to_string(),
                                                    code: v.get("code")?.as_str()?.to_string(),
                                                    description: v.get("description")?.as_str()?.to_string(),
                                                    document_no: v.get("header")?.get("document_no")?.as_str()?.to_string(),
                                                    status_norm,
                                                    substatus_raw,
                                                    created_at_source,
                                                    total_amount,
                                                    line_count,
                                                    is_posted,
                                                });
                                                if result.is_none() {
                                                    log!("Failed to parse item {}", idx);
                                                }
                                                result
                                            })
                                            .collect();
                                        log!("Loaded {} FBO postings", items.len());
                                        all_rows.set(items);
                                        state.update(|s| { s.page = 0; s.is_loaded = true; });
                                        refresh_view();
                                    }
                                    Err(e) => set_error.set(Some(format!("Ошибка парсинга: {}", e))),
                                }
                            }
                            Err(e) => set_error.set(Some(format!("Ошибка чтения ответа: {}", e))),
                        }
                    } else {
                        set_error.set(Some(format!("Ошибка сервера: {}", response.status())));
                    }
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Ошибка сети: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_postings();
        }
    });

    let resize_initialized = StoredValue::new(false);
    Effect::new(move |_| {
        if !resize_initialized.get_value() {
            resize_initialized.set_value(true);
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(100).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    let open_detail_modal = move |id: String| {
        let reload = detail_reload_trigger;
        modal_stack.push_with_frame(
            Some("max-width: min(1200px, 95vw); width: min(1200px, 95vw); height: calc(100vh - 80px); overflow: hidden;".to_string()),
            Some("ozon-fbo-posting-detail-modal".to_string()),
            move |handle| {
                view! {
                    <OzonFboPostingDetail
                        id=id.clone()
                        on_close=Callback::new({
                            let handle = handle.clone();
                            move |_| handle.close()
                        })
                        reload_trigger=reload
                    />
                }
                .into_any()
            },
        );
    };

    let post_batch = move |post: bool| {
        let ids: Vec<String> = state.with_untracked(|s| s.selected_ids.iter().cloned().collect());
        if ids.is_empty() { return; }
        let total = ids.len();
        set_posting_in_progress.set(true);
        set_current_operation.set(Some((0, total)));
        spawn_local(async move {
            let action = if post { "post" } else { "unpost" };
            for (index, id) in ids.iter().enumerate() {
                set_current_operation.set(Some((index + 1, total)));
                let url = format!("{}/api/a011/ozon-fbo-posting/{}/{}", api_base(), id, action);
                let _ = Request::post(&url).send().await;
            }
            set_posting_in_progress.set(false);
            set_current_operation.set(None);
            state.update(|s| s.selected_ids.clear());
            set_detail_reload_trigger.update(|v| *v += 1);
            load_postings();
        });
    };

    let active_filters_count = Signal::derive(move || {
        let mut count = 0;
        if !date_from.get().is_empty() { count += 1; }
        if !date_to.get().is_empty() { count += 1; }
        if !status_filter.get().is_empty() { count += 1; }
        count
    });

    let toggle_sort = move |field: &'static str| {
        state.update(|s| {
            if s.sort_field == field { s.sort_ascending = !s.sort_ascending; }
            else { s.sort_field = field.to_string(); s.sort_ascending = true; }
            s.page = 0;
        });
        refresh_view();
    };

    let go_to_page = move |new_page: usize| {
        state.update(|s| s.page = new_page);
        refresh_view();
    };

    let change_page_size = move |new_size: usize| {
        state.update(|s| { s.page_size = new_size; s.page = 0; });
        refresh_view();
    };

    let selected = RwSignal::new(state.with_untracked(|s| s.selected_ids.clone()));

    let toggle_selection = move |id: String, checked: bool| {
        selected.update(|s| { if checked { s.insert(id.clone()); } else { s.remove(&id); } });
        state.update(|s| { if checked { s.selected_ids.insert(id); } else { s.selected_ids.remove(&id); } });
    };

    let toggle_all = move |check_all: bool| {
        let items = state.get().items;
        if check_all {
            selected.update(|s| { s.clear(); for item in items.iter() { s.insert(item.id.clone()); } });
            state.update(|s| { s.selected_ids.clear(); for item in items.iter() { s.selected_ids.insert(item.id.clone()); } });
        } else {
            selected.update(|s| s.clear());
            state.update(|s| s.selected_ids.clear());
        }
    };

    let items_signal = Signal::derive(move || state.get().items);
    let selected_signal = Signal::derive(move || selected.get());
    let selected_count = Signal::derive(move || selected.get().len());

    view! {
        <PageFrame page_id="a011_ozon_fbo_posting--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Постинги FBO (OZON)"</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </UiBadge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=move |_| post_batch(true)
                        disabled=Signal::derive(move || selected_count.get() == 0 || posting_in_progress.get())
                    >
                        {move || format!("Провести ({})", selected_count.get())}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=move |_| post_batch(false)
                        disabled=Signal::derive(move || selected_count.get() == 0 || posting_in_progress.get())
                    >
                        {move || format!("Отменить ({})", selected_count.get())}
                    </Button>
                    {move || {
                        current_operation.get().map(|(cur, total)| view! {
                            <span class="page__status">{format!("Обработка {}/{} ...", cur, total)}</span>
                        })
                    }}
                </div>
            </div>

            <div class="page__content">
                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div
                            class="filter-panel-header__left"
                            on:click=move |_| set_is_filter_expanded.update(|e| *e = !*e)
                        >
                            <svg
                                width="16" height="16" viewBox="0 0 24 24"
                                fill="none" stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round"
                                class=move || if is_filter_expanded.get() {
                                    "filter-panel__chevron filter-panel__chevron--expanded"
                                } else {
                                    "filter-panel__chevron"
                                }
                            >
                                <polyline points="6 9 12 15 18 9"></polyline>
                            </svg>
                            {icon("filter")}
                            <span class="filter-panel__title">"Фильтры"</span>
                            {move || {
                                let count = active_filters_count.get();
                                if count > 0 {
                                    view! { <span class="filter-panel__badge">{count}</span> }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}
                        </div>

                        <div class="filter-panel-header__center">
                            <PaginationControls
                                current_page=Signal::derive(move || state.get().page)
                                total_pages=Signal::derive(move || state.get().total_pages)
                                total_count=Signal::derive(move || state.get().total_count)
                                page_size=Signal::derive(move || state.get().page_size)
                                on_page_change=Callback::new(go_to_page)
                                on_page_size_change=Callback::new(change_page_size)
                                page_size_options=vec![50, 100, 200, 500]
                            />
                        </div>

                        <div class="filter-panel-header__right">
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| load_postings()
                                disabled=Signal::derive(move || loading.get())
                            >
                                {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                            </Button>
                        </div>
                    </div>

                    <Show when=move || is_filter_expanded.get()>
                        <div class="filter-panel-content">
                            <Flex gap=FlexGap::Small align=FlexAlign::End>
                                <div style="min-width: 420px;">
                                    <DateRangePicker
                                        date_from=Signal::derive(move || date_from.get())
                                        date_to=Signal::derive(move || date_to.get())
                                        on_change=Callback::new(move |(from, to)| {
                                            date_from.set(from);
                                            date_to.set(to);
                                            state.update(|s| s.page = 0);
                                            refresh_view();
                                        })
                                        label="Период (дата создания):".to_string()
                                    />
                                </div>

                                <div style="flex: 1; max-width: 220px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Статус:"</Label>
                                        <Input
                                            value=status_filter
                                            placeholder="DELIVERED, CANCELLED..."
                                        />
                                    </Flex>
                                </div>

                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click=move |_| {
                                        state.update(|s| { s.status_filter = status_filter.get_untracked(); s.page = 0; });
                                        refresh_view();
                                    }
                                    disabled=Signal::derive(move || loading.get())
                                >
                                    "Применить"
                                </Button>
                            </Flex>
                        </div>
                    </Show>
                </div>

                {move || error.get().map(|err| view! { <div class="alert alert--error">{err}</div> })}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 950px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=items_signal
                                    selected=selected_signal
                                    get_id=Callback::new(|row: OzonFboPostingDto| row.id.clone())
                                    on_change=Callback::new(toggle_all)
                                />

                                <TableHeaderCell resizable=false min_width=130.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("created_at_source")>
                                        "Дата создания"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "created_at_source"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "created_at_source", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=160.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("document_no")>
                                        "Номер"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "document_no"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "document_no", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=200.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("description")>
                                        "Описание"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "description"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "description", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("status_norm")>
                                        "Статус"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "status_norm"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "status_norm", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=80.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("line_count")>
                                        "Строк"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "line_count"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "line_count", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("total_amount")>
                                        "Сумма"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "total_amount"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "total_amount", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("is_posted")>
                                        "Проведен"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "is_posted"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "is_posted", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || state.get().items
                                key=|item| item.id.clone()
                                children=move |item| {
                                    let item_id = item.id.clone();
                                    let item_id_click = item.id.clone();
                                    let created_str = item.created_at_source.as_deref().map(format_date).unwrap_or_else(|| "—".to_string());
                                    let is_posted = item.is_posted;
                                    view! {
                                        <TableRow>
                                            <TableCellCheckbox
                                                item_id=item_id.clone()
                                                selected=selected_signal
                                                on_change=Callback::new(move |(id, checked)| toggle_selection(id, checked))
                                            />
                                            <TableCell>
                                                <TableCellLayout>
                                                    <a href="#" class="table__link"
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            open_detail_modal(item_id_click.clone());
                                                        }
                                                    >
                                                        {created_str}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.document_no.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.description.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {item.status_norm.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <span style="font-variant-numeric: tabular-nums;">{item.line_count}</span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCellMoney
                                                value=Signal::derive(move || Some(item.total_amount))
                                                show_currency=false
                                                color_by_sign=false
                                            />
                                            <TableCell>
                                                <TableCellLayout>
                                                    {if is_posted {
                                                        view! { <span class="badge badge--success">"Проведен"</span> }.into_any()
                                                    } else {
                                                        view! { <span class="badge badge--neutral">"Не проведен"</span> }.into_any()
                                                    }}
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }
                                }
                            />
                        </TableBody>
                    </Table>
                </div>
            </div>
        </PageFrame>
    }
}
