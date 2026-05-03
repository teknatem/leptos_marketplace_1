use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::table::TableCrosshairHighlight;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::init_column_resize;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use thaw::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WbAdvertCampaignListDto {
    pub id: String,
    pub advert_id: i64,
    pub description: String,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub campaign_type: Option<i32>,
    pub status: Option<i32>,
    pub nm_count: i32,
    pub change_time: Option<String>,
    pub fetched_at: String,
}

impl Sortable for WbAdvertCampaignListDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "advert_id" => self.advert_id.cmp(&other.advert_id),
            "description" => self
                .description
                .to_lowercase()
                .cmp(&other.description.to_lowercase()),
            "campaign_type" => self.campaign_type.cmp(&other.campaign_type),
            "status" => self.status.cmp(&other.status),
            "nm_count" => self.nm_count.cmp(&other.nm_count),
            "change_time" => self.change_time.cmp(&other.change_time),
            "fetched_at" => self.fetched_at.cmp(&other.fetched_at),
            "connection_id" => self.connection_id.cmp(&other.connection_id),
            _ => Ordering::Equal,
        }
    }
}

const TABLE_ID: &str = "a030-wb-advert-campaign-table";
const COLUMN_WIDTHS_KEY: &str = "a030_wb_advert_campaign_column_widths";

fn fmt_dt(value: &str) -> String {
    if value.is_empty() {
        return "—".to_string();
    }
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

fn campaign_type_label(t: Option<i32>) -> &'static str {
    match t {
        Some(4) => "Каталог",
        Some(5) => "Карточка товара",
        Some(6) => "Поиск",
        Some(7) => "Рекомендации",
        Some(8) => "Поиск+Рек.",
        Some(9) => "Авто",
        _ => "—",
    }
}

fn campaign_status_label(s: Option<i32>) -> &'static str {
    match s {
        Some(4) => "Готова к запуску",
        Some(7) => "Завершена",
        Some(9) => "Идут показы",
        Some(11) => "Пауза",
        _ => "—",
    }
}

fn campaign_status_badge_color(s: Option<i32>) -> &'static str {
    match s {
        Some(9) => "var(--colorPaletteGreenBackground2)",
        Some(11) => "var(--colorPaletteYellowBackground2)",
        Some(7) => "var(--color-text-tertiary)",
        _ => "transparent",
    }
}

#[component]
pub fn WbAdvertCampaignList() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (items, set_items) = signal::<Vec<WbAdvertCampaignListDto>>(Vec::new());
    let (connections, set_connections) = signal::<Vec<ConnectionMP>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let search_query = RwSignal::new(String::new());
    let selected_connection_id = RwSignal::new(String::new());
    let selected_type = RwSignal::new(String::new());
    let selected_status = RwSignal::new(String::new());
    let sort_field = RwSignal::new("advert_id".to_string());
    let sort_ascending = RwSignal::new(true);

    let load_items = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let selected = selected_connection_id.get_untracked();
            let mut url = format!("{}/api/a030/wb-advert-campaign/list", api_base());
            if !selected.trim().is_empty() {
                url.push_str(&format!(
                    "?connection_id={}",
                    urlencoding::encode(&selected)
                ));
            }

            match Request::get(&url).send().await {
                Ok(response) if response.ok() => {
                    match response.json::<Vec<WbAdvertCampaignListDto>>().await {
                        Ok(payload) => set_items.set(payload),
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
        spawn_local(async move {
            match fetch_connections().await {
                Ok(mut payload) => {
                    payload.sort_by(|left, right| {
                        left.base
                            .description
                            .to_lowercase()
                            .cmp(&right.base.description.to_lowercase())
                    });
                    set_connections.set(payload);
                }
                Err(err) => set_error.set(Some(err)),
            }
        });
        load_items();
    });

    let resize_initialized = leptos::prelude::StoredValue::new(false);
    Effect::new(move |_| {
        if !resize_initialized.get_value() {
            resize_initialized.set_value(true);
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(100).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    let filtered_items = Signal::derive(move || {
        let query = search_query.get().to_lowercase();
        let selected = selected_connection_id.get();
        let sel_type = selected_type.get();
        let sel_status = selected_status.get();
        let mut filtered = items
            .get()
            .into_iter()
            .filter(|item| {
                let type_match = sel_type.is_empty()
                    || item
                        .campaign_type
                        .map(|v| v.to_string() == sel_type)
                        .unwrap_or(false);
                let status_match = sel_status.is_empty()
                    || item
                        .status
                        .map(|v| v.to_string() == sel_status)
                        .unwrap_or(false);
                let conn_match = selected.is_empty() || item.connection_id == selected;
                let text_match = query.is_empty()
                    || item.advert_id.to_string().contains(&query)
                    || item.description.to_lowercase().contains(&query)
                    || item.connection_id.to_lowercase().contains(&query);
                conn_match && type_match && status_match && text_match
            })
            .collect::<Vec<_>>();

        let field = sort_field.get();
        let ascending = sort_ascending.get();
        filtered.sort_by(|a, b| {
            let cmp = a.compare_by_field(b, &field);
            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });
        filtered
    });

    let open_detail = move |id: String, advert_id: i64| {
        tabs_store.open_tab(
            &format!("a030_wb_advert_campaign_details_{}", id),
            &format!("WB Campaign {}", advert_id),
        );
    };

    // Helper: resolve connection name from uuid
    let connection_name = move |conn_id: &str| -> String {
        connections
            .get()
            .into_iter()
            .find(|c| c.base.id.as_string() == conn_id)
            .map(|c| c.base.description.clone())
            .unwrap_or_else(|| conn_id.to_string())
    };

    let toggle_sort = move |field: &'static str| {
        if sort_field.get_untracked() == field {
            sort_ascending.update(|ascending| *ascending = !*ascending);
        } else {
            sort_field.set(field.to_string());
            sort_ascending.set(true);
        }
    };

    view! {
        <PageFrame page_id="a030_wb_advert_campaign--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Рекламные кампании WB"</h1>
                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                        {move || filtered_items.get().len().to_string()}
                    </Badge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| load_items()
                        disabled=Signal::derive(move || loading.get())
                    >
                        {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                    </Button>
                </div>
            </div>

            <div class="page__content">
                <div class="filter-panel">
                    <div class="filter-panel-content" style="display: block;">
                        <Flex gap=FlexGap::Small align=FlexAlign::End style="flex-wrap: wrap;">
                            // WB кабинет
                            <div style="min-width: 260px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"WB кабинет"</Label>
                                    <select
                                        class="form__input"
                                        prop:value=move || selected_connection_id.get()
                                        on:change=move |ev| {
                                            selected_connection_id.set(event_target_value(&ev));
                                            load_items();
                                        }
                                    >
                                        <option value="">"Все кабинеты"</option>
                                        <For
                                            each=move || connections.get()
                                            key=|conn| conn.base.id.as_string()
                                            children=move |conn| {
                                                let id = conn.base.id.as_string();
                                                let label = conn.base.description.clone();
                                                view! { <option value=id>{label}</option> }
                                            }
                                        />
                                    </select>
                                </Flex>
                            </div>
                            // Тип кампании
                            <div style="min-width: 180px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Тип"</Label>
                                    <select
                                        class="form__input"
                                        prop:value=move || selected_type.get()
                                        on:change=move |ev| selected_type.set(event_target_value(&ev))
                                    >
                                        <option value="">"Все типы"</option>
                                        <option value="4">"Каталог"</option>
                                        <option value="5">"Карточка товара"</option>
                                        <option value="6">"Поиск"</option>
                                        <option value="7">"Рекомендации"</option>
                                        <option value="8">"Поиск+Рек."</option>
                                        <option value="9">"Авто"</option>
                                    </select>
                                </Flex>
                            </div>
                            // Статус
                            <div style="min-width: 180px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Статус"</Label>
                                    <select
                                        class="form__input"
                                        prop:value=move || selected_status.get()
                                        on:change=move |ev| selected_status.set(event_target_value(&ev))
                                    >
                                        <option value="">"Все статусы"</option>
                                        <option value="4">"Готова к запуску"</option>
                                        <option value="7">"Завершена"</option>
                                        <option value="9">"Идут показы"</option>
                                        <option value="11">"Пауза"</option>
                                    </select>
                                </Flex>
                            </div>
                            // Поиск
                            <div style="min-width: 240px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Поиск"</Label>
                                    <Input
                                        value=search_query
                                        placeholder="advertId, название..."
                                    />
                                </Flex>
                            </div>
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| {
                                    search_query.set(String::new());
                                    selected_connection_id.set(String::new());
                                    selected_type.set(String::new());
                                    selected_status.set(String::new());
                                    load_items();
                                }
                            >
                                {icon("x")}
                                " Сбросить"
                            </Button>
                        </Flex>
                    </div>
                </div>

                {move || {
                    error.get().map(|err| view! {
                        <div class="alert alert--error">{err}</div>
                    })
                }}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />
                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1100px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("advert_id")>
                                        "advertId"
                                        <span class=move || get_sort_class(&sort_field.get(), "advert_id")>
                                            {move || get_sort_indicator(&sort_field.get(), "advert_id", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=220.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("description")>
                                        "Название"
                                        <span class=move || get_sort_class(&sort_field.get(), "description")>
                                            {move || get_sort_indicator(&sort_field.get(), "description", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("campaign_type")>
                                        "Тип"
                                        <span class=move || get_sort_class(&sort_field.get(), "campaign_type")>
                                            {move || get_sort_indicator(&sort_field.get(), "campaign_type", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("status")>
                                        "Статус"
                                        <span class=move || get_sort_class(&sort_field.get(), "status")>
                                            {move || get_sort_indicator(&sort_field.get(), "status", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=80.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("nm_count")>
                                        "Позиций"
                                        <span class=move || get_sort_class(&sort_field.get(), "nm_count")>
                                            {move || get_sort_indicator(&sort_field.get(), "nm_count", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=160.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("change_time")>
                                        "Изменено WB"
                                        <span class=move || get_sort_class(&sort_field.get(), "change_time")>
                                            {move || get_sort_indicator(&sort_field.get(), "change_time", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=160.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("fetched_at")>
                                        "Обновлено"
                                        <span class=move || get_sort_class(&sort_field.get(), "fetched_at")>
                                            {move || get_sort_indicator(&sort_field.get(), "fetched_at", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=200.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("connection_id")>
                                        "Кабинет"
                                        <span class=move || get_sort_class(&sort_field.get(), "connection_id")>
                                            {move || get_sort_indicator(&sort_field.get(), "connection_id", sort_ascending.get())}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            <For
                                each=move || filtered_items.get()
                                key=|item| item.id.clone()
                                children=move |item| {
                                    let id = item.id.clone();
                                    let advert_id = item.advert_id;
                                    let conn_name = connection_name(&item.connection_id);
                                    let status_color = campaign_status_badge_color(item.status);
                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <a
                                                        href="#"
                                                        class="table__link"
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            open_detail(id.clone(), advert_id);
                                                        }
                                                    >
                                                        {advert_id.to_string()}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.description.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {campaign_type_label(item.campaign_type)}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <span style=format!(
                                                        "display:inline-block; padding:2px 8px; border-radius:4px; font-size:0.82em; background:{};",
                                                        status_color
                                                    )>
                                                        {campaign_status_label(item.status)}
                                                    </span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {if item.nm_count > 0 {
                                                        item.nm_count.to_string()
                                                    } else {
                                                        "—".to_string()
                                                    }}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {item.change_time.as_deref().map(fmt_dt).unwrap_or_else(|| "—".to_string())}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {fmt_dt(&item.fetched_at)}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {conn_name}
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

async fn fetch_connections() -> Result<Vec<ConnectionMP>, String> {
    let url = format!("{}/api/connection_mp", api_base());
    let response = Request::get(&url).send().await.map_err(|e| e.to_string())?;
    if !response.ok() {
        return Err(format!(
            "Ошибка загрузки кабинетов: HTTP {}",
            response.status()
        ));
    }
    response
        .json::<Vec<ConnectionMP>>()
        .await
        .map_err(|e| e.to_string())
}
