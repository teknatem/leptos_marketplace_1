pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::init_column_resize;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use thaw::*;

use crate::shared::api_utils::api_base;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KitVariantDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub owner_ref: Option<String>,
    pub owner_description: Option<String>,
    pub owner_article: Option<String>,
    pub goods_json: Option<String>,
    pub connection_id: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<serde_json::Value>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

const TABLE_ID: &str = "a022-kit-variant-table";
const COLUMN_WIDTHS_KEY: &str = "a022_kit_variant_column_widths";

#[component]
pub fn KitVariantList() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    let open_detail = move |id: String, description: String| {
        tabs_store.open_tab(
            &format!("a022_kit_variant_detail_{}", id),
            &format!("Комплект {}", description),
        );
    };

    let load_items = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let search_query_val = state.with_untracked(|s| s.search_query.clone());
            let page = state.with_untracked(|s| s.page);
            let page_size = state.with_untracked(|s| s.page_size);
            let sort_field = state.with_untracked(|s| s.sort_field.clone());
            let sort_ascending = state.with_untracked(|s| s.sort_ascending);
            let offset = page * page_size;
            let cache_buster = js_sys::Date::now() as i64;

            let mut url = format!(
                "{}/api/a022/kit-variant/list?limit={}&offset={}&sort_by={}&sort_desc={}&_ts={}",
                api_base(),
                page_size,
                offset,
                sort_field,
                !sort_ascending,
                cache_buster
            );

            if !search_query_val.is_empty() {
                url.push_str(&format!(
                    "&search_query={}",
                    urlencoding::encode(&search_query_val)
                ));
            }

            match Request::get(&url)
                .header("Cache-Control", "no-cache, no-store, must-revalidate")
                .header("Pragma", "no-cache")
                .send()
                .await
            {
                Ok(response) => {
                    if response.ok() {
                        match response.json::<PaginatedResponse>().await {
                            Ok(paginated) => {
                                let parsed: Vec<KitVariantDto> = paginated
                                    .items
                                    .into_iter()
                                    .filter_map(|v| {
                                        Some(KitVariantDto {
                                            id: v.get("id")?.as_str()?.to_string(),
                                            code: v
                                                .get("code")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            description: v
                                                .get("description")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            owner_ref: v
                                                .get("owner_ref")
                                                .and_then(|x| x.as_str())
                                                .map(String::from),
                                            owner_description: v
                                                .get("owner_description")
                                                .and_then(|x| x.as_str())
                                                .map(String::from),
                                            owner_article: v
                                                .get("owner_article")
                                                .and_then(|x| x.as_str())
                                                .map(String::from),
                                            goods_json: v
                                                .get("goods_json")
                                                .and_then(|x| x.as_str())
                                                .map(String::from),
                                            connection_id: v
                                                .get("connection_id")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            fetched_at: v
                                                .get("fetched_at")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                        })
                                    })
                                    .collect();

                                state.update(|s| {
                                    s.items = parsed;
                                    s.total_count = paginated.total;
                                    s.total_pages = paginated.total_pages;
                                    s.page = paginated.page;
                                    s.page_size = paginated.page_size;
                                    s.is_loaded = true;
                                });
                                set_loading.set(false);
                            }
                            Err(e) => {
                                set_error.set(Some(format!("Ошибка парсинга: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Ошибка сервера: {}", response.status())));
                        set_loading.set(false);
                    }
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
            log!("Loading kit variants...");
            load_items();
        }
    });

    let search_query = RwSignal::new(state.get_untracked().search_query.clone());
    Effect::new(move || {
        let v = search_query.get();
        untrack(move || {
            state.update(|s| s.search_query = v);
        });
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

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        if !s.search_query.is_empty() { 1 } else { 0 }
    });

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

    let selected = RwSignal::new(state.with_untracked(|s| s.selected_ids.clone()));

    let toggle_selection = move |id: String, checked: bool| {
        selected.update(|s| {
            if checked {
                s.insert(id.clone());
            } else {
                s.remove(&id);
            }
        });
        state.update(|s| {
            if checked {
                s.selected_ids.insert(id);
            } else {
                s.selected_ids.remove(&id);
            }
        });
    };

    let toggle_all = move |check_all: bool| {
        if check_all {
            let items = state.get().items;
            selected.update(|s| {
                s.clear();
                for item in items.iter() {
                    s.insert(item.id.clone());
                }
            });
            state.update(|s| {
                s.selected_ids.clear();
                for item in s.items.iter() {
                    s.selected_ids.insert(item.id.clone());
                }
            });
        } else {
            selected.update(|s| s.clear());
            state.update(|s| s.selected_ids.clear());
        }
    };

    let items_signal = Signal::derive(move || state.get().items);
    let selected_signal = Signal::derive(move || selected.get());

    view! {
        <PageFrame page_id="a022_kit_variant--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Варианты комплектации"</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </UiBadge>
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
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                class=move || {
                                    if is_filter_expanded.get() {
                                        "filter-panel__chevron filter-panel__chevron--expanded"
                                    } else {
                                        "filter-panel__chevron"
                                    }
                                }
                            >
                                <polyline points="6 9 12 15 18 9"></polyline>
                            </svg>
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
                                <div style="flex: 1; max-width: 360px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Поиск:"</Label>
                                        <Input
                                            value=search_query
                                            placeholder="Наименование, артикул..."
                                        />
                                    </Flex>
                                </div>
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click=move |_| {
                                        state.update(|s| s.page = 0);
                                        load_items();
                                    }
                                    disabled=Signal::derive(move || loading.get())
                                >
                                    "Найти"
                                </Button>
                            </Flex>
                        </div>
                    </Show>
                </div>

                {move || {
                    error.get().map(|err| view! {
                        <div class="alert alert--error">{err}</div>
                    })
                }}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 800px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=items_signal
                                    selected=selected_signal
                                    get_id=Callback::new(|row: KitVariantDto| row.id.clone())
                                    on_change=Callback::new(toggle_all)
                                />
                                <TableHeaderCell resizable=false min_width=80.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("code")>
                                        "Код"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "code"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "code", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=250.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("description")>
                                        "Наименование варианта"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "description"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "description", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=200.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("owner_description")>
                                        "Номенклатура (владелец)"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "owner_description"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "owner_description", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=130.0 class="resizable">
                                    "Артикул"
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=80.0 class="resizable">
                                    "Состав"
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || state.get().items
                                key=|item| item.id.clone()
                                children=move |item| {
                                    let item_id = item.id.clone();
                                    let item_id_for_link = item_id.clone();
                                    let desc_for_link = item.description.clone();
                                    let desc_text = item.description.clone();

                                    let goods_count = item.goods_json
                                        .as_deref()
                                        .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
                                        .map(|v| v.len())
                                        .unwrap_or(0);

                                    view! {
                                        <TableRow>
                                            <TableCellCheckbox
                                                item_id=item_id.clone()
                                                selected=selected_signal
                                                on_change=Callback::new(move |(id, checked)| toggle_selection(id, checked))
                                            />
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <span style="font-family: monospace; font-size: var(--font-size-xs);">
                                                        {item.code.clone()}
                                                    </span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <a
                                                        href="#"
                                                        class="table__link"
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            open_detail(item_id_for_link.clone(), desc_for_link.clone());
                                                        }
                                                    >
                                                        {desc_text}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.owner_description.clone().unwrap_or_else(|| "—".to_string())}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <span style="font-family: monospace; font-size: var(--font-size-xs);">
                                                        {item.owner_article.clone().unwrap_or_else(|| "—".to_string())}
                                                    </span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {if goods_count > 0 {
                                                        view! {
                                                            <span class="badge badge--primary">
                                                                {format!("{} поз.", goods_count)}
                                                            </span>
                                                        }.into_any()
                                                    } else {
                                                        view! { <span class="badge badge--secondary">"—"</span> }.into_any()
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
