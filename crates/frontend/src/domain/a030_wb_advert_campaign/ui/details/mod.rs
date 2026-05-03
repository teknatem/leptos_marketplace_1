use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::components::table::TableCrosshairHighlight;
use crate::shared::icons::icon;
use crate::shared::json_viewer::widget::JsonViewer;
use crate::shared::page_frame::PageFrame;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use thaw::*;

// ── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct WbAdvertCampaignDetailsDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub advert_id: i64,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub campaign_type: Option<i32>,
    pub status: Option<i32>,
    pub change_time: Option<String>,
    pub fetched_at: String,
    pub info_json: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdvertDailyStatRow {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub lines_count: i32,
    pub total_views: i64,
    pub total_clicks: i64,
    pub total_orders: i64,
    pub total_sum: f64,
    pub total_sum_price: f64,
    pub is_posted: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NmPositionDto {
    pub nm_id: i64,
    pub article: Option<String>,
    pub name: Option<String>,
    pub nm_data: serde_json::Value,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

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
        Some(4) => "Каталог (4)",
        Some(5) => "Карточка товара (5)",
        Some(6) => "Поиск (6)",
        Some(7) => "Рекомендации (7)",
        Some(8) => "Поиск + Рекомендации (8)",
        Some(9) => "Авто (9)",
        _ => "—",
    }
}

fn campaign_status_label(s: Option<i32>) -> &'static str {
    match s {
        Some(4) => "Готова к запуску (4)",
        Some(7) => "Завершена (7)",
        Some(9) => "Идут показы (9)",
        Some(11) => "Пауза (11)",
        _ => "—",
    }
}

fn extract_placements(info: &serde_json::Value) -> Vec<(String, String)> {
    if let Some(placements) = info
        .get("settings")
        .and_then(|s| s.get("placements"))
        .and_then(|p| p.as_object())
    {
        return placements
            .iter()
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::Bool(b) => if *b { "Да" } else { "Нет" }.to_string(),
                    other => other.to_string(),
                };
                (k.clone(), val)
            })
            .collect();
    }
    Vec::new()
}

// ── Main component ────────────────────────────────────────────────────────────

#[component]
pub fn WbAdvertCampaignDetails(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs_ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    // Non-reactive storage for fetched objects
    let stored_data: StoredValue<Option<WbAdvertCampaignDetailsDto>> = StoredValue::new(None);
    let stored_nm: StoredValue<Vec<NmPositionDto>> = StoredValue::new(Vec::new());

    // Signals only for UI state (loading flags, active tab)
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (data_ready, set_data_ready) = signal(false);

    let (nm_loading, set_nm_loading) = signal(false);
    let (nm_ready, set_nm_ready) = signal(false);

    let (stats_loading, set_stats_loading) = signal(false);
    let (stats_ready, set_stats_ready) = signal(false);
    let stored_stats: StoredValue<Vec<AdvertDailyStatRow>> = StoredValue::new(Vec::new());

    let active_tab = RwSignal::new("general".to_string());

    let campaign_id = StoredValue::new(id.clone());

    // ── Load campaign ─────────────────────────────────────────────────────────
    let do_load = move || {
        let cid = campaign_id.get_value();
        let tabs_ctx = tabs_ctx.clone();
        spawn_local(async move {
            set_loading.set(true);
            set_data_ready.set(false);
            set_error.set(None);
            let url = format!("{}/api/a030/wb-advert-campaign/{}", api_base(), cid);
            match Request::get(&url).send().await {
                Ok(resp) if resp.ok() => match resp.json::<WbAdvertCampaignDetailsDto>().await {
                    Ok(data) => {
                        tabs_ctx.update_tab_title(
                            &format!("a030_wb_advert_campaign_details_{}", data.id),
                            &format!("WB Campaign {}", data.advert_id),
                        );
                        stored_data.set_value(Some(data));
                        set_data_ready.set(true);
                    }
                    Err(e) => set_error.set(Some(format!("Ошибка парсинга: {e}"))),
                },
                Ok(resp) => set_error.set(Some(format!("Ошибка сервера: HTTP {}", resp.status()))),
                Err(e) => set_error.set(Some(format!("Ошибка сети: {e}"))),
            }
            set_loading.set(false);
        });
    };

    // ── Load nm positions ─────────────────────────────────────────────────────
    let do_load_nm = move || {
        if nm_loading.get_untracked() || nm_ready.get_untracked() {
            return;
        }
        let cid = campaign_id.get_value();
        spawn_local(async move {
            set_nm_loading.set(true);
            let url = format!(
                "{}/api/a030/wb-advert-campaign/{}/nm-positions",
                api_base(),
                cid
            );
            match Request::get(&url).send().await {
                Ok(resp) if resp.ok() => match resp.json::<Vec<NmPositionDto>>().await {
                    Ok(data) => stored_nm.set_value(data),
                    Err(e) => leptos::logging::error!("nm-positions parse: {e}"),
                },
                Ok(resp) => leptos::logging::error!("nm-positions HTTP {}", resp.status()),
                Err(e) => leptos::logging::error!("nm-positions network: {e}"),
            }
            set_nm_ready.set(true);
            set_nm_loading.set(false);
        });
    };

    // ── Load advert daily stats ───────────────────────────────────────────────
    let do_load_stats = move || {
        if stats_loading.get_untracked() || stats_ready.get_untracked() {
            return;
        }
        let cid = campaign_id.get_value();
        spawn_local(async move {
            set_stats_loading.set(true);
            let url = format!(
                "{}/api/a030/wb-advert-campaign/{}/advert-stats",
                api_base(),
                cid
            );
            match Request::get(&url).send().await {
                Ok(resp) if resp.ok() => match resp.json::<Vec<AdvertDailyStatRow>>().await {
                    Ok(data) => stored_stats.set_value(data),
                    Err(e) => leptos::logging::error!("advert-stats parse: {e}"),
                },
                Ok(resp) => leptos::logging::error!("advert-stats HTTP {}", resp.status()),
                Err(e) => leptos::logging::error!("advert-stats network: {e}"),
            }
            set_stats_ready.set(true);
            set_stats_loading.set(false);
        });
    };

    // On mount — load campaign
    Effect::new(move |_| do_load());

    // When positions tab is first opened — load nm positions
    Effect::new(move |_| {
        if active_tab.get() == "positions" && !nm_ready.get() {
            do_load_nm();
        }
    });

    // When stats tab is first opened — load advert daily stats
    Effect::new(move |_| {
        if active_tab.get() == "stats" && !stats_ready.get() {
            do_load_stats();
        }
    });

    // Page title from stored data
    let page_title = move || {
        stored_data
            .get_value()
            .map(|d| format!("Кампания WB {}", d.advert_id))
            .unwrap_or_else(|| "Кампания WB".to_string())
    };

    let page_subtitle = move || {
        stored_data.get_value().and_then(|d| {
            if !d.description.is_empty() && !d.description.starts_with("WB advert campaign ") {
                Some(d.description)
            } else {
                None
            }
        })
    };

    view! {
        <PageFrame page_id="a030_wb_advert_campaign--detail" category="detail">

            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">{page_title}</h1>
                    {move || page_subtitle().map(|s| view! {
                        <span style="font-size:0.9em; color:var(--color-text-secondary); margin-left:8px;">{s}</span>
                    })}
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| do_load()
                        disabled=Signal::derive(move || loading.get())
                    >
                        "Обновить"
                    </Button>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_close.run(())>
                        "Закрыть"
                    </Button>
                </div>
            </div>

            <div class="page__tabs">
                <button
                    class="page__tab"
                    class:page__tab--active=move || active_tab.get() == "general"
                    on:click=move |_| active_tab.set("general".to_string())
                >
                    {icon("file-text")} " Основные данные"
                </button>
                <button
                    class="page__tab"
                    class:page__tab--active=move || active_tab.get() == "positions"
                    on:click=move |_| active_tab.set("positions".to_string())
                >
                    {icon("list")} " Позиции"
                </button>
                <button
                    class="page__tab"
                    class:page__tab--active=move || active_tab.get() == "stats"
                    on:click=move |_| active_tab.set("stats".to_string())
                >
                    {icon("bar-chart-2")} " Статистика"
                </button>
                <button
                    class="page__tab"
                    class:page__tab--active=move || active_tab.get() == "json"
                    on:click=move |_| active_tab.set("json".to_string())
                >
                    {icon("code")} " JSON"
                </button>
            </div>

            <div class="page__content">
                // Loading / error / content — rendered by these three separate signals
                <Show when=move || loading.get()>
                    <Flex gap=FlexGap::Small style="align-items:center; justify-content:center; padding:var(--spacing-4xl);">
                        <Spinner />
                        <span>"Загрузка..."</span>
                    </Flex>
                </Show>

                <Show when=move || !loading.get() && error.get().is_some()>
                    <div class="alert alert--error">
                        {move || error.get().unwrap_or_default()}
                    </div>
                </Show>

                // Tab content — only when data is ready; reactive only on active_tab changes
                <Show when=move || data_ready.get() && !loading.get()>
                    {move || {
                        // get_value() is NOT reactive — just reads stored data
                        let data = stored_data.get_value().unwrap();
                        match active_tab.get().as_str() {
                            "positions" => view! {
                                <PositionsTab
                                    nm_ready=nm_ready
                                    nm_loading=nm_loading
                                    stored_nm=stored_nm
                                />
                            }.into_any(),
                            "stats" => view! {
                                <StatsTab
                                    stats_ready=stats_ready
                                    stats_loading=stats_loading
                                    stored_stats=stored_stats
                                />
                            }.into_any(),
                            "json" => view! {
                                <JsonTabContent info_json=data.info_json />
                            }.into_any(),
                            _ => view! {
                                <GeneralTab data=data />
                            }.into_any(),
                        }
                    }}
                </Show>
            </div>
        </PageFrame>
    }
}

// ── General tab ───────────────────────────────────────────────────────────────

#[component]
fn ReadField(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="form__group" style="margin-bottom:10px;">
            <label class="form__label">{label}</label>
            <Input value=RwSignal::new(value) attr:readonly=true />
        </div>
    }
}

#[component]
fn GeneralTab(data: WbAdvertCampaignDetailsDto) -> impl IntoView {
    let placements = extract_placements(&data.info_json);
    let has_placements = !placements.is_empty();

    view! {
        <div class="detail-grid">
            <CardAnimated delay_ms=0>
                <h4 class="details-section__title">"Основные данные"</h4>
                <div class="form-grid">
                    <ReadField label="advertId" value=data.advert_id.to_string() />
                    <ReadField label="Код" value=data.code.clone() />
                    <ReadField label="Название" value=data.description.clone() />
                    <ReadField label="Тип кампании" value=campaign_type_label(data.campaign_type).to_string() />
                    <ReadField label="Статус" value=campaign_status_label(data.status).to_string() />
                    <ReadField label="changeTime WB"
                        value=data.change_time.as_deref().map(fmt_dt).unwrap_or_else(|| "—".to_string()) />
                    <ReadField label="Обновлено из WB" value=fmt_dt(&data.fetched_at) />
                    <ReadField label="Создано" value=fmt_dt(&data.created_at) />
                    <ReadField label="Изменено" value=fmt_dt(&data.updated_at) />
                </div>
            </CardAnimated>

            <CardAnimated delay_ms=60>
                <h4 class="details-section__title">"Связи"</h4>
                <div class="form-grid">
                    <ReadField label="connection_id" value=data.connection_id.clone() />
                    <ReadField label="organization_id" value=data.organization_id.clone() />
                    <ReadField label="marketplace_id" value=data.marketplace_id.clone() />
                </div>
            </CardAnimated>

            {if has_placements {
                view! {
                    <CardAnimated delay_ms=120>
                        <h4 class="details-section__title">"Площадки размещения"</h4>
                        <div class="form-grid">
                            {placements.into_iter().map(|(label, value)| {
                                let label_static: &'static str = Box::leak(label.into_boxed_str());
                                view! { <ReadField label=label_static value=value /> }
                            }).collect::<Vec<_>>()}
                        </div>
                    </CardAnimated>
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }}
        </div>
    }
}

// ── JSON tab ──────────────────────────────────────────────────────────────────

#[component]
fn JsonTabContent(info_json: serde_json::Value) -> impl IntoView {
    let info_pretty =
        serde_json::to_string_pretty(&info_json).unwrap_or_else(|_| info_json.to_string());
    view! {
        <div style="padding:var(--spacing-sm); max-height:calc(100vh - 200px); overflow:auto;">
            <JsonViewer json_content=info_pretty title="WB advert campaign info".to_string() />
        </div>
    }
}

// ── Stats tab ─────────────────────────────────────────────────────────────────

fn fmt_num(v: f64) -> String {
    if v == 0.0 {
        return "—".to_string();
    }
    let s = format!("{:.2}", v);
    // trim unnecessary trailing zeros after decimal point
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

#[component]
fn StatsTab(
    stats_ready: ReadSignal<bool>,
    stats_loading: ReadSignal<bool>,
    stored_stats: StoredValue<Vec<AdvertDailyStatRow>>,
) -> impl IntoView {
    view! {
        <div style="padding:var(--spacing-sm);">
            <Show when=move || stats_loading.get()>
                <Flex gap=FlexGap::Small style="align-items:center; padding:var(--spacing-4xl); justify-content:center;">
                    <Spinner />
                    <span>"Загрузка статистики..."</span>
                </Flex>
            </Show>

            <Show when=move || stats_ready.get() && !stats_loading.get()>
                {move || {
                    let items = stored_stats.get_value();
                    if items.is_empty() {
                        view! {
                            <div style="color:var(--color-text-secondary); padding:var(--spacing-lg);">
                                "Документы статистики рекламы (a026) по этой кампании не найдены."
                            </div>
                        }.into_any()
                    } else {
                        let total_views: i64 = items.iter().map(|r| r.total_views).sum();
                        let total_clicks: i64 = items.iter().map(|r| r.total_clicks).sum();
                        let total_orders: i64 = items.iter().map(|r| r.total_orders).sum();
                        let total_sum: f64 = items.iter().map(|r| r.total_sum).sum();
                        view! {
                            <div class="table-wrapper">
                                <TableCrosshairHighlight
                                    table_id="a030-advert-stats-table".to_string()
                                />
                                <Table
                                    attr:id="a030-advert-stats-table"
                                    attr:style="width:100%;"
                                >
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell min_width=100.0>"Дата"</TableHeaderCell>
                                            <TableHeaderCell min_width=80.0 attr:style="text-align:right;">"Показы"</TableHeaderCell>
                                            <TableHeaderCell min_width=80.0 attr:style="text-align:right;">"Клики"</TableHeaderCell>
                                            <TableHeaderCell min_width=80.0 attr:style="text-align:right;">"Заказы"</TableHeaderCell>
                                            <TableHeaderCell min_width=100.0 attr:style="text-align:right;">"Расход, ₽"</TableHeaderCell>
                                            <TableHeaderCell min_width=80.0 attr:style="text-align:right;">"Товаров"</TableHeaderCell>
                                            <TableHeaderCell min_width=70.0 attr:style="text-align:center;">"Провед."</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {items.into_iter().map(|row| {
                                            let posted_label = if row.is_posted { "✓" } else { "—" };
                                            let posted_color = if row.is_posted {
                                                "color:var(--colorSuccessForeground1);"
                                            } else {
                                                "color:var(--color-text-tertiary);"
                                            };
                                            view! {
                                                <TableRow>
                                                    <TableCell>
                                                        <span style="font-family:monospace; font-size:13px;">
                                                            {row.document_date}
                                                        </span>
                                                    </TableCell>
                                                    <TableCell attr:style="text-align:right;">
                                                        <span style="font-size:13px;">{row.total_views.to_string()}</span>
                                                    </TableCell>
                                                    <TableCell attr:style="text-align:right;">
                                                        <span style="font-size:13px;">{row.total_clicks.to_string()}</span>
                                                    </TableCell>
                                                    <TableCell attr:style="text-align:right;">
                                                        <span style="font-size:13px;">{row.total_orders.to_string()}</span>
                                                    </TableCell>
                                                    <TableCell attr:style="text-align:right;">
                                                        <span style="font-size:13px; font-weight:600;">{fmt_num(row.total_sum)}</span>
                                                    </TableCell>
                                                    <TableCell attr:style="text-align:right;">
                                                        <span style="font-size:13px; color:var(--color-text-secondary);">{row.lines_count.to_string()}</span>
                                                    </TableCell>
                                                    <TableCell attr:style="text-align:center;">
                                                        <span style=posted_color>{posted_label}</span>
                                                    </TableCell>
                                                </TableRow>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </TableBody>
                                    <tfoot>
                                        <TableRow>
                                            <TableCell><strong>"Итого"</strong></TableCell>
                                            <TableCell attr:style="text-align:right;"><strong>{total_views.to_string()}</strong></TableCell>
                                            <TableCell attr:style="text-align:right;"><strong>{total_clicks.to_string()}</strong></TableCell>
                                            <TableCell attr:style="text-align:right;"><strong>{total_orders.to_string()}</strong></TableCell>
                                            <TableCell attr:style="text-align:right;"><strong>{fmt_num(total_sum)}</strong></TableCell>
                                            <TableCell></TableCell>
                                            <TableCell></TableCell>
                                        </TableRow>
                                    </tfoot>
                                </Table>
                            </div>
                        }.into_any()
                    }
                }}
            </Show>
        </div>
    }
}

// ── Positions tab ─────────────────────────────────────────────────────────────

#[component]
fn PositionsTab(
    nm_ready: ReadSignal<bool>,
    nm_loading: ReadSignal<bool>,
    stored_nm: StoredValue<Vec<NmPositionDto>>,
) -> impl IntoView {
    view! {
        <div style="padding:var(--spacing-sm);">
            <Show when=move || nm_loading.get()>
                <Flex gap=FlexGap::Small style="align-items:center; padding:var(--spacing-4xl); justify-content:center;">
                    <Spinner />
                    <span>"Загрузка позиций..."</span>
                </Flex>
            </Show>

            <Show when=move || nm_ready.get() && !nm_loading.get()>
                {move || {
                    let items = stored_nm.get_value();
                    if items.is_empty() {
                        view! {
                            <div style="color:var(--color-text-secondary); padding:var(--spacing-lg);">
                                "Позиции nm_settings не найдены в данных кампании."
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="table-wrapper">
                                <TableCrosshairHighlight
                                    table_id="a030-nm-positions-table".to_string()
                                />
                                <Table
                                    attr:id="a030-nm-positions-table"
                                    attr:style="width:100%; min-width:700px;"
                                >
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell min_width=120.0>"nm_id"</TableHeaderCell>
                                            <TableHeaderCell min_width=160.0>"Артикул"</TableHeaderCell>
                                            <TableHeaderCell min_width=260.0>"Наименование"</TableHeaderCell>
                                            <TableHeaderCell min_width=200.0>"Данные позиции"</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {items.into_iter().map(|pos| {
                                            let nm_data_str = serde_json::to_string_pretty(&pos.nm_data)
                                                .unwrap_or_else(|_| pos.nm_data.to_string());
                                            view! {
                                                <TableRow>
                                                    <TableCell>
                                                        <TableCellLayout>
                                                            <span style="font-family:monospace;">
                                                                {pos.nm_id.to_string()}
                                                            </span>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {pos.article.unwrap_or_else(|| "—".to_string())}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {pos.name.unwrap_or_else(|| "—".to_string())}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout>
                                                            <details>
                                                                <summary style="cursor:pointer; font-size:0.82em; color:var(--color-text-secondary);">
                                                                    "показать"
                                                                </summary>
                                                                <pre style="font-size:0.78em; white-space:pre-wrap; margin:4px 0 0 0; max-width:320px; overflow:auto;">
                                                                    {nm_data_str}
                                                                </pre>
                                                            </details>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                </TableRow>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </TableBody>
                                </Table>
                            </div>
                        }.into_any()
                    }
                }}
            </Show>
        </div>
    }
}
