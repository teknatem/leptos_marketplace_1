pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::auth_download::download_authenticated_file;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::TableCrosshairHighlight;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use thaw::*;

fn format_date(value: &str) -> String {
    if let Some((date, time)) = value.split_once('T') {
        let time_clean = time
            .split('Z')
            .next()
            .unwrap_or(time)
            .split('+')
            .next()
            .unwrap_or(time)
            .split('.')
            .next()
            .unwrap_or(time);
        if let Some((year, rest)) = date.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                return format!("{}.{}.{} {}", day, month, year, time_clean);
            }
        }
    }
    value.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WbDocumentsListDto {
    pub id: String,
    pub service_name: String,
    pub name: String,
    pub category: String,
    pub creation_time: String,
    pub is_weekly_report: bool,
    pub report_period_from: Option<String>,
    pub report_period_to: Option<String>,
    pub viewed: bool,
    pub extensions: Vec<String>,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_name: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<WbDocumentsListDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

const TABLE_ID: &str = "a027-wb-documents-table";

#[component]
pub fn WbDocumentsList() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);
    let (connections, set_connections) = signal::<Vec<ConnectionMP>>(Vec::new());

    let load_items = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let s = state.get_untracked();
            let offset = s.page * s.page_size;
            let mut url = format!(
                "{}/api/a027/wb-documents/list?date_from={}&date_to={}&weekly_only={}&limit={}&offset={}&sort_by={}&sort_desc={}",
                api_base(),
                s.date_from,
                s.date_to,
                s.weekly_only,
                s.page_size,
                offset,
                s.sort_field,
                !s.sort_ascending
            );

            if let Some(connection_id) = s.selected_connection_id.filter(|v| !v.is_empty()) {
                url.push_str(&format!(
                    "&connection_id={}",
                    urlencoding::encode(&connection_id)
                ));
            }
            if !s.search_query.is_empty() {
                url.push_str(&format!(
                    "&search_query={}",
                    urlencoding::encode(&s.search_query)
                ));
            }

            match Request::get(&url).send().await {
                Ok(response) if response.ok() => match response.json::<PaginatedResponse>().await {
                    Ok(paginated) => {
                        state.update(|s| {
                            s.items = paginated.items;
                            s.total_count = paginated.total;
                            s.total_pages = paginated.total_pages;
                            s.page = paginated.page;
                            s.page_size = paginated.page_size;
                            s.is_loaded = true;
                        });
                    }
                    Err(e) => set_error.set(Some(format!("Ошибка парсинга: {}", e))),
                },
                Ok(response) => {
                    set_error.set(Some(format!("Ошибка сервера: {}", response.status())))
                }
                Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
            }

            set_loading.set(false);
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_items();
        }
    });

    Effect::new(move |_| {
        spawn_local(async move {
            match fetch_connections().await {
                Ok(mut items) => {
                    items.sort_by(|left, right| {
                        left.base
                            .description
                            .to_lowercase()
                            .cmp(&right.base.description.to_lowercase())
                    });
                    set_connections.set(items);
                }
                Err(err) => set_error.set(Some(err)),
            }
        });
    });

    let search_query = RwSignal::new(state.get_untracked().search_query.clone());
    Effect::new(move || {
        let value = search_query.get();
        untrack(move || state.update(|s| s.search_query = value));
    });

    let selected_connection_id = RwSignal::new(
        state
            .get_untracked()
            .selected_connection_id
            .clone()
            .unwrap_or_default(),
    );
    Effect::new(move || {
        let value = selected_connection_id.get();
        untrack(move || {
            state.update(|s| {
                s.selected_connection_id = if value.trim().is_empty() {
                    None
                } else {
                    Some(value)
                };
            });
        });
    });

    let weekly_only = RwSignal::new(state.get_untracked().weekly_only);
    Effect::new(move || {
        let value = weekly_only.get();
        untrack(move || state.update(|s| s.weekly_only = value));
    });

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0;
        if !s.date_from.is_empty() {
            count += 1;
        }
        if !s.date_to.is_empty() {
            count += 1;
        }
        if s.selected_connection_id
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        {
            count += 1;
        }
        if !s.search_query.is_empty() {
            count += 1;
        }
        if s.weekly_only {
            count += 1;
        }
        count
    });

    let open_detail = move |id: String, service_name: String| {
        tabs_store.open_tab(
            &format!("a027_wb_documents_details_{}", id),
            &format!("WB Doc {}", service_name),
        );
    };

    let toggle_sort = move |field: &'static str| {
        state.update(|s| {
            if s.sort_field == field {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_field = field.to_string();
                s.sort_ascending = true;
            }
            s.page = 0;
        });
        load_items();
    };

    let go_to_page = move |new_page: usize| {
        state.update(|s| s.page = new_page);
        load_items();
    };

    let change_page_size = move |new_size: usize| {
        state.update(|s| {
            s.page_size = new_size;
            s.page = 0;
        });
        load_items();
    };

    let start_download = Callback::new(
        move |(document_id, service_name, extension): (String, String, String)| {
            let set_error = set_error;
            spawn_local(async move {
                let url = format!(
                    "{}/api/a027/wb-documents/{}/download/{}",
                    api_base(),
                    document_id,
                    urlencoding::encode(&extension)
                );
                let fallback_filename = format!("{}_document.{}", service_name, extension);
                if let Err(err) = download_authenticated_file(&url, &fallback_filename).await {
                    set_error.set(Some(format!("РћС€РёР±РєР° СЃРєР°С‡РёРІР°РЅРёСЏ: {}", err)));
                }
            });
        },
    );

    view! {
        <PageFrame page_id="a027_wb_documents--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Документы WB"</h1>
                    <Badge appearance=BadgeAppearance::Filled>
                        {move || state.get().total_count.to_string()}
                    </Badge>
                </div>
            </div>

            <div class="page__content">
                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div
                            class="filter-panel-header__left"
                            on:click=move |_| set_is_filter_expanded.update(|e| *e = !*e)
                        >
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
                                page_size_options=vec![50, 100, 200]
                            />
                        </div>

                        <div class="filter-panel-header__right">
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| load_items()
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
                                        date_from=Signal::derive(move || state.with(|s| s.date_from.clone()))
                                        date_to=Signal::derive(move || state.with(|s| s.date_to.clone()))
                                        on_change=Callback::new(move |(from, to)| {
                                            state.update(|s| {
                                                s.date_from = from;
                                                s.date_to = to;
                                                s.page = 0;
                                            });
                                            load_items();
                                        })
                                        label="Период:".to_string()
                                    />
                                </div>

                                <div style="width: 280px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Кабинет"</Label>
                                        <Select value=selected_connection_id>
                                            <option value="">"Все кабинеты"</option>
                                            {move || {
                                                connections.get().into_iter().map(|connection| {
                                                    let id = connection.base.id.as_string();
                                                    let label = connection.base.description;
                                                    view! { <option value=id>{label}</option> }
                                                }).collect_view()
                                            }}
                                        </Select>
                                    </Flex>
                                </div>

                                <div style="min-width: 320px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Поиск"</Label>
                                        <Input
                                            value=search_query
                                            placeholder="serviceName, category, connection"
                                        />
                                    </Flex>
                                </div>
                                <div style="min-width: 220px; padding-bottom: 4px;">
                                    <Checkbox checked=weekly_only label="Только еженедельные отчеты" />
                                </div>
                            </Flex>
                        </div>
                    </Show>
                </div>

                {move || error.get().map(|err| view! { <div class="alert alert--error">{err}</div> })}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />
                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1380px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell>
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("document_date")>
                                        "Создан"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "document_date"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "document_date", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell>
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("service_name")>
                                        "Service Name"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "service_name"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "service_name", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell>"Category ID"</TableHeaderCell>
                                <TableHeaderCell>
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("is_weekly_report")>
                                        "Нед."
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "is_weekly_report"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "is_weekly_report", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell>
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("category")>
                                        "Категория"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "category"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "category", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell>"Форматы"</TableHeaderCell>
                                <TableHeaderCell>
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("report_period_from")>
                                        "Период отчета"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "report_period_from"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "report_period_from", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell>
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("viewed")>
                                        "Просмотрен"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "viewed"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "viewed", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell>"Кабинет"</TableHeaderCell>
                                <TableHeaderCell>"Организация"</TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            <For
                                each=move || state.get().items
                                key=|item| item.id.clone()
                                children=move |item| {
                                    let detail_id = item.id.clone();
                                    let detail_name = item.service_name.clone();
                                    let detail_name_for_link = detail_name.clone();
                                    let download_id = item.id.clone();
                                    let primary_date_display = item
                                        .report_period_to
                                        .clone()
                                        .unwrap_or_else(|| format_date(&item.creation_time));
                                    let period_display =
                                        match (item.report_period_from.clone(), item.report_period_to.clone()) {
                                            (Some(from), Some(to)) => format!("{} — {}", from, to),
                                            (Some(from), None) => from,
                                            (None, Some(to)) => to,
                                            (None, None) => "—".to_string(),
                                        };
                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <a href="#" class="table__link" on:click=move |ev| {
                                                        ev.prevent_default();
                                                        open_detail(detail_id.clone(), detail_name_for_link.clone());
                                                    }>{primary_date_display.clone()}</a>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell><TableCellLayout truncate=true>{item.service_name.clone()}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>{item.name.clone()}</TableCellLayout></TableCell>
                                            <TableCell>
                                                <TableCellLayout>{if item.is_weekly_report { "Да" } else { "Нет" }}</TableCellLayout>
                                            </TableCell>
                                            <TableCell><TableCellLayout truncate=true>{item.category.clone()}</TableCellLayout></TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {period_display.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <div style="display:flex;gap:6px;flex-wrap:wrap;">
                                                        <For
                                                            each=move || item.extensions.clone()
                                                            key=|ext| ext.clone()
                                                            children=move |ext| {
                                                                let download_id_value = download_id.clone();
                                                                let extension = ext.clone();
                                                                let service_name = detail_name.clone();
                                                                view! {
                                                                    <Button
                                                                        size=ButtonSize::Small
                                                                        appearance=ButtonAppearance::Subtle
                                                                        on_click=move |_| {
                                                                            set_error.set(None);
                                                                            start_download.run((
                                                                                download_id_value.clone(),
                                                                                service_name.clone(),
                                                                                extension.clone(),
                                                                            ));
                                                                        }
                                                                    >
                                                                        {ext}
                                                                    </Button>
                                                                }
                                                            }
                                                        />
                                                    </div>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell><TableCellLayout>{if item.viewed { "Да" } else { "Нет" }}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout truncate=true>{item.connection_name.unwrap_or(item.connection_id)}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout truncate=true>{item.organization_name.unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
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

async fn fetch_connections() -> Result<Vec<ConnectionMP>, String> {
    let url = format!("{}/api/connection_mp", api_base());
    let response = Request::get(&url).send().await.map_err(|e| e.to_string())?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    response
        .json::<Vec<ConnectionMP>>()
        .await
        .map_err(|e| e.to_string())
}
