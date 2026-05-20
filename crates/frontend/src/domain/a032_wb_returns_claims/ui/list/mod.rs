pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::close_page_button::ClosePageButton;
use crate::shared::components::table::TableCrosshairHighlight;
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::init_column_resize;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use thaw::*;

const TABLE_ID: &str = "a032-wb-returns-claims-table";
const COLUMN_WIDTHS_KEY: &str = "a032_wb_returns_claims_column_widths";

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

fn status_label(status: Option<i32>) -> &'static str {
    match status {
        Some(1) => "Открыта",
        Some(2) => "На рассмотрении",
        Some(3) => "Одобрена",
        Some(4) => "Отклонена",
        Some(5) => "Закрыта",
        _ => "—",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WbReturnsClaimsListDto {
    pub id: String,
    #[serde(rename = "claimId")]
    pub claim_id: String,
    #[serde(rename = "nmId")]
    pub nm_id: i64,
    #[serde(rename = "imtName")]
    pub imt_name: Option<String>,
    pub status: Option<i32>,
    pub dt: String,
    #[serde(rename = "orderDt")]
    pub order_dt: Option<String>,
    #[serde(rename = "dtUpdate")]
    pub dt_update: Option<String>,
    pub price: Option<f64>,
    #[serde(rename = "currencyCode")]
    pub currency_code: Option<String>,
    pub srid: Option<String>,
    #[serde(rename = "isArchive")]
    pub is_archive: bool,
    #[serde(rename = "userComment")]
    pub user_comment: Option<String>,
    #[serde(rename = "orgName")]
    pub org_name: Option<String>,
}

#[component]
pub fn WbReturnsClaimsList() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    let open_detail = move |id: String, claim_id: String| {
        tabs_store.open_tab(
            &format!("a032_wb_returns_claims_details_{}", id),
            &format!("Заявка {}", &claim_id[..claim_id.len().min(12)]),
        );
    };

    let load_claims = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("{}/api/a032/wb-returns-claims", api_base());

            match Request::get(&url).send().await {
                Ok(response) if response.ok() => {
                    match response.json::<Vec<WbReturnsClaimsListDto>>().await {
                        Ok(items) => {
                            state.update(|s| {
                                s.items = items;
                                s.is_loaded = true;
                            });
                        }
                        Err(e) => set_error.set(Some(format!("Ошибка парсинга: {}", e))),
                    }
                }
                Ok(response) => {
                    set_error.set(Some(format!("Ошибка сервера: HTTP {}", response.status())))
                }
                Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
            }
            set_loading.set(false);
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_claims();
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

    let search_query = RwSignal::new(String::new());
    let show_archived = RwSignal::new(true);

    // Фильтр по статусам — по одному сигналу на каждый статус
    let status1_on = RwSignal::new(false); // Открыта
    let status2_on = RwSignal::new(false); // На рассмотрении
    let status3_on = RwSignal::new(false); // Одобрена
    let status4_on = RwSignal::new(false); // Отклонена
    let status5_on = RwSignal::new(false); // Закрыта

    Effect::new(move || {
        let v = search_query.get();
        untrack(move || state.update(|s| s.search_query = v));
    });

    Effect::new(move || {
        let v = show_archived.get();
        untrack(move || state.update(|s| s.show_archived = v));
    });

    Effect::new(move || {
        let mut v: Vec<i32> = Vec::new();
        if status1_on.get() {
            v.push(1);
        }
        if status2_on.get() {
            v.push(2);
        }
        if status3_on.get() {
            v.push(3);
        }
        if status4_on.get() {
            v.push(4);
        }
        if status5_on.get() {
            v.push(5);
        }
        untrack(move || state.update(|s| s.selected_statuses = v));
    });

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0usize;
        if !s.search_query.is_empty() {
            count += 1;
        }
        if !s.show_archived {
            count += 1;
        }
        if !s.selected_statuses.is_empty() {
            count += 1;
        }
        count
    });

    let displayed_items = Signal::derive(move || {
        let s = state.get();
        let query = s.search_query.to_lowercase();
        let mut items: Vec<WbReturnsClaimsListDto> = s
            .items
            .iter()
            .filter(|item| {
                if !s.show_archived && item.is_archive {
                    return false;
                }
                if !s.selected_statuses.is_empty() {
                    let item_status = item.status.unwrap_or(-1);
                    if !s.selected_statuses.contains(&item_status) {
                        return false;
                    }
                }
                if query.is_empty() {
                    return true;
                }
                item.claim_id.to_lowercase().contains(&query)
                    || item
                        .imt_name
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
                    || item
                        .srid
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
                    || item.nm_id.to_string().contains(&query)
                    || item
                        .user_comment
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
                    || item
                        .org_name
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
            })
            .cloned()
            .collect();

        match s.sort_field.as_str() {
            "dt" => items.sort_by(|a, b| {
                if s.sort_ascending {
                    a.dt.cmp(&b.dt)
                } else {
                    b.dt.cmp(&a.dt)
                }
            }),
            "nm_id" => items.sort_by(|a, b| {
                if s.sort_ascending {
                    a.nm_id.cmp(&b.nm_id)
                } else {
                    b.nm_id.cmp(&a.nm_id)
                }
            }),
            "price" => items.sort_by(|a, b| {
                let av = a.price.unwrap_or(0.0);
                let bv = b.price.unwrap_or(0.0);
                if s.sort_ascending {
                    av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
            }),
            "order_dt" => items.sort_by(|a, b| {
                let av = a.order_dt.as_deref().unwrap_or("");
                let bv = b.order_dt.as_deref().unwrap_or("");
                if s.sort_ascending {
                    av.cmp(bv)
                } else {
                    bv.cmp(av)
                }
            }),
            _ => {}
        }
        items
    });

    let toggle_sort = move |field: &'static str| {
        state.update(|s| {
            if s.sort_field == field {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_field = field.to_string();
                s.sort_ascending = false;
            }
        });
    };

    view! {
        <PageFrame page_id="a032_wb_returns_claims--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Заявки на возврат WB"</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || displayed_items.get().len().to_string()}
                    </UiBadge>
                </div>
                <div class="page__header-right">
                    <ClosePageButton />
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
                        <div class="filter-panel-header__right">
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| load_claims()
                                disabled=Signal::derive(move || loading.get())
                            >
                                {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                            </Button>
                        </div>
                    </div>

                    {move || {
                        if is_filter_expanded.get() {
                            view! {
                                <div class="filter-panel-content">
                                    <Flex gap=FlexGap::Small align=FlexAlign::End>
                                        <div style="flex: 1; max-width: 320px;">
                                            <Flex vertical=true gap=FlexGap::Small>
                                                <Label>"Поиск:"</Label>
                                                <Input
                                                    value=search_query
                                                    placeholder="ID заявки, nmId, товар, srid, комментарий..."
                                                />
                                            </Flex>
                                        </div>
                                        <div style="width: 200px;">
                                            <Flex vertical=true gap=FlexGap::Small>
                                                <Label>" "</Label>
                                                <Checkbox
                                                    checked=show_archived
                                                    label="Показать архивные"
                                                />
                                            </Flex>
                                        </div>
                                    </Flex>
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Статусы:"</Label>
                                        <div style="display: flex; flex-wrap: wrap; gap: 8px; align-items: center;">
                                            <Checkbox checked=status1_on label="Открыта" />
                                            <Checkbox checked=status2_on label="На рассмотрении" />
                                            <Checkbox checked=status3_on label="Одобрена" />
                                            <Checkbox checked=status4_on label="Отклонена" />
                                            <Checkbox checked=status5_on label="Закрыта" />
                                        </div>
                                    </Flex>
                                </div>
                            }
                            .into_any()
                        } else {
                            view! { <></> }.into_any()
                        }
                    }}
                </div>

                {move || {
                    if let Some(err) = error.get() {
                        view! { <div class="alert alert--error">{err}</div> }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1200px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div
                                        class="table__sortable-header"
                                        style="cursor: pointer;"
                                        on:click=move |_| toggle_sort("dt")
                                    >
                                        "Дата заявки"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "dt"))>
                                            {move || get_sort_indicator(
                                                &state.with(|s| s.sort_field.clone()),
                                                "dt",
                                                state.with(|s| s.sort_ascending),
                                            )}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=220.0 class="resizable">
                                    "ID заявки WB"
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                    <div
                                        class="table__sortable-header"
                                        style="cursor: pointer;"
                                        on:click=move |_| toggle_sort("nm_id")
                                    >
                                        "nmId"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "nm_id"))>
                                            {move || get_sort_indicator(
                                                &state.with(|s| s.sort_field.clone()),
                                                "nm_id",
                                                state.with(|s| s.sort_ascending),
                                            )}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=180.0 class="resizable">
                                    "Товар"
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                    "Статус"
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                    <div
                                        class="table__sortable-header"
                                        style="cursor: pointer;"
                                        on:click=move |_| toggle_sort("price")
                                    >
                                        "Цена"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "price"))>
                                            {move || get_sort_indicator(
                                                &state.with(|s| s.sort_field.clone()),
                                                "price",
                                                state.with(|s| s.sort_ascending),
                                            )}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div
                                        class="table__sortable-header"
                                        style="cursor: pointer;"
                                        on:click=move |_| toggle_sort("order_dt")
                                    >
                                        "Дата заказа"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "order_dt"))>
                                            {move || get_sort_indicator(
                                                &state.with(|s| s.sort_field.clone()),
                                                "order_dt",
                                                state.with(|s| s.sort_ascending),
                                            )}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=200.0 class="resizable">
                                    "srid"
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=160.0 class="resizable">
                                    "Организация"
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=220.0 class="resizable">
                                    "Комментарий"
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || displayed_items.get()
                                key=|item| item.id.clone()
                                children=move |item| {
                                    let item_id = item.id.clone();
                                    let claim_id = item.claim_id.clone();
                                    let claim_id_for_link = item.claim_id.clone();
                                    let id_for_link = item_id.clone();
                                    let date_str = format_date(&item.dt);
                                    let nm_id = item.nm_id;
                                    let imt_name = item.imt_name.clone().unwrap_or_else(|| "—".to_string());
                                    let badge_text = status_label(item.status);
                                    let price_str = item.price.map(|p| format!("{:.0}", p)).unwrap_or_else(|| "—".to_string());
                                    let order_date_str = item.order_dt.as_deref().map(format_date).unwrap_or_else(|| "—".to_string());
                                    let srid_str = item.srid.clone().unwrap_or_else(|| "—".to_string());
                                    let org_name_str = item.org_name.clone().unwrap_or_else(|| "—".to_string());
                                    let comment_str = item.user_comment.clone().unwrap_or_else(|| "—".to_string());

                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {date_str}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <a
                                                        href="#"
                                                        class="table__link"
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            open_detail(
                                                                id_for_link.clone(),
                                                                claim_id_for_link.clone(),
                                                            );
                                                        }
                                                    >
                                                        {claim_id}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {nm_id}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {imt_name}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    <span class="status-badge">{badge_text}</span>
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {price_str}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {order_date_str}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {srid_str}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {org_name_str}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {comment_str}
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
