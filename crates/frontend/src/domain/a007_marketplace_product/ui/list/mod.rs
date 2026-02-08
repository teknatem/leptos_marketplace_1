mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::ui::badge::Badge;
use crate::shared::export::{export_to_excel, ExcelExportable};
use crate::shared::icons::icon;
use crate::shared::list_utils::{
    get_sort_class, get_sort_indicator, highlight_matches,
};
use contracts::domain::a005_marketplace::aggregate::Marketplace;
use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProductListItemDto;
use contracts::domain::common::AggregateId;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use crate::shared::api_utils::api_base;
use thaw::*;

impl ExcelExportable for MarketplaceProductListItemDto {
    fn headers() -> Vec<&'static str> {
        vec![
            "Код",
            "Описание",
            "Артикул",
            "SKU",
            "Штрихкод",
            "1С",
        ]
    }

    fn to_csv_row(&self) -> Vec<String> {
        vec![
            self.code.clone(),
            self.description.clone(),
            self.article.clone(),
            self.marketplace_sku.clone(),
            self.barcode.clone().unwrap_or_else(|| "-".to_string()),
            if self.nomenclature_ref.is_some() { "Да" } else { "Нет" }.to_string(),
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

#[component]
#[allow(non_snake_case)]
pub fn MarketplaceProductList() -> impl IntoView {
    let state = create_state();
    let global_ctx = expect_context::<AppGlobalContext>();

    let (items, set_items) = signal::<Vec<MarketplaceProductListItemDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (marketplaces, set_marketplaces) = signal::<Vec<Marketplace>>(Vec::new());

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
                url.push_str(&format!("&marketplace_ref={}", mp));
            }
            if !current_state.search.is_empty() {
                url.push_str(&format!("&search={}", current_state.search));
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

    // Initial load
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            fetch_marketplaces();
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

    let open_detail = move |id: String| {
        global_ctx.open_tab(
            &format!("a007_marketplace_product_detail_{}", id),
            &format!("MP Prod {}", &id[..8.min(id.len())]),
        );
    };

    let handle_create_new = move |_| {
        global_ctx.open_tab("a007_marketplace_product_new", "New MP Product");
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
                    let _ = Request::delete(&format!("{}/api/marketplace_product/{}", api_base(), id))
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
        if !s.search.is_empty() { count += 1; }
        if s.marketplace_ref.is_some() { count += 1; }
        count
    });

    let clear_all_filters = move |_| {
        state.update(|s| {
            s.search = String::new();
            s.marketplace_ref = None;
            s.page = 0;
        });
        search_text.set(String::new());
        filter_marketplace.set(String::new());
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
        <div class="page">
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
                        on_click=move |_| handle_export(())
                    >
                        {icon("excel")}
                        " Excel"
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
                        on_click=move |_| load_data()
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
                            on_click=move |_| load_data()
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

            <div class="page-content">
                <div class="list-container">
                    <table class="table__data table--striped">
                        <thead class="table__head">
                            <tr>
                                <th class="table__header-cell table__header-cell--checkbox">
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
                                <th class="table__header-cell table__header-cell--sortable" on:click=toggle_sort("code")>
                                    "Код"
                                    <span class={move || get_sort_class("code", &state.get().sort_field)}>{move || get_sort_indicator("code", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </th>
                                <th class="table__header-cell table__header-cell--sortable" on:click=toggle_sort("description")>
                                    "Описание"
                                    <span class={move || get_sort_class("description", &state.get().sort_field)}>{move || get_sort_indicator("description", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </th>
                                <th class="table__header-cell table__header-cell--sortable" on:click=toggle_sort("article")>
                                    "Артикул"
                                    <span class={move || get_sort_class("article", &state.get().sort_field)}>{move || get_sort_indicator("article", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </th>
                                <th class="table__header-cell table__header-cell--sortable" on:click=toggle_sort("marketplace_sku")>
                                    "SKU"
                                    <span class={move || get_sort_class("marketplace_sku", &state.get().sort_field)}>{move || get_sort_indicator("marketplace_sku", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </th>
                                <th class="table__header-cell table__header-cell--sortable" on:click=toggle_sort("barcode")>
                                    "Штрихкод"
                                    <span class={move || get_sort_class("barcode", &state.get().sort_field)}>{move || get_sort_indicator("barcode", &state.get().sort_field, state.get().sort_ascending)}</span>
                                </th>
                                <th class="table__header-cell table__header-cell--center">"1С"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || {
                                items.get().into_iter().map(|item| {
                                    let id = item.id.clone();
                                    let id_for_click = id.clone();
                                    let id_for_checkbox = id.clone();
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
                                            <td class="table__cell" style="cursor: pointer; color: var(--color-primary); font-weight: 600;" on:click={
                                                let id = id_for_click.clone();
                                                move |_| open_detail(id.clone())
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
        </div>
    }
}
