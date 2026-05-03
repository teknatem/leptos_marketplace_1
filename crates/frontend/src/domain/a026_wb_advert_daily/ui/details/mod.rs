use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::table::TableCrosshairHighlight;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, sort_list, Sortable};
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
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

fn fmt_int_list<T: ToString>(values: &[T]) -> String {
    if values.is_empty() {
        "—".to_string()
    } else {
        values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn fmt_advert_id(advert_id: i64) -> String {
    if advert_id > 0 {
        advert_id.to_string()
    } else {
        "—".to_string()
    }
}

fn fmt_placements(values: &[String]) -> String {
    if values.is_empty() {
        return "—".to_string();
    }

    values
        .iter()
        .map(|value| match value.as_str() {
            "search" => "Поиск".to_string(),
            "recommendations" => "Рекомендации".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
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

#[derive(Debug, Clone, Deserialize)]
struct LineDto {
    nm_id: i64,
    wb_name: String,
    nomenclature_ref: Option<String>,
    nomenclature_article: Option<String>,
    nomenclature_name: Option<String>,
    advert_ids: Vec<i64>,
    app_types: Vec<i32>,
    placements: Vec<String>,
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
            "nomenclature_name" => {
                cmp_optional_text(&self.nomenclature_name, &other.nomenclature_name)
            }
            "nomenclature_ref" => {
                cmp_optional_text(&self.nomenclature_ref, &other.nomenclature_ref)
            }
            "advert_ids" => cmp_text(
                &fmt_int_list(&self.advert_ids),
                &fmt_int_list(&other.advert_ids),
            ),
            "app_types" => cmp_text(
                &fmt_int_list(&self.app_types),
                &fmt_int_list(&other.app_types),
            ),
            "placements" => cmp_text(
                &fmt_placements(&self.placements),
                &fmt_placements(&other.placements),
            ),
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
            <div class="table__sortable-header" style=header_style on:click=move |_| on_toggle.run(field)>
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
    let (lines_sort_field, set_lines_sort_field) = signal("wb_name".to_string());
    let (lines_sort_ascending, set_lines_sort_ascending) = signal(true);

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
                match Request::get(&format!(
                    "{}/api/a026/wb-advert-daily/{}/journal",
                    api_base(),
                    current_id
                ))
                .send()
                .await
                {
                    Ok(resp) if resp.ok() => {
                        if let Ok(value) = resp.json::<serde_json::Value>().await {
                            let rows = value["general_ledger_entries"]
                                .as_array()
                                .map(|rows| {
                                    rows.iter()
                                        .filter_map(|row| {
                                            serde_json::from_value::<GeneralLedgerEntryDto>(
                                                row.clone(),
                                            )
                                            .ok()
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            set_journal.set(rows);
                            set_journal_loaded.set(true);
                        }
                    }
                    Ok(resp) => {
                        set_error.set(Some(format!("Ошибка сервера: HTTP {}", resp.status())))
                    }
                    Err(err) => set_error.set(Some(format!("Ошибка сети: {}", err))),
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
                        set_journal.set(Vec::new());
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

    let open_journal = {
        let tabs = tabs.clone();
        move |journal_id: String| {
            tabs.open_tab(
                &format!("general_ledger_details_{}", journal_id),
                &format!("Главная книга {}", &journal_id[..journal_id.len().min(8)]),
            );
        }
    };

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

    view! {
        <PageFrame page_id="a026_wb_advert_daily--detail" category=PAGE_CAT_DETAIL class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
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
                <button class="page__tab" class:page__tab--active=move || tab.get() == "journal" on:click=move |_| set_tab.set("journal".to_string())>
                    "Журнал"
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
                        "general" => view! {
                            <div class="detail-grid">
                                <div class="detail-grid__col">
                                    <ReadField label="ID" value=d.id.clone() />
                                    <ReadField label="Номер" value=d.document_no.clone() />
                                    <ReadField label="Дата" value=fmt_date(&d.document_date) />
                                    <ReadField label="Кампания (advert_id)" value=fmt_advert_id(d.advert_id) />
                                    <ReadField label="Кабинет" value=d.connection_name.clone().unwrap_or(d.connection_id.clone()) />
                                    <ReadField label="Организация" value=d.organization_name.clone().unwrap_or(d.organization_id.clone()) />
                                    <ReadField label="Маркетплейс" value=d.marketplace_name.clone().unwrap_or(d.marketplace_id.clone()) />
                                    <ReadField label="Источник" value=d.source.clone() />
                                    <ReadField label="Загружено" value=fmt_dt(&d.fetched_at) />
                                    <ReadField label="Создано" value=fmt_dt(&d.created_at) />
                                    <ReadField label="Обновлено" value=fmt_dt(&d.updated_at) />
                                    <Show when=move || journal_id.get().is_some()>
                                        {move || {
                                            let entry_id = journal_id.get().unwrap_or_default();
                                            view! {
                                                <div style="display:flex;gap:12px;flex-wrap:wrap;margin-top:12px;">
                                                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| open_journal(entry_id.clone())>
                                                        "Открыть проводку"
                                                    </Button>
                                                </div>
                                            }
                                        }}
                                    </Show>
                                </div>
                                <div class="detail-grid__col">
                                    <ReadField label="Итоговый расход" value=fmt_money(d.totals.sum) />
                                    <ReadField label="Не распределено" value=fmt_money(d.unattributed_totals.sum) />
                                    <ReadField label="Просмотры" value=d.totals.views.to_string() />
                                    <ReadField label="Клики" value=d.totals.clicks.to_string() />
                                    <ReadField label="Заказы" value=d.totals.orders.to_string() />
                                </div>
                            </div>
                        }.into_any(),
                        "lines" => {
                            let total_lines = d.lines.len();
                            let without_nomenclature =
                                d.lines.iter().filter(|line| line.nomenclature_ref.is_none()).count();

                            view! {
                                <div style="display:flex;flex-direction:column;gap:12px;">
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

                                    <div class="table-wrapper">
                                        <TableCrosshairHighlight table_id=LINES_TABLE_ID.to_string() />
                                        <Table attr:id=LINES_TABLE_ID attr:style="width:100%;min-width:2200px;">
                                            <TableHeader>
                                                <TableRow>
                                                    <SortHeaderCell label="nmID" field="nm_id" min_width=90.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort />
                                                    <SortHeaderCell label="WB наименование" field="wb_name" min_width=240.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort />
                                                    <SortHeaderCell label="Артикул 1С" field="nomenclature_article" min_width=140.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort />
                                                    <SortHeaderCell label="Номенклатура 1С" field="nomenclature_name" min_width=240.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort />
                                                    <SortHeaderCell label="UUID номенклатуры" field="nomenclature_ref" min_width=270.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort />
                                                    <SortHeaderCell label="Advert IDs" field="advert_ids" min_width=170.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort />
                                                    <SortHeaderCell label="App types" field="app_types" min_width=140.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort />
                                                    <SortHeaderCell label="Зоны показа" field="placements" min_width=170.0 sort_field=lines_sort_field sort_ascending=lines_sort_ascending on_toggle=toggle_lines_sort />
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
                                                        let tabs = tabs.clone();
                                                        let article = line.nomenclature_article.clone().unwrap_or_else(|| "—".to_string());
                                                        let nomenclature_name = line.nomenclature_name.clone().unwrap_or_else(|| "—".to_string());
                                                        let nomenclature_ref = line.nomenclature_ref.clone();
                                                        let advert_ids = fmt_int_list(&line.advert_ids);
                                                        let app_types = fmt_int_list(&line.app_types);
                                                        let placements = fmt_placements(&line.placements);

                                                        view! {
                                                            <TableRow>
                                                                <TableCell><TableCellLayout>{line.nm_id}</TableCellLayout></TableCell>
                                                                <TableCell><TableCellLayout truncate=true>{line.wb_name}</TableCellLayout></TableCell>
                                                                <TableCell><TableCellLayout truncate=true>{article}</TableCellLayout></TableCell>
                                                                <TableCell><TableCellLayout truncate=true>{nomenclature_name}</TableCellLayout></TableCell>
                                                                <TableCell>
                                                                    <TableCellLayout truncate=true>
                                                                        {if let Some(nom_ref) = nomenclature_ref {
                                                                            let nom_ref_for_nav = nom_ref.clone();
                                                                            view! {
                                                                                <a
                                                                                    href="#"
                                                                                    class="table__link"
                                                                                    on:click=move |e| {
                                                                                        e.prevent_default();
                                                                                        tabs.open_tab(
                                                                                            &format!("a004_nomenclature_details_{}", nom_ref_for_nav.clone()),
                                                                                            &format!("Номенклатура {}", nom_ref_for_nav.clone()),
                                                                                        );
                                                                                    }
                                                                                >
                                                                                    {nom_ref}
                                                                                </a>
                                                                            }.into_any()
                                                                        } else {
                                                                            view! { <span>"—"</span> }.into_any()
                                                                        }}
                                                                    </TableCellLayout>
                                                                </TableCell>
                                                                <TableCell><TableCellLayout truncate=true>{advert_ids}</TableCellLayout></TableCell>
                                                                <TableCell><TableCellLayout truncate=true>{app_types}</TableCellLayout></TableCell>
                                                                <TableCell><TableCellLayout truncate=true>{placements}</TableCellLayout></TableCell>
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
                                </div>
                            }.into_any()
                        },
                        "journal" => view! {
                            <div class="table-wrapper">
                                <Table attr:style="width:100%;min-width:950px;">
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell>"ID"</TableHeaderCell>
                                            <TableHeaderCell>"Дата"</TableHeaderCell>
                                            <TableHeaderCell>"Оборот"</TableHeaderCell>
                                            <TableHeaderCell>"Дт"</TableHeaderCell>
                                            <TableHeaderCell>"Кт"</TableHeaderCell>
                                            <TableHeaderCell>"Сумма"</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        <For each=move || journal.get() key=|r| r.id.clone() children=move |r| {
                                            let jid = r.id.clone();
                                            view! {
                                                <TableRow>
                                                    <TableCell>
                                                        <TableCellLayout>
                                                            <a href="#" class="table__link" on:click=move |e| { e.prevent_default(); open_journal(jid.clone()); }>{r.id}</a>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell><TableCellLayout>{r.entry_date}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{r.turnover_code}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{r.debit_account}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{r.credit_account}</TableCellLayout></TableCell>
                                                    <TableCell class="table__cell--right"><TableCellLayout>{fmt_money(r.amount)}</TableCellLayout></TableCell>
                                                </TableRow>
                                            }
                                        } />
                                    </TableBody>
                                </Table>
                            </div>
                        }.into_any(),
                        _ => view! { <div class="text-muted">"Нет данных"</div> }.into_any(),
                    }
                } else {
                    view! { <div class="alert">"Документ не найден."</div> }.into_any()
                }}
            </div>
        </PageFrame>
    }
}
