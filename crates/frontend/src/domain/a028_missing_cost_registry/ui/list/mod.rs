pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::TableCrosshairHighlight;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use thaw::*;

const TABLE_ID: &str = "a028-missing-cost-registry-table";

fn format_date(iso_date: &str) -> String {
    let date_part = iso_date.split('T').next().unwrap_or(iso_date);
    if let Some((year, rest)) = date_part.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            return format!("{}.{}.{}", day, month, year);
        }
    }
    iso_date.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MissingCostRegistryListItemDto {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub updated_at: String,
    pub is_posted: bool,
    pub lines_total: usize,
    pub lines_filled: usize,
    pub lines_missing: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<MissingCostRegistryListItemDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[component]
pub fn MissingCostRegistryList() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    let open_detail = move |id: String, document_no: String| {
        tabs_store.open_tab(
            &format!("a028_missing_cost_registry_details_{}", id),
            &format!("Реестр цен {}", document_no),
        );
    };

    let load_items = {
        let state = state;
        Callback::new(move |_| {
            set_loading.set(true);
            set_error.set(None);

            let current = state.get_untracked();
            let offset = current.page * current.page_size;
            let mut url = format!(
                "{}/api/a028/missing-cost-registry/list?date_from={}&date_to={}&limit={}&offset={}&sort_by={}&sort_desc={}",
                api_base(),
                current.date_from,
                current.date_to,
                current.page_size,
                offset,
                current.sort_field,
                !current.sort_ascending
            );

            if !current.search_query.is_empty() {
                url.push_str(&format!(
                    "&search_query={}",
                    urlencoding::encode(&current.search_query)
                ));
            }

            spawn_local(async move {
                match Request::get(&url).send().await {
                    Ok(response) if response.ok() => match response
                        .json::<PaginatedResponse>()
                        .await
                    {
                        Ok(payload) => {
                            state.update(|s| {
                                s.items = payload.items;
                                s.total_count = payload.total;
                                s.total_pages = payload.total_pages;
                                s.page = payload.page;
                                s.page_size = payload.page_size;
                                s.is_loaded = true;
                            });
                        }
                        Err(error) => set_error.set(Some(format!("Ошибка парсинга: {}", error))),
                    },
                    Ok(response) => {
                        set_error.set(Some(format!("Ошибка сервера: HTTP {}", response.status())))
                    }
                    Err(error) => set_error.set(Some(format!("Ошибка сети: {}", error))),
                }

                set_loading.set(false);
            });
        })
    };

    let load_items_effect = load_items.clone();
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_items_effect.run(());
        }
    });

    let search_query = RwSignal::new(state.get_untracked().search_query.clone());

    Effect::new(move || {
        let search = search_query.get();
        untrack(move || {
            state.update(|s| {
                s.search_query = search;
            });
        });
    });

    let toggle_sort = {
        let load_items = load_items.clone();
        move |field: &'static str| {
            state.update(|s| {
                if s.sort_field == field {
                    s.sort_ascending = !s.sort_ascending;
                } else {
                    s.sort_field = field.to_string();
                    s.sort_ascending = true;
                }
                s.page = 0;
            });
            load_items.run(());
        }
    };

    let go_to_page = {
        let load_items = load_items.clone();
        move |page: usize| {
            state.update(|s| s.page = page);
            load_items.run(());
        }
    };

    let change_page_size = {
        let load_items = load_items.clone();
        move |page_size: usize| {
            state.update(|s| {
                s.page_size = page_size;
                s.page = 0;
            });
            load_items.run(());
        }
    };

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0;
        if !s.date_from.is_empty() {
            count += 1;
        }
        if !s.date_to.is_empty() {
            count += 1;
        }
        if !s.search_query.is_empty() {
            count += 1;
        }
        count
    });

    view! {
        <PageFrame page_id="a028_missing_cost_registry--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Реестр отсутствующих цен"</h1>
                </div>
            </div>

            <div class="page__content">
                <div class="filter-panel" style="margin-bottom: 16px;">
                    <div class="filter-panel-header">
                        <div class="filter-panel-header__left">
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
                                on_click=move |_| load_items.run(())
                                disabled=Signal::derive(move || loading.get())
                            >
                                {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                            </Button>
                        </div>
                    </div>

                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End style="flex-wrap: wrap;">
                            <DateRangePicker
                                date_from=Signal::derive(move || state.with(|s| s.date_from.clone()))
                                date_to=Signal::derive(move || state.with(|s| s.date_to.clone()))
                                on_change=Callback::new({
                                    let load_items = load_items.clone();
                                    move |(from, to)| {
                                        state.update(|s| {
                                            s.date_from = from;
                                            s.date_to = to;
                                            s.page = 0;
                                        });
                                        load_items.run(());
                                    }
                                })
                                label="Период:".to_string()
                            />

                            <div style="flex: 1; min-width: 260px; max-width: 340px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Поиск:"</Label>
                                    <Input
                                        value=search_query
                                        placeholder="Номер документа или комментарий"
                                    />
                                </Flex>
                            </div>

                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| {
                                    state.update(|s| s.page = 0);
                                    load_items.run(());
                                }
                            >
                                "Применить"
                            </Button>
                        </Flex>
                    </div>
                </div>

                {move || {
                    error.get().map(|err| {
                        view! {
                            <div class="alert alert--error">
                                {err}
                            </div>
                        }
                    })
                }}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 980px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("document_date")>
                                        "Дата"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "document_date"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "document_date", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=180.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("document_no")>
                                        "Документ"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "document_no"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "document_no", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("is_posted")>
                                        "Статус"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "is_posted"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "is_posted", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("lines_total")>
                                        "Всего"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "lines_total"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "lines_total", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("lines_filled")>
                                        "Заполнено"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "lines_filled"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "lines_filled", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("lines_missing")>
                                        "Без цены"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "lines_missing"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "lines_missing", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("updated_at")>
                                        "Изменен"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "updated_at"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "updated_at", state.with(|s| s.sort_ascending))}
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
                                    let id = item.id.clone();
                                    let id_for_link = id.clone();
                                    let document_no_for_link = item.document_no.clone();
                                    let document_no_text = item.document_no.clone();
                                    let formatted_document_date = format_date(&item.document_date);
                                    let formatted_updated_at = format_date(&item.updated_at);

                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {formatted_document_date}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <a
                                                        href="#"
                                                        class="table__link"
                                                        style="color: #0f6cbd; text-decoration: underline;"
                                                        on:click=move |ev| {
                                                            ev.prevent_default();
                                                            open_detail(id_for_link.clone(), document_no_for_link.clone());
                                                        }
                                                    >
                                                        {document_no_text}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {if item.is_posted {
                                                        view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>"Проведен"</Badge> }.into_any()
                                                    } else {
                                                        view! { <Badge appearance=BadgeAppearance::Tint>"Черновик"</Badge> }.into_any()
                                                    }}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {item.lines_total}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {item.lines_filled}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {item.lines_missing}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {formatted_updated_at}
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
