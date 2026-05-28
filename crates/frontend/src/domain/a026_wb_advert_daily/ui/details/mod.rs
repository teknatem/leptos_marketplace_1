use crate::general_ledger::api::fetch_document_general_ledger_entries;
use crate::general_ledger::ui::{
    document_general_ledger_entries_nav_id, DocumentGeneralLedgerEntries,
};
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::components::table::TableCrosshairHighlight;
use crate::shared::export::{export_to_excel, ExcelExportable};
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, sort_list, Sortable};
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use crate::shared::table_utils::{clear_resize_flag, init_column_resize, was_just_resizing};
use crate::system::favorites::ui::FavoriteButton;
use contracts::domain::a026_wb_advert_daily::aggregate::WbAdvertDailyMetrics;
use contracts::general_ledger::GeneralLedgerEntryDto;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use std::cmp::Ordering;
use thaw::*;

fn fmt_date(v: &str) -> String {
    if let Some((y, rest)) = v.split_once('-') {
        if let Some((m, d)) = rest.split_once('-') {
            return format!("{}.{}.{}", d, m, y);
        }
    }
    v.to_string()
}

fn fmt_dt(v: &str) -> String {
    if let Some((d, t)) = v.split_once('T') {
        return format!(
            "{} {}",
            fmt_date(d),
            t.split(['Z', '+', '.']).next().unwrap_or(t)
        );
    }
    fmt_date(v)
}

fn fmt_money(v: f64) -> String {
    format!("{:.2}", v)
}

fn fmt_ratio(v: f64) -> String {
    format!("{:.2}", v)
}

/// Минимальный расход для отображения строки в таблице атрибуции (1 коп.).
const MIN_ALLOCATED_COST_DISPLAY: f64 = 0.01;

fn fmt_expense_share(expense: f64, total_expense: f64) -> String {
    if total_expense.abs() <= f64::EPSILON {
        "—".to_string()
    } else {
        fmt_ratio(expense / total_expense * 100.0)
    }
}

fn should_show_linked_order(order: &FoundOrderDto) -> bool {
    !order.is_allocated || order.allocated_cost.abs() >= MIN_ALLOCATED_COST_DISPLAY
}

fn should_show_linked_group(group: &LinkedOrdersByNmDto) -> bool {
    if group.wb_reported_orders > 0 && group.found_orders.is_empty() {
        return true;
    }
    if group.wb_advert_sum.abs() >= MIN_ALLOCATED_COST_DISPLAY {
        return true;
    }
    group.found_orders.iter().any(should_show_linked_order)
}

fn fmt_csv_decimal(v: f64) -> String {
    format!("{:.2}", v).replace('.', ",")
}

fn fmt_advert_id(advert_id: i64) -> String {
    if advert_id > 0 {
        advert_id.to_string()
    } else {
        "—".to_string()
    }
}

fn cmp_text(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

fn cmp_optional_text(a: &Option<String>, b: &Option<String>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => cmp_text(a, b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn cmp_float(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

const LINES_TABLE_ID: &str = "a026-wb-advert-daily-lines-table";
const LINES_COLUMN_WIDTHS_KEY: &str = "a026_wb_advert_daily_details_lines_column_widths";
const LINKED_ORDERS_TABLE_ID: &str = "a026-wb-advert-daily-linked-orders-table";
const LINKED_ORDERS_COLUMN_WIDTHS_KEY: &str =
    "a026_wb_advert_daily_details_linked_orders_column_widths";

#[derive(Debug, Clone, Deserialize)]
struct LineDto {
    nm_id: i64,
    wb_name: String,
    nomenclature_ref: Option<String>,
    nomenclature_article: Option<String>,
    metrics: WbAdvertDailyMetrics,
}

impl Sortable for LineDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "nm_id" => self.nm_id.cmp(&other.nm_id),
            "wb_name" => cmp_text(&self.wb_name, &other.wb_name),
            "nomenclature_article" => {
                cmp_optional_text(&self.nomenclature_article, &other.nomenclature_article)
            }
            "views" => self.metrics.views.cmp(&other.metrics.views),
            "clicks" => self.metrics.clicks.cmp(&other.metrics.clicks),
            "ctr" => cmp_float(self.metrics.ctr, other.metrics.ctr),
            "cpc" => cmp_float(self.metrics.cpc, other.metrics.cpc),
            "atbs" => self.metrics.atbs.cmp(&other.metrics.atbs),
            "orders" => self.metrics.orders.cmp(&other.metrics.orders),
            "shks" => self.metrics.shks.cmp(&other.metrics.shks),
            "sum" => cmp_float(self.metrics.sum, other.metrics.sum),
            "sum_price" => cmp_float(self.metrics.sum_price, other.metrics.sum_price),
            "cr" => cmp_float(self.metrics.cr, other.metrics.cr),
            "canceled" => self.metrics.canceled.cmp(&other.metrics.canceled),
            _ => Ordering::Equal,
        }
    }
}

impl ExcelExportable for LineDto {
    fn headers() -> Vec<&'static str> {
        vec![
            "nmID",
            "WB наименование",
            "Артикул 1С",
            "Просмотры",
            "Клики",
            "CTR, %",
            "CPC",
            "В корзину",
            "Заказы",
            "Штуки",
            "Расход",
            "Выручка",
            "CR, %",
            "Отмены",
        ]
    }

    fn to_csv_row(&self) -> Vec<String> {
        vec![
            self.nm_id.to_string(),
            self.wb_name.clone(),
            self.nomenclature_article
                .clone()
                .unwrap_or_else(|| "—".to_string()),
            self.metrics.views.to_string(),
            self.metrics.clicks.to_string(),
            fmt_csv_decimal(self.metrics.ctr),
            fmt_csv_decimal(self.metrics.cpc),
            self.metrics.atbs.to_string(),
            self.metrics.orders.to_string(),
            self.metrics.shks.to_string(),
            fmt_csv_decimal(self.metrics.sum),
            fmt_csv_decimal(self.metrics.sum_price),
            fmt_csv_decimal(self.metrics.cr),
            self.metrics.canceled.to_string(),
        ]
    }
}

#[derive(Debug, Clone, Deserialize)]
struct FoundOrderDto {
    order_key: String,
    #[serde(default)]
    order_id: Option<String>,
    #[serde(default)]
    order_date: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    nomenclature_ref: Option<String>,
    #[serde(default)]
    finished_price: Option<f64>,
    #[serde(default)]
    is_cancel: bool,
    #[serde(default)]
    allocation_basis: f64,
    #[serde(default)]
    is_allocated: bool,
    #[serde(default)]
    #[allow(dead_code)]
    allocation_ratio: f64,
    #[serde(default)]
    allocated_cost: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct LinkedOrdersByNmDto {
    nm_id: i64,
    nm_name: String,
    #[serde(default)]
    nomenclature_ref: Option<String>,
    #[serde(default)]
    nomenclature_article: Option<String>,
    #[serde(default)]
    wb_reported_orders: i64,
    #[serde(default)]
    wb_advert_sum: f64,
    #[serde(default)]
    found_orders: Vec<FoundOrderDto>,
}

#[derive(Debug, Clone, Deserialize)]
struct DetailsDto {
    id: String,
    document_no: String,
    document_date: String,
    #[serde(default)]
    advert_id: i64,
    connection_id: String,
    connection_name: Option<String>,
    organization_id: String,
    organization_name: Option<String>,
    marketplace_id: String,
    marketplace_name: Option<String>,
    totals: WbAdvertDailyMetrics,
    unattributed_totals: WbAdvertDailyMetrics,
    source: String,
    fetched_at: String,
    created_at: String,
    updated_at: String,
    is_posted: bool,
    lines: Vec<LineDto>,
    #[serde(default)]
    has_linked_orders: bool,
    #[serde(default)]
    linked_orders_count: i64,
    #[serde(default)]
    linked_orders: Vec<LinkedOrdersByNmDto>,
}

#[derive(Debug, Clone, Deserialize)]
struct P913Row {
    id: String,
    entry_date: String,
    turnover_code: String,
    amount: f64,
    #[serde(default)]
    #[allow(dead_code)]
    nomenclature_ref: Option<String>,
    wb_advert_campaign_code: String,
    order_key: String,
    #[serde(default)]
    is_problem: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct P911Row {
    id: String,
    entry_date: String,
    #[allow(dead_code)]
    turnover_code: String,
    amount: f64,
    #[serde(default)]
    nomenclature_ref: Option<String>,
    #[serde(default)]
    marketplace_product_ref: Option<String>,
    wb_advert_campaign_code: String,
    #[serde(default)]
    is_problem: bool,
}

#[component]
fn ReadField(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="form__group">
            <label class="form__label">{label}</label>
            <Input value=RwSignal::new(value) attr:readonly=true />
        </div>
    }
}

#[component]
fn SortHeaderCell(
    label: &'static str,
    field: &'static str,
    min_width: f32,
    sort_field: ReadSignal<String>,
    sort_ascending: ReadSignal<bool>,
    on_toggle: Callback<&'static str>,
    #[prop(default = false)] numeric: bool,
) -> impl IntoView {
    let header_style = if numeric {
        "cursor: pointer; text-align: right; justify-content: flex-end;"
    } else {
        "cursor: pointer;"
    };

    view! {
        <TableHeaderCell resizable=false min_width=min_width class="resizable">
            <div class="table__sortable-header" style=header_style on:click=move |_| {
                if was_just_resizing() {
                    clear_resize_flag();
                    return;
                }
                on_toggle.run(field)
            }>
                {label}
                <span class=move || get_sort_class(&sort_field.get(), field)>
                    {move || get_sort_indicator(&sort_field.get(), field, sort_ascending.get())}
                </span>
            </div>
        </TableHeaderCell>
    }
}

#[component]
pub fn WbAdvertDailyDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (doc, set_doc) = signal::<Option<DetailsDto>>(None);
    let (tab, set_tab) = signal("general".to_string());
    let (posting, set_posting) = signal(false);
    let (journal, set_journal) = signal(Vec::<GeneralLedgerEntryDto>::new());
    let (journal_loaded, set_journal_loaded) = signal(false);
    let (journal_loading, set_journal_loading) = signal(false);
    let (journal_error, set_journal_error) = signal::<Option<String>>(None);
    let (projections, set_projections) = signal(Vec::<P913Row>::new());
    let (projections_p911, set_projections_p911) = signal(Vec::<P911Row>::new());
    let (projections_loaded, set_projections_loaded) = signal(false);
    let (lines_sort_field, set_lines_sort_field) = signal("wb_name".to_string());
    let (lines_sort_ascending, set_lines_sort_ascending) = signal(true);
    let lines_resize_initialized = StoredValue::new(false);
    let linked_orders_resize_initialized = StoredValue::new(false);
    let (linked_orders_tree_expanded, set_linked_orders_tree_expanded) = signal(true);

    let load_doc = {
        let tabs = tabs.clone();
        let stored_id = stored_id;
        Callback::new(move |()| {
            let current_id = stored_id.get_value();
            let tab_id = stored_id.get_value();
            let tabs = tabs.clone();
            spawn_local(async move {
                set_loading.set(true);
                set_error.set(None);
                match Request::get(&format!(
                    "{}/api/a026/wb-advert-daily/{}",
                    api_base(),
                    current_id
                ))
                .send()
                .await
                {
                    Ok(resp) if resp.ok() => match resp.json::<DetailsDto>().await {
                        Ok(data) => {
                            let title = if data.advert_id > 0 {
                                format!("WB Ads {} · {}", data.document_date, data.advert_id)
                            } else {
                                format!("WB Ads {}", data.document_date)
                            };
                            tabs.update_tab_title(
                                &format!("a026_wb_advert_daily_details_{tab_id}"),
                                &title,
                            );
                            set_doc.set(Some(data));
                        }
                        Err(err) => set_error.set(Some(format!("Ошибка парсинга: {}", err))),
                    },
                    Ok(resp) => {
                        set_error.set(Some(format!("Ошибка сервера: HTTP {}", resp.status())))
                    }
                    Err(err) => set_error.set(Some(format!("Ошибка сети: {}", err))),
                }
                set_loading.set(false);
            });
        })
    };

    let load_journal = {
        let stored_id = stored_id;
        Callback::new(move |()| {
            let current_id = stored_id.get_value();
            spawn_local(async move {
                set_journal_loading.set(true);
                set_journal_error.set(None);
                match fetch_document_general_ledger_entries("a026_wb_advert_daily", &current_id)
                    .await
                {
                    Ok(rows) => {
                        set_journal.set(rows);
                        set_journal_loaded.set(true);
                    }
                    Err(err) => {
                        set_journal_error.set(Some(err));
                    }
                }
                set_journal_loading.set(false);
            });
        })
    };

    let load_projections = {
        let stored_id = stored_id;
        Callback::new(move |()| {
            let current_id = stored_id.get_value();
            spawn_local(async move {
                match Request::get(&format!(
                    "{}/api/a026/wb-advert-daily/{}/projections",
                    api_base(),
                    current_id
                ))
                .send()
                .await
                {
                    Ok(resp) if resp.ok() => {
                        if let Ok(value) = resp.json::<serde_json::Value>().await {
                            let rows = value["p913_wb_advert_order_attr"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|row| {
                                            serde_json::from_value::<P913Row>(row.clone()).ok()
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            let rows_p911 = value["p911_wb_advert_by_items"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|row| {
                                            serde_json::from_value::<P911Row>(row.clone()).ok()
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            set_projections.set(rows);
                            set_projections_p911.set(rows_p911);
                            set_projections_loaded.set(true);
                        }
                    }
                    _ => {}
                }
            });
        })
    };

    Effect::new({
        let load_doc = load_doc.clone();
        move |_| load_doc.run(())
    });

    Effect::new({
        let load_journal = load_journal.clone();
        move |_| {
            if doc.get().as_ref().is_some_and(|item| item.is_posted) && !journal_loaded.get() {
                load_journal.run(());
            }
            if tab.get() == "journal" && !journal_loaded.get() {
                load_journal.run(());
            }
        }
    });

    Effect::new({
        let load_projections = load_projections.clone();
        move |_| {
            if tab.get() == "projections" && !projections_loaded.get() {
                load_projections.run(());
            }
        }
    });

    Effect::new(move |_| {
        if tab.get() == "lines" && doc.get().is_some() && !lines_resize_initialized.get_value() {
            lines_resize_initialized.set_value(true);
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(100).await;
                init_column_resize(LINES_TABLE_ID, LINES_COLUMN_WIDTHS_KEY);
            });
        }
    });

    Effect::new(move |_| {
        if tab.get() == "linked_orders"
            && doc.get().is_some()
            && !linked_orders_resize_initialized.get_value()
        {
            linked_orders_resize_initialized.set_value(true);
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(100).await;
                init_column_resize(LINKED_ORDERS_TABLE_ID, LINKED_ORDERS_COLUMN_WIDTHS_KEY);
            });
        }
    });

    let journal_id = Signal::derive(move || journal.get().first().map(|row| row.id.clone()));

    let run_post = {
        let stored_id = stored_id;
        let load_doc = load_doc.clone();
        Callback::new(move |mode: &'static str| {
            let current_id = stored_id.get_value();
            let load_doc = load_doc.clone();
            spawn_local(async move {
                set_posting.set(true);
                match Request::post(&format!(
                    "{}/api/a026/wb-advert-daily/{}/{}",
                    api_base(),
                    current_id,
                    mode
                ))
                .send()
                .await
                {
                    Ok(resp) if resp.ok() => {
                        set_journal_loaded.set(false);
                        set_journal_loading.set(false);
                        set_journal_error.set(None);
                        set_journal.set(Vec::new());
                        set_projections_loaded.set(false);
                        set_projections.set(Vec::new());
                        set_projections_p911.set(Vec::new());
                        load_doc.run(());
                    }
                    Ok(resp) => {
                        set_error.set(Some(format!("Ошибка сервера: HTTP {}", resp.status())))
                    }
                    Err(err) => set_error.set(Some(format!("Ошибка сети: {}", err))),
                }
                set_posting.set(false);
            });
        })
    };

    let post_click = {
        let run_post = run_post.clone();
        move |_| run_post.run("post")
    };
    let unpost_click = {
        let run_post = run_post.clone();
        move |_| run_post.run("unpost")
    };

    let open_journal = Callback::new({
        let tabs = tabs.clone();
        move |journal_id: String| {
            tabs.open_tab(
                &format!("general_ledger_details_{}", journal_id),
                &format!("Главная книга {}", &journal_id[..journal_id.len().min(8)]),
            );
        }
    });

    let open_order = Callback::new({
        let tabs = tabs.clone();
        move |order_id: String| {
            tabs.open_tab(&format!("a015_wb_orders_details_{}", order_id), "WB Order");
        }
    });

    let open_nomenclature = Callback::new({
        let tabs = tabs.clone();
        move |nom_ref: String| {
            tabs.open_tab(
                &format!("a004_nomenclature_details_{}", nom_ref),
                "Номенклатура",
            );
        }
    });

    let open_product = Callback::new({
        let tabs = tabs.clone();
        move |product_ref: String| {
            tabs.open_tab(
                &format!("a007_marketplace_product_details_{}", product_ref),
                "Товар маркетплейса",
            );
        }
    });

    let toggle_lines_sort = Callback::new(move |field: &'static str| {
        if lines_sort_field.get_untracked() == field {
            set_lines_sort_ascending.update(|value| *value = !*value);
        } else {
            set_lines_sort_field.set(field.to_string());
            set_lines_sort_ascending.set(true);
        }
    });

    let sorted_lines = Signal::derive(move || {
        let mut lines = doc.get().map(|item| item.lines).unwrap_or_default();
        let current_field = lines_sort_field.get();
        sort_list(&mut lines, &current_field, lines_sort_ascending.get());
        lines
    });

    let favorite_target_id = stored_id.get_value();
    let favorite_tab_key = format!("a026_wb_advert_daily_details_{}", stored_id.get_value());
    let favorite_title = Signal::derive(move || {
        doc.get()
            .map(|d| {
                if d.advert_id > 0 {
                    format!("WB Ads {} · {}", d.document_date, d.advert_id)
                } else {
                    format!("WB Ads {}", d.document_date)
                }
            })
            .unwrap_or_else(|| "WB Ads".to_string())
    });

    view! {
        <PageFrame page_id="a026_wb_advert_daily--detail" category=PAGE_CAT_DETAIL class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    <FavoriteButton
                        target_kind="a026_wb_advert_daily_details".to_string()
                        target_id=favorite_target_id
                        target_title=favorite_title
                        tab_key=favorite_tab_key
                    />
                    <h1 class="page__title">
                        {move || doc.get().map(|d| {
                            if d.advert_id > 0 {
                                format!("WB Ads {} · {} от {}", d.document_no, d.advert_id, fmt_date(&d.document_date))
                            } else {
                                format!("WB Ads {} от {}", d.document_no, fmt_date(&d.document_date))
                            }
                        }).unwrap_or_else(|| "WB Ads".to_string())}
                    </h1>
                    <Show when=move || doc.get().is_some()>
                        {move || view! {
                            <Badge appearance=BadgeAppearance::Tint color=if doc.get().map(|d| d.is_posted).unwrap_or(false) { BadgeColor::Success } else { BadgeColor::Informative }>
                                {if doc.get().map(|d| d.is_posted).unwrap_or(false) { "Проведен" } else { "Не проведен" }}
                            </Badge>
                        }}
                    </Show>
                </div>
                <div class="page__header-right">
                    <Show when=move || doc.get().is_some()>
                        <Show when=move || !doc.get().map(|d| d.is_posted).unwrap_or(false)>
                            <Button appearance=ButtonAppearance::Primary on_click=post_click disabled=Signal::derive(move || posting.get())>
                                {move || if posting.get() { "Проведение..." } else { "Post" }}
                            </Button>
                        </Show>
                        <Show when=move || doc.get().map(|d| d.is_posted).unwrap_or(false)>
                            <Button appearance=ButtonAppearance::Secondary on_click=unpost_click disabled=Signal::derive(move || posting.get())>
                                {move || if posting.get() { "Отмена..." } else { "Unpost" }}
                            </Button>
                        </Show>
                    </Show>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_close.run(())>
                        "Закрыть"
                    </Button>
                </div>
            </div>

            <div class="page__tabs">
                <button class="page__tab" class:page__tab--active=move || tab.get() == "general" on:click=move |_| set_tab.set("general".to_string())>
                    "Общие"
                </button>
                <button class="page__tab" class:page__tab--active=move || tab.get() == "lines" on:click=move |_| set_tab.set("lines".to_string())>
                    "Позиции"
                </button>
                <button class="page__tab" class:page__tab--active=move || tab.get() == "linked_orders" on:click=move |_| set_tab.set("linked_orders".to_string())>
                    "Атрибуция"
                </button>
                <button class="page__tab" class:page__tab--active=move || tab.get() == "journal" on:click=move |_| set_tab.set("journal".to_string())>
                    "Журнал"
                </button>
                <button class="page__tab" class:page__tab--active=move || tab.get() == "projections" on:click=move |_| set_tab.set("projections".to_string())>
                    "Проекции"
                </button>
            </div>

            <div class="page__content">
                {move || if loading.get() {
                    view! {
                        <Flex gap=FlexGap::Small style="align-items:center;justify-content:center;padding:var(--spacing-4xl);">
                            <Spinner />
                            <span>"Загрузка..."</span>
                        </Flex>
                    }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <div class="alert alert--error">{err}</div> }.into_any()
                } else if let Some(d) = doc.get() {
                    match tab.get().as_str() {
                        "general" => {
                            let open_journal_general = open_journal.clone();
                            view! {
                                <div class="detail-grid">
                                <div class="detail-grid__col">
                                    <CardAnimated delay_ms=0 nav_id="a026_wb_advert_daily_details_general_document">
                                        <h4 class="details-section__title">"Документ"</h4>
                                        <div style="display:grid;grid-template-columns:1fr 1fr;gap:var(--spacing-sm);">
                                            <ReadField label="Номер" value=d.document_no.clone() />
                                            <ReadField label="Дата" value=fmt_date(&d.document_date) />
                                        </div>
                                        <div style="display:grid;grid-template-columns:1fr 1fr;gap:var(--spacing-sm);">
                                            <ReadField label="Кампания (advert_id)" value=fmt_advert_id(d.advert_id) />
                                            <ReadField label="Статус" value=if d.is_posted { "Проведен".to_string() } else { "Не проведен".to_string() } />
                                        </div>
                                        <ReadField label="ID" value=d.id.clone() />
                                    </CardAnimated>

                                    <CardAnimated delay_ms=80 nav_id="a026_wb_advert_daily_details_general_links">
                                        <h4 class="details-section__title">"Связи"</h4>
                                        <ReadField label="Кабинет" value=d.connection_name.clone().unwrap_or(d.connection_id.clone()) />
                                        <ReadField label="Организация" value=d.organization_name.clone().unwrap_or(d.organization_id.clone()) />
                                        <ReadField label="Маркетплейс" value=d.marketplace_name.clone().unwrap_or(d.marketplace_id.clone()) />
                                    </CardAnimated>

                                </div>
                                <div class="detail-grid__col">
                                    <CardAnimated delay_ms=40 nav_id="a026_wb_advert_daily_details_general_metrics">
                                        <h4 class="details-section__title">"Метрики"</h4>
                                        <div style="display:grid;grid-template-columns:1fr 1fr;gap:var(--spacing-sm);">
                                            <ReadField label="Итоговый расход" value=fmt_money(d.totals.sum) />
                                            <ReadField label="Не распределено" value=fmt_money(d.unattributed_totals.sum) />
                                        </div>
                                        <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:var(--spacing-sm);">
                                            <ReadField label="Просмотры" value=d.totals.views.to_string() />
                                            <ReadField label="Клики" value=d.totals.clicks.to_string() />
                                            <ReadField label="Заказы" value=d.totals.orders.to_string() />
                                        </div>
                                    </CardAnimated>

                                    <CardAnimated delay_ms=120 nav_id="a026_wb_advert_daily_details_general_technical">
                                        <h4 class="details-section__title">"Технические данные"</h4>
                                        <ReadField label="Источник" value=d.source.clone() />
                                        <ReadField label="Загружено" value=fmt_dt(&d.fetched_at) />
                                        <div style="display:grid;grid-template-columns:1fr 1fr;gap:var(--spacing-sm);">
                                            <ReadField label="Создано" value=fmt_dt(&d.created_at) />
                                            <ReadField label="Обновлено" value=fmt_dt(&d.updated_at) />
                                        </div>

                                    </CardAnimated>
                                </div>
                                </div>
                            }.into_any()
                        },
                        "lines" => {
                            let total_lines = d.lines.len();
                            let without_nomenclature =
                                d.lines.iter().filter(|line| line.nomenclature_ref.is_none()).count();

                            let export_filename = format!(
                                "wb_advert_daily_positions_{}_{}.csv",
                                d.document_date,
                                d.document_no
                            );
                            let export_lines = move || {
                                let lines = sorted_lines.get_untracked();
                                if let Err(err) = export_to_excel(&lines, &export_filename) {
                                    set_error.set(Some(format!("CSV: {}", err)));
                                }
                            };

                            view! {
                                <CardAnimated delay_ms=0 nav_id="a026_wb_advert_daily_details_lines_table">
                                    <div style="display:flex;gap:12px;flex-wrap:wrap;align-items:center;justify-content:space-between;">
                                        <div style="display:flex;gap:12px;flex-wrap:wrap;align-items:center;">
                                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                                            {format!("Позиции: {}", total_lines)}
                                        </Badge>
                                        <Badge
                                            appearance=BadgeAppearance::Tint
                                            color={
                                                if without_nomenclature > 0 {
                                                    BadgeColor::Danger
                                                } else {
                                                    BadgeColor::Success
                                                }
                                            }
                                        >
                                            {if without_nomenclature > 0 {
                                                format!("Без номенклатуры: {}", without_nomenclature)
                                            } else {
                                                "Все позиции сопоставлены".to_string()
                                            }}
                                        </Badge>
                                        </div>
                                        <Button appearance=ButtonAppearance::Secondary on_click=move |_| export_lines()>
                                            {icon("download")}
                                            "Excel (csv)"
                                        </Button>
                                    </div>

                                    <div class="table-wrapper">
                                        <TableCrosshairHighlight table_id=LINES_TABLE_ID.to_string() />
                                        <Table attr:id=LINES_TABLE_ID attr:style="width:100%;min-width:1500px;">
                                            <TableHeader>
                                                <TableRow>
                                                    <SortHeaderCell label="nmID" field="nm_id" min_width=90.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort />
                                                    <SortHeaderCell label="WB наименование" field="wb_name" min_width=240.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort />
                                                    <SortHeaderCell label="Артикул 1С" field="nomenclature_article" min_width=140.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort />
                                                    <SortHeaderCell label="Просмотры" field="views" min_width=100.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort numeric=true />
                                                    <SortHeaderCell label="Клики" field="clicks" min_width=90.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort numeric=true />
                                                    <SortHeaderCell label="CTR, %" field="ctr" min_width=90.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort numeric=true />
                                                    <SortHeaderCell label="CPC" field="cpc" min_width=90.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort numeric=true />
                                                    <SortHeaderCell label="В корзину" field="atbs" min_width=110.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort numeric=true />
                                                    <SortHeaderCell label="Заказы" field="orders" min_width=90.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort numeric=true />
                                                    <SortHeaderCell label="Штуки" field="shks" min_width=110.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort numeric=true />
                                                    <SortHeaderCell label="Расход" field="sum" min_width=110.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort numeric=true />
                                                    <SortHeaderCell label="Выручка" field="sum_price" min_width=120.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort numeric=true />
                                                    <SortHeaderCell label="CR, %" field="cr" min_width=90.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort numeric=true />
                                                    <SortHeaderCell label="Отмены" field="canceled" min_width=90.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort numeric=true />
                                                </TableRow>
                                            </TableHeader>
                                            <TableBody>
                                                <For
                                                    each=move || sorted_lines.get()
                                                    key=|line| format!("{}:{}", line.nm_id, line.nomenclature_ref.clone().unwrap_or_default())
                                                    children=move |line| {
                                                        let article = line.nomenclature_article.clone().unwrap_or_else(|| "—".to_string());

                                                        view! {
                                                            <TableRow>
                                                                <TableCell><TableCellLayout>{line.nm_id}</TableCellLayout></TableCell>
                                                                <TableCell><TableCellLayout truncate=true>{line.wb_name}</TableCellLayout></TableCell>
                                                                <TableCell><TableCellLayout truncate=true>{article}</TableCellLayout></TableCell>
                                                                <TableCell class="text-right"><TableCellLayout>{line.metrics.views}</TableCellLayout></TableCell>
                                                                <TableCell class="text-right"><TableCellLayout>{line.metrics.clicks}</TableCellLayout></TableCell>
                                                                <TableCell class="text-right"><TableCellLayout>{fmt_ratio(line.metrics.ctr)}</TableCellLayout></TableCell>
                                                                <TableCell class="text-right"><TableCellLayout>{fmt_money(line.metrics.cpc)}</TableCellLayout></TableCell>
                                                                <TableCell class="text-right"><TableCellLayout>{line.metrics.atbs}</TableCellLayout></TableCell>
                                                                <TableCell class="text-right"><TableCellLayout>{line.metrics.orders}</TableCellLayout></TableCell>
                                                                <TableCell class="text-right"><TableCellLayout>{line.metrics.shks}</TableCellLayout></TableCell>
                                                                <TableCell class="text-right"><TableCellLayout>{fmt_money(line.metrics.sum)}</TableCellLayout></TableCell>
                                                                <TableCell class="text-right"><TableCellLayout>{fmt_money(line.metrics.sum_price)}</TableCellLayout></TableCell>
                                                                <TableCell class="text-right"><TableCellLayout>{fmt_ratio(line.metrics.cr)}</TableCellLayout></TableCell>
                                                                <TableCell class="text-right"><TableCellLayout>{line.metrics.canceled}</TableCellLayout></TableCell>
                                                            </TableRow>
                                                        }
                                                    }
                                                />
                                            </TableBody>
                                        </Table>
                                    </div>
                                </CardAnimated>
                            }.into_any()
                        },
                        "linked_orders" => {
                            let has_linked = d.has_linked_orders;
                            let count = d.linked_orders_count;
                            let wb_orders_total: i64 = d.linked_orders.iter().map(|g| g.wb_reported_orders).sum();
                            let groups: Vec<_> = d
                                .linked_orders
                                .iter()
                                .filter(|group| should_show_linked_group(group))
                                .cloned()
                                .collect();
                            let total_expense = d.totals.sum;
                            let posted = d.is_posted;
                            let open_order_for_table = open_order.clone();
                            let open_nom_for_table = open_nomenclature.clone();

                            view! {
                                <div style="display:flex;flex-direction:column;gap:var(--spacing-md);width:100%;">
                                    <CardAnimated delay_ms=0 nav_id="a026_wb_advert_daily_details_linked_orders_summary">
                                        <h4 class="details-section__title">"Сводка"</h4>
                                        <div style="display:flex;gap:12px;flex-wrap:wrap;align-items:center;">
                                            <Badge
                                                appearance=BadgeAppearance::Tint
                                                color={ if has_linked { BadgeColor::Success } else if posted { BadgeColor::Warning } else { BadgeColor::Informative } }
                                            >
                                                { if !posted {
                                                    "Документ не проведён".to_string()
                                                } else if has_linked {
                                                    "Найдены связанные заказы".to_string()
                                                } else {
                                                    "Связанные заказы не найдены".to_string()
                                                } }
                                            </Badge>
                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                                                {format!("Найдено: {}", count)}
                                            </Badge>
                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                                                {format!("По данным WB: {}", wb_orders_total)}
                                            </Badge>
                                        </div>
                                        <Show when=move || !posted>
                                            <div class="form__hint">
                                                "Поиск связанных заказов выполняется при проведении документа. Проведите документ, чтобы увидеть результат."
                                            </div>
                                        </Show>
                                        <Show when=move || posted && !has_linked>
                                            <div class="form__hint">
                                                "По строкам отчёта WB с заказами (orders > 0) нет данных для отображения."
                                            </div>
                                        </Show>
                                        <Show when=move || posted && has_linked && count < wb_orders_total>
                                            <div class="form__hint">
                                                "Часть заказов WB не сопоставлена с a015 за дату документа — см. позиции со статусом «Нет в a015»."
                                            </div>
                                        </Show>
                                    </CardAnimated>
                                    <CardAnimated delay_ms=40 nav_id="a026_wb_advert_daily_details_linked_orders_table">
                                        <div style="display:flex;align-items:center;justify-content:space-between;gap:var(--spacing-sm);flex-wrap:wrap;margin-bottom:var(--spacing-xs);">
                                            <h4 class="details-section__title" style="margin:0;">"Найденные заказы по позициям"</h4>
                                            <div style="display:flex;gap:var(--spacing-xs);align-items:center;">
                                                <Button
                                                    size=ButtonSize::Small
                                                    appearance=Signal::derive(move || {
                                                        if linked_orders_tree_expanded.get() {
                                                            ButtonAppearance::Primary
                                                        } else {
                                                            ButtonAppearance::Subtle
                                                        }
                                                    })
                                                    attr:title="Развернуть все уровни"
                                                    on_click=move |_| set_linked_orders_tree_expanded.set(true)
                                                >
                                                    {icon("chevron-down")}
                                                </Button>
                                                <Button
                                                    size=ButtonSize::Small
                                                    appearance=Signal::derive(move || {
                                                        if linked_orders_tree_expanded.get() {
                                                            ButtonAppearance::Subtle
                                                        } else {
                                                            ButtonAppearance::Primary
                                                        }
                                                    })
                                                    attr:title="Только 1 уровень"
                                                    on_click=move |_| set_linked_orders_tree_expanded.set(false)
                                                >
                                                    {icon("chevron-right")}
                                                </Button>
                                            </div>
                                        </div>
                                        {
                                            if groups.is_empty() {
                                                view! {
                                                    <div class="text-muted">"Нет данных для отображения."</div>
                                                }.into_any()
                                            } else {
                                                let groups_for_each = groups.clone();
                                                view! {
                                                    <div class="table-wrapper" style="overflow-x: auto;">
                                                        <Table attr:id=LINKED_ORDERS_TABLE_ID attr:style="width:100%;min-width:870px;table-layout:fixed;">
                                                            <TableHeader>
                                                                <TableRow>
                                                                    <TableHeaderCell resizable=false min_width=60.0 class="resizable" attr:style="width:250px;">"Наименование"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=false min_width=60.0 class="resizable" attr:style="width:160px;">"Артикул / Заказ"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=false min_width=50.0 class="resizable text-right" attr:style="width:65px;">"WB"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=false min_width=50.0 class="resizable text-right" attr:style="width:75px;">"Найдено"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=false min_width=50.0 class="resizable" attr:style="width:85px;">"Статус"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=false min_width=50.0 class="resizable text-right" attr:style="width:80px;">"Цена"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=false min_width=50.0 class="resizable text-right" attr:style="width:90px;">"Расход"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=false min_width=50.0 class="resizable text-right" attr:style="width:65px;">"Доля, %"</TableHeaderCell>
                                                                </TableRow>
                                                            </TableHeader>
                                                            <TableBody>
                                                                <For
                                                                    each=move || groups_for_each.clone()
                                                                    key=|group| group.nm_id
                                                                    children=move |group| {
                                                                        let header_name = group.nm_name.clone();
                                                                        let wb_reported = group.wb_reported_orders;
                                                                        let wb_advert_sum = group.wb_advert_sum;
                                                                        let found_count = group.found_orders.len() as i64;
                                                                        let allocated_count = group.found_orders.iter().filter(|o| o.is_allocated).count() as i64;
                                                                        let extra_count = found_count - allocated_count;
                                                                        let missing_a015 = wb_reported > 0 && found_count == 0;
                                                                        let header_summary = if missing_a015 {
                                                                            format!("0 / {wb_reported}")
                                                                        } else if extra_count > 0 {
                                                                            format!("{} (+{})", allocated_count, extra_count)
                                                                        } else {
                                                                            allocated_count.to_string()
                                                                        };
                                                                        let orders_for_each: Vec<_> = group
                                                                            .found_orders
                                                                            .iter()
                                                                            .filter(|order| should_show_linked_order(order))
                                                                            .cloned()
                                                                            .collect();
                                                                        let orders_stored = StoredValue::new(orders_for_each);
                                                                        let group_share =
                                                                            fmt_expense_share(wb_advert_sum, total_expense);
                                                                        let open_order_inner = open_order_for_table.clone();
                                                                        let open_nom_inner = open_nom_for_table.clone();
                                                                        let nom_ref_val = group.nomenclature_ref.clone().unwrap_or_default();
                                                                        let article_text = group.nomenclature_article.clone().unwrap_or_else(|| "—".to_string());

                                                                        view! {
                                                                            // ── nm-group header row ──────────────────────────────────────────
                                                                            <TableRow>
                                                                                <TableCell><TableCellLayout truncate=true><strong>{header_name}</strong></TableCellLayout></TableCell>
                                                                                <TableCell><TableCellLayout truncate=true>
                                                                                    <strong>
                                                                                    {if nom_ref_val.is_empty() {
                                                                                        view! { <span>{article_text}</span> }.into_any()
                                                                                    } else {
                                                                                        let nom_ref_click = nom_ref_val.clone();
                                                                                        view! {
                                                                                            <a href="#" class="table__link" on:click=move |e| {
                                                                                                e.prevent_default();
                                                                                                open_nom_inner.run(nom_ref_click.clone());
                                                                                            }>{article_text}</a>
                                                                                        }.into_any()
                                                                                    }}
                                                                                    </strong>
                                                                                </TableCellLayout></TableCell>
                                                                                <TableCell class="text-right"><TableCellLayout>{wb_reported}</TableCellLayout></TableCell>
                                                                                <TableCell class="text-right"><TableCellLayout>{header_summary}</TableCellLayout></TableCell>
                                                                                <TableCell>
                                                                                    <TableCellLayout truncate=true>
                                                                                        {if missing_a015 {
                                                                                            view! {
                                                                                                <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Warning>
                                                                                                    "Нет в a015"
                                                                                                </Badge>
                                                                                            }.into_any()
                                                                                        } else {
                                                                                            view! { <span>""</span> }.into_any()
                                                                                        }}
                                                                                    </TableCellLayout>
                                                                                </TableCell>
                                                                                <TableCell class="text-right"><TableCellLayout>"—"</TableCellLayout></TableCell>
                                                                                <TableCell class="text-right"><TableCellLayout><strong>{fmt_money(wb_advert_sum)}</strong></TableCellLayout></TableCell>
                                                                                <TableCell class="text-right"><TableCellLayout><strong>{group_share}</strong></TableCellLayout></TableCell>
                                                                            </TableRow>
                                                                            // ── per-order rows ───────────────────────────────────────────────
                                                                            <Show when=move || linked_orders_tree_expanded.get()>
                                                                            <Show when=move || missing_a015>
                                                                                <TableRow>
                                                                                    <TableCell><TableCellLayout truncate=true>
                                                                                        <span class="table__tree-child-marker" aria-hidden="true">"└─ "</span>
                                                                                        "Нет проведённых заказов a015"
                                                                                    </TableCellLayout></TableCell>
                                                                                    <TableCell><TableCellLayout truncate=true>
                                                                                        <span class="text-muted">"за дату документа"</span>
                                                                                    </TableCellLayout></TableCell>
                                                                                    <TableCell class="text-right"><TableCellLayout>""</TableCellLayout></TableCell>
                                                                                    <TableCell class="text-right"><TableCellLayout>""</TableCellLayout></TableCell>
                                                                                    <TableCell><TableCellLayout truncate=true>""</TableCellLayout></TableCell>
                                                                                    <TableCell class="text-right"><TableCellLayout>"—"</TableCellLayout></TableCell>
                                                                                    <TableCell class="text-right"><TableCellLayout>"—"</TableCellLayout></TableCell>
                                                                                    <TableCell class="text-right"><TableCellLayout>"—"</TableCellLayout></TableCell>
                                                                                </TableRow>
                                                                            </Show>
                                                                            <For
                                                                                each=move || orders_stored.get_value()
                                                                                key=|order| order.order_key.clone()
                                                                                children=move |order| {
                                                                                    let price = order.finished_price.map(fmt_money).unwrap_or_else(|| "—".to_string());
                                                                                    let price_basis = if order.allocation_basis.abs() > f64::EPSILON {
                                                                                        fmt_money(order.allocation_basis)
                                                                                    } else {
                                                                                        price
                                                                                    };
                                                                                    let allocated = if order.is_allocated {
                                                                                        fmt_money(order.allocated_cost)
                                                                                    } else {
                                                                                        "—".to_string()
                                                                                    };
                                                                                    let ratio_str = if order.is_allocated {
                                                                                        fmt_expense_share(
                                                                                            order.allocated_cost,
                                                                                            total_expense,
                                                                                        )
                                                                                    } else {
                                                                                        "—".to_string()
                                                                                    };
                                                                                    let (status_color, status_label) = if order.is_cancel {
                                                                                        (BadgeColor::Danger, "Отменён")
                                                                                    } else if !order.is_allocated {
                                                                                        (BadgeColor::Warning, "Не в выборке")
                                                                                    } else {
                                                                                        (BadgeColor::Success, "Активен")
                                                                                    };
                                                                                    let order_date_display = order.order_date.as_deref().map(fmt_date).unwrap_or_else(|| order.order_key.clone());
                                                                                    let order_id_val = order.order_id.clone().unwrap_or_default();
                                                                                    let order_key_display = order.order_key.clone();
                                                                                    let open_order_click = open_order_inner.clone();
                                                                                    view! {
                                                                                        <TableRow>
                                                                                            // Наименование → "Заказ dd.mm.yyyy"
                                                                                            <TableCell><TableCellLayout truncate=true>
                                                                                                <span class="table__tree-child-marker" aria-hidden="true">"└─ "</span>
                                                                                                {format!("Заказ {}", order_date_display)}
                                                                                            </TableCellLayout></TableCell>
                                                                                            // Артикул/Заказ → srid as link to a015
                                                                                            <TableCell><TableCellLayout truncate=true>
                                                                                                {if order_id_val.is_empty() {
                                                                                                    view! { <span>{order_key_display}</span> }.into_any()
                                                                                                } else {
                                                                                                    view! {
                                                                                                        <a href="#" class="table__link" on:click=move |e| {
                                                                                                            e.prevent_default();
                                                                                                            open_order_click.run(order_id_val.clone());
                                                                                                        }>{order_key_display}</a>
                                                                                                    }.into_any()
                                                                                                }}
                                                                                            </TableCellLayout></TableCell>
                                                                                            <TableCell class="text-right"><TableCellLayout>""</TableCellLayout></TableCell>
                                                                                            <TableCell class="text-right"><TableCellLayout>""</TableCellLayout></TableCell>
                                                                                            <TableCell>
                                                                                                <TableCellLayout truncate=true>
                                                                                                    <Badge appearance=BadgeAppearance::Tint color=status_color>{status_label}</Badge>
                                                                                                </TableCellLayout>
                                                                                            </TableCell>
                                                                                            <TableCell class="text-right"><TableCellLayout>{price_basis}</TableCellLayout></TableCell>
                                                                                            <TableCell class="text-right"><TableCellLayout>{allocated}</TableCellLayout></TableCell>
                                                                                            <TableCell class="text-right"><TableCellLayout>{ratio_str}</TableCellLayout></TableCell>
                                                                                        </TableRow>
                                                                                    }
                                                                                }
                                                                            />
                                                                            </Show>
                                                                        }
                                                                    }
                                                                />
                                                            </TableBody>
                                                        </Table>
                                                    </div>
                                                }.into_any()
                                            }
                                        }
                                    </CardAnimated>
                                </div>
                            }.into_any()
                        },
                        "journal" => {
                            view! {
                                <DocumentGeneralLedgerEntries
                                    entries=Signal::derive(move || journal.get())
                                    loading=Signal::derive(move || journal_loading.get())
                                    error=Signal::derive(move || journal_error.get())
                                    nav_id=document_general_ledger_entries_nav_id("a026_wb_advert_daily")
                                    title="Журнал операций"
                                    empty_message="Записи General Ledger не найдены. Проведите документ для формирования проводок."
                                />
                            }.into_any()
                        },
                        "projections" => {
                            let rows = projections.get();
                            let reserve_total: f64 = rows.iter()
                                .filter(|r| r.turnover_code == "advert_clicks_order_accrual")
                                .map(|r| r.amount)
                                .sum();
                            let expense_total: f64 = rows.iter()
                                .filter(|r| r.turnover_code == "advert_clicks_order_expense")
                                .map(|r| r.amount)
                                .sum();
                            let problem_count = rows.iter().filter(|r| r.is_problem).count();
                            let row_count = rows.len();
                            let is_empty = rows.is_empty();

                            let rows_p911 = projections_p911.get();
                            let p911_total: f64 = rows_p911.iter().map(|r| r.amount).sum();
                            let p911_problem_count = rows_p911.iter().filter(|r| r.is_problem).count();
                            let p911_row_count = rows_p911.len();
                            let p911_is_empty = rows_p911.is_empty();
                            let open_nom_p911 = open_nomenclature.clone();
                            let open_product_p911 = open_product.clone();
                            view! {
                                <div style="display:flex;flex-direction:column;gap:var(--spacing-md);width:100%;">
                                    <CardAnimated delay_ms=0 nav_id="a026_wb_advert_daily_details_projections_summary">
                                        <h4 class="details-section__title">"p913 — Атрибуция рекламных расходов"</h4>
                                        <div style="display:flex;gap:12px;flex-wrap:wrap;align-items:center;">
                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                                                {format!("Строк: {}", row_count)}
                                            </Badge>
                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                                                {format!("Резерв: {}", fmt_money(reserve_total))}
                                            </Badge>
                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>
                                                {format!("Расход: {}", fmt_money(expense_total))}
                                            </Badge>
                                            {if problem_count > 0 {
                                                view! {
                                                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Danger>
                                                        {format!("Проблем: {}", problem_count)}
                                                    </Badge>
                                                }.into_any()
                                            } else {
                                                view! { <span></span> }.into_any()
                                            }}
                                        </div>
                                    </CardAnimated>
                                    <CardAnimated delay_ms=40 nav_id="a026_wb_advert_daily_details_projections_table">
                                        {if is_empty {
                                            view! {
                                                <div class="text-muted">"Нет записей p913. Проведите документ для создания записей атрибуции."</div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="table-wrapper">
                                                <Table attr:style="width:100%;min-width:800px;">
                                                    <TableHeader>
                                                        <TableRow>
                                                            <TableHeaderCell>"Дата"</TableHeaderCell>
                                                            <TableHeaderCell>"Тип"</TableHeaderCell>
                                                            <TableHeaderCell>"Заказ"</TableHeaderCell>
                                                            <TableHeaderCell>"Кампания"</TableHeaderCell>
                                                            <TableHeaderCell class="text-right">"Сумма"</TableHeaderCell>
                                                            <TableHeaderCell>"Статус"</TableHeaderCell>
                                                        </TableRow>
                                                    </TableHeader>
                                                    <TableBody>
                                                        <For each=move || projections.get() key=|r| r.id.clone() children=move |r| {
                                                            let (badge_color, badge_label) = if r.is_problem {
                                                                (BadgeColor::Danger, "Проблема")
                                                            } else {
                                                                (BadgeColor::Success, "OK")
                                                            };
                                                            let type_label = if r.turnover_code == "advert_clicks_order_accrual" {
                                                                "Резерв"
                                                            } else {
                                                                "Расход"
                                                            };
                                                            let order_key = if r.order_key.is_empty() { "—".to_string() } else { r.order_key.clone() };
                                                            view! {
                                                                <TableRow>
                                                                    <TableCell><TableCellLayout>{fmt_date(&r.entry_date)}</TableCellLayout></TableCell>
                                                                    <TableCell><TableCellLayout>{type_label}</TableCellLayout></TableCell>
                                                                    <TableCell><TableCellLayout truncate=true>{order_key}</TableCellLayout></TableCell>
                                                                    <TableCell><TableCellLayout truncate=true>{r.wb_advert_campaign_code.clone()}</TableCellLayout></TableCell>
                                                                    <TableCell class="text-right"><TableCellLayout>{fmt_money(r.amount)}</TableCellLayout></TableCell>
                                                                    <TableCell><TableCellLayout>
                                                                        <Badge appearance=BadgeAppearance::Tint color=badge_color>{badge_label}</Badge>
                                                                    </TableCellLayout></TableCell>
                                                                </TableRow>
                                                            }
                                                        } />
                                                    </TableBody>
                                                </Table>
                                                </div>
                                            }.into_any()
                                        }}
                                    </CardAnimated>

                                    <CardAnimated delay_ms=80 nav_id="a026_wb_advert_daily_details_projections_p911_summary">
                                        <h4 class="details-section__title">"p911 — Обороты рекламы по номенклатуре"</h4>
                                        <div style="display:flex;gap:12px;flex-wrap:wrap;align-items:center;">
                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                                                {format!("Строк: {}", p911_row_count)}
                                            </Badge>
                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>
                                                {format!("Оборот: {}", fmt_money(p911_total))}
                                            </Badge>
                                            {if p911_problem_count > 0 {
                                                view! {
                                                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Danger>
                                                        {format!("Без номенклатуры: {}", p911_problem_count)}
                                                    </Badge>
                                                }.into_any()
                                            } else {
                                                view! { <span></span> }.into_any()
                                            }}
                                        </div>
                                    </CardAnimated>
                                    <CardAnimated delay_ms=120 nav_id="a026_wb_advert_daily_details_projections_p911_table">
                                        {if p911_is_empty {
                                            view! {
                                                <div class="text-muted">"Нет записей p911. Обороты по номенклатуре формируются при проведении документа."</div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="table-wrapper">
                                                <Table attr:style="width:100%;min-width:760px;table-layout:fixed;">
                                                    <TableHeader>
                                                        <TableRow>
                                                            <TableHeaderCell attr:style="width:80px;">"Дата"</TableHeaderCell>
                                                            <TableHeaderCell>"Товар МП"</TableHeaderCell>
                                                            <TableHeaderCell>"Номенклатура"</TableHeaderCell>
                                                            <TableHeaderCell attr:style="width:110px;">"Кампания"</TableHeaderCell>
                                                            <TableHeaderCell class="text-right" attr:style="width:90px;">"Сумма"</TableHeaderCell>
                                                            <TableHeaderCell>"Статус"</TableHeaderCell>
                                                        </TableRow>
                                                    </TableHeader>
                                                    <TableBody>
                                                        <For each=move || projections_p911.get() key=|r| r.id.clone() children=move |r| {
                                                            let (badge_color, badge_label) = if r.is_problem {
                                                                (BadgeColor::Danger, "Без номенклатуры")
                                                            } else {
                                                                (BadgeColor::Success, "OK")
                                                            };
                                                            let nom_ref_val = r.nomenclature_ref.clone().unwrap_or_default();
                                                            let product_ref_val = r.marketplace_product_ref.clone().unwrap_or_default();
                                                            let open_nom_click = open_nom_p911.clone();
                                                            let open_product_click = open_product_p911.clone();
                                                            view! {
                                                                <TableRow>
                                                                    <TableCell><TableCellLayout>{fmt_date(&r.entry_date)}</TableCellLayout></TableCell>
                                                                    <TableCell><TableCellLayout truncate=true>
                                                                        {if product_ref_val.is_empty() {
                                                                            view! { <span class="text-muted">"—"</span> }.into_any()
                                                                        } else {
                                                                            let product_ref_click = product_ref_val.clone();
                                                                            view! {
                                                                                <a href="#" class="table__link" on:click=move |e| {
                                                                                    e.prevent_default();
                                                                                    open_product_click.run(product_ref_click.clone());
                                                                                }>{product_ref_val.clone()}</a>
                                                                            }.into_any()
                                                                        }}
                                                                    </TableCellLayout></TableCell>
                                                                    <TableCell><TableCellLayout truncate=true>
                                                                        {if nom_ref_val.is_empty() {
                                                                            view! { <span class="text-muted">"—"</span> }.into_any()
                                                                        } else {
                                                                            let nom_ref_click = nom_ref_val.clone();
                                                                            view! {
                                                                                <a href="#" class="table__link" on:click=move |e| {
                                                                                    e.prevent_default();
                                                                                    open_nom_click.run(nom_ref_click.clone());
                                                                                }>{nom_ref_val.clone()}</a>
                                                                            }.into_any()
                                                                        }}
                                                                    </TableCellLayout></TableCell>
                                                                    <TableCell><TableCellLayout truncate=true>{r.wb_advert_campaign_code.clone()}</TableCellLayout></TableCell>
                                                                    <TableCell class="text-right"><TableCellLayout>{fmt_money(r.amount)}</TableCellLayout></TableCell>
                                                                    <TableCell><TableCellLayout>
                                                                        <Badge appearance=BadgeAppearance::Tint color=badge_color>{badge_label}</Badge>
                                                                    </TableCellLayout></TableCell>
                                                                </TableRow>
                                                            }
                                                        } />
                                                    </TableBody>
                                                </Table>
                                                </div>
                                            }.into_any()
                                        }}
                                    </CardAnimated>
                                </div>
                            }.into_any()
                        },
                        _ => view! { <div class="text-muted">"Нет данных"</div> }.into_any(),
                    }
                } else {
                    view! { <div class="alert">"Документ не найден."</div> }.into_any()
                }}
            </div>
        </PageFrame>
    }
}
