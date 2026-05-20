use crate::domain::a033_wb_day_close::ui::list::{format_date, format_money, WbDayCloseListDto};
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::clipboard::copy_to_clipboard_with_callback;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use chrono::{Datelike, Duration, Utc};
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thaw::*;

// ─────────────────────────────────────────────────────────────────────────────
// DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WbDayCloseLineDto {
    pub srid: String,
    pub nomenclature_ref: Option<String>,
    pub nm_id: Option<i64>,
    pub sa_name: Option<String>,
    pub event: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub detail: String,
    pub qty_sold: i64,
    pub qty_returned: i64,
    // ── Связь с a015 ──────────────────────────────────────────────────────────
    #[serde(default)]
    pub order_id: Option<String>,
    #[serde(default)]
    pub order_date: Option<String>,
    #[serde(default)]
    pub order_is_cancelled: bool,
    // ── Связь с a012 ──────────────────────────────────────────────────────────
    #[serde(default)]
    pub sales_doc_id: Option<String>,
    #[serde(default)]
    pub sales_doc_no: Option<String>,
    #[serde(default)]
    pub sales_event_type: Option<String>,
    #[serde(default)]
    pub sales_extra_ids: Vec<String>,
    #[serde(default)]
    pub sales_sale_id: Option<String>,
    // ── Ссылка на p903 ────────────────────────────────────────────────────────
    #[serde(default)]
    pub p903_ref_id: Option<String>,
    #[serde(default)]
    pub p903_rrd_id: Option<i64>,
    // ── 10 колонок ────────────────────────────────────────────────────────────
    pub revenue: f64,
    pub advertising: f64,
    pub logistics: f64,
    pub acquiring: f64,
    pub commission: f64,
    pub penalty: f64,
    pub other: f64,
    pub result: f64,
    pub dealer_price: f64,
    pub margin_diff: f64,
    #[serde(default)]
    pub problem_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WbDayCloseProblemDto {
    pub code: String,
    pub severity: String,
    pub srid: Option<String>,
    pub nomenclature_ref: Option<String>,
    #[serde(default)]
    pub a012_ids: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WbDayCloseTotalsDto {
    pub lines_count: i64,
    pub revenue: f64,
    pub advertising: f64,
    pub logistics: f64,
    pub acquiring: f64,
    pub commission: f64,
    pub penalty: f64,
    pub other: f64,
    pub result: f64,
    pub dealer_price: f64,
    pub margin_diff: f64,
    pub problems_block: i64,
    pub problems_warn: i64,
    pub problems_info: i64,
    #[serde(default)]
    pub problem_lines: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WbDayCloseDetailDto {
    pub id: String,
    pub connection_id: String,
    pub business_date: String,
    pub is_archived: bool,
    pub archived_at: Option<String>,
    pub archived_reason: Option<String>,
    pub replaces_id: Option<String>,
    pub last_recalculated_at: Option<String>,
    pub snapshot_hash: String,
    #[serde(default)]
    pub lines: Vec<WbDayCloseLineDto>,
    #[serde(default)]
    pub problems: Vec<WbDayCloseProblemDto>,
    pub totals: WbDayCloseTotalsDto,
}

// ─────────────────────────────────────────────────────────────────────────────
// API helpers
// ─────────────────────────────────────────────────────────────────────────────

async fn fetch_detail(id: &str) -> Result<WbDayCloseDetailDto, String> {
    let url = format!("{}/api/a033/wb-day-close/{}", api_base(), id);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Network: {}", e))?;
    if resp.status() == 404 {
        return Err("Документ не найден".to_string());
    }
    if !resp.ok() {
        return Err(format!("Ошибка сервера: {}", resp.status()));
    }
    resp.json::<WbDayCloseDetailDto>()
        .await
        .map_err(|e| format!("Ошибка разбора: {}", e))
}

async fn api_post_no_body(path: &str) -> Result<(), String> {
    let url = format!("{}{}", api_base(), path);
    let resp = Request::post(&url)
        .header("Content-Type", "application/json")
        .body("{}")
        .map_err(|e| format!("Build: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Network: {}", e))?;
    if !resp.ok() {
        return Err(format!("Ошибка сервера: {}", resp.status()));
    }
    Ok(())
}

async fn api_repost_all(id: &str) -> Result<RepostResultDto, String> {
    let url = format!(
        "{}/api/a033/wb-day-close/{}/repost-problematic-a012",
        api_base(),
        id
    );
    let body = serde_json::json!({ "onlyProblemCodes": [] });
    let resp = Request::post(&url)
        .json(&body)
        .map_err(|e| format!("Serialize: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Network: {}", e))?;
    if !resp.ok() {
        return Err(format!("Ошибка сервера: {}", resp.status()));
    }
    resp.json::<RepostResultDto>()
        .await
        .map_err(|e| format!("Parse: {}", e))
}

async fn api_archive_and_recreate(id: &str, reason: &str) -> Result<String, String> {
    let url = format!(
        "{}/api/a033/wb-day-close/{}/archive-and-recreate",
        api_base(),
        id
    );
    let body = if reason.is_empty() {
        serde_json::json!({ "reason": null })
    } else {
        serde_json::json!({ "reason": reason })
    };
    let resp = Request::post(&url)
        .json(&body)
        .map_err(|e| format!("Serialize: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Network: {}", e))?;
    if !resp.ok() {
        return Err(format!("Ошибка сервера: {}", resp.status()));
    }
    let val: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
    Ok(val["id"].as_str().unwrap_or_default().to_string())
}

async fn fetch_by_day(
    connection_id: &str,
    business_date: &str,
) -> Result<Vec<WbDayCloseListDto>, String> {
    let url = format!(
        "{}/api/a033/wb-day-close/by-day/{}/{}",
        api_base(),
        connection_id,
        business_date
    );
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Network: {}", e))?;
    if !resp.ok() {
        return Err(format!("Ошибка: {}", resp.status()));
    }
    resp.json::<Vec<WbDayCloseListDto>>()
        .await
        .map_err(|e| format!("Parse: {}", e))
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RepostResultDto {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    #[serde(default)]
    pub errors: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Main component
// ─────────────────────────────────────────────────────────────────────────────

#[component]
pub fn WbDayCloseDetails(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let (doc, set_doc) = signal::<Option<WbDayCloseDetailDto>>(None);
    let (loading, set_loading) = signal(true);
    let (action_loading, set_action_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (action_msg, set_action_msg) = signal::<Option<(bool, String)>>(None); // (is_error, msg)
    let (show_archive_form, set_show_archive_form) = signal(false);
    let archive_reason = RwSignal::new(String::new());
    let (archived_versions, set_archived_versions) = signal::<Vec<WbDayCloseListDto>>(vec![]);
    let selected_tab = RwSignal::new("result".to_string());
    let problems_filter = RwSignal::new("all".to_string()); // "all", "block", "warn"
    let lines_sort = RwSignal::new(SortState::new("srid")); // persists across tab switches

    let stored_id = StoredValue::new(id.clone());

    let load_doc = {
        let stored_id = stored_id;
        move || {
            let id = stored_id.get_value();
            set_loading.set(true);
            set_error.set(None);
            spawn_local(async move {
                match fetch_detail(&id).await {
                    Ok(d) => {
                        set_doc.set(Some(d));
                        set_loading.set(false);
                    }
                    Err(e) => {
                        set_error.set(Some(e));
                        set_loading.set(false);
                    }
                }
            });
        }
    };

    let load_doc_clone = load_doc.clone();
    Effect::new(move || {
        load_doc_clone();
    });

    // Recalculate
    let on_recalculate = {
        let stored_id = stored_id;
        let load_doc = load_doc.clone();
        move |_| {
            let id = stored_id.get_value();
            set_action_loading.set(true);
            set_action_msg.set(None);
            let load_doc = load_doc.clone();
            spawn_local(async move {
                let path = format!("/api/a033/wb-day-close/{}/recalculate", id);
                match api_post_no_body(&path).await {
                    Ok(()) => {
                        set_action_msg.set(Some((false, "Пересчёт выполнен".to_string())));
                        load_doc();
                    }
                    Err(e) => set_action_msg.set(Some((true, e))),
                }
                set_action_loading.set(false);
            });
        }
    };

    // Repost all problematic a012
    let do_repost_all = StoredValue::new({
        let stored_id = stored_id;
        let load_doc = load_doc.clone();
        move || {
            let id = stored_id.get_value();
            set_action_loading.set(true);
            set_action_msg.set(None);
            let load_doc = load_doc.clone();
            spawn_local(async move {
                match api_repost_all(&id).await {
                    Ok(res) => {
                        set_action_msg.set(Some((
                            res.failed > 0,
                            format!(
                                "Перепроведено: {} из {}. Ошибок: {}",
                                res.success, res.total, res.failed
                            ),
                        )));
                        load_doc();
                    }
                    Err(e) => set_action_msg.set(Some((true, e))),
                }
                set_action_loading.set(false);
            });
        }
    });
    // Two independent closures sharing do_repost_all via Copy (StoredValue is Copy)
    let on_repost_all_header = move |_: leptos::ev::MouseEvent| do_repost_all.get_value()();
    let on_repost_all_tab = move |_: leptos::ev::MouseEvent| do_repost_all.get_value()();

    // Archive and recreate
    let on_archive_and_recreate = {
        let stored_id = stored_id;
        let tabs_store = tabs_store.clone();
        move |_| {
            let id = stored_id.get_value();
            let reason = archive_reason.get_untracked();
            set_action_loading.set(true);
            set_action_msg.set(None);
            set_show_archive_form.set(false);
            let tabs_store = tabs_store.clone();
            spawn_local(async move {
                match api_archive_and_recreate(&id, &reason).await {
                    Ok(new_id) => {
                        set_action_msg.set(Some((
                            false,
                            "Заархивировано, открываем новый документ…".to_string(),
                        )));
                        let key = format!("a033_wb_day_close_details_{}", new_id);
                        tabs_store.open_tab(&key, "Закрытие дня WB");
                    }
                    Err(e) => set_action_msg.set(Some((true, e))),
                }
                set_action_loading.set(false);
            });
        }
    };

    // Load versions for compare
    let on_load_versions = {
        move |_| {
            if let Some(d) = doc.get_untracked() {
                let cid = d.connection_id.clone();
                let bdate = d.business_date.clone();
                spawn_local(async move {
                    if let Ok(versions) = fetch_by_day(&cid, &bdate).await {
                        set_archived_versions.set(versions);
                    }
                });
            }
        }
    };

    view! {
        <PageFrame page_id="a033_wb_day_close_details" category="detail">
            // Header
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || doc.get()
                            .map(|d| format!("Закрытие дня WB — {}", format_date(&d.business_date)))
                            .unwrap_or_else(|| "Закрытие дня WB".to_string())}
                    </h1>
                    {move || doc.get().map(|d| view! {
                        <span style="margin-left: var(--spacing-sm);">
                            {if d.is_archived {
                                view! {
                                    <Badge appearance=BadgeAppearance::Outline color=BadgeColor::Subtle>
                                        "Архивный"
                                    </Badge>
                                }.into_any()
                            } else {
                                view! {
                                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Success>
                                        "Активный"
                                    </Badge>
                                }.into_any()
                            }}
                        </span>
                    })}
                </div>
                <div class="page__header-right" style="display: flex; gap: var(--spacing-xs);">
                    <Button
                        size=ButtonSize::Small
                        disabled=Signal::derive(move || action_loading.get() || loading.get())
                        on_click=on_recalculate
                    >
                        {icon("zap")} " Провести"
                    </Button>
                    <Button
                        size=ButtonSize::Small
                        disabled=Signal::derive(move || action_loading.get() || loading.get())
                        on_click=on_repost_all_header
                    >
                        {icon("zap")} " Перепровести проблемные"
                    </Button>
                    {move || {
                        if doc.get().map(|d| !d.is_archived).unwrap_or(false) {
                            view! {
                                <Button
                                    size=ButtonSize::Small
                                    disabled=Signal::derive(move || action_loading.get())
                                    on_click=move |_| set_show_archive_form.update(|v| *v = !*v)
                                >
                                    {icon("archive")} " Архив + новый"
                                </Button>
                            }.into_any()
                        } else {
                            view! { <span /> }.into_any()
                        }
                    }}
                    <Button
                        size=ButtonSize::Small
                        on_click=on_load_versions
                    >
                        {icon("git-compare")} " Версии"
                    </Button>
                    <Button
                        size=ButtonSize::Small
                        on_click=move |_| on_close.run(())
                    >
                        "Закрыть"
                    </Button>
                </div>
            </div>

            // Archive form
            {move || if show_archive_form.get() {
                view! {
                    <div style="padding: var(--spacing-md); background: var(--color-bg-elevated); border-bottom: 1px solid var(--color-border); display: flex; gap: var(--spacing-md); align-items: flex-end;">
                        <div>
                            <Label>"Причина архивации"</Label>
                            <Input value=archive_reason placeholder="Опционально" />
                        </div>
                        <Button appearance=ButtonAppearance::Primary size=ButtonSize::Small on_click=on_archive_and_recreate>
                            "Подтвердить"
                        </Button>
                        <Button size=ButtonSize::Small on_click=move |_| set_show_archive_form.set(false)>
                            "Отмена"
                        </Button>
                    </div>
                }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}

            // Action message bar
            {move || action_msg.get().map(|(is_err, msg)| {
                let style = if is_err {
                    "padding: var(--spacing-sm) var(--spacing-md); color: var(--color-danger); background: var(--color-danger-50);"
                } else {
                    "padding: var(--spacing-sm) var(--spacing-md); color: var(--color-success); background: var(--color-success-50);"
                };
                view! { <div style=style>{msg}</div> }
            })}

            // Main content
            <div class="page__content">
                {move || {
                    if loading.get() {
                        return view! {
                            <Flex gap=FlexGap::Small style="justify-content: center; padding: var(--spacing-4xl); align-items: center;">
                                <Spinner />
                                <span>"Загрузка..."</span>
                            </Flex>
                        }.into_any();
                    }
                    if let Some(err) = error.get() {
                        return view! {
                            <div style="padding: var(--spacing-lg); color: var(--color-danger);">
                                {err}
                            </div>
                        }.into_any();
                    }

                    let Some(d) = doc.get() else {
                        return view! { <span /> }.into_any();
                    };

                    let lines_count = d.totals.lines_count;
                    let problem_lines = d.totals.problem_lines;
                    let problems_block = d.totals.problems_block;
                    let total_probs = d.problems.len() as i64;
                    let tabs_store_inner = tabs_store.clone();
                    let lines = d.lines.clone();
                    let problems = d.problems.clone();
                    let d_for_result = d.clone();

                    view! {
                        <div style="display: flex; flex-direction: column; height: 100%; overflow: hidden;">
                            <DocHeaderInfo doc=d.clone() />
                            <TotalsRow totals=d.totals.clone() />

                            // Tabs navigation
                            <div style="border-bottom: 1px solid var(--color-border);">
                                <TabList selected_value=selected_tab>
                                <Tab value="result".to_string()>
                                    "Результат"
                                </Tab>
                                <Tab value="lines".to_string()>
                                    {format!("Строки ({})", lines_count)}
                                    {if problem_lines > 0 {
                                        view! {
                                            <span style="margin-left: 4px;">
                                                <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Warning>
                                                    {format!("{} с пробл.", problem_lines)}
                                                </Badge>
                                            </span>
                                        }.into_any()
                                    } else { view! { <span /> }.into_any() }}
                                </Tab>
                                <Tab value="problems".to_string()>
                                    {if total_probs > 0 {
                                        let badge_color = if problems_block > 0 { BadgeColor::Danger } else { BadgeColor::Warning };
                                        view! {
                                            <span>
                                                "Проблемы "
                                                <span style="margin-left: 4px;">
                                                    <Badge appearance=BadgeAppearance::Filled color=badge_color>
                                                        {total_probs}
                                                    </Badge>
                                                </span>
                                            </span>
                                        }.into_any()
                                    } else {
                                        view! { <span>"Проблемы (нет)"</span> }.into_any()
                                    }}
                                </Tab>
                                </TabList>
                            </div>

                            // Tab content
                            <div style="flex: 1; overflow: auto;">
                                // Archived versions bar (visible in both tabs)
                                {move || {
                                    let versions = archived_versions.get();
                                    if versions.is_empty() {
                                        return view! { <span /> }.into_any();
                                    }
                                    let tabs = tabs_store_inner.clone();
                                    view! {
                                        <div style="padding: var(--spacing-xs) var(--spacing-md); border-bottom: 1px solid var(--color-border); display: flex; align-items: center; gap: var(--spacing-xs); flex-wrap: wrap; background: var(--color-bg-elevated);">
                                            <span style="font-size: 0.8em; color: var(--color-text-secondary); white-space: nowrap;">"Версии:"</span>
                                            {versions.into_iter().map(|v| {
                                                let vid = v.id.clone();
                                                let vdate = v.business_date.clone();
                                                let tabs = tabs.clone();
                                                view! {
                                                    <Button
                                                        size=ButtonSize::Small
                                                        on_click=move |_| {
                                                            let key = format!("a033_wb_day_close_details_{}", vid);
                                                            let label = if v.is_archived {
                                                                format!("Закрытие {} [архив]", vdate)
                                                            } else {
                                                                format!("Закрытие {}", vdate)
                                                            };
                                                            tabs.open_tab(&key, &label);
                                                        }
                                                    >
                                                        {if v.is_archived { "[A] " } else { "" }}
                                                        {format_date(&v.business_date)}
                                                        " · " {format_money(v.result)}
                                                    </Button>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                }}

                                // Tab panel: Результат
                                {move || {
                                    if selected_tab.get() != "result" {
                                        return view! { <span /> }.into_any();
                                    }
                                    view! {
                                        <ResultTab doc=d_for_result.clone() />
                                    }.into_any()
                                }}

                                // Tab panel: Строки
                                {move || {
                                    if selected_tab.get() != "lines" {
                                        return view! { <span /> }.into_any();
                                    }
                                    view! {
                                        <LinesTable lines=lines.clone() tabs_store=tabs_store.clone() sort=lines_sort />
                                    }.into_any()
                                }}

                                // Tab panel: Проблемы
                                {move || {
                                    if selected_tab.get() != "problems" {
                                        return view! { <span /> }.into_any();
                                    }
                                    let probs_for_tab = problems.clone();
                                    view! {
                                        <div style="padding: var(--spacing-md);">
                                            // Toolbar
                                            <div style="display: flex; gap: var(--spacing-sm); align-items: center; margin-bottom: var(--spacing-sm); flex-wrap: wrap;">
                                                <span style="font-weight: 600;">"Фильтр:"</span>
                                                <Button size=ButtonSize::Small
                                                    appearance=Signal::derive(move || if problems_filter.get() == "all" { ButtonAppearance::Primary } else { ButtonAppearance::Secondary })
                                                    on_click=move |_| problems_filter.set("all".to_string())
                                                >"Все"</Button>
                                                <Button size=ButtonSize::Small
                                                    appearance=Signal::derive(move || if problems_filter.get() == "block" { ButtonAppearance::Primary } else { ButtonAppearance::Secondary })
                                                    on_click=move |_| problems_filter.set("block".to_string())
                                                >"Только Block"</Button>
                                                <Button size=ButtonSize::Small
                                                    appearance=Signal::derive(move || if problems_filter.get() == "warn" { ButtonAppearance::Primary } else { ButtonAppearance::Secondary })
                                                    on_click=move |_| problems_filter.set("warn".to_string())
                                                >"Только Warn"</Button>
                                                <Button appearance=ButtonAppearance::Primary size=ButtonSize::Small
                                                    disabled=Signal::derive(move || action_loading.get())
                                                    on_click=on_repost_all_tab
                                                >
                                                    {icon("zap")} " Перепровести проблемные"
                                                </Button>
                                            </div>
                                            // Table
                                            <div style="overflow-x: auto;">
                                                <table class="data-table" style="font-size: 0.84em; width: 100%;">
                                                    <thead>
                                                        <tr>
                                                            <th>"Серьёзность"</th>
                                                            <th>"Код"</th>
                                                            <th>"srid"</th>
                                                            <th>"Сообщение"</th>
                                                            <th>"a012"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody>
                                                        {move || {
                                                            let f = problems_filter.get();
                                                            probs_for_tab.iter().filter(|p| {
                                                                match f.as_str() {
                                                                    "block" => p.severity == "block",
                                                                    "warn" => p.severity == "warn",
                                                                    _ => true,
                                                                }
                                                            }).map(|p| {
                                                                let (badge_color, severity_label) = match p.severity.as_str() {
                                                                    "block" => (BadgeColor::Danger, "Блок"),
                                                                    "warn" => (BadgeColor::Warning, "Пред."),
                                                                    _ => (BadgeColor::Informative, "Инфо"),
                                                                };
                                                                let srid_display = p.srid.as_deref().unwrap_or("—");
                                                                let srid_short = srid_display.chars().take(20).collect::<String>();
                                                                view! {
                                                                    <tr>
                                                                        <td>
                                                                            <Badge appearance=BadgeAppearance::Filled color=badge_color>{severity_label}</Badge>
                                                                        </td>
                                                                        <td><code style="font-size: 0.85em;">{p.code.clone()}</code></td>
                                                                        <td><code style="font-size: 0.85em;">{srid_short}</code></td>
                                                                        <td style="max-width: 400px;">{p.message.clone()}</td>
                                                                        <td style="text-align: right;">
                                                                            {if p.a012_ids.is_empty() { "—".to_string() } else { p.a012_ids.len().to_string() }}
                                                                        </td>
                                                                    </tr>
                                                                }
                                                            }).collect_view()
                                                        }}
                                                    </tbody>
                                                </table>
                                            </div>
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        </div>
                    }.into_any()
                }}
            </div>
        </PageFrame>
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// New document page
// ─────────────────────────────────────────────────────────────────────────────

async fn do_create_active(connection_id: &str, business_date: &str) -> Result<String, String> {
    let url = format!("{}/api/a033/wb-day-close", api_base());
    let body = serde_json::json!({
        "connectionId": connection_id,
        "businessDate": business_date,
    });
    let resp = Request::post(&url)
        .json(&body)
        .map_err(|e| format!("Serialize: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Network: {}", e))?;
    if !resp.ok() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Server error {}: {}", status, text));
    }
    let val: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
    Ok(val["id"].as_str().unwrap_or_default().to_string())
}

async fn fetch_connections() -> Result<Vec<ConnectionMP>, String> {
    let url = format!("{}/api/connection_mp", api_base());
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|err| format!("Ошибка загрузки кабинетов: {}", err))?;
    if !response.ok() {
        return Err(format!(
            "Ошибка загрузки кабинетов: HTTP {}",
            response.status()
        ));
    }
    response
        .json::<Vec<ConnectionMP>>()
        .await
        .map_err(|err| format!("Ошибка разбора кабинетов: {}", err))
}

fn default_business_date() -> String {
    let yesterday = Utc::now().date_naive() - Duration::days(1);
    yesterday.format("%Y-%m-%d").to_string()
}

/// Generate (value, label) options for the last `days` calendar days,
/// newest first. value = "YYYY-MM-DD" (API format), label = "DD.MM.YYYY (Пн)".
fn generate_date_options(days: i64) -> Vec<(String, String)> {
    let today = Utc::now().date_naive();
    let weekday_ru = |n: u32| match n {
        0 => "Пн",
        1 => "Вт",
        2 => "Ср",
        3 => "Чт",
        4 => "Пт",
        5 => "Сб",
        6 => "Вс",
        _ => "?",
    };
    (0..days)
        .map(|i| {
            let d = today - Duration::days(i);
            let value = d.format("%Y-%m-%d").to_string();
            let weekday = weekday_ru(d.weekday().num_days_from_monday());
            let suffix = match i {
                0 => " — сегодня",
                1 => " — вчера",
                _ => "",
            };
            let label = format!("{} ({}){}", d.format("%d.%m.%Y"), weekday, suffix);
            (value, label)
        })
        .collect()
}

/// Страница создания нового документа.
/// Пользователь выбирает кабинет и дату из списков и нажимает "Провести" —
/// документ создаётся и сразу пересчитывается.
#[component]
pub fn WbDayCloseNewPage(#[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let new_connection_id = RwSignal::new(String::new());
    let new_business_date = RwSignal::new(default_business_date());
    let connections = RwSignal::new(Vec::<ConnectionMP>::new());
    let connections_loading = RwSignal::new(false);
    let date_options = StoredValue::new(generate_date_options(60));

    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    Effect::new(move |_| {
        connections_loading.set(true);
        spawn_local(async move {
            match fetch_connections().await {
                Ok(mut items) => {
                    items.sort_by(|a, b| {
                        a.base
                            .description
                            .to_lowercase()
                            .cmp(&b.base.description.to_lowercase())
                    });
                    connections.set(items);
                }
                Err(err) => set_error.set(Some(err)),
            }
            connections_loading.set(false);
        });
    });

    let can_submit = Signal::derive(move || {
        !loading.get()
            && !new_connection_id.get().trim().is_empty()
            && !new_business_date.get().trim().is_empty()
    });

    let on_post = {
        let tabs = tabs_store.clone();
        move |_| {
            let cid = new_connection_id.get_untracked().trim().to_string();
            let bdate = new_business_date.get_untracked().trim().to_string();
            log!("a033 new: create+post cid='{}' bdate='{}'", cid, bdate);
            set_loading.set(true);
            set_error.set(None);
            let tabs = tabs.clone();
            spawn_local(async move {
                let new_id = match do_create_active(&cid, &bdate).await {
                    Ok(id) => id,
                    Err(e) => {
                        set_error.set(Some(format!("Ошибка создания: {}", e)));
                        set_loading.set(false);
                        return;
                    }
                };
                log!("a033 new: created id={}", new_id);

                let path = format!("/api/a033/wb-day-close/{}/recalculate", new_id);
                if let Err(e) = api_post_no_body(&path).await {
                    set_error.set(Some(format!(
                        "Создан {}, но ошибка проведения: {}",
                        new_id, e
                    )));
                    set_loading.set(false);
                    let key = format!("a033_wb_day_close_details_{}", new_id);
                    let label = format!("Закрытие {}", bdate);
                    tabs.open_tab(&key, &label);
                    return;
                }
                log!("a033 new: recalculated");

                set_loading.set(false);
                on_close.run(());
                let key = format!("a033_wb_day_close_details_{}", new_id);
                let label = format!("Закрытие {}", bdate);
                tabs.open_tab(&key, &label);
            });
        }
    };

    view! {
        <PageFrame page_id="a033_wb_day_close_new" category="detail">
            // ── Header ────────────────────────────────────────────────
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Новый документ — Закрытие дня WB"</h1>
                    <Badge appearance=BadgeAppearance::Outline color=BadgeColor::Subtle>
                        "Черновик"
                    </Badge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Primary
                        size=ButtonSize::Medium
                        disabled=Signal::derive(move || !can_submit.get())
                        on_click=on_post
                    >
                        {move || if loading.get() {
                            view! { <><Spinner size=SpinnerSize::Tiny /> " Создание и проведение..."</> }.into_any()
                        } else {
                            view! { <>{icon("zap")} " Провести"</> }.into_any()
                        }}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        size=ButtonSize::Medium
                        on_click=move |_| on_close.run(())
                    >
                        "Закрыть"
                    </Button>
                </div>
            </div>

            // ── Content ───────────────────────────────────────────────
            <div class="page__content">
                {move || error.get().map(|e| view! {
                    <div style="margin: var(--spacing-sm) var(--spacing-md); padding: var(--spacing-sm) var(--spacing-md); background: var(--color-error-50, #fef2f2); border: 1px solid var(--color-error-100, #fee2e2); border-radius: var(--radius-sm); color: var(--color-error, #dc2626); display: flex; align-items: center; gap: var(--spacing-xs);">
                        {icon("alert-circle")} " " {e}
                    </div>
                })}

                <div class="detail-grid">
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=0 nav_id="a033_wb_day_close_new_params">
                            <h4 class="details-section__title">"Параметры документа"</h4>

                            <div class="form__group">
                                <label class="form__label">"Кабинет"</label>
                                <Select value=new_connection_id>
                                    <option value="">
                                        {move || if connections_loading.get() {
                                            "— загрузка кабинетов… —".to_string()
                                        } else {
                                            "— выберите кабинет —".to_string()
                                        }}
                                    </option>
                                    {move || {
                                        connections.get().into_iter().map(|conn| {
                                            let id = conn.base.id.as_string();
                                            let label = if conn.base.description.trim().is_empty() {
                                                conn.base.code.clone()
                                            } else {
                                                conn.base.description.clone()
                                            };
                                            view! { <option value=id>{label}</option> }
                                        }).collect_view()
                                    }}
                                </Select>
                            </div>

                            <div class="form__group">
                                <label class="form__label">"Дата закрытия"</label>
                                <Select value=new_business_date>
                                    {date_options.get_value().into_iter().map(|(value, label)| {
                                        view! { <option value=value>{label}</option> }
                                    }).collect_view()}
                                </Select>
                            </div>

                            <div class="form__group">
                                <label class="form__label">"Краткое описание"</label>
                                <FieldDisplaySummary
                                    connection_id=new_connection_id
                                    business_date=new_business_date
                                    connections=connections
                                />
                            </div>
                        </CardAnimated>
                    </div>

                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=80 nav_id="a033_wb_day_close_new_info">
                            <h4 class="details-section__title">"Что произойдёт при проведении"</h4>
                            <ol style="margin: 0; padding-left: var(--spacing-lg); display: flex; flex-direction: column; gap: var(--spacing-xs); line-height: 1.55;">
                                <li>"Создаётся активный документ закрытия дня для выбранного кабинета и даты."</li>
                                <li>"Документ сразу пересчитывается: подгружаются продажи, возвраты, реклама, логистика и комиссии."</li>
                                <li>"Выявляются проблемные строки (блок./пред./инфо) — они будут показаны в боковой панели документа."</li>
                                <li>"После завершения этот таб закроется и откроется детальная карточка нового документа."</li>
                            </ol>

                            <div style="margin-top: var(--spacing-md); padding: var(--spacing-sm) var(--spacing-md); background: var(--color-info-50, #eff6ff); border-left: 3px solid var(--color-info, #3b82f6); border-radius: var(--radius-sm); font-size: 0.875em;">
                                {icon("info")} " Если для этого кабинета и даты уже есть активный документ — он будет переиспользован."
                            </div>
                        </CardAnimated>
                    </div>
                </div>
            </div>
        </PageFrame>
    }
}

/// Compact summary line for the chosen cabinet + date inside the params card.
#[component]
fn FieldDisplaySummary(
    connection_id: RwSignal<String>,
    business_date: RwSignal<String>,
    connections: RwSignal<Vec<ConnectionMP>>,
) -> impl IntoView {
    view! {
        <input
            type="text"
            readonly=true
            class="form__input"
            prop:value=move || {
                let cid = connection_id.get();
                let bdate = business_date.get();
                if cid.is_empty() {
                    return "Выберите кабинет и дату".to_string();
                }
                let conn_label = connections
                    .with(|list| {
                        list.iter()
                            .find(|c| c.base.id.as_string() == cid)
                            .map(|c| {
                                if c.base.description.trim().is_empty() {
                                    c.base.code.clone()
                                } else {
                                    c.base.description.clone()
                                }
                            })
                    })
                    .unwrap_or_else(|| "—".to_string());
                format!("{} · {}", conn_label, format_date(&bdate))
            }
        />
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Subcomponents
// ─────────────────────────────────────────────────────────────────────────────

#[component]
fn DocHeaderInfo(doc: WbDayCloseDetailDto) -> impl IntoView {
    view! {
        <div style="padding: var(--spacing-sm) var(--spacing-md); display: flex; gap: var(--spacing-lg); flex-wrap: wrap; border-bottom: 1px solid var(--color-border); font-size: 0.9em; align-items: center;">
            <span>"Кабинет: " <code style="font-size: 0.85em;">{doc.connection_id[..doc.connection_id.len().min(24)].to_string()}</code></span>
            <span>"Дата: " <strong>{format_date(&doc.business_date)}</strong></span>
            <span>"Строк: " <strong>{doc.totals.lines_count}</strong></span>
            {doc.last_recalculated_at.as_ref().map(|at| view! {
                <span style="color: var(--color-text-secondary);">
                    "Пересчитан: " <strong style="color: var(--color-text-base);">{format_datetime_msk(at)}</strong>
                </span>
            })}
            <span style="color: var(--color-text-secondary); font-size: 0.8em;">
                "Хэш: " <code>{doc.snapshot_hash[..doc.snapshot_hash.len().min(16)].to_string()}</code>
            </span>
        </div>
    }
}

/// Format an ISO datetime string (RFC3339/UTC) to "DD.MM.YYYY HH:MM МСК" (UTC+3).
fn format_datetime_msk(iso: &str) -> String {
    // Parse "2026-05-15T04:57:20..." — add 3h offset manually
    let s = iso.trim();
    let date_time = s.split('T').collect::<Vec<_>>();
    if date_time.len() < 2 {
        return format_date(s);
    }
    let date_part = date_time[0];
    let time_part = date_time[1];
    let (year, month, day) = {
        let p: Vec<&str> = date_part.split('-').collect();
        if p.len() < 3 {
            return format_date(s);
        }
        (p[0], p[1], p[2])
    };
    let (hour_utc, minute) = {
        let p: Vec<&str> = time_part.splitn(3, ':').collect();
        if p.len() < 2 {
            return format_date(s);
        }
        (p[0].parse::<u32>().unwrap_or(0), p[1])
    };
    // Add 3 hours for MSK, handling day rollover simply
    let hour_msk = (hour_utc + 3) % 24;
    format!("{}.{}.{} {:02}:{} МСК", day, month, year, hour_msk, minute)
}

fn signed_style(value: f64) -> String {
    if value > 0.0 {
        "text-align: right; color: var(--color-success);".to_string()
    } else if value < 0.0 {
        "text-align: right; color: var(--color-danger);".to_string()
    } else {
        "text-align: right; color: var(--color-text-secondary);".to_string()
    }
}

fn format_money_excel(value: f64) -> String {
    format!("{:.2}", value).replace('.', ",")
}

fn build_totals_excel_tsv(totals: &WbDayCloseTotalsDto) -> String {
    let rows: [(&str, f64); 10] = [
        ("1. Реализация", totals.revenue),
        ("2. Реклама", totals.advertising),
        ("3. Логистика", totals.logistics),
        ("4. Эквайринг", totals.acquiring),
        ("5. Комиссия", totals.commission),
        ("6. Штрафы", totals.penalty),
        ("7. Прочее", totals.other),
        ("8. Результат", totals.result),
        ("9. Цена дилер", totals.dealer_price),
        ("10. Маржа", totals.margin_diff),
    ];
    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push("Колонка\tСумма".to_string());
    for (label, value) in rows.iter() {
        lines.push(format!("{}\t{}", label, format_money_excel(*value)));
    }
    lines.join("\n")
}

/// Возвращает "виртуальный" тип строки: возвраты в типе Продажа (srid начинается с R)
/// выделяются в отдельный тип "sale_return".
fn effective_kind(line: &WbDayCloseLineDto) -> &'static str {
    if line.kind == "sale" && line.srid.starts_with('R') {
        "sale_return"
    } else {
        match line.kind.as_str() {
            "sale" => "sale",
            "return" => "return",
            "commission_adjustment" => "commission_adjustment",
            "logistics" => "logistics",
            "storage" => "storage",
            "penalty" => "penalty",
            "ppvz_reward" => "ppvz_reward",
            "voluntary_return_compensation" => "voluntary_return_compensation",
            "transport_storage_reimbursement" => "transport_storage_reimbursement",
            "acceptance" => "acceptance",
            "info" => "info",
            _ => "other",
        }
    }
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "sale" => "Продажа",
        "sale_return" => "Возврат",
        "return" => "Возврат",
        "commission_adjustment" => "Корр.комиссии",
        "logistics" => "Логистика",
        "storage" => "Хранение",
        "penalty" => "Штраф",
        "ppvz_reward" => "Возм.ПВЗ",
        "voluntary_return_compensation" => "Добр.компенс.",
        "transport_storage_reimbursement" => "Возмещение",
        "acceptance" => "Приёмка",
        "info" => "Инфо",
        _ => "Прочее",
    }
}

fn kind_badge_color(kind: &str) -> BadgeColor {
    match kind {
        "sale" => BadgeColor::Success,
        "sale_return" | "return" => BadgeColor::Warning,
        "penalty" => BadgeColor::Danger,
        "storage"
        | "acceptance"
        | "ppvz_reward"
        | "logistics"
        | "transport_storage_reimbursement" => BadgeColor::Informative,
        "info" => BadgeColor::Subtle,
        _ => BadgeColor::Subtle,
    }
}

/// Sort key for the lines table. String = column id.
#[derive(Clone, PartialEq, Eq)]
struct SortState {
    col: String,
    asc: bool,
}

impl SortState {
    fn new(col: &str) -> Self {
        Self {
            col: col.to_string(),
            asc: true,
        }
    }
    fn toggle(&self, col: &str) -> Self {
        if self.col == col {
            Self {
                col: self.col.clone(),
                asc: !self.asc,
            }
        } else {
            Self {
                col: col.to_string(),
                asc: true,
            }
        }
    }
    fn indicator(&self, col: &str) -> &'static str {
        if self.col != col {
            return "";
        }
        if self.asc {
            " ▲"
        } else {
            " ▼"
        }
    }
}

fn sort_lines(lines: &mut Vec<WbDayCloseLineDto>, s: &SortState) {
    lines.sort_by(|a, b| {
        let ord = match s.col.as_str() {
            "srid" => a.srid.cmp(&b.srid),
            "sa_name" => a.sa_name.cmp(&b.sa_name),
            "kind" => a.kind.cmp(&b.kind),
            "order_date" => a.order_date.cmp(&b.order_date),
            "revenue" => a
                .revenue
                .partial_cmp(&b.revenue)
                .unwrap_or(std::cmp::Ordering::Equal),
            "advertising" => a
                .advertising
                .partial_cmp(&b.advertising)
                .unwrap_or(std::cmp::Ordering::Equal),
            "logistics" => a
                .logistics
                .partial_cmp(&b.logistics)
                .unwrap_or(std::cmp::Ordering::Equal),
            "acquiring" => a
                .acquiring
                .partial_cmp(&b.acquiring)
                .unwrap_or(std::cmp::Ordering::Equal),
            "commission" => a
                .commission
                .partial_cmp(&b.commission)
                .unwrap_or(std::cmp::Ordering::Equal),
            "penalty" => a
                .penalty
                .partial_cmp(&b.penalty)
                .unwrap_or(std::cmp::Ordering::Equal),
            "other" => a
                .other
                .partial_cmp(&b.other)
                .unwrap_or(std::cmp::Ordering::Equal),
            "result" => a
                .result
                .partial_cmp(&b.result)
                .unwrap_or(std::cmp::Ordering::Equal),
            "dealer_price" => a
                .dealer_price
                .partial_cmp(&b.dealer_price)
                .unwrap_or(std::cmp::Ordering::Equal),
            "margin_diff" => a
                .margin_diff
                .partial_cmp(&b.margin_diff)
                .unwrap_or(std::cmp::Ordering::Equal),
            "margin_pct" => {
                let pct_a = if a.revenue.abs() > 0.01 {
                    a.margin_diff / a.revenue.abs() * 100.0
                } else {
                    f64::NEG_INFINITY
                };
                let pct_b = if b.revenue.abs() > 0.01 {
                    b.margin_diff / b.revenue.abs() * 100.0
                } else {
                    f64::NEG_INFINITY
                };
                pct_a
                    .partial_cmp(&pct_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
            "rrd_id" => a.p903_rrd_id.cmp(&b.p903_rrd_id),
            _ => std::cmp::Ordering::Equal,
        };
        if s.asc {
            ord
        } else {
            ord.reverse()
        }
    });
}

const TH_LINK: &str = "cursor: pointer; user-select: none; white-space: nowrap;";
const CELL: &str = "white-space: nowrap;";

#[component]
fn LinesTable(
    lines: Vec<WbDayCloseLineDto>,
    tabs_store: AppGlobalContext,
    sort: RwSignal<SortState>,
) -> impl IntoView {
    // Порядок отображения типов
    const KIND_ORDER: &[&str] = &[
        "sale",
        "sale_return",
        "return",
        "logistics",
        "storage",
        "penalty",
        "commission_adjustment",
        "ppvz_reward",
        "voluntary_return_compensation",
        "transport_storage_reimbursement",
        "acceptance",
        "info",
        "other",
    ];

    let present: HashSet<String> = lines
        .iter()
        .map(|l| effective_kind(l).to_string())
        .collect();
    let ordered_kinds: Vec<String> = KIND_ORDER
        .iter()
        .filter(|k| present.contains(**k))
        .map(|k| k.to_string())
        .chain(
            present
                .iter()
                .filter(|k| !KIND_ORDER.contains(&k.as_str()))
                .cloned(),
        )
        .collect();

    let kind_counts: HashMap<String, usize> = lines.iter().fold(HashMap::new(), |mut acc, l| {
        *acc.entry(effective_kind(l).to_string()).or_insert(0) += 1;
        acc
    });

    // Сигнал на каждый тип — все включены по умолчанию
    let kind_toggles: StoredValue<Vec<(String, RwSignal<bool>)>> = StoredValue::new(
        ordered_kinds
            .iter()
            .map(|k| (k.clone(), RwSignal::new(true)))
            .collect(),
    );

    view! {
        <div>
            // ── Строка фильтров по типу ───────────────────────────────
            <div style="display: flex; gap: var(--spacing-xs); flex-wrap: wrap; padding: var(--spacing-xs) var(--spacing-md); background: var(--color-bg-elevated); border-bottom: 1px solid var(--color-border); align-items: center;">
                <span style="font-size: 0.84em; font-weight: 600; color: var(--color-text-secondary); white-space: nowrap; margin-right: 4px;">"Тип:"</span>
                {kind_toggles.get_value().into_iter().map(|(kind, sig)| {
                    let label = kind_label(&kind);
                    let count = kind_counts.get(&kind).copied().unwrap_or(0);
                    let color = kind_badge_color(&kind);
                    view! {
                        <label style="display: inline-flex; align-items: center; gap: 4px; cursor: pointer; font-size: 0.84em; user-select: none;">
                            <input
                                type="checkbox"
                                prop:checked=move || sig.get()
                                on:change=move |_| sig.update(|v| *v = !*v)
                            />
                            <span style="font-size: 0.82em;">
                                <Badge appearance=BadgeAppearance::Filled color=color>
                                    {format!("{} ({})", label, count)}
                                </Badge>
                            </span>
                        </label>
                    }
                }).collect_view()}
            </div>

            // ── Таблица ───────────────────────────────────────────────
            <div style="overflow-x: auto;">
                <table class="data-table" style="min-width: 1600px; font-size: 0.82em;">
                    <thead>
                        <tr>
                            <th
                                style=format!("{TH_LINK} min-width: 110px; position: sticky; left: 0; z-index: 2; background: var(--color-bg-base);")
                                on:click=move |_| sort.update(|s| *s = s.toggle("sa_name"))
                            >
                                "Артикул" {move || sort.with(|s| s.indicator("sa_name"))}
                            </th>
                            <th
                                style=format!("{TH_LINK} min-width: 130px;")
                                on:click=move |_| sort.update(|s| *s = s.toggle("srid"))
                            >
                                "srid" {move || sort.with(|s| s.indicator("srid"))}
                            </th>
                            <th
                                style=format!("{TH_LINK} min-width: 110px; text-align: center;")
                                on:click=move |_| sort.update(|s| *s = s.toggle("kind"))
                            >
                                "Тип" {move || sort.with(|s| s.indicator("kind"))}
                            </th>
                            <th
                                style=format!("{TH_LINK} min-width: 80px; text-align: center;")
                                on:click=move |_| sort.update(|s| *s = s.toggle("order_date"))
                            >
                                "Заказ (a015)" {move || sort.with(|s| s.indicator("order_date"))}
                            </th>
                            <th style="min-width: 120px; text-align: center;">"Реализация (a012)"</th>
                            <th
                                style=format!("{TH_LINK} text-align: right;")
                                on:click=move |_| sort.update(|s| *s = s.toggle("rrd_id"))
                            >
                                "RRD" {move || sort.with(|s| s.indicator("rrd_id"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: right;") on:click=move |_| sort.update(|s| *s = s.toggle("revenue"))>
                                "1. Реализация" {move || sort.with(|s| s.indicator("revenue"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: right;") on:click=move |_| sort.update(|s| *s = s.toggle("advertising"))>
                                "2. Реклама" {move || sort.with(|s| s.indicator("advertising"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: right;") on:click=move |_| sort.update(|s| *s = s.toggle("logistics"))>
                                "3. Логист." {move || sort.with(|s| s.indicator("logistics"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: right;") on:click=move |_| sort.update(|s| *s = s.toggle("acquiring"))>
                                "4. Эквайр." {move || sort.with(|s| s.indicator("acquiring"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: right;") on:click=move |_| sort.update(|s| *s = s.toggle("commission"))>
                                "5. Комиссия" {move || sort.with(|s| s.indicator("commission"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: right;") on:click=move |_| sort.update(|s| *s = s.toggle("penalty"))>
                                "6. Штрафы" {move || sort.with(|s| s.indicator("penalty"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: right;") on:click=move |_| sort.update(|s| *s = s.toggle("other"))>
                                "7. Прочее" {move || sort.with(|s| s.indicator("other"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: right; font-weight: 700;") on:click=move |_| sort.update(|s| *s = s.toggle("result"))>
                                "8. Результат" {move || sort.with(|s| s.indicator("result"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: right;") on:click=move |_| sort.update(|s| *s = s.toggle("dealer_price"))>
                                "9. ЦенаДилер" {move || sort.with(|s| s.indicator("dealer_price"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: right; font-weight: 700;") on:click=move |_| sort.update(|s| *s = s.toggle("margin_diff"))>
                                "10. Маржа" {move || sort.with(|s| s.indicator("margin_diff"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: right;") on:click=move |_| sort.update(|s| *s = s.toggle("margin_pct"))>
                                "11. Маржа%" {move || sort.with(|s| s.indicator("margin_pct"))}
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let mut sorted = lines.clone();
                            sort.with(|s| sort_lines(&mut sorted, s));

                            let active_kinds: HashSet<String> = kind_toggles.get_value().iter()
                                .filter(|(_, sig)| sig.get())
                                .map(|(k, _)| k.clone())
                                .collect();

                            sorted.into_iter()
                                .filter(|l| active_kinds.contains(effective_kind(l)))
                                .map(|line| {
                                    let eff_kind = effective_kind(&line);
                                    let has_problems = !line.problem_codes.is_empty();
                                    let row_class = if has_problems { "data-table__row--problem" } else { "" };
                                    let label = kind_label(eff_kind);
                                    let color = kind_badge_color(eff_kind);
                                    let is_info = eff_kind == "info";

                                    // Реклама — только для Продажи (не для Возврата и остальных)
                                    let (adv_style, adv_display) = if eff_kind == "sale" {
                                        (signed_style(line.advertising), format_money(line.advertising))
                                    } else {
                                        ("text-align: right; color: var(--color-text-secondary);".to_string(), "—".to_string())
                                    };

                                    let order_id = line.order_id.clone();
                                    let order_date_display = line.order_date.clone().map(|d| format_date(&d));
                                    let order_is_cancelled = line.order_is_cancelled;

                                    let sales_doc_id = line.sales_doc_id.clone();
                                    let sales_doc_no = line.sales_doc_no.clone();
                                    let sales_sale_id = line.sales_sale_id.clone();
                                    let extra_count = line.sales_extra_ids.len();

                                    let margin_pct_display = if !is_info && line.revenue.abs() > 0.01 {
                                        format!("{:.1}%", line.margin_diff / line.revenue.abs() * 100.0)
                                    } else {
                                        "—".to_string()
                                    };

                                    let p903_ref_id = line.p903_ref_id.clone();
                                    let rrd_id_display = line.p903_rrd_id.map(|n| n.to_string()).unwrap_or_else(|| "—".to_string());
                                    let nomenclature_ref = line.nomenclature_ref.clone();
                                    let sa_name_str = line.sa_name.clone().unwrap_or_else(|| "—".to_string());

                                    let tabs_for_order = tabs_store.clone();
                                    let tabs_for_sales = tabs_store.clone();
                                    let tabs_for_nom = tabs_store.clone();
                                    let tabs_for_p903 = tabs_store.clone();

                                    let srid_display = if line.srid.is_empty() { "—".to_string() } else { line.srid.clone() };
                                    let is_sale_or_return = eff_kind == "sale" || eff_kind == "sale_return" || eff_kind == "return";

                                    view! {
                                        <tr class=row_class>
                                            // Артикул (sticky)
                                            <td style=format!("{CELL} position: sticky; left: 0; z-index: 1; background: inherit;")>
                                                {if has_problems {
                                                    view! { <span style="color: var(--color-warning); margin-right: 2px;">"⚠"</span> }.into_any()
                                                } else { view! { <span /> }.into_any() }}
                                                {match nomenclature_ref {
                                                    Some(nref) => {
                                                        let sa = sa_name_str.clone();
                                                        view! {
                                                            <button
                                                                style="cursor: pointer; text-decoration: underline; background: none; border: none; padding: 0; font-size: inherit; color: var(--color-link);"
                                                                on:click=move |_| {
                                                                    let key = format!("a004_nomenclature_details_{}", nref);
                                                                    tabs_for_nom.open_tab(&key, &sa);
                                                                }
                                                            >{sa_name_str}</button>
                                                        }.into_any()
                                                    }
                                                    None => view! { <span style="color: var(--color-text-secondary);">{sa_name_str}</span> }.into_any(),
                                                }}
                                            </td>
                                            // srid
                                            <td style=format!("{CELL} font-size: 0.82em; color: var(--color-text-secondary);")>
                                                {srid_display}
                                            </td>
                                            // Тип
                                            <td style=format!("{CELL} text-align: center;")>
                                                <span style="font-size: 0.78em;">
                                                    <Badge appearance=BadgeAppearance::Filled color=color>{label}</Badge>
                                                </span>
                                            </td>
                                            // Заказ a015
                                            <td style=format!("{CELL} text-align: center;")>
                                                {match (order_id.clone(), order_date_display.clone()) {
                                                    (Some(oid), Some(odate)) => {
                                                        let style = if order_is_cancelled {
                                                            "text-decoration: line-through; color: var(--color-text-secondary); cursor: pointer; background: none; border: none; padding: 0; font-size: inherit;"
                                                        } else {
                                                            "cursor: pointer; text-decoration: underline; background: none; border: none; padding: 0; font-size: inherit;"
                                                        };
                                                        view! {
                                                            <button style=style
                                                                on:click=move |_| {
                                                                    let key = format!("a015_wb_orders_details_{}", oid);
                                                                    tabs_for_order.open_tab(&key, "Заказ WB");
                                                                }
                                                            >{odate}</button>
                                                        }.into_any()
                                                    }
                                                    _ => {
                                                        if is_sale_or_return {
                                                            view! { <span style="color: var(--color-danger); font-size: 0.85em;">"нет заказа"</span> }.into_any()
                                                        } else {
                                                            view! { <span style="color: var(--color-text-secondary);">"—"</span> }.into_any()
                                                        }
                                                    }
                                                }}
                                            </td>
                                            // Реализация a012
                                            <td style=format!("{CELL} text-align: center;")>
                                                {match sales_doc_id.clone() {
                                                    Some(did) => {
                                                        let col = if extra_count > 0 { "color: var(--color-warning);" } else { "" };
                                                        let dup_suffix = if extra_count > 0 { format!(" +{}", extra_count) } else { String::new() };
                                                        let display = format!(
                                                            "{}{}",
                                                            sales_sale_id.as_deref().or(sales_doc_no.as_deref()).unwrap_or("—"),
                                                            dup_suffix
                                                        );
                                                        view! {
                                                            <button
                                                                style=format!("{col} cursor: pointer; text-decoration: underline; background: none; border: none; padding: 0; font-size: 0.82em;")
                                                                on:click=move |_| {
                                                                    let key = format!("a012_wb_sales_details_{}", did);
                                                                    tabs_for_sales.open_tab(&key, "Реализация WB");
                                                                }
                                                            >{display}</button>
                                                        }.into_any()
                                                    }
                                                    None => {
                                                        if is_sale_or_return {
                                                            view! { <span style="color: var(--color-danger); font-size: 0.85em;">"нет реализации"</span> }.into_any()
                                                        } else {
                                                            view! { <span style="color: var(--color-text-secondary);">"—"</span> }.into_any()
                                                        }
                                                    }
                                                }}
                                            </td>
                                            // RRD (p903)
                                            <td style=format!("{CELL} text-align: right;")>
                                                {match p903_ref_id {
                                                    Some(pid) => {
                                                        let rrd_label = rrd_id_display.clone();
                                                        view! {
                                                            <button
                                                                style="cursor: pointer; text-decoration: underline; background: none; border: none; padding: 0; font-size: inherit; color: var(--color-link);"
                                                                on:click=move |_| {
                                                                    let key = format!("p903_wb_finance_report_details_id_{}", pid);
                                                                    tabs_for_p903.open_tab(&key, &format!("p903 {}", rrd_label));
                                                                }
                                                            >{rrd_id_display}</button>
                                                        }.into_any()
                                                    },
                                                    None => view! { <span style="color: var(--color-text-secondary);">"—"</span> }.into_any(),
                                                }}
                                            </td>
                                            // 10 финансовых колонок
                                            <td style=signed_style(line.revenue)>{format_money(line.revenue)}</td>
                                            <td style=adv_style>{adv_display}</td>
                                            <td style=signed_style(line.logistics)>{format_money(line.logistics)}</td>
                                            <td style=signed_style(line.acquiring)>{format_money(line.acquiring)}</td>
                                            <td style=signed_style(line.commission)>{format_money(line.commission)}</td>
                                            <td style=signed_style(line.penalty)>{format_money(line.penalty)}</td>
                                            <td style=signed_style(line.other)>{format_money(line.other)}</td>
                                            <td style={format!("{}; font-weight: 700;", signed_style(line.result))}>{format_money(line.result)}</td>
                                            <td style=format!("{} text-align: right;", CELL)>
                                                {if is_info {
                                                    view! { <span style="color: var(--color-text-secondary);">"—"</span> }.into_any()
                                                } else {
                                                    view! { <span style=signed_style(line.dealer_price)>{format_money(line.dealer_price)}</span> }.into_any()
                                                }}
                                            </td>
                                            <td style=format!("{} text-align: right; font-weight: 700;", CELL)>
                                                {if is_info {
                                                    view! { <span style="color: var(--color-text-secondary);">"—"</span> }.into_any()
                                                } else {
                                                    view! { <span style=signed_style(line.margin_diff)>{format_money(line.margin_diff)}</span> }.into_any()
                                                }}
                                            </td>
                                            <td style=format!("{} text-align: right;", CELL)>
                                                {if is_info {
                                                    view! { <span style="color: var(--color-text-secondary);">"—"</span> }.into_any()
                                                } else {
                                                    view! { <span>{margin_pct_display}</span> }.into_any()
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
    }
}

#[component]
fn ResultTab(doc: WbDayCloseDetailDto) -> impl IntoView {
    let lines = doc.lines;
    let totals = doc.totals.clone();

    // Продажи: kind=sale, srid НЕ начинается с R
    let sales_count = lines.iter().filter(|l| effective_kind(l) == "sale").count();
    let sales_qty: i64 = lines
        .iter()
        .filter(|l| effective_kind(l) == "sale")
        .map(|l| l.qty_sold)
        .sum();
    let sales_revenue: f64 = lines
        .iter()
        .filter(|l| effective_kind(l) == "sale")
        .map(|l| l.revenue)
        .sum();

    // Возвраты: sale_return (kind=sale, srid начинается с R) + return
    let ret_count = lines
        .iter()
        .filter(|l| effective_kind(l) == "sale_return" || l.kind == "return")
        .count();
    let ret_qty: i64 = lines
        .iter()
        .filter(|l| effective_kind(l) == "sale_return" || l.kind == "return")
        .map(|l| l.qty_returned)
        .sum();
    let ret_revenue: f64 = lines
        .iter()
        .filter(|l| effective_kind(l) == "sale_return" || l.kind == "return")
        .map(|l| l.revenue)
        .sum();

    // Возмещения (transport_storage_reimbursement)
    let reimb_count = lines
        .iter()
        .filter(|l| l.kind == "transport_storage_reimbursement")
        .count();
    let reimb_total: f64 = lines
        .iter()
        .filter(|l| l.kind == "transport_storage_reimbursement")
        .map(|l| l.revenue)
        .sum();

    let last_recalc = doc
        .last_recalculated_at
        .as_deref()
        .map(format_datetime_msk)
        .unwrap_or_else(|| "—".to_string());

    let ret_style = if ret_revenue < -0.01 {
        "text-align: right; color: var(--color-danger);"
    } else {
        "text-align: right;"
    };

    let totals_copy_tsv = build_totals_excel_tsv(&totals);
    let totals_copied = RwSignal::new(false);

    view! {
        <div class="detail-grid" style="padding: var(--spacing-md);">
            // Левая колонка: сводка реализации/возвратов/возмещений
            <div class="detail-grid__col">
                <CardAnimated delay_ms=0 nav_id="a033_wb_day_close_result_summary">
                    <h4 class="details-section__title">"Реализация и возвраты"</h4>
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell>"Показатель"</TableHeaderCell>
                                <TableHeaderCell attr:style="text-align: right;">"Кол-во / Строк"</TableHeaderCell>
                                <TableHeaderCell attr:style="text-align: right; min-width: 120px;">"Сумма"</TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            <TableRow>
                                <TableCell><TableCellLayout>
                                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Success>"Продажа"</Badge>
                                </TableCellLayout></TableCell>
                                <TableCell attr:style="text-align: right;">
                                    <TableCellLayout>{format!("{} шт. / {} строк", sales_qty, sales_count)}</TableCellLayout>
                                </TableCell>
                                <TableCell attr:style="text-align: right; color: var(--color-success);">
                                    <TableCellLayout><strong>{format_money(sales_revenue)}</strong></TableCellLayout>
                                </TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>
                                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Warning>"Возврат"</Badge>
                                </TableCellLayout></TableCell>
                                <TableCell attr:style="text-align: right;">
                                    <TableCellLayout>{format!("{} шт. / {} строк", ret_qty, ret_count)}</TableCellLayout>
                                </TableCell>
                                <TableCell attr:style=ret_style>
                                    <TableCellLayout><strong>{format_money(ret_revenue)}</strong></TableCellLayout>
                                </TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>
                                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Informative>"Возмещение"</Badge>
                                </TableCellLayout></TableCell>
                                <TableCell attr:style="text-align: right;">
                                    <TableCellLayout>{format!("{} строк", reimb_count)}</TableCellLayout>
                                </TableCell>
                                <TableCell attr:style="text-align: right;">
                                    <TableCellLayout><strong>{format_money(reimb_total)}</strong></TableCellLayout>
                                </TableCell>
                            </TableRow>
                        </TableBody>
                    </Table>
                </CardAnimated>

                <CardAnimated delay_ms=40 nav_id="a033_wb_day_close_result_info">
                    <h4 class="details-section__title">"Общие сведения"</h4>
                    <Table>
                        <TableBody>
                            <TableRow>
                                <TableCell><TableCellLayout>"Всего строк"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><strong>{totals.lines_count}</strong></TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"Дата обновления"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout>{last_recalc}</TableCellLayout></TableCell>
                            </TableRow>
                            {if totals.problem_lines > 0 {
                                view! {
                                    <TableRow>
                                        <TableCell><TableCellLayout>"Проблемных строк"</TableCellLayout></TableCell>
                                        <TableCell attr:style="color: var(--color-warning);"><TableCellLayout>
                                            <strong>{format!("{} из {}", totals.problem_lines, totals.lines_count)}</strong>
                                        </TableCellLayout></TableCell>
                                    </TableRow>
                                }.into_any()
                            } else {
                                view! {
                                    <TableRow>
                                        <TableCell><TableCellLayout>"Проблем"</TableCellLayout></TableCell>
                                        <TableCell attr:style="color: var(--color-success);"><TableCellLayout>"нет"</TableCellLayout></TableCell>
                                    </TableRow>
                                }.into_any()
                            }}
                        </TableBody>
                    </Table>
                </CardAnimated>
            </div>

            // Правая колонка: итоги по всем 10 колонкам
            <div class="detail-grid__col">
                <CardAnimated delay_ms=80 nav_id="a033_wb_day_close_result_totals">
                    <div style="display:flex;align-items:center;justify-content:space-between;gap:var(--spacing-md);margin-bottom:var(--spacing-sm);flex-wrap:wrap;">
                        <h4 class="details-section__title" style="margin:0;">"Итоги по колонкам"</h4>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            size=ButtonSize::Small
                            on_click=move |_| {
                                totals_copied.set(false);
                                let copied = totals_copied;
                                copy_to_clipboard_with_callback(&totals_copy_tsv, move || copied.set(true));
                            }
                        >
                            {icon("copy")}
                            {move || if totals_copied.get() { " Скопировано" } else { " Копировать в Excel" }}
                        </Button>
                    </div>
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell>"Колонка"</TableHeaderCell>
                                <TableHeaderCell attr:style="text-align: right; min-width: 130px;">"Сумма"</TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            <TableRow>
                                <TableCell><TableCellLayout>"1. Реализация"</TableCellLayout></TableCell>
                                <TableCell attr:style=signed_style(totals.revenue)><TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                                    <strong>{format_money(totals.revenue)}</strong>
                                </TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"2. Реклама"</TableCellLayout></TableCell>
                                <TableCell attr:style=signed_style(totals.advertising)><TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                                    {format_money(totals.advertising)}
                                </TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"3. Логистика"</TableCellLayout></TableCell>
                                <TableCell attr:style=signed_style(totals.logistics)><TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                                    {format_money(totals.logistics)}
                                </TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"4. Эквайринг"</TableCellLayout></TableCell>
                                <TableCell attr:style=signed_style(totals.acquiring)><TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                                    {format_money(totals.acquiring)}
                                </TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"5. Комиссия"</TableCellLayout></TableCell>
                                <TableCell attr:style=signed_style(totals.commission)><TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                                    {format_money(totals.commission)}
                                </TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"6. Штрафы"</TableCellLayout></TableCell>
                                <TableCell attr:style=signed_style(totals.penalty)><TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                                    {format_money(totals.penalty)}
                                </TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"7. Прочее"</TableCellLayout></TableCell>
                                <TableCell attr:style=signed_style(totals.other)><TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                                    {format_money(totals.other)}
                                </TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout><strong>"8. Результат"</strong></TableCellLayout></TableCell>
                                <TableCell attr:style=format!("{}; font-weight: 700;", signed_style(totals.result))><TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                                    <strong>{format_money(totals.result)}</strong>
                                </TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"9. Цена дилер"</TableCellLayout></TableCell>
                                <TableCell attr:style=signed_style(totals.dealer_price)><TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                                    {format_money(totals.dealer_price)}
                                </TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout><strong>"10. Маржа"</strong></TableCellLayout></TableCell>
                                <TableCell attr:style=format!("{}; font-weight: 700;", signed_style(totals.margin_diff))><TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                                    <strong>{format_money(totals.margin_diff)}</strong>
                                </TableCellLayout></TableCell>
                            </TableRow>
                        </TableBody>
                    </Table>
                </CardAnimated>
            </div>
        </div>
    }
}

#[component]
fn TotalsRow(totals: WbDayCloseTotalsDto) -> impl IntoView {
    view! {
        <div style="padding: var(--spacing-sm) var(--spacing-md); background: var(--color-bg-elevated); border-top: 2px solid var(--color-border); display: flex; gap: var(--spacing-lg); flex-wrap: wrap; font-size: 0.88em; align-items: center;">
            <strong>"Итого:"</strong>
            <span>"Строк: " <strong>{totals.lines_count}</strong></span>
            <span>"Реализация: " <strong>{format_money(totals.revenue)}</strong></span>
            <span>"Реклама: " <strong>{format_money(totals.advertising)}</strong></span>
            <span>"Логистика: " {format_money(totals.logistics)}</span>
            <span>"Комиссия: " {format_money(totals.commission)}</span>
            <span>"Результат: " <strong>{format_money(totals.result)}</strong></span>
            <span>"Маржа: " <strong>{format_money(totals.margin_diff)}</strong></span>
            {if totals.problem_lines > 0 {
                view! {
                    <span style="color: var(--color-warning); font-weight: 600;">
                        "Проблемных строк: " {totals.problem_lines} " из " {totals.lines_count}
                    </span>
                }.into_any()
            } else {
                view! {
                    <span style="color: var(--color-success);">{icon("check-circle")} " Строки без проблем"</span>
                }.into_any()
            }}
        </div>
    }
}
