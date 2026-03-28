use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use contracts::domain::a026_wb_advert_daily::aggregate::WbAdvertDailyMetrics;
use contracts::projections::general_ledger::GeneralLedgerEntryDto;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
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

#[derive(Debug, Clone, Deserialize)]
struct LineDto {
    nm_id: i64,
    wb_name: String,
    nomenclature_ref: Option<String>,
    nomenclature_article: Option<String>,
    nomenclature_name: Option<String>,
    metrics: WbAdvertDailyMetrics,
}

#[derive(Debug, Clone, Deserialize)]
struct DetailsDto {
    id: String,
    document_no: String,
    document_date: String,
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
                            tabs.update_tab_title(
                                &format!("a026_wb_advert_daily_details_{tab_id}"),
                                &format!("WB Ads {}", data.document_date),
                            );
                            set_doc.set(Some(data));
                        }
                        Err(err) => set_error.set(Some(format!("Ошибка парсинга: {}", err))),
                    },
                    Ok(resp) => set_error.set(Some(format!("Ошибка сервера: HTTP {}", resp.status()))),
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
                    Ok(resp) => set_error.set(Some(format!("Ошибка сервера: HTTP {}", resp.status()))),
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
                    Ok(resp) => set_error.set(Some(format!("Ошибка сервера: HTTP {}", resp.status()))),
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

    view! {
        <PageFrame page_id="a026_wb_advert_daily--detail" category=PAGE_CAT_DETAIL>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || {
                            doc.get()
                                .map(|d| format!("WB Ads {} от {}", d.document_no, fmt_date(&d.document_date)))
                                .unwrap_or_else(|| "WB Ads".to_string())
                        }}
                    </h1>
                    <Show when=move || doc.get().is_some()>
                        {move || {
                            view! {
                                <Badge
                                    appearance=BadgeAppearance::Tint
                                    color=if doc.get().map(|d| d.is_posted).unwrap_or(false) {
                                        BadgeColor::Success
                                    } else {
                                        BadgeColor::Informative
                                    }
                                >
                                    {if doc.get().map(|d| d.is_posted).unwrap_or(false) {
                                        "Проведен"
                                    } else {
                                        "Не проведен"
                                    }}
                                </Badge>
                            }
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
                        "lines" => view! {
                            <div class="table-wrapper">
                                <Table attr:style="width:100%;min-width:1000px;">
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell>"nmID"</TableHeaderCell>
                                            <TableHeaderCell>"Артикул"</TableHeaderCell>
                                            <TableHeaderCell>"Наименование"</TableHeaderCell>
                                            <TableHeaderCell>"Номенклатура"</TableHeaderCell>
                                            <TableHeaderCell>"Расход"</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        <For each=move || d.lines.clone() key=|l| l.nm_id children=move |l| {
                                            let nom = l.nomenclature_ref.clone();
                                            let article = l.nomenclature_article.clone().unwrap_or_else(|| "—".to_string());
                                            let name = l.nomenclature_name.clone().unwrap_or(l.wb_name.clone());
                                            let tabs = tabs.clone();
                                            view! {
                                                <TableRow>
                                                    <TableCell><TableCellLayout>{l.nm_id}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{article}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{name}</TableCellLayout></TableCell>
                                                    <TableCell>
                                                        <TableCellLayout>
                                                            {if let Some(nom_ref) = nom {
                                                                let r = nom_ref.clone();
                                                                view! {
                                                                    <a href="#" class="table__link" on:click=move |e| {
                                                                        e.prevent_default();
                                                                        tabs.open_tab(
                                                                            &format!("a004_nomenclature_details_{}", r.clone()),
                                                                            &format!("Номенклатура {}", r.clone()),
                                                                        );
                                                                    }>{nom_ref}</a>
                                                                }.into_any()
                                                            } else {
                                                                view! { <span>"—"</span> }.into_any()
                                                            }}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell class="table__cell--right"><TableCellLayout>{fmt_money(l.metrics.sum)}</TableCellLayout></TableCell>
                                                </TableRow>
                                            }
                                        } />
                                    </TableBody>
                                </Table>
                            </div>
                        }.into_any(),
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
