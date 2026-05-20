mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::ui::badge::Badge;
use crate::shared::export::{export_to_excel, ExcelExportable};
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, highlight_matches};
use crate::shared::page_frame::PageFrame;
use contracts::domain::a005_marketplace::aggregate::Marketplace;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a007_marketplace_product::aggregate::{
    MarketplaceProductListItemDto, WbMappingProblemDto, WbMappingProblemsResponse,
    WbStalePostingsRepostResponse, WbStalePostingsRequest, WbStalePostingsSummary,
};
use contracts::domain::common::AggregateId;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thaw::*;

#[derive(Debug, Clone, Default)]
struct StaleEntry {
    pub loading: bool,
    pub posting: bool,
    pub summary: Option<WbStalePostingsSummary>,
    pub error: Option<String>,
}

fn stale_key(connection_mp_ref: &str, nm_id: Option<i64>, supplier_article: Option<&str>) -> String {
    format!(
        "{}|{}|{}",
        connection_mp_ref,
        nm_id.unwrap_or(0),
        supplier_article.unwrap_or("")
    )
}

fn spawn_fetch_stale_summary(
    set_map: WriteSignal<HashMap<String, StaleEntry>>,
    connection_mp_ref: String,
    nm_id: i64,
    supplier_article: Option<String>,
    date_from: String,
    date_to: String,
) {
    let key = stale_key(&connection_mp_ref, Some(nm_id), supplier_article.as_deref());

    set_map.update(|m| {
        let entry = m.entry(key.clone()).or_default();
        entry.loading = true;
        entry.error = None;
    });

    let key_for_async = key.clone();
    spawn_local(async move {
        let mut url = format!(
            "{}/api/a007/marketplace-product/wb-stale-postings/summary?connection_mp_ref={}&nm_id={}&date_from={}&date_to={}",
            api_base(),
            urlencoding::encode(&connection_mp_ref),
            nm_id,
            urlencoding::encode(&date_from),
            urlencoding::encode(&date_to),
        );
        if let Some(sa) = supplier_article.as_deref().filter(|v| !v.is_empty()) {
            url.push_str(&format!("&supplier_article={}", urlencoding::encode(sa)));
        }

        let result: Result<WbStalePostingsSummary, String> = match Request::get(&url).send().await {
            Ok(resp) if resp.status() == 200 => resp
                .json::<WbStalePostingsSummary>()
                .await
                .map_err(|e| format!("parse: {}", e)),
            Ok(resp) => Err(format!("HTTP {}", resp.status())),
            Err(e) => Err(format!("net: {}", e)),
        };

        set_map.update(|m| {
            let entry = m.entry(key_for_async.clone()).or_default();
            entry.loading = false;
            match result {
                Ok(summary) => {
                    entry.summary = Some(summary);
                    entry.error = None;
                }
                Err(err) => entry.error = Some(err),
            }
        });
    });
}

fn spawn_repost_stale(
    set_map: WriteSignal<HashMap<String, StaleEntry>>,
    connection_mp_ref: String,
    nm_id: i64,
    supplier_article: Option<String>,
    date_from: String,
    date_to: String,
) {
    let key = stale_key(&connection_mp_ref, Some(nm_id), supplier_article.as_deref());

    set_map.update(|m| {
        let entry = m.entry(key.clone()).or_default();
        entry.posting = true;
        entry.error = None;
    });

    let key_for_async = key.clone();
    let conn_for_summary = connection_mp_ref.clone();
    let sa_for_summary = supplier_article.clone();
    let from_for_summary = date_from.clone();
    let to_for_summary = date_to.clone();

    spawn_local(async move {
        let url = format!(
            "{}/api/a007/marketplace-product/wb-stale-postings/repost",
            api_base()
        );
        let body = WbStalePostingsRequest {
            connection_mp_ref,
            nm_id,
            supplier_article,
            date_from,
            date_to,
        };
        let result: Result<WbStalePostingsRepostResponse, String> =
            match Request::post(&url).json(&body) {
                Ok(req) => match req.send().await {
                    Ok(resp) if resp.status() == 200 => resp
                        .json::<WbStalePostingsRepostResponse>()
                        .await
                        .map_err(|e| format!("parse: {}", e)),
                    Ok(resp) => Err(format!("HTTP {}", resp.status())),
                    Err(e) => Err(format!("net: {}", e)),
                },
                Err(e) => Err(format!("build: {}", e)),
            };

        set_map.update(|m| {
            let entry = m.entry(key_for_async.clone()).or_default();
            entry.posting = false;
            match result {
                Ok(resp) => {
                    if resp.failed > 0 {
                        entry.error = Some(format!(
                            "{} из {} ошиблись",
                            resp.failed, resp.total
                        ));
                    } else {
                        entry.error = None;
                    }
                }
                Err(err) => entry.error = Some(err),
            }
        });

        // Перезапрашиваем summary, чтобы счётчик/период обновились.
        spawn_fetch_stale_summary(
            set_map,
            conn_for_summary,
            nm_id,
            sa_for_summary,
            from_for_summary,
            to_for_summary,
        );
    });
}

impl ExcelExportable for MarketplaceProductListItemDto {
    fn headers() -> Vec<&'static str> {
        vec!["Код", "Описание", "Артикул", "SKU", "Штрихкод", "1С"]
    }

    fn to_csv_row(&self) -> Vec<String> {
        vec![
            self.code.clone(),
            self.description.clone(),
            self.article.clone(),
            self.marketplace_sku.clone(),
            self.barcode.clone().unwrap_or_else(|| "-".to_string()),
            if self.nomenclature_ref.is_some() {
                "Да"
            } else {
                "Нет"
            }
            .to_string(),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<MarketplaceProductListItemDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

fn wb_problem_label(kind: &str) -> &'static str {
    match kind {
        "missing_a007" => "Нет товара МП",
        "missing_nomenclature" => "Нет номенклатуры",
        "stale_document_link" => "Документы не пересчитаны",
        "article_ambiguous" => "Неоднозначный артикул",
        "missing_nm_id" => "Нет nm_id",
        _ => "Проблема",
    }
}

#[component]
#[allow(non_snake_case)]
pub fn MarketplaceProductList() -> impl IntoView {
    let state = create_state();
    let global_ctx = expect_context::<AppGlobalContext>();

    let (items, set_items) = signal::<Vec<MarketplaceProductListItemDto>>(Vec::new());
    let (wb_problem_items, set_wb_problem_items) = signal::<Vec<WbMappingProblemDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (marketplaces, set_marketplaces) = signal::<Vec<Marketplace>>(Vec::new());
    let (connections, set_connections) = signal::<Vec<ConnectionMP>>(Vec::new());
    let (wb_problems_mode, set_wb_problems_mode) = signal(false);
    // Сводки по «устаревшим» проводкам для строк stale_document_link.
    // Ключ строится через stale_key(connection_mp_ref, nm_id, supplier_article).
    let (stale_map, set_stale_map) = signal::<HashMap<String, StaleEntry>>(HashMap::new());

    // Filter panel expansion state
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    // RwSignal для Thaw Input/Select компонентов
    let search_text = RwSignal::new(state.get_untracked().search.clone());
    let filter_marketplace = RwSignal::new(
        state
            .get_untracked()
            .marketplace_ref
            .clone()
            .unwrap_or_default(),
    );
    let filter_connection = RwSignal::new(
        state
            .get_untracked()
            .connection_mp_ref
            .clone()
            .unwrap_or_default(),
    );
    let filter_problems_only = RwSignal::new(state.get_untracked().problems_only);
    let today = chrono::Utc::now().date_naive();
    let wb_date_to = RwSignal::new(today.to_string());
    let wb_date_from = RwSignal::new((today - chrono::Duration::days(30)).to_string());

    // Обновление state при изменении RwSignal
    Effect::new(move || {
        let text = search_text.get();
        untrack(move || {
            state.update(|s| {
                s.search = text;
                s.page = 0;
            });
        });
    });

    Effect::new(move || {
        let mp = filter_marketplace.get();
        untrack(move || {
            state.update(|s| {
                s.marketplace_ref = if mp.is_empty() { None } else { Some(mp) };
                s.page = 0;
            });
        });
    });

    Effect::new(move || {
        let connection = filter_connection.get();
        untrack(move || {
            state.update(|s| {
                s.connection_mp_ref = if connection.is_empty() {
                    None
                } else {
                    Some(connection)
                };
                s.page = 0;
            });
        });
    });

    Effect::new(move || {
        let problems_only = filter_problems_only.get();
        untrack(move || {
            state.update(|s| {
                s.problems_only = problems_only;
                s.page = 0;
            });
        });
    });

    let load_data = move || {
        let current_state = state.get_untracked();
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            let offset = current_state.page * current_state.page_size;
            let mut url = format!(
                "{}/api/a007/marketplace-product?limit={}&offset={}&sort_by={}&sort_desc={}",
                api_base(),
                current_state.page_size,
                offset,
                current_state.sort_field,
                !current_state.sort_ascending
            );

            if let Some(ref mp) = current_state.marketplace_ref {
                url.push_str(&format!("&marketplace_ref={}", urlencoding::encode(mp)));
            }
            if let Some(ref connection) = current_state.connection_mp_ref {
                url.push_str(&format!(
                    "&connection_mp_ref={}",
                    urlencoding::encode(connection)
                ));
            }
            if current_state.problems_only {
                url.push_str("&problems_only=true");
            }
            if !current_state.search.is_empty() {
                url.push_str(&format!(
                    "&search={}",
                    urlencoding::encode(&current_state.search)
                ));
            }

            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        match response.json::<PaginatedResponse>().await {
                            Ok(data) => {
                                set_items.set(data.items);
                                state.update(|s| {
                                    s.total_count = data.total;
                                    s.total_pages = data.total_pages;
                                    s.is_loaded = true;
                                });
                                set_loading.set(false);
                            }
                            Err(e) => {
                                log!("Failed to parse response: {:?}", e);
                                set_error.set(Some(format!("Failed to parse: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Server error: {}", response.status())));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to fetch: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    let load_wb_problems = move || {
        let connection = filter_connection.get_untracked();
        let date_from = wb_date_from.get_untracked();
        let date_to = wb_date_to.get_untracked();
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            let mut url = format!(
                "{}/api/a007/marketplace-product/wb-mapping-problems?date_from={}&date_to={}&limit=500",
                api_base(),
                urlencoding::encode(&date_from),
                urlencoding::encode(&date_to)
            );
            if !connection.trim().is_empty() {
                url.push_str(&format!(
                    "&connection_mp_ref={}",
                    urlencoding::encode(&connection)
                ));
            }

            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        match response.json::<WbMappingProblemsResponse>().await {
                            Ok(data) => {
                                // Ленивая подгрузка сводок по stale-проводкам только для
                                // строк, где это нужно. Параметры периода берём из
                                // ответа сервера — те же, по которым посчитаны проблемы.
                                let summary_date_from = data.date_from.clone();
                                let summary_date_to = data.date_to.clone();
                                for item in &data.items {
                                    if item.problem_kind == "stale_document_link" {
                                        if let Some(nm_id) = item.nm_id {
                                            spawn_fetch_stale_summary(
                                                set_stale_map,
                                                item.connection_mp_ref.clone(),
                                                nm_id,
                                                item.supplier_article.clone(),
                                                summary_date_from.clone(),
                                                summary_date_to.clone(),
                                            );
                                        }
                                    }
                                }

                                // Сбрасываем устаревшие сводки от предыдущих загрузок.
                                set_stale_map.update(|m| {
                                    let valid_keys: std::collections::HashSet<String> = data
                                        .items
                                        .iter()
                                        .filter(|i| i.problem_kind == "stale_document_link")
                                        .filter_map(|i| {
                                            i.nm_id.map(|nm_id| {
                                                stale_key(
                                                    &i.connection_mp_ref,
                                                    Some(nm_id),
                                                    i.supplier_article.as_deref(),
                                                )
                                            })
                                        })
                                        .collect();
                                    m.retain(|k, _| valid_keys.contains(k));
                                });

                                set_wb_problem_items.set(data.items);
                                state.update(|s| {
                                    s.total_count = data.total;
                                    s.total_pages = 1;
                                    s.page = 0;
                                    s.is_loaded = true;
                                });
                                set_loading.set(false);
                            }
                            Err(e) => {
                                log!("Failed to parse WB mapping problems: {:?}", e);
                                set_error.set(Some(format!("Failed to parse: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Server error: {}", response.status())));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to fetch WB mapping problems: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    let fetch_marketplaces = move || {
        spawn_local(async move {
            match Request::get(&format!("{}/api/marketplace", api_base()))
                .send()
                .await
            {
                Ok(resp) => {
                    if let Ok(v) = resp.json::<Vec<Marketplace>>().await {
                        set_marketplaces.set(v);
                    }
                }
                Err(e) => log!("Error fetching marketplaces: {:?}", e),
            }
        });
    };

    let fetch_connections = move || {
        spawn_local(async move {
            match Request::get(&format!("{}/api/connection_mp", api_base()))
                .send()
                .await
            {
                Ok(resp) => {
                    if let Ok(v) = resp.json::<Vec<ConnectionMP>>().await {
                        set_connections.set(v);
                    }
                }
                Err(e) => log!("Error fetching connections: {:?}", e),
            }
        });
    };

    // Initial load
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            fetch_marketplaces();
            fetch_connections();
            load_data();
        }
    });

    // Auto-reload при изменении фильтров
    let search_first_run = StoredValue::new(true);
    Effect::new(move || {
        let _ = search_text.get();
        if !search_first_run.get_value() {
            load_data();
        } else {
            search_first_run.set_value(false);
        }
    });

    let mp_first_run = StoredValue::new(true);
    Effect::new(move || {
        let _ = filter_marketplace.get();
        if !mp_first_run.get_value() {
            load_data();
        } else {
            mp_first_run.set_value(false);
        }
    });

    let connection_first_run = StoredValue::new(true);
    Effect::new(move || {
        let _ = filter_connection.get();
        if !connection_first_run.get_value() {
            load_data();
        } else {
            connection_first_run.set_value(false);
        }
    });

    let problems_first_run = StoredValue::new(true);
    Effect::new(move || {
        let _ = filter_problems_only.get();
        if !problems_first_run.get_value() {
            load_data();
        } else {
            problems_first_run.set_value(false);
        }
    });

    // Handlers
    let toggle_sort = move |field: &'static str| {
        move |_| {
            state.update(|s| {
                if s.sort_field == field {
                    s.sort_ascending = !s.sort_ascending;
                } else {
                    s.sort_field = field.to_string();
                    s.sort_ascending = true;
                }
                s.page = 0;
            });
            load_data();
        }
    };

    let go_to_page = move |page: usize| {
        state.update(|s| s.page = page);
        load_data();
    };

    let change_page_size = move |size: usize| {
        state.update(|s| {
            s.page_size = size;
            s.page = 0;
        });
        load_data();
    };

    let toggle_select = move |id: String| {
        state.update(|s| {
            if s.selected_ids.contains(&id) {
                s.selected_ids.retain(|x| x != &id);
            } else {
                s.selected_ids.push(id);
            }
        });
    };

    let toggle_select_all = move |_| {
        let current_items = items.get();
        let current_selected = state.get().selected_ids.clone();
        let all_on_page: Vec<String> = current_items.iter().map(|i| i.id.clone()).collect();

        if all_on_page.iter().all(|id| current_selected.contains(id)) {
            state.update(|s| {
                s.selected_ids.retain(|id| !all_on_page.contains(id));
            });
        } else {
            state.update(|s| {
                for id in all_on_page {
                    if !s.selected_ids.contains(&id) {
                        s.selected_ids.push(id);
                    }
                }
            });
        }
    };

    let open_detail = move |id: String, article: String, description: String| {
        use crate::layout::tabs::{detail_tab_label, pick_identifier};
        let identifier = pick_identifier(None, Some(&article), Some(&description), &id);
        global_ctx.open_tab(
            &format!("a007_marketplace_product_details_{}", id),
            &detail_tab_label("Товар МП", identifier),
        );
    };

    let handle_create_new = move |_| {
        global_ctx.open_tab("a007_marketplace_product_new", "New MP Product");
    };

    let open_matching_usecase = move |_| {
        global_ctx.open_tab(
            "u505_match_nomenclature",
            crate::layout::tabs::tab_label_for_key("u505_match_nomenclature"),
        );
    };

    let delete_selected = move |_| {
        let ids = state.get().selected_ids.clone();
        if ids.is_empty() {
            return;
        }

        let confirmed = {
            if let Some(win) = web_sys::window() {
                win.confirm_with_message(&format!("Удалить выбранные элементы ({})?", ids.len()))
                    .unwrap_or(false)
            } else {
                false
            }
        };

        if confirmed {
            spawn_local(async move {
                for id in ids {
                    let _ =
                        Request::delete(&format!("{}/api/marketplace_product/{}", api_base(), id))
                            .send()
                            .await;
                }
                state.update(|s| s.selected_ids.clear());
                load_data();
            });
        }
    };

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0;
        if !s.search.is_empty() {
            count += 1;
        }
        if s.marketplace_ref.is_some() {
            count += 1;
        }
        if s.connection_mp_ref.is_some() {
            count += 1;
        }
        if s.problems_only {
            count += 1;
        }
        count
    });

    let clear_all_filters = move |_| {
        state.update(|s| {
            s.search = String::new();
            s.marketplace_ref = None;
            s.connection_mp_ref = None;
            s.problems_only = false;
            s.page = 0;
        });
        search_text.set(String::new());
        filter_marketplace.set(String::new());
        filter_connection.set(String::new());
        filter_problems_only.set(false);
        load_data();
    };

    let handle_export = move |_| {
        let current_items = items.get();
        if current_items.is_empty() {
            return;
        }
        let _ = export_to_excel(&current_items, "marketplace_products.csv");
    };

    view! {
        <PageFrame page_id="a007_marketplace_product--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Товары маркетплейсов"</h1>
                    <Badge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </Badge>
                </div>
                <div class="page__header-right">
                    <thaw::Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| handle_create_new(())
                    >
                        {icon("plus")}
                        " Создать"
                    </thaw::Button>
                    <thaw::Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| open_matching_usecase(())
                    >
                        {icon("link")}
                        " 1С matching"
                    </thaw::Button>
                    <thaw::Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| {
                            let next = !wb_problems_mode.get_untracked();
                            set_wb_problems_mode.set(next);
                            if next {
                                load_wb_problems();
                            } else {
                                load_data();
                            }
                        }
                    >
                        {icon("warning")}
                        {move || if wb_problems_mode.get() { " Список товаров" } else { " Проблемы WB" }}
                    </thaw::Button>
                    <thaw::Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| handle_export(())
                    >
                        {icon("excel")}
                        "Excel (csv)"
                    </thaw::Button>
                    <thaw::Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| delete_selected(())
                        disabled=Signal::derive(move || state.get().selected_ids.is_empty())
                    >
                        {icon("delete")}
                        {move || format!(" Удалить ({})", state.get().selected_ids.len())}
                    </thaw::Button>
                    <thaw::Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| {
                            if wb_problems_mode.get_untracked() {
                                load_wb_problems();
                            } else {
                                load_data();
                            }
                        }
                    >
                        {icon("refresh")}
                        " Обновить"
                    </thaw::Button>
                </div>
            </div>

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
                        {icon("filter")}
                        <span class="filter-panel__title">"Фильтры"</span>
                        {move || {
                            let count = active_filters_count.get();
                            if count > 0 {
                                view! {
                                    <Badge variant="primary".to_string()>{count}</Badge>
                                }.into_any()
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
                        />
                    </div>
                    <div class="filter-panel-header__right">
                        <thaw::Button
                            appearance=ButtonAppearance::Subtle
                            on_click=move |_| {
                                if wb_problems_mode.get_untracked() {
                                    load_wb_problems();
                                } else {
                                    load_data();
                                }
                            }
                            disabled=loading.get()
                        >
                            {icon("refresh")}
                            {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                        </thaw::Button>
                    </div>
                </div>

                <div class=move || {
                    if is_filter_expanded.get() {
                        "filter-panel__collapsible filter-panel__collapsible--expanded"
                    } else {
                        "filter-panel__collapsible filter-panel__collapsible--collapsed"
                    }
                }>
                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End>
                            <div style="flex: 1; min-width: 200px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Поиск:"</Label>
                                    <Input
                                        value=search_text
                                        placeholder="Код, описание, SKU, артикул..."
                                    />
                                </Flex>
                            </div>
                            <div style="width: 200px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Маркетплейс:"</Label>
                                    <Select value=filter_marketplace>
                                        <option value="">"Все"</option>
                                        {move || marketplaces.get().into_iter().map(|mp| {
                                            let id = mp.base.id.as_string();
                                            view! { <option value=id>{mp.base.description}</option> }
                                        }).collect_view()}
                                    </Select>
                                </Flex>
                            </div>
                            <div style="width: 240px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Кабинет:"</Label>
                                    <Select value=filter_connection>
                                        <option value="">"Все"</option>
                                        {move || connections.get().into_iter().map(|conn| {
                                            let id = conn.base.id.as_string();
                                            view! { <option value=id>{conn.base.description}</option> }
                                        }).collect_view()}
                                    </Select>
                                </Flex>
                            </div>
                            <div style="width: 180px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Проблемы:"</Label>
                                    <Field>
                                        <Checkbox
                                            checked=filter_problems_only
                                            label="Только проблемные"
                                        />
                                    </Field>
                                </Flex>
                            </div>
                            {move || if wb_problems_mode.get() {
                                view! {
                                    <>
                                        <div style="width: 150px;">
                                            <Flex vertical=true gap=FlexGap::Small>
                                                <Label>"WB дата с:"</Label>
                                                <Input value=wb_date_from />
                                            </Flex>
                                        </div>
                                        <div style="width: 150px;">
                                            <Flex vertical=true gap=FlexGap::Small>
                                                <Label>"WB дата по:"</Label>
                                                <Input value=wb_date_to />
                                            </Flex>
                                        </div>
                                        <thaw::Button
                                            appearance=ButtonAppearance::Primary
                                            on_click=move |_| load_wb_problems()
                                        >
                                            {icon("search")}
                                            " Показать WB"
                                        </thaw::Button>
                                    </>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                            <thaw::Button
                                appearance=ButtonAppearance::Subtle
                                on_click=move |_| clear_all_filters(())
                            >
                                "Сбросить"
                            </thaw::Button>
                        </Flex>
                    </div>
                </div>
            </div>

            {move || error.get().map(|e| view! { <div class="warning-box" style="margin: 10px;">{e}</div> })}

            <div class="page__content">
                <div
                    class="a007-mp-list__table-shell"
                    style=move || if wb_problems_mode.get() {
                        "border: none; border-radius: 0; padding: 0;"
                    } else {
                        "display:none;"
                    }
                >
                    <table class="table__data table--striped">
                        <thead class="table__head a007-mp-list__sticky-head">
                            <tr>
                                <th class="table__header-cell a007-mp-list__sticky-cell">"Проблема"</th>
                                <th class="table__header-cell a007-mp-list__sticky-cell">"Кабинет"</th>
                                <th class="table__header-cell a007-mp-list__sticky-cell">"nm_id"</th>
                                <th class="table__header-cell a007-mp-list__sticky-cell">"Артикул WB"</th>
                                <th class="table__header-cell a007-mp-list__sticky-cell">"SKU a007"</th>
                                <th class="table__header-cell a007-mp-list__sticky-cell">"Номенклатура"</th>
                                <th class="table__header-cell a007-mp-list__sticky-cell">"Данные"</th>
                                <th class="table__header-cell a007-mp-list__sticky-cell">"Перепровести"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || {
                                let summary_date_from = wb_date_from.get();
                                let summary_date_to = wb_date_to.get();
                                wb_problem_items.get().into_iter().map(|item| {
                                    let product_id = item.marketplace_product_id.clone();
                                    let article = item
                                        .marketplace_article
                                        .clone()
                                        .or_else(|| item.supplier_article.clone())
                                        .unwrap_or_default();
                                    let description = item
                                        .nomenclature_name
                                        .clone()
                                        .unwrap_or_else(|| item.supplier_article.clone().unwrap_or_default());

                                    let problem_kind = item.problem_kind.clone();
                                    let connection_mp_ref = item.connection_mp_ref.clone();
                                    let supplier_article = item.supplier_article.clone();
                                    let nm_id = item.nm_id;
                                    let entry_key = nm_id
                                        .map(|v| stale_key(&connection_mp_ref, Some(v), supplier_article.as_deref()))
                                        .unwrap_or_default();

                                    let is_stale = problem_kind == "stale_document_link";

                                    let date_from_for_btn = summary_date_from.clone();
                                    let date_to_for_btn = summary_date_to.clone();
                                    let conn_for_btn = connection_mp_ref.clone();
                                    let sa_for_btn = supplier_article.clone();

                                    view! {
                                        <tr
                                            class="table__row"
                                            on:click=move |_| {
                                                if let Some(id) = product_id.clone() {
                                                    open_detail(id, article.clone(), description.clone());
                                                }
                                            }
                                        >
                                            <td class="table__cell">
                                                <Badge variant="danger".to_string()>{wb_problem_label(&problem_kind)}</Badge>
                                            </td>
                                            <td class="table__cell">{item.connection_name.unwrap_or(item.connection_mp_ref)}</td>
                                            <td class="table__cell">{item.nm_id.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string())}</td>
                                            <td class="table__cell">{item.supplier_article.unwrap_or_else(|| "-".to_string())}</td>
                                            <td class="table__cell">{item.marketplace_sku.unwrap_or_else(|| "-".to_string())}</td>
                                            <td class="table__cell">
                                                {item.nomenclature_name
                                                    .or(item.marketplace_nomenclature_ref)
                                                    .unwrap_or_else(|| "-".to_string())}
                                            </td>
                                            <td class="table__cell">
                                                {if is_stale {
                                                    let key_summary = entry_key.clone();
                                                    view! {
                                                        {move || {
                                                            let map = stale_map.get();
                                                            let entry = map.get(&key_summary).cloned().unwrap_or_default();
                                                            if let Some(err) = entry.error.clone() {
                                                                view! { <span style="color:var(--color-error);">{err}</span> }.into_any()
                                                            } else if let Some(summary) = entry.summary.clone() {
                                                                let period = match (summary.period_from.as_deref(), summary.period_to.as_deref()) {
                                                                    (Some(f), Some(t)) if f == t => f.to_string(),
                                                                    (Some(f), Some(t)) => format!("{} — {}", f, t),
                                                                    _ => "—".to_string(),
                                                                };
                                                                view! { <span>{format!("{} док., {}", summary.doc_count, period)}</span> }.into_any()
                                                            } else if entry.loading {
                                                                view! { <span style="color:var(--color-text-secondary);">"…"</span> }.into_any()
                                                            } else {
                                                                view! { <span style="color:var(--color-text-secondary);">"—"</span> }.into_any()
                                                            }
                                                        }}
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <span>{format!(
                                                            "p903: {}, orders: {}, sales: {}, empty: {}, mismatch: {}, article matches: {}",
                                                            item.p903_rows,
                                                            item.order_rows,
                                                            item.sale_rows,
                                                            item.missing_document_links,
                                                            item.mismatched_document_links,
                                                            item.article_match_count
                                                        )}</span>
                                                    }.into_any()
                                                }}
                                            </td>
                                            <td class="table__cell" style="text-align:center;">
                                                {if is_stale {
                                                    let key_btn = entry_key.clone();
                                                    let conn = conn_for_btn.clone();
                                                    let sa = sa_for_btn.clone();
                                                    let df = date_from_for_btn.clone();
                                                    let dt = date_to_for_btn.clone();
                                                    let nm = nm_id.unwrap_or(0);
                                                    view! {
                                                        <thaw::Button
                                                            appearance=ButtonAppearance::Subtle
                                                            size=ButtonSize::Small
                                                            disabled=Signal::derive(move || {
                                                                let map = stale_map.get();
                                                                let entry = map.get(&key_btn).cloned().unwrap_or_default();
                                                                entry.posting
                                                                    || entry.loading
                                                                    || entry.summary.as_ref().map(|s| s.doc_count == 0).unwrap_or(false)
                                                                    || nm_id.is_none()
                                                            })
                                                            on_click=move |ev: leptos::ev::MouseEvent| {
                                                                ev.stop_propagation();
                                                                spawn_repost_stale(
                                                                    set_stale_map,
                                                                    conn.clone(),
                                                                    nm,
                                                                    sa.clone(),
                                                                    df.clone(),
                                                                    dt.clone(),
                                                                );
                                                            }
                                                        >
                                                            "Перепровести"
                                                        </thaw::Button>
                                                    }.into_any()
                                                } else {
                                                    view! { <></> }.into_any()
                                                }}
                                            </td>
                                        </tr>
                                    }
                                }).collect_view()
                            }}
                        </tbody>
                    </table>
                </div>
                <div
                    class="list-container a007-mp-list__table-shell"
                    style=move || if wb_problems_mode.get() { "display:none;" } else { "" }
                >
                    <table class="table__data table--striped">
                        <thead class="table__head a007-mp-list__sticky-head">
                            <tr>
                                <th
                                    class="table__header-cell table__header-cell--checkbox a007-mp-list__sticky-cell a007-mp-list__sticky-cell--checkbox"
                                >
                                    <input
                                        type="checkbox"
                                        on:change=toggle_select_all
                                        prop:checked=move || {
                                            let current_items = items.get();
                                            let selected = state.get().selected_ids;
                                            !current_items.is_empty() && current_items.iter().all(|i| selected.contains(&i.id))
                                        }
                                    />
                                </th>
                                <th
                                    class="table__header-cell table__header-cell--sortable a007-mp-list__sticky-cell"
                                    on:click=toggle_sort("code")
                                >
                                    "Код"
                                    <span class={move || get_sort_class("code", &state.get().sort_field)}>{move || get_sort_indicator("code", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </th>
                                <th
                                    class="table__header-cell table__header-cell--sortable a007-mp-list__sticky-cell"
                                    on:click=toggle_sort("description")
                                >
                                    "Описание"
                                    <span class={move || get_sort_class("description", &state.get().sort_field)}>{move || get_sort_indicator("description", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </th>
                                <th
                                    class="table__header-cell table__header-cell--sortable a007-mp-list__sticky-cell"
                                    on:click=toggle_sort("article")
                                >
                                    "Артикул"
                                    <span class={move || get_sort_class("article", &state.get().sort_field)}>{move || get_sort_indicator("article", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </th>
                                <th
                                    class="table__header-cell table__header-cell--sortable a007-mp-list__sticky-cell"
                                    on:click=toggle_sort("marketplace_sku")
                                >
                                    "SKU"
                                    <span class={move || get_sort_class("marketplace_sku", &state.get().sort_field)}>{move || get_sort_indicator("marketplace_sku", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </th>
                                <th
                                    class="table__header-cell table__header-cell--sortable a007-mp-list__sticky-cell"
                                    on:click=toggle_sort("barcode")
                                >
                                    "Штрихкод"
                                    <span class={move || get_sort_class("barcode", &state.get().sort_field)}>{move || get_sort_indicator("barcode", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </th>
                                <th class="table__header-cell table__header-cell--center a007-mp-list__sticky-cell">"1С"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || {
                                items.get().into_iter().map(|item| {
                                    let id = item.id.clone();
                                    let id_for_click = id.clone();
                                    let id_for_checkbox = id.clone();
                                    let article_for_tab = item.article.clone();
                                    let description_for_tab = item.description.clone();
                                    let is_selected = state.get().selected_ids.contains(&id);
                                    let search = state.get().search;

                                    view! {
                                        <tr
                                            class="table__row"
                                            class:table__row--selected=is_selected
                                        >
                                            <td class="table__cell table__cell--checkbox" on:click=|e| e.stop_propagation()>
                                                <input
                                                    type="checkbox"
                                                    class="table__checkbox"
                                                    prop:checked=move || state.get().selected_ids.contains(&id_for_checkbox)
                                                    on:change={
                                                        let id = id.clone();
                                                        move |_| toggle_select(id.clone())
                                                    }
                                                />
                                            </td>
                                            <td class="table__cell a007-mp-list__code-cell" on:click={
                                                let id = id_for_click.clone();
                                                move |_| open_detail(id.clone(), article_for_tab.clone(), description_for_tab.clone())
                                            }>
                                                {highlight_matches(&item.code, &search)}
                                            </td>
                                            <td class="table__cell">{highlight_matches(&item.description, &search)}</td>
                                            <td class="table__cell">{highlight_matches(&item.article, &search)}</td>
                                            <td class="table__cell">{highlight_matches(&item.marketplace_sku, &search)}</td>
                                            <td class="table__cell">{
                                                let search = search.clone();
                                                item.barcode.map(|b| highlight_matches(&b, &search)).unwrap_or_else(|| view! { <></> }.into_any())
                                            }</td>
                                            <td class="table__cell table__cell--center">
                                                {if item.nomenclature_ref.is_some() {
                                                    view! { <span style="color: var(--color-success); font-weight: bold;">"✓"</span> }.into_any()
                                                } else {
                                                    view! { <span style="color: var(--color-text-tertiary);">"—"</span> }.into_any()
                                                }}
                                            </td>
                                        </tr>
                                    }
                                }).collect_view()
                            }}
                        </tbody>
                    </table>
                </div>
            </div>
        </PageFrame>
    }
}
