pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::close_page_button::ClosePageButton;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::TableCrosshairHighlight;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::init_column_resize;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use thaw::*;
use wasm_bindgen::JsCast;
use web_sys::{
    Blob, BlobPropertyBag, HtmlAnchorElement, Request as WebRequest, RequestInit, RequestMode,
    Response, Url,
};

const TABLE_ID: &str = "a029-wb-supply-table";
const COLUMN_WIDTHS_KEY: &str = "a029_wb_supply_column_widths";

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

fn format_time(iso_date: &str) -> String {
    if let Some((_, time_part)) = iso_date.split_once('T') {
        let time_clean = time_part
            .split('Z')
            .next()
            .unwrap_or(time_part)
            .split('+')
            .next()
            .unwrap_or(time_part);
        let time_clean = time_clean.split('-').next().unwrap_or(time_clean);
        if let Some(hms) = time_clean.split('.').next() {
            let mut parts = hms.split(':');
            if let (Some(h), Some(m)) = (parts.next(), parts.next()) {
                return format!("{}:{}", h, m);
            }
        }
    }
    "—".to_string()
}

fn cargo_type_label(cargo_type: Option<i32>) -> &'static str {
    match cargo_type {
        Some(0) => "Виртуальная",
        Some(1) => "Короб",
        Some(2) => "Монопаллета",
        Some(5) => "Суперсейф",
        _ => "—",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub code: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WbSupplyDto {
    pub id: String,
    pub supply_id: String,
    pub supply_name: Option<String>,
    pub is_deleted: bool,
    pub is_done: bool,
    pub is_b2b: bool,
    pub created_at_wb: Option<String>,
    pub closed_at_wb: Option<String>,
    pub cargo_type: Option<i32>,
    pub connection_id: String,
    pub organization_name: Option<String>,
    pub orders_count: i64,
    pub is_posted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<WbSupplyDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[component]
pub fn WbSupplyList() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (organizations, set_organizations) = signal::<Vec<Organization>>(Vec::new());

    // Use supply_id (WB-GI-XXXXX) as the tab key so opening from the list
    // and from an order link always activates the same tab.
    let open_detail = move |_id: String, supply_id: String| {
        tabs_store.open_tab(
            &format!("a029_wb_supply_details_{}", supply_id),
            &format!("Поставка {}", supply_id),
        );
    };

    let load_supplies = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let date_from_val = state.with_untracked(|s| s.date_from.clone());
            let date_to_val = state.with_untracked(|s| s.date_to.clone());
            let org_id = state.with_untracked(|s| s.selected_organization_id.clone());
            let search_query_val = state.with_untracked(|s| s.search_query.clone());
            let page = state.with_untracked(|s| s.page);
            let page_size = state.with_untracked(|s| s.page_size);
            let sort_field = state.with_untracked(|s| s.sort_field.clone());
            let sort_ascending = state.with_untracked(|s| s.sort_ascending);
            let show_done_val = state.with_untracked(|s| s.show_done);
            let offset = page * page_size;

            let mut url = format!(
                "{}/api/a029/wb-supply?date_from={}&date_to={}&limit={}&offset={}&sort_by={}&sort_desc={}&show_done={}",
                api_base(),
                date_from_val,
                date_to_val,
                page_size,
                offset,
                sort_field,
                !sort_ascending,
                show_done_val,
            );
            if let Some(org_id) = org_id {
                url.push_str(&format!("&organization_id={}", org_id));
            }
            if !search_query_val.is_empty() {
                url.push_str(&format!(
                    "&search_query={}",
                    urlencoding::encode(&search_query_val)
                ));
            }

            match Request::get(&url).send().await {
                Ok(response) if response.ok() => match response.json::<PaginatedResponse>().await {
                    Ok(payload) => {
                        state.update(|s| {
                            s.supplies = payload.items;
                            s.total_count = payload.total;
                            s.total_pages = payload.total_pages;
                            s.page = payload.page;
                            s.page_size = payload.page_size;
                            s.is_loaded = true;
                        });
                    }
                    Err(e) => set_error.set(Some(format!("Ошибка парсинга: {}", e))),
                },
                Ok(response) => {
                    set_error.set(Some(format!("Ошибка сервера: HTTP {}", response.status())))
                }
                Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
            }
            set_loading.set(false);
        });
    };

    // Initial load
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            spawn_local(async move {
                match fetch_organizations().await {
                    Ok(orgs) => set_organizations.set(orgs),
                    Err(_) => {}
                }
            });
            load_supplies();
        }
    });

    // Column resize
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

    let search_query = RwSignal::new(state.get_untracked().search_query.clone());
    let selected_org_id = RwSignal::new(
        state
            .get_untracked()
            .selected_organization_id
            .clone()
            .unwrap_or_default(),
    );
    let show_done = RwSignal::new(state.get_untracked().show_done);

    Effect::new(move || {
        let v = search_query.get();
        untrack(move || state.update(|s| s.search_query = v));
    });

    Effect::new(move || {
        let v = selected_org_id.get();
        untrack(move || {
            state.update(|s| {
                s.selected_organization_id = if v.is_empty() { None } else { Some(v.clone()) };
            });
        });
    });

    Effect::new(move || {
        let v = show_done.get();
        untrack(move || state.update(|s| s.show_done = v));
    });

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0usize;
        if !s.date_from.is_empty() {
            count += 1;
        }
        if !s.date_to.is_empty() {
            count += 1;
        }
        if s.selected_organization_id.is_some() {
            count += 1;
        }
        if !s.search_query.is_empty() {
            count += 1;
        }
        count
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
        load_supplies();
    };

    let go_to_page = move |page: usize| {
        state.update(|s| s.page = page);
        load_supplies();
    };

    let change_page_size = move |page_size: usize| {
        state.update(|s| {
            s.page_size = page_size;
            s.page = 0;
        });
        load_supplies();
    };

    view! {
        <PageFrame page_id="a029_wb_supply--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Поставки WB (FBS)"</h1>
                    <span class="badge badge--primary">
                        {move || state.get().total_count.to_string()}
                    </span>
                </div>
                <div class="page__header-right">
                    <Space>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=move |_| {
                                let data = state.get().supplies;
                                if let Err(e) = export_to_csv(&data) {
                                    leptos::logging::log!("Failed to export: {}", e);
                                }
                            }
                            disabled=Signal::derive(move || loading.get() || state.get().supplies.is_empty())
                        >
                            {icon("download")}
                            "Excel (csv)"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=move |_| load_supplies()
                            disabled=Signal::derive(move || loading.get())
                        >
                            {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                        </Button>
                        <ClosePageButton />
                    </Space>
                </div>
            </div>

            <div class="page__content">
                <div class="filter-panel">
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
                        </div>
                    </div>

                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End style="flex-wrap: wrap;">
                            <DateRangePicker
                                date_from=Signal::derive(move || state.with(|s| s.date_from.clone()))
                                date_to=Signal::derive(move || state.with(|s| s.date_to.clone()))
                                on_change=Callback::new(move |(from, to)| {
                                    state.update(|s| {
                                        s.date_from = from;
                                        s.date_to = to;
                                        s.page = 0;
                                    });
                                    load_supplies();
                                })
                                label="Период:".to_string()
                            />

                            <div style="width: 260px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Организация:"</Label>
                                    <Select value=selected_org_id>
                                        <option value="">"Все организации"</option>
                                        {move || organizations.get().into_iter().map(|org| {
                                            let id = org.id.clone();
                                            view! {
                                                <option value=id>{org.description}</option>
                                            }
                                        }).collect_view()}
                                    </Select>
                                </Flex>
                            </div>

                            <div style="flex: 1; max-width: 300px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Поиск:"</Label>
                                    <Input
                                        value=search_query
                                        placeholder="ID поставки, название..."
                                    />
                                </Flex>
                            </div>

                            <div style="width: 180px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>" "</Label>
                                    <Checkbox
                                        checked=show_done
                                        label="Показать завершённые"
                                    />
                                </Flex>
                            </div>

                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| {
                                    state.update(|s| s.page = 0);
                                    load_supplies();
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

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 900px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=false min_width=160.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("supply_id")>
                                        "ID поставки"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "supply_id"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "supply_id", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=180.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("supply_name")>
                                        "Название"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "supply_name"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "supply_name", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("is_done")>
                                        "Статус"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "is_done"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "is_done", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    "Тип упаковки"
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("created_at_wb")>
                                        "Дата создания"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "created_at_wb"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "created_at_wb", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                    "Время"
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("closed_at_wb")>
                                        "Дата закрытия"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "closed_at_wb"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "closed_at_wb", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                    "Заказов"
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=150.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("organization_name")>
                                        "Организация"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "organization_name"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "organization_name", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || state.get().supplies
                                key=|item| item.id.clone()
                                children=move |supply| {
                                    let supply_uuid = supply.id.clone();
                                    let supply_id_for_link = supply.supply_id.clone();
                                    let supply_id_display = supply.supply_id.clone();
                                    let supply_name = supply.supply_name.clone().unwrap_or_else(|| "—".to_string());
                                    let cargo = cargo_type_label(supply.cargo_type);
                                    let created_date = supply.created_at_wb.as_deref().map(format_date).unwrap_or_else(|| "—".to_string());
                                    let created_time = supply.created_at_wb.as_deref().map(format_time).unwrap_or_else(|| "—".to_string());
                                    let closed_date = supply.closed_at_wb.as_deref().map(format_date).unwrap_or_else(|| "—".to_string());
                                    let org_name = supply.organization_name.clone().unwrap_or_else(|| "—".to_string());
                                    let is_done = supply.is_done;
                                    let orders_count = supply.orders_count.to_string();

                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <a
                                                        href="#"
                                                        class="table__link"
                                                        style="color: #0f6cbd; text-decoration: underline;"
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            open_detail(supply_uuid.clone(), supply_id_for_link.clone());
                                                        }
                                                    >
                                                        {supply_id_display}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell><TableCellLayout truncate=true>{supply_name}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>{if is_done { "Завершена" } else { "Открыта" }}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>{cargo}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>{created_date}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>{created_time}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>{closed_date}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>{orders_count}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout truncate=true>{org_name}</TableCellLayout></TableCell>
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

async fn fetch_organizations() -> Result<Vec<Organization>, String> {
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/organization", api_base());
    let request = WebRequest::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Vec<Organization> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

fn export_to_csv(data: &[WbSupplyDto]) -> Result<(), String> {
    let mut csv = String::from("\u{FEFF}");
    csv.push_str("ID поставки;Название;Статус;B2B;Тип упаковки;Дата создания;Дата закрытия;Заказов;Организация\n");

    for s in data {
        let status = if s.is_done {
            "Завершена"
        } else {
            "Открыта"
        };
        let b2b = if s.is_b2b { "Да" } else { "Нет" };
        let cargo = cargo_type_label(s.cargo_type);
        let created = s
            .created_at_wb
            .as_deref()
            .map(format_date)
            .unwrap_or_else(|| "—".to_string());
        let closed = s
            .closed_at_wb
            .as_deref()
            .map(format_date)
            .unwrap_or_else(|| "—".to_string());
        let org = s.organization_name.as_deref().unwrap_or("—");
        let name = s.supply_name.as_deref().unwrap_or("—");

        csv.push_str(&format!(
            "{};{};{};{};{};{};{};{};{}\n",
            s.supply_id, name, status, b2b, cargo, created, closed, s.orders_count, org,
        ));
    }

    let arr = js_sys::Array::new();
    arr.push(&wasm_bindgen::JsValue::from_str(&csv));

    let options = BlobPropertyBag::new();
    options.set_type("text/csv;charset=utf-8");
    let blob = Blob::new_with_str_sequence_and_options(&arr, &options)
        .map_err(|e| format!("Blob error: {:?}", e))?;

    let url = Url::create_object_url_with_blob(&blob).map_err(|e| format!("URL error: {:?}", e))?;

    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| "No document".to_string())?;
    let a: HtmlAnchorElement = document
        .create_element("a")
        .map_err(|e| format!("{:?}", e))?
        .dyn_into()
        .map_err(|e| format!("{:?}", e))?;
    a.set_href(&url);
    a.set_download("wb_supplies.csv");
    a.click();

    Url::revoke_object_url(&url).ok();
    Ok(())
}
