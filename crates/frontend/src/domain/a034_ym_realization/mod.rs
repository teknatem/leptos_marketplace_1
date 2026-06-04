//! Фронтенд агрегата a034_ym_realization (Отчёт о реализации YM, слой ybuh).
//! Лёгкие компоненты: список документов и страница деталей с журналом GL.

use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::table::format_money;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_LIST;
use contracts::domain::a034_ym_realization::aggregate::YmRealizationLine;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use thaw::*;

#[derive(Debug, Clone, Deserialize)]
struct ListItem {
    id: String,
    document_date: String,
    lines_count: i32,
    total_sales_revenue: f64,
    total_return_revenue: f64,
    net_revenue: f64,
    connection_name: Option<String>,
    connection_id: String,
    is_posted: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ListResponse {
    items: Vec<ListItem>,
    total: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct DetailsDto {
    document_no: String,
    document_date: String,
    connection_name: Option<String>,
    organization_name: Option<String>,
    total_sales_revenue: f64,
    total_return_revenue: f64,
    net_revenue: f64,
    source: String,
    fetched_at: String,
    is_posted: bool,
    lines: Vec<YmRealizationLine>,
}

#[derive(Debug, Clone, Deserialize)]
struct JournalEntryDto {
    entry_date: String,
    turnover_code: String,
    layer: String,
    debit_account: String,
    credit_account: String,
    amount: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct JournalResponse {
    general_ledger_entries: Vec<JournalEntryDto>,
}

#[component]
pub fn YmRealizationList() -> impl IntoView {
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let items = RwSignal::new(Vec::<ListItem>::new());
    let total = RwSignal::new(0usize);
    let loading = RwSignal::new(false);
    let error_msg = RwSignal::new(Option::<String>::None);

    let load = move || {
        spawn_local(async move {
            loading.set(true);
            error_msg.set(None);
            let url = format!("{}/api/a034/ym-realization/list?limit=200", api_base());
            match Request::get(&url).send().await {
                Ok(resp) if resp.ok() => match resp.json::<ListResponse>().await {
                    Ok(data) => {
                        total.set(data.total);
                        items.set(data.items);
                    }
                    Err(e) => error_msg.set(Some(format!("Ошибка разбора: {e}"))),
                },
                Ok(resp) => error_msg.set(Some(format!("HTTP {}", resp.status()))),
                Err(e) => error_msg.set(Some(format!("Ошибка запроса: {e}"))),
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| load());

    view! {
        <PageFrame page_id="a034_ym_realization--list" category=PAGE_CAT_LIST>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Реализация YM (Отчёт о реализации)"</h1>
                    <Badge appearance=BadgeAppearance::Filled>{move || total.get().to_string()}</Badge>
                </div>
                <div class="page__header-right">
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| load()>
                        "Обновить"
                    </Button>
                </div>
            </div>
            <div class="page__content">
                {move || error_msg.get().map(|err| view! { <div class="alert alert--error">{err}</div> })}
                {move || {
                    if loading.get() {
                        view! { <div class="page__placeholder"><Spinner /> " Загрузка..."</div> }.into_any()
                    } else if items.get().is_empty() {
                        view! { <div class="page__placeholder">"Нет документов. Импортируйте отчёт о реализации (u503)."</div> }.into_any()
                    } else {
                        let tabs_store = tabs_store.clone();
                        view! {
                            <div class="table-wrap" style="width:100%;">
                                <table class="table" style="width:100%;">
                                    <thead>
                                        <tr class="table__header-row">
                                            <th class="table__header-cell">"Дата"</th>
                                            <th class="table__header-cell">"Кабинет"</th>
                                            <th class="table__header-cell table__header-cell--right">"Строк"</th>
                                            <th class="table__header-cell table__header-cell--right">"Продажи"</th>
                                            <th class="table__header-cell table__header-cell--right">"Возвраты"</th>
                                            <th class="table__header-cell table__header-cell--right">"Нетто-выручка"</th>
                                            <th class="table__header-cell">"Проведён"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || {
                                            let tabs_store = tabs_store.clone();
                                            items.get().into_iter().map(move |item| {
                                                let tabs_store = tabs_store.clone();
                                                let id = item.id.clone();
                                                let date = item.document_date.clone();
                                                let open = move |_| {
                                                    tabs_store.open_tab(
                                                        &format!("a034_ym_realization_details_{}", id),
                                                        &format!("Реализация YM {}", date),
                                                    );
                                                };
                                                view! {
                                                    <tr class="table__row">
                                                        <td class="table__cell">
                                                            <a href="#" class="table__link" on:click=move |ev| { ev.prevent_default(); open(()); }>
                                                                {item.document_date.clone()}
                                                            </a>
                                                        </td>
                                                        <td class="table__cell">{item.connection_name.clone().unwrap_or(item.connection_id.clone())}</td>
                                                        <td class="table__cell table__cell--right">{item.lines_count}</td>
                                                        <td class="table__cell table__cell--right">{format_money(item.total_sales_revenue)}</td>
                                                        <td class="table__cell table__cell--right">{format_money(item.total_return_revenue)}</td>
                                                        <td class="table__cell table__cell--right">{format_money(item.net_revenue)}</td>
                                                        <td class="table__cell">{if item.is_posted { "✓" } else { "—" }}</td>
                                                    </tr>
                                                }
                                            }).collect_view()
                                        }}
                                    </tbody>
                                </table>
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}

#[component]
pub fn YmRealizationDetail(id: String) -> impl IntoView {
    let doc = RwSignal::new(Option::<DetailsDto>::None);
    let journal = RwSignal::new(Vec::<JournalEntryDto>::new());
    let error_msg = RwSignal::new(Option::<String>::None);
    let busy = RwSignal::new(false);

    let id_for_load = id.clone();
    let load = move || {
        let id = id_for_load.clone();
        spawn_local(async move {
            error_msg.set(None);
            let url = format!("{}/api/a034/ym-realization/{}", api_base(), id);
            match Request::get(&url).send().await {
                Ok(resp) if resp.ok() => match resp.json::<DetailsDto>().await {
                    Ok(data) => doc.set(Some(data)),
                    Err(e) => error_msg.set(Some(format!("Ошибка разбора: {e}"))),
                },
                Ok(resp) => error_msg.set(Some(format!("HTTP {}", resp.status()))),
                Err(e) => error_msg.set(Some(format!("Ошибка запроса: {e}"))),
            }
            let jurl = format!("{}/api/a034/ym-realization/{}/journal", api_base(), id);
            if let Ok(resp) = Request::get(&jurl).send().await {
                if resp.ok() {
                    if let Ok(jr) = resp.json::<JournalResponse>().await {
                        journal.set(jr.general_ledger_entries);
                    }
                }
            }
        });
    };

    {
        let load = load.clone();
        Effect::new(move |_| load());
    }

    let id_for_post = id.clone();
    let post = move |unpost: bool| {
        let id = id_for_post.clone();
        let load2 = load.clone();
        spawn_local(async move {
            busy.set(true);
            let action = if unpost { "unpost" } else { "post" };
            let url = format!("{}/api/a034/ym-realization/{}/{}", api_base(), id, action);
            let _ = Request::post(&url).send().await;
            busy.set(false);
            load2();
        });
    };
    let post_click = post.clone();
    let unpost_click = post;

    view! {
        <PageFrame page_id="a034_ym_realization--details" category=PAGE_CAT_LIST>
            {move || error_msg.get().map(|err| view! { <div class="alert alert--error">{err}</div> })}
            {move || {
                let Some(d) = doc.get() else {
                    return view! { <div class="page__placeholder"><Spinner /> " Загрузка..."</div> }.into_any();
                };
                let post_click = post_click.clone();
                let unpost_click = unpost_click.clone();
                view! {
                    <div class="page__header">
                        <div class="page__header-left">
                            <h1 class="page__title">{format!("Реализация YM {}", d.document_date)}</h1>
                        </div>
                        <div class="page__header-right">
                            <Button appearance=ButtonAppearance::Primary disabled=Signal::derive(move || busy.get())
                                on_click=move |_| post_click(false)>"Провести"</Button>
                            <Button appearance=ButtonAppearance::Secondary disabled=Signal::derive(move || busy.get())
                                on_click=move |_| unpost_click(true)>"Распровести"</Button>
                        </div>
                    </div>
                    <div class="page__content">
                        <div class="card" style="padding:12px; margin-bottom:12px;">
                            <div>{format!("Документ: {}", d.document_no)}</div>
                            <div>{format!("Кабинет: {}", d.connection_name.clone().unwrap_or_default())}</div>
                            <div>{format!("Организация: {}", d.organization_name.clone().unwrap_or_default())}</div>
                            <div>{format!("Источник: {} ({})", d.source, d.fetched_at)}</div>
                            <div>{format!("Проведён: {}", if d.is_posted { "да" } else { "нет" })}</div>
                            <div style="margin-top:6px;">
                                {format!("Продажи: {}  Возвраты: {}  Нетто: {}",
                                    format_money(d.total_sales_revenue),
                                    format_money(d.total_return_revenue),
                                    format_money(d.net_revenue))}
                            </div>
                        </div>

                        <h3>"GL-проводки (слой ybuh)"</h3>
                        <div class="table-wrap" style="width:100%; margin-bottom:16px;">
                            <table class="table" style="width:100%;">
                                <thead>
                                    <tr class="table__header-row">
                                        <th class="table__header-cell">"Дата"</th>
                                        <th class="table__header-cell">"Оборот"</th>
                                        <th class="table__header-cell">"Слой"</th>
                                        <th class="table__header-cell">"Дт"</th>
                                        <th class="table__header-cell">"Кт"</th>
                                        <th class="table__header-cell table__header-cell--right">"Сумма"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {move || journal.get().into_iter().map(|e| view! {
                                        <tr class="table__row">
                                            <td class="table__cell">{e.entry_date}</td>
                                            <td class="table__cell">{e.turnover_code}</td>
                                            <td class="table__cell">{e.layer}</td>
                                            <td class="table__cell">{e.debit_account}</td>
                                            <td class="table__cell">{e.credit_account}</td>
                                            <td class="table__cell table__cell--right">{format_money(e.amount)}</td>
                                        </tr>
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>

                        <h3>"Строки реализации"</h3>
                        <div class="table-wrap" style="width:100%;">
                            <table class="table" style="width:100%;">
                                <thead>
                                    <tr class="table__header-row">
                                        <th class="table__header-cell">"SKU"</th>
                                        <th class="table__header-cell">"Наименование"</th>
                                        <th class="table__header-cell table__header-cell--right">"Кол-во"</th>
                                        <th class="table__header-cell table__header-cell--right">"Выручка"</th>
                                        <th class="table__header-cell">"Возврат"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {move || d.lines.clone().into_iter().map(|l| view! {
                                        <tr class="table__row">
                                            <td class="table__cell">{l.shop_sku}</td>
                                            <td class="table__cell">{l.offer_name}</td>
                                            <td class="table__cell table__cell--right">{format!("{:.0}", l.quantity)}</td>
                                            <td class="table__cell table__cell--right">{format_money(l.revenue_amount)}</td>
                                            <td class="table__cell">{if l.is_return { "возврат" } else { "" }}</td>
                                        </tr>
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    </div>
                }.into_any()
            }}
        </PageFrame>
    }
}
