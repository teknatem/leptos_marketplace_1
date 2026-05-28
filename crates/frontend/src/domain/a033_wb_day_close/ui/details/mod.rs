use crate::domain::a033_wb_day_close::ui::list::{format_date, format_money, WbDayCloseListDto};
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::clipboard::copy_to_clipboard_with_callback;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::components::more_actions_menu::{use_more_actions_close, MoreActionsMenu};
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::system::favorites::ui::FavoriteButton;
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
    pub sales_doc_date: Option<String>,
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
    #[serde(default)]
    pub a012_advert_expense: Option<f64>,
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
pub struct WbDayCloseAdvertNoOrderLineDto {
    pub projection_ref_id: String,
    pub nomenclature_ref: Option<String>,
    pub sa_name: Option<String>,
    pub amount: f64,
    pub general_ledger_ref: Option<String>,
    pub campaign_code: String,
    pub campaign_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WbDayCloseAdvertOrderAccrualLineDto {
    pub projection_ref_id: String,
    pub nomenclature_ref: Option<String>,
    pub sa_name: Option<String>,
    pub amount: f64,
    pub order_key: String,
    pub order_id: Option<String>,
    pub order_date: Option<String>,
    pub campaign_code: String,
    pub campaign_ref: Option<String>,
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
    #[serde(default)]
    pub advert_clicks_no_order_lines: Vec<WbDayCloseAdvertNoOrderLineDto>,
    #[serde(default)]
    pub advert_clicks_order_accrual_lines: Vec<WbDayCloseAdvertOrderAccrualLineDto>,
    #[serde(default)]
    pub gl_advert_no_order: f64,
    #[serde(default)]
    pub gl_advert_order_accrual: f64,
    #[serde(default)]
    pub gl_advert_order_expense: f64,
    #[serde(default)]
    pub snap_advert_order_expense: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Live advert totals DTO
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct RegistratorRowDto {
    pub registrator_ref: String,
    pub p913_sum: f64,
    pub p913_rows: u64,
    pub gl_sum: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct AdvertLiveTotalsDto {
    pub p913_no_order: f64,
    pub p913_order_accrual: f64,
    pub p913_order_expense: f64,
    pub gl_no_order: f64,
    pub gl_order_accrual: f64,
    pub gl_order_expense: f64,
    pub p913_accrual_rows: u64,
    pub gl_accrual_entries: u64,
    pub accrual_by_registrator: Vec<RegistratorRowDto>,
}

async fn fetch_advert_live(id: &str) -> Result<AdvertLiveTotalsDto, String> {
    let url = format!("{}/api/a033/wb-day-close/{}/advert-live", api_base(), id);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Network: {}", e))?;
    resp.json::<AdvertLiveTotalsDto>()
        .await
        .map_err(|e| format!("Parse: {}", e))
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

#[derive(Deserialize)]
struct EntityIdDto {
    id: String,
}

async fn resolve_wb_order_id_by_srid(srid: &str) -> Option<String> {
    let url = format!(
        "{}/api/a015/wb-orders/search-by-srid?srid={}",
        api_base(),
        urlencoding::encode(srid)
    );
    let response = Request::get(&url).send().await.ok()?;
    if !response.ok() {
        return None;
    }
    let rows: Vec<EntityIdDto> = response.json().await.ok()?;
    rows.into_iter().next().map(|r| r.id)
}

fn open_wb_order_by_srid(srid: String, tabs: AppGlobalContext) {
    if srid.is_empty() {
        return;
    }
    spawn_local(async move {
        if let Some(id) = resolve_wb_order_id_by_srid(&srid).await {
            tabs.open_tab(&format!("a015_wb_orders_details_{}", id), "Заказ WB");
        }
    });
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
    let lines_sort = RwSignal::new(SortState::new("sa_name")); // persists across tab switches

    let stored_id = StoredValue::new(id.clone());
    let cabinets: RwSignal<Vec<(String, String)>> = RwSignal::new(vec![]);

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(conns) = fetch_connections().await {
                let opts: Vec<(String, String)> = conns
                    .into_iter()
                    .map(|c| {
                        let label = if c.base.description.trim().is_empty() {
                            c.base.code.clone()
                        } else {
                            c.base.description.clone()
                        };
                        (c.base.id.as_string(), label)
                    })
                    .collect();
                cabinets.set(opts);
            }
        });
    });

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

    let favorite_tab_key = format!("a033_wb_day_close_details_{}", stored_id.get_value());
    let favorite_title = Signal::derive(move || {
        doc.get()
            .map(|d| format!("Закрытие дня WB — {}", format_date(&d.business_date)))
            .unwrap_or_else(|| "Закрытие дня WB".to_string())
    });

    view! {
        <PageFrame page_id="a033_wb_day_close_details" category="detail">
            // Header
            <div class="page__header">
                <div class="page__header-left">
                    <FavoriteButton
                        target_kind="a033_wb_day_close_details".to_string()
                        target_id=favorite_tab_key.clone()
                        target_title=favorite_title
                        tab_key=favorite_tab_key
                    />
                    <div style="display: flex; flex-direction: column; gap: 2px;">
                        <div style="display: flex; align-items: center; gap: var(--spacing-sm);">
                            <h1 class="page__title">
                                {move || favorite_title.get()}
                            </h1>
                            {move || doc.get().map(|d| {
                                if d.is_archived {
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
                                }
                            })}
                        </div>
                    </div>
                </div>
                <div class="page__header-right">
                    // ── Провести ──────────────────────────────────────────────────────
                    <Button
                        appearance=ButtonAppearance::Subtle
                        size=ButtonSize::Medium
                        disabled=Signal::derive(move || action_loading.get() || loading.get())
                        on_click=on_recalculate
                    >
                        <span class="page-action-button__content">
                            <span class="page-action-button__icon">{icon("zap")}</span>
                            <span class="page-action-button__text">"Провести"</span>
                        </span>
                    </Button>

                    // ── Ещё... (dropdown) ─────────────────────────────────────────────
                    <MoreActionsMenu>
                        // Перепровести проблемные
                        <button
                            class="theme-dropdown__item"
                            disabled=move || action_loading.get() || loading.get()
                            on:click=move |_| {
                                use_more_actions_close();
                                do_repost_all.get_value()();
                            }
                        >
                            <span style="display: flex; align-items: center; gap: 8px;">
                                {icon("zap")} "Перепровести проблемные"
                            </span>
                        </button>
                        // Архив + новый (только для активного документа)
                        {move || {
                            if doc.get().map(|d| !d.is_archived).unwrap_or(false) {
                                view! {
                                    <button
                                        class="theme-dropdown__item"
                                        disabled=move || action_loading.get()
                                        on:click=move |_| {
                                            use_more_actions_close();
                                            set_show_archive_form.update(|v| *v = !*v);
                                        }
                                    >
                                        <span style="display: flex; align-items: center; gap: 8px;">
                                            {icon("archive")} "Архив + новый"
                                        </span>
                                    </button>
                                }.into_any()
                            } else {
                                view! { <span /> }.into_any()
                            }
                        }}
                        // Версии
                        <button
                            class="theme-dropdown__item"
                            on:click=move |e| {
                                use_more_actions_close();
                                on_load_versions(e);
                            }
                        >
                            <span style="display: flex; align-items: center; gap: 8px;">
                                {icon("git-compare")} "Версии"
                            </span>
                        </button>
                    </MoreActionsMenu>

                    // ── Закрыть ───────────────────────────────────────────────────────
                    <Button
                        appearance=ButtonAppearance::Subtle
                        size=ButtonSize::Medium
                        on_click=move |_| on_close.run(())
                    >
                        <span class="page-action-button__content">
                            <span class="page-action-button__icon page-action-button__icon--close">{icon("x")}</span>
                            <span class="page-action-button__text">"Закрыть"</span>
                        </span>
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
                    let visible_probs_list = visible_problems(&d.problems, &d.lines);
                    let total_probs = visible_probs_list.len() as i64;
                    let tabs_store_inner = tabs_store.clone();
                    let lines = d.lines.clone();
                    let problems = visible_probs_list;
                    let advert_no_order = d.advert_clicks_no_order_lines.clone();
                    let advert_order_accrual = d.advert_clicks_order_accrual_lines.clone();
                    let total_advert = advert_no_order.len() + advert_order_accrual.len();
                    let d_for_result = d.clone();

                    let cabinet = cabinets.with(|cabs| {
                        cabs.iter()
                            .find(|(cid, _)| cid == &d.connection_id)
                            .map(|(_, label)| label.clone())
                            .unwrap_or_else(|| {
                                d.connection_id[..d.connection_id.len().min(8)].to_string()
                            })
                    });
                    view! {
                        <div style="display: flex; flex-direction: column; height: 100%; overflow: hidden;">
                            // Tabs navigation
                            <div style="border-bottom: 1px solid var(--color-border);">
                                <TabList selected_value=selected_tab>
                                <Tab value="result".to_string()>
                                    "Результат"
                                </Tab>
                                <Tab value="lines".to_string()>
                                    {format!("Строки ({})", lines_count)}
                                </Tab>
                                <Tab value="problems".to_string()>
                                    {if total_probs > 0 {
                                        format!("Проблемы ({})", total_probs)
                                    } else {
                                        "Проблемы".to_string()
                                    }}
                                </Tab>
                                <Tab value="advert".to_string()>
                                    {if total_advert > 0 {
                                        format!("Реклама ({})", total_advert)
                                    } else {
                                        "Реклама".to_string()
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
                                        <ResultTab doc=d_for_result.clone() cabinet=cabinet.clone() doc_id=d_for_result.id.clone() />
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
                                    let fin_date_probs = fin_date_mismatch_problems(&probs_for_tab);
                                    let business_date_for_probs = d.business_date.clone();
                                    let tabs_for_fin_date = tabs_store.clone();
                                    let tabs_for_probs = tabs_store.clone();
                                    view! {
                                        <div style="padding: var(--spacing-md); display: flex; flex-direction: column; gap: var(--spacing-md);">

                                            // ── Блок 1: расхождение дат a012 / fin report ────────────────────
                                            {if !fin_date_probs.is_empty() {
                                                view! {
                                                    <CardAnimated delay_ms=0 nav_id="a033_wb_day_close_problems_fin_date">
                                                        <h4 class="details-section__title">
                                                            {format!(
                                                                "Расхождение дат a012 / fin report ({})",
                                                                fin_date_probs.len()
                                                            )}
                                                        </h4>
                                                        <p style="margin: 0 0 var(--spacing-sm); color: var(--color-text-secondary);">
                                                            "Дата реализации a012 ("
                                                            <code>"sale_date"</code>
                                                            ") не совпадает с датой фин. отчёта WB ("
                                                            <code>"rr_dt"</code>
                                                            "). Дата документа — "
                                                            {format_date(&business_date_for_probs)}
                                                            ". GL "
                                                            <code>"advert_clicks_order_expense"</code>
                                                            " проводится по "
                                                            <code>"sale_date"</code>
                                                            "."
                                                        </p>
                                                        <div style="overflow-x: auto;">
                                                            <table class="data-table" style="width: 100%;">
                                                                <thead>
                                                                    <tr>
                                                                        <th>"Документ a012"</th>
                                                                        <th>"srid"</th>
                                                                        <th>"sale_date"</th>
                                                                        <th>"fin report (rr_dt)"</th>
                                                                        <th style="text-align: right;">"Реклама GL"</th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody>
                                                                    {fin_date_probs.into_iter().map(|p| {
                                                                        let a012_id = p.a012_ids.first().cloned();
                                                                        let srid_opt = p.srid.clone();
                                                                        let srid_display = p.srid.as_deref().unwrap_or("—");
                                                                        let srid_label = srid_display.chars().take(24).collect::<String>();
                                                                        let sale_date = fin_date_mismatch_sale_date(&p.message);
                                                                        let fin_report = fin_date_mismatch_fin_report(&p.message);
                                                                        let tabs = tabs_for_fin_date.clone();
                                                                        let tabs_srid = tabs.clone();
                                                                        view! {
                                                                            <tr>
                                                                                <td>
                                                                                    {match a012_id {
                                                                                        Some(id) => view! {
                                                                                            <button
                                                                                                style=LINK_BTN
                                                                                                on:click=move |_| {
                                                                                                    let key = format!("a012_wb_sales_details_{}", id);
                                                                                                    tabs.open_tab(&key, "Реализация WB");
                                                                                                }
                                                                                            >{format_date(&sale_date)}</button>
                                                                                        }.into_any(),
                                                                                        None => view! { <span>"—"</span> }.into_any(),
                                                                                    }}
                                                                                </td>
                                                                                <td>
                                                                                    {problem_srid_cell(srid_opt, srid_label, tabs_srid)}
                                                                                </td>
                                                                                <td>{format_date(&sale_date)}</td>
                                                                                <td style="color: var(--color-warning);">{format_date(&fin_report)}</td>
                                                                                <td style="text-align: right; font-variant-numeric: tabular-nums;">
                                                                                    {p.a012_advert_expense.map(format_money).unwrap_or_else(|| "—".to_string())}
                                                                                </td>
                                                                            </tr>
                                                                        }
                                                                    }).collect_view()}
                                                                </tbody>
                                                            </table>
                                                        </div>
                                                    </CardAnimated>
                                                }.into_any()
                                            } else {
                                                view! { <span /> }.into_any()
                                            }}

                                            // ── Блок 2: все проблемы ────────────────────────────────────────
                                            <CardAnimated delay_ms=30 nav_id="a033_wb_day_close_problems_list">
                                                <div style="display: flex; align-items: center; justify-content: space-between; gap: var(--spacing-sm); flex-wrap: wrap; margin-bottom: var(--spacing-sm);">
                                                    <h4 class="details-section__title" style="margin: 0;">
                                                        {format!("Список проблем ({})", probs_for_tab.len())}
                                                    </h4>
                                                    <div style="display: flex; gap: var(--spacing-xs); align-items: center; flex-wrap: wrap;">
                                                        <Button size=ButtonSize::Small
                                                            appearance=Signal::derive(move || if problems_filter.get() == "all" { ButtonAppearance::Primary } else { ButtonAppearance::Secondary })
                                                            on_click=move |_| problems_filter.set("all".to_string())
                                                        >"Все"</Button>
                                                        <Button size=ButtonSize::Small
                                                            appearance=Signal::derive(move || if problems_filter.get() == "block" { ButtonAppearance::Primary } else { ButtonAppearance::Secondary })
                                                            on_click=move |_| problems_filter.set("block".to_string())
                                                        >"Block"</Button>
                                                        <Button size=ButtonSize::Small
                                                            appearance=Signal::derive(move || if problems_filter.get() == "warn" { ButtonAppearance::Primary } else { ButtonAppearance::Secondary })
                                                            on_click=move |_| problems_filter.set("warn".to_string())
                                                        >"Warn"</Button>
                                                        <Button appearance=ButtonAppearance::Primary size=ButtonSize::Small
                                                            disabled=Signal::derive(move || action_loading.get())
                                                            on_click=on_repost_all_tab
                                                        >
                                                            {icon("zap")} " Перепровести"
                                                        </Button>
                                                    </div>
                                                </div>
                                                <div style="overflow-x: auto;">
                                                    <table class="data-table" style="width: 100%;">
                                                        <thead>
                                                            <tr>
                                                                <th style="width: 80px;">"Серьёзность"</th>
                                                                <th>"Описание проблемы"</th>
                                                                <th style="width: 140px;">"srid"</th>
                                                                <th style="text-align: right; width: 90px;">"Реклама"</th>
                                                                <th style="text-align: center; width: 60px;">"a012"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>
                                                            {move || {
                                                                let f = problems_filter.get();
                                                                probs_for_tab.iter().filter(|p| {
                                                                    match f.as_str() {
                                                                        "block" => p.severity == "block",
                                                                        "warn"  => p.severity == "warn",
                                                                        _       => true,
                                                                    }
                                                                }).map(|p| {
                                                                    let (badge_color, severity_label) = match p.severity.as_str() {
                                                                        "block" => (BadgeColor::Danger, "Block"),
                                                                        "warn"  => (BadgeColor::Warning, "Warn"),
                                                                        _       => (BadgeColor::Informative, "Info"),
                                                                    };
                                                                    let srid_opt = p.srid.clone();
                                                                    let srid_display = p.srid.as_deref().unwrap_or("—");
                                                                    let srid_short = srid_display.chars().take(22).collect::<String>();
                                                                    let a012_count = p.a012_ids.len();
                                                                    let tabs_prob = tabs_for_probs.clone();
                                                                    view! {
                                                                        <tr>
                                                                            <td style="white-space: nowrap;">
                                                                                <Badge appearance=BadgeAppearance::Filled color=badge_color>{severity_label}</Badge>
                                                                            </td>
                                                                            <td>
                                                                                <div>{p.message.clone()}</div>
                                                                                <div style="margin-top: 2px;">
                                                                                    <code style="font-size: 0.78em; color: var(--color-text-secondary);">{p.code.clone()}</code>
                                                                                </div>
                                                                            </td>
                                                                            <td>
                                                                                {problem_srid_cell(srid_opt, srid_short, tabs_prob)}
                                                                            </td>
                                                                            <td style="text-align: right; font-variant-numeric: tabular-nums; white-space: nowrap;">
                                                                                {p.a012_advert_expense.map(format_money).unwrap_or_else(|| "—".to_string())}
                                                                            </td>
                                                                            <td style="text-align: center; color: var(--color-text-secondary);">
                                                                                {if a012_count == 0 { "—".to_string() } else { a012_count.to_string() }}
                                                                            </td>
                                                                        </tr>
                                                                    }
                                                                }).collect_view()
                                                            }}
                                                        </tbody>
                                                    </table>
                                                </div>
                                            </CardAnimated>

                                        </div>
                                    }.into_any()
                                }}
                                // Tab panel: Реклама
                                {move || {
                                    if selected_tab.get() != "advert" {
                                        return view! { <span /> }.into_any();
                                    }
                                    view! {
                                        <AdvertTab
                                            no_order=advert_no_order.clone()
                                            order_accrual=advert_order_accrual.clone()
                                            tabs_store=tabs_store.clone()
                                        />
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
                                <li>"Дозаполняются реализации a012 по заказам из финотчёта (sale_date не позже даты закрытия; продажи — сумма>0, возвраты — сумма<0), затем пересчитываются строки p903, реклама, логистика и комиссии."</li>
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

fn normalize_money(value: f64) -> f64 {
    if value.abs() < 0.005 {
        0.0
    } else {
        value
    }
}

fn signed_style(value: f64) -> String {
    let value = normalize_money(value);
    if value > 0.0 {
        "text-align: right; color: var(--color-success);".to_string()
    } else if value < 0.0 {
        "text-align: right; color: var(--color-danger);".to_string()
    } else {
        "text-align: right; color: var(--color-text-secondary);".to_string()
    }
}

fn format_money_excel(value: f64) -> String {
    format!("{:.2}", normalize_money(value)).replace('.', ",")
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

fn is_info_line(line: &WbDayCloseLineDto) -> bool {
    effective_kind(line) == "info"
}

fn line_has_visible_problems(line: &WbDayCloseLineDto) -> bool {
    !is_info_line(line) && !line.problem_codes.is_empty()
}

fn info_line_srids(lines: &[WbDayCloseLineDto]) -> HashSet<String> {
    lines
        .iter()
        .filter(|l| is_info_line(l))
        .filter(|l| !l.srid.is_empty())
        .map(|l| l.srid.clone())
        .collect()
}

fn visible_problems(
    problems: &[WbDayCloseProblemDto],
    lines: &[WbDayCloseLineDto],
) -> Vec<WbDayCloseProblemDto> {
    let info_srids = info_line_srids(lines);
    problems
        .iter()
        .filter(|p| {
            p.srid
                .as_ref()
                .map(|s| !info_srids.contains(s))
                .unwrap_or(true)
        })
        .cloned()
        .collect()
}

fn count_visible_problem_lines(lines: &[WbDayCloseLineDto]) -> i64 {
    lines
        .iter()
        .filter(|l| line_has_visible_problems(l))
        .count() as i64
}

fn fin_date_mismatch_problems(problems: &[WbDayCloseProblemDto]) -> Vec<WbDayCloseProblemDto> {
    problems
        .iter()
        .filter(|p| p.code == "a012_sale_date_mismatch_fin_report")
        .cloned()
        .collect()
}

fn parse_fin_date_mismatch_field(message: &str, prefix: &str) -> Option<String> {
    let start = message.find(prefix)? + prefix.len();
    let rest = &message[start..];
    if rest.len() >= 10 && rest.as_bytes()[4] == b'-' && rest.as_bytes()[7] == b'-' {
        Some(rest[..10].to_string())
    } else {
        None
    }
}

fn fin_date_mismatch_sale_date(message: &str) -> String {
    parse_fin_date_mismatch_field(message, "sale_date=").unwrap_or_else(|| "—".to_string())
}

fn fin_date_mismatch_fin_report(message: &str) -> String {
    parse_fin_date_mismatch_field(message, "fin_report(rr_dt)=").unwrap_or_else(|| "—".to_string())
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
            "sales_doc" => a.sales_doc_date.cmp(&b.sales_doc_date),
            "problems" => {
                let a_has = line_has_visible_problems(a);
                let b_has = line_has_visible_problems(b);
                b_has.cmp(&a_has)
            }
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
const KIND_COL: &str = "white-space: nowrap; min-width: 80px; width: 80px; max-width: 80px; text-align: left; padding: 0 4px;";
const LINK_BTN: &str = "cursor: pointer; text-decoration: underline; background: none; border: none; padding: 0; font-size: inherit; color: var(--color-link);";

fn problem_srid_cell(srid: Option<String>, label: String, tabs: AppGlobalContext) -> AnyView {
    let Some(srid) = srid.filter(|s| !s.is_empty()) else {
        return view! { <span>{label}</span> }.into_any();
    };
    view! {
        <button
            style=LINK_BTN
            on:click=move |_| open_wb_order_by_srid(srid.clone(), tabs.clone())
        >{label}</button>
    }
    .into_any()
}
const REF_CELL: &str = "text-align: right;";
const DATE_COL: &str = "text-align: right; min-width: 80px; width: 80px; max-width: 80px; white-space: nowrap; padding: 0 4px;";

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
            <div style="display: flex; gap: var(--spacing-xl); flex-wrap: wrap; padding: var(--spacing-xs) var(--spacing-md); background: var(--color-bg-elevated); border-bottom: 1px solid var(--color-border); align-items: center;">
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
                <table class="data-table" style="min-width: 1200px; font-size: 11px;">
                    <thead>
                        <tr>
                            <th
                                style=format!("{TH_LINK} width: 28px; min-width: 28px; text-align: center; padding: 0 4px;")
                                on:click=move |_| sort.update(|s| *s = s.toggle("problems"))
                            >
                                "⚠" {move || sort.with(|s| s.indicator("problems"))}
                            </th>
                            <th
                                style=format!("{TH_LINK} min-width: 80px; width: 80px; max-width: 80px; text-align: center; padding: 0 4px;")
                                on:click=move |_| sort.update(|s| *s = s.toggle("kind"))
                            >
                                "Тип" {move || sort.with(|s| s.indicator("kind"))}
                            </th>
                            <th
                                style=format!("{TH_LINK} min-width: 80px; width: 80px; max-width: 80px; text-align: center; padding: 0 4px;")
                                on:click=move |_| sort.update(|s| *s = s.toggle("sa_name"))
                            >
                                "Артикул" {move || sort.with(|s| s.indicator("sa_name"))}
                            </th>
                            <th
                                style=format!("{TH_LINK} min-width: 80px; width: 80px; max-width: 80px; text-align: center; padding: 0 4px;")
                                on:click=move |_| sort.update(|s| *s = s.toggle("order_date"))
                            >
                                "Заказ (a015)" {move || sort.with(|s| s.indicator("order_date"))}
                            </th>
                            <th
                                style=format!("{TH_LINK} min-width: 80px; width: 80px; max-width: 80px; text-align: center; padding: 0 4px;")
                                on:click=move |_| sort.update(|s| *s = s.toggle("sales_doc"))
                            >
                                "Реализация (a012)" {move || sort.with(|s| s.indicator("sales_doc"))}
                            </th>
                            <th
                                style=format!("{TH_LINK} text-align: center;")
                                on:click=move |_| sort.update(|s| *s = s.toggle("rrd_id"))
                            >
                                "RRD" {move || sort.with(|s| s.indicator("rrd_id"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: center;") on:click=move |_| sort.update(|s| *s = s.toggle("revenue"))>
                                "1. Реализация" {move || sort.with(|s| s.indicator("revenue"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: center;") on:click=move |_| sort.update(|s| *s = s.toggle("advertising"))>
                                "2. Реклама" {move || sort.with(|s| s.indicator("advertising"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: center;") on:click=move |_| sort.update(|s| *s = s.toggle("logistics"))>
                                "3. Логист." {move || sort.with(|s| s.indicator("logistics"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: center;") on:click=move |_| sort.update(|s| *s = s.toggle("acquiring"))>
                                "4. Эквайр." {move || sort.with(|s| s.indicator("acquiring"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: center;") on:click=move |_| sort.update(|s| *s = s.toggle("commission"))>
                                "5. Комиссия" {move || sort.with(|s| s.indicator("commission"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: center;") on:click=move |_| sort.update(|s| *s = s.toggle("penalty"))>
                                "6. Штрафы" {move || sort.with(|s| s.indicator("penalty"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: center;") on:click=move |_| sort.update(|s| *s = s.toggle("other"))>
                                "7. Прочее" {move || sort.with(|s| s.indicator("other"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: center; font-weight: 700;") on:click=move |_| sort.update(|s| *s = s.toggle("result"))>
                                "8. Результат" {move || sort.with(|s| s.indicator("result"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: center;") on:click=move |_| sort.update(|s| *s = s.toggle("dealer_price"))>
                                "9. ЦенаДилер" {move || sort.with(|s| s.indicator("dealer_price"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: center; font-weight: 700;") on:click=move |_| sort.update(|s| *s = s.toggle("margin_diff"))>
                                "10. Маржа" {move || sort.with(|s| s.indicator("margin_diff"))}
                            </th>
                            <th style=format!("{TH_LINK} text-align: center;") on:click=move |_| sort.update(|s| *s = s.toggle("margin_pct"))>
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
                                    let has_problems = line_has_visible_problems(&line);
                                    let row_class = if has_problems { "data-table__row--problem" } else { "" };
                                    let label = kind_label(eff_kind);
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
                                    let sales_doc_date_display =
                                        line.sales_doc_date.clone().map(|d| format_date(&d));
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

                                    let is_sale_or_return = eff_kind == "sale" || eff_kind == "sale_return" || eff_kind == "return";

                                    view! {
                                        <tr class=row_class>
                                            // ⚠
                                            <td style="text-align: center; padding: 0 4px; width: 28px;">
                                                {if has_problems {
                                                    view! { <span style="color: var(--color-warning);">"⚠"</span> }.into_any()
                                                } else {
                                                    view! { <span /> }.into_any()
                                                }}
                                            </td>
                                            // Тип
                                            <td style=KIND_COL>
                                                {label}
                                            </td>
                                            // Артикул
                                            <td style=KIND_COL>
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
                                            // Заказ a015
                                            <td style=DATE_COL>
                                                {match (order_id.clone(), order_date_display.clone()) {
                                                    (Some(oid), Some(odate)) => {
                                                        let style = if order_is_cancelled {
                                                            format!("{LINK_BTN} text-decoration: line-through underline;")
                                                        } else {
                                                            LINK_BTN.to_string()
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
                                                            view! { <span style="color: var(--color-danger);">"нет заказа"</span> }.into_any()
                                                        } else {
                                                            view! { <span style="color: var(--color-text-secondary);">"—"</span> }.into_any()
                                                        }
                                                    }
                                                }}
                                            </td>
                                            // Реализация a012
                                            <td style=DATE_COL>
                                                {match (sales_doc_id.clone(), sales_doc_date_display.clone()) {
                                                    (Some(did), Some(sdate)) => {
                                                        let dup_suffix = if extra_count > 0 {
                                                            format!(" +{}", extra_count)
                                                        } else {
                                                            String::new()
                                                        };
                                                        let display = format!("{}{}", sdate, dup_suffix);
                                                        view! {
                                                            <button
                                                                style=LINK_BTN
                                                                on:click=move |_| {
                                                                    let key = format!("a012_wb_sales_details_{}", did);
                                                                    tabs_for_sales.open_tab(&key, "Реализация WB");
                                                                }
                                                            >{display}</button>
                                                        }.into_any()
                                                    }
                                                    _ => {
                                                        if is_sale_or_return {
                                                            view! { <span style="color: var(--color-danger);">"нет реализации"</span> }.into_any()
                                                        } else {
                                                            view! { <span style="color: var(--color-text-secondary);">"—"</span> }.into_any()
                                                        }
                                                    }
                                                }}
                                            </td>
                                            // RRD (p903)
                                            <td style=REF_CELL>
                                                {match p903_ref_id {
                                                    Some(pid) => {
                                                        let rrd_label = rrd_id_display.clone();
                                                        view! {
                                                            <button
                                                                style=LINK_BTN
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
                                            // Финансовые колонки
                                            <td style=signed_style(line.revenue)>{format_money(line.revenue)}</td>
                                            <td style=adv_style>{adv_display}</td>
                                            <td style=signed_style(line.logistics)>{format_money(line.logistics)}</td>
                                            <td style=signed_style(line.acquiring)>{format_money(line.acquiring)}</td>
                                            <td style=signed_style(line.commission)>{format_money(line.commission)}</td>
                                            <td style=signed_style(line.penalty)>{format_money(line.penalty)}</td>
                                            <td style=signed_style(line.other)>{format_money(line.other)}</td>
                                            <td style=format!("{}; font-weight: 700;", signed_style(line.result))>{format_money(line.result)}</td>
                                            <td style="text-align: right;">
                                                {if is_info {
                                                    view! { <span style="color: var(--color-text-secondary);">"—"</span> }.into_any()
                                                } else {
                                                    view! { <span style=signed_style(line.dealer_price)>{format_money(line.dealer_price)}</span> }.into_any()
                                                }}
                                            </td>
                                            <td style="text-align: right; font-weight: 700;">
                                                {if is_info {
                                                    view! { <span style="color: var(--color-text-secondary);">"—"</span> }.into_any()
                                                } else {
                                                    view! { <span style=signed_style(line.margin_diff)>{format_money(line.margin_diff)}</span> }.into_any()
                                                }}
                                            </td>
                                            <td style="text-align: right;">
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
fn ResultTab(doc: WbDayCloseDetailDto, cabinet: String, doc_id: String) -> impl IntoView {
    let lines = doc.lines;
    let totals = doc.totals.clone();

    // GL-итоги из sys_general_ledger (layer=oper, по дате документа)
    let gl_no_order = doc.gl_advert_no_order;
    let gl_order_accrual = doc.gl_advert_order_accrual;
    let gl_order_expense = doc.gl_advert_order_expense;

    // Снапшот-итоги из документа (из проекций, отфильтрованных по дате)
    let snap_no_order: f64 = doc
        .advert_clicks_no_order_lines
        .iter()
        .map(|r| r.amount)
        .sum();
    let snap_order_accrual: f64 = doc
        .advert_clicks_order_accrual_lines
        .iter()
        .map(|r| r.amount)
        .sum();
    // Документ: фактические расходы на рекламу из строк a033 (через matched a012 → p913).
    // snap_advert_order_expense ≈ gl_order_expense (p913 INNER JOIN GL по дате), поэтому
    // его Δ всегда ≈ 0 и он не раскрывает расхождение документ/GL.
    // Используем -totals.advertising — реальную сумму из строк документа.
    let doc_advert_expense: f64 = -totals.advertising;

    // Живые p913/GL итоги — загружаются асинхронно для диагностики стагнации снапшота
    let live: RwSignal<Option<AdvertLiveTotalsDto>> = RwSignal::new(None);
    {
        let id = doc_id.clone();
        spawn_local(async move {
            if let Ok(data) = fetch_advert_live(&id).await {
                live.set(Some(data));
            }
        });
    }
    let visible_problem_lines = count_visible_problem_lines(&lines);

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
                                <TableCell><TableCellLayout>"Кабинет"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><strong>{cabinet.clone()}</strong></TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"Всего строк"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><strong>{totals.lines_count}</strong></TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"Дата обновления"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout>{last_recalc}</TableCellLayout></TableCell>
                            </TableRow>
                            {if visible_problem_lines > 0 {
                                view! {
                                    <TableRow>
                                        <TableCell><TableCellLayout>"Проблемных строк"</TableCellLayout></TableCell>
                                        <TableCell attr:style="color: var(--color-warning);"><TableCellLayout>
                                            <strong>{format!("{} из {}", visible_problem_lines, totals.lines_count)}</strong>
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
                <CardAnimated delay_ms=60 nav_id="a033_wb_day_close_result_advert">
                    <h4 class="details-section__title">"Реклама (сверка)"</h4>
                    {move || {
                        let live_data = live.get();
                        view! {
                        <table style="width: 100%; border-collapse: collapse; font-size: 0.88em;">
                            <thead>
                                <tr style="background: var(--color-bg-elevated);">
                                    <th style="padding: 5px 8px; text-align: left; border-bottom: 2px solid var(--color-border); font-size: 0.8em; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-text-secondary);">"Оборот"</th>
                                    <th style="padding: 5px 8px; text-align: right; border-bottom: 2px solid var(--color-border); font-size: 0.8em; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-text-secondary); min-width: 90px;" title="no_order/order_accrual — снапшот проекции из документа; order_expense — расходы из строк документа (-totals.advertising)">"Документ"</th>
                                    <th style="padding: 5px 8px; text-align: right; border-bottom: 2px solid var(--color-border); font-size: 0.8em; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-info, #0ea5e9); min-width: 90px;" title="Живые данные из проекций: no_order — p911, order_accrual — p913 accrual, order_expense — p913 expense">"Проекция live"</th>
                                    <th style="padding: 5px 8px; text-align: right; border-bottom: 2px solid var(--color-border); font-size: 0.8em; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-text-secondary); min-width: 90px;">"GL (oper)"</th>
                                    <th style="padding: 5px 8px; text-align: right; border-bottom: 2px solid var(--color-border); font-size: 0.8em; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-text-secondary); min-width: 80px;" title="Δ между колонкой «Документ» и GL">"Δ doc-GL"</th>
                                    <th style="padding: 5px 8px; text-align: right; border-bottom: 2px solid var(--color-border); font-size: 0.8em; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-info, #0ea5e9); min-width: 80px;">"Δ live-GL"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {
                                    // order_expense: используем -totals.advertising (из строк документа),
                                    // а НЕ snap_order_expense (= p913 INNER JOIN GL ≈ GL → Δ всегда ≈ 0).
                                    // is_doc_snap=true: первая колонка = итог строк документа, не снапшот проекции.
                                    // (description, gl_code, doc_snap, is_doc_snap, proj_live, gl, d_doc_gl, d_live_gl)
                                    let rows: Vec<(&str, &str, f64, bool, f64, f64, f64, f64)> = vec![
                                        (
                                            "Реклама без заказа",
                                            "advert_clicks_no_order",
                                            snap_no_order, false,
                                            live_data.as_ref().map(|l| l.p913_no_order).unwrap_or(f64::NAN),
                                            gl_no_order,
                                            snap_no_order - gl_no_order,
                                            live_data.as_ref().map(|l| l.p913_no_order - l.gl_no_order).unwrap_or(f64::NAN),
                                        ),
                                        (
                                            "Реклама по заказам (резерв)",
                                            "advert_clicks_order_accrual",
                                            snap_order_accrual, false,
                                            live_data.as_ref().map(|l| l.p913_order_accrual).unwrap_or(f64::NAN),
                                            gl_order_accrual,
                                            snap_order_accrual - gl_order_accrual,
                                            live_data.as_ref().map(|l| l.p913_order_accrual - l.gl_order_accrual).unwrap_or(f64::NAN),
                                        ),
                                        (
                                            "Реклама по заказам (списание)",
                                            "advert_clicks_order_expense",
                                            doc_advert_expense, true,
                                            live_data.as_ref().map(|l| l.p913_order_expense).unwrap_or(f64::NAN),
                                            gl_order_expense,
                                            doc_advert_expense - gl_order_expense,
                                            live_data.as_ref().map(|l| l.p913_order_expense - l.gl_order_expense).unwrap_or(f64::NAN),
                                        ),
                                    ];
                                    rows.into_iter().map(|(desc, gl_code, snap, is_doc_snap, proj_live, gl, d_doc_gl, d_live_gl)| {
                                        // Для снапшота проекции: расхождение snap vs live = снапшот устарел.
                                        // Для документа (order_expense): snap = doc total, snap_stale неприменим.
                                        let snap_stale = !is_doc_snap && !proj_live.is_nan() && (snap - proj_live).abs() > 0.005;
                                        let doc_gl_diff = d_doc_gl.abs() > 0.005;
                                        let live_gl_diff = !d_live_gl.is_nan() && d_live_gl.abs() > 0.005;
                                        let td = "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border));";
                                        let td_r = "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; font-variant-numeric: tabular-nums;";
                                        view! {
                                            <tr>
                                                <td style=td>
                                                    <div style="font-size: 0.88em; color: var(--color-text-primary);">{desc}</div>
                                                    <div style="font-family: monospace; font-size: 0.78em; color: var(--color-text-secondary); margin-top: 1px;">
                                                        {gl_code}
                                                        {if snap_stale { view! { <span style="margin-left: 6px; font-size: 0.9em; color: var(--color-warning); font-weight: 600;" title="Снапшот устарел — нужен recalculate">"⚠ устарел"</span> }.into_any() } else { view! { <span /> }.into_any() }}
                                                    </div>
                                                </td>
                                                <td style=if is_doc_snap { "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; font-variant-numeric: tabular-nums; font-weight: 600;" } else { td_r }>
                                                    {format_money(snap)}
                                                </td>
                                                <td style=if snap_stale { "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; font-variant-numeric: tabular-nums; color: var(--color-warning);" } else { "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; font-variant-numeric: tabular-nums; color: var(--color-info, #0ea5e9);" }>
                                                    {if proj_live.is_nan() { "…".to_string() } else { format_money(proj_live) }}
                                                </td>
                                                <td style=td_r>{format_money(gl)}</td>
                                                <td style=if doc_gl_diff { "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; color: var(--color-warning); font-weight: 600;" } else { "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; color: var(--color-success);" }>
                                                    {format_money(d_doc_gl)}
                                                </td>
                                                <td style=if live_gl_diff { "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; color: var(--color-warning); font-weight: 600;" } else { "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; color: var(--color-success);" }>
                                                    {if d_live_gl.is_nan() { "…".to_string() } else { format_money(d_live_gl) }}
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()
                                }
                            </tbody>
                        </table>
                        }
                    }}
                    // Детализация по a026-регистраторам (показываем когда есть расхождение)
                    {move || {
                        let live_data = live.get();
                        let show = live_data.as_ref().map(|l| (l.p913_order_accrual - l.gl_order_accrual).abs() > 0.005).unwrap_or(false);
                        if !show { return view! { <span /> }.into_any(); }
                        let d = live_data.unwrap();
                        view! {
                        <div style="margin-top: var(--spacing-md);">
                            <div style="font-size: 0.78em; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-text-secondary); margin-bottom: var(--spacing-xs); font-weight: 600;">
                                {format!("Детализация по документам a026 — p913 строк: {}, GL записей: {}", d.p913_accrual_rows, d.gl_accrual_entries)}
                            </div>
                            <table style="width: 100%; border-collapse: collapse; font-size: 0.82em;">
                                <thead>
                                    <tr style="background: var(--color-bg-elevated);">
                                        <th style="padding: 4px 6px; text-align: left; border-bottom: 1px solid var(--color-border); color: var(--color-text-secondary); font-size: 0.85em; text-transform: uppercase;">"a026 UUID"</th>
                                        <th style="padding: 4px 6px; text-align: right; border-bottom: 1px solid var(--color-border); color: var(--color-text-secondary); font-size: 0.85em; text-transform: uppercase; min-width: 90px;">"p913"</th>
                                        <th style="padding: 4px 6px; text-align: center; border-bottom: 1px solid var(--color-border); color: var(--color-text-secondary); font-size: 0.85em; text-transform: uppercase;">"строк"</th>
                                        <th style="padding: 4px 6px; text-align: right; border-bottom: 1px solid var(--color-border); color: var(--color-text-secondary); font-size: 0.85em; text-transform: uppercase; min-width: 90px;">"GL"</th>
                                        <th style="padding: 4px 6px; text-align: right; border-bottom: 1px solid var(--color-border); color: var(--color-text-secondary); font-size: 0.85em; text-transform: uppercase; min-width: 70px;">"Δ"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {d.accrual_by_registrator.into_iter().map(|r| {
                                        let has_delta = r.delta.abs() > 0.005;
                                        let td = "padding: 4px 6px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border));";
                                        let td_r = "padding: 4px 6px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; font-variant-numeric: tabular-nums;";
                                        let delta_style = if has_delta {
                                            "padding: 4px 6px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; color: var(--color-warning); font-weight: 600;"
                                        } else {
                                            "padding: 4px 6px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; color: var(--color-success);"
                                        };
                                        view! {
                                            <tr>
                                                <td style=td>
                                                    <span style="font-family: monospace; font-size: 0.9em; color: var(--color-text-secondary);">
                                                        {r.registrator_ref[..r.registrator_ref.len().min(8)].to_string()}
                                                        "…"
                                                    </span>
                                                </td>
                                                <td style=td_r>{format_money(r.p913_sum)}</td>
                                                <td style="padding: 4px 6px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: center; color: var(--color-text-secondary);">{r.p913_rows}</td>
                                                <td style=td_r>{format_money(r.gl_sum)}</td>
                                                <td style=delta_style>{format_money(r.delta)}</td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                        }.into_any()
                    }}
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

// ─────────────────────────────────────────────────────────────────────────────
// AdvertTab — Реклама (два снапшота, группировка по номенклатуре)
// ─────────────────────────────────────────────────────────────────────────────

/// Сгруппированная строка для таблицы «Клики (без заказов)».
#[derive(Clone)]
struct NoOrderGrouped {
    nomenclature_ref: Option<String>,
    sa_name: String,
    amount: f64,
    /// (campaign_code, campaign_ref, сумма)
    campaigns: Vec<(String, Option<String>, f64)>,
}

/// Сгруппированная строка для таблицы «Клики (с заказами)».
#[derive(Clone)]
struct OrderAccrualGrouped {
    nomenclature_ref: Option<String>,
    sa_name: String,
    amount: f64,
    campaigns: Vec<(String, Option<String>, f64)>,
    /// Заказы: (order_key, order_id, order_date)
    orders: Vec<(String, Option<String>, Option<String>)>,
}

fn group_no_order(rows: &[WbDayCloseAdvertNoOrderLineDto]) -> Vec<NoOrderGrouped> {
    let mut map: std::collections::HashMap<Option<String>, NoOrderGrouped> =
        std::collections::HashMap::new();
    for r in rows {
        let key = r.nomenclature_ref.clone();
        let entry = map.entry(key.clone()).or_insert_with(|| NoOrderGrouped {
            nomenclature_ref: key,
            sa_name: r.sa_name.clone().unwrap_or_else(|| "—".to_string()),
            amount: 0.0,
            campaigns: Vec::new(),
        });
        entry.amount += r.amount;
        if let Some(c) = entry
            .campaigns
            .iter_mut()
            .find(|(c, _, _)| c == &r.campaign_code)
        {
            c.2 += r.amount;
        } else {
            entry
                .campaigns
                .push((r.campaign_code.clone(), r.campaign_ref.clone(), r.amount));
        }
    }
    let mut result: Vec<NoOrderGrouped> = map.into_values().collect();
    result.sort_by(|a, b| {
        b.amount
            .partial_cmp(&a.amount)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    result
}

fn group_order_accrual(rows: &[WbDayCloseAdvertOrderAccrualLineDto]) -> Vec<OrderAccrualGrouped> {
    let mut map: std::collections::HashMap<Option<String>, OrderAccrualGrouped> =
        std::collections::HashMap::new();
    for r in rows {
        let key = r.nomenclature_ref.clone();
        let entry = map
            .entry(key.clone())
            .or_insert_with(|| OrderAccrualGrouped {
                nomenclature_ref: key,
                sa_name: r.sa_name.clone().unwrap_or_else(|| "—".to_string()),
                amount: 0.0,
                campaigns: Vec::new(),
                orders: Vec::new(),
            });
        entry.amount += r.amount;
        if let Some(c) = entry
            .campaigns
            .iter_mut()
            .find(|(c, _, _)| c == &r.campaign_code)
        {
            c.2 += r.amount;
        } else {
            entry
                .campaigns
                .push((r.campaign_code.clone(), r.campaign_ref.clone(), r.amount));
        }
        if !r.order_key.is_empty() && !entry.orders.iter().any(|(ok, _, _)| ok == &r.order_key) {
            entry.orders.push((
                r.order_key.clone(),
                r.order_id.clone(),
                r.order_date.clone(),
            ));
        }
    }
    let mut result: Vec<OrderAccrualGrouped> = map.into_values().collect();
    result.sort_by(|a, b| {
        b.amount
            .partial_cmp(&a.amount)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    result
}

const ADVERT_TH: &str = "padding: 6px 8px; font-size: 0.78em; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-text-secondary); border-bottom: 2px solid var(--color-border); white-space: nowrap; cursor: pointer; user-select: none;";
const ADVERT_TH_R: &str = "padding: 6px 8px; font-size: 0.78em; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-text-secondary); border-bottom: 2px solid var(--color-border); white-space: nowrap; cursor: pointer; user-select: none; text-align: right;";
const ADVERT_TD: &str = "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); vertical-align: top;";
const ADVERT_TD_R: &str = "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); vertical-align: top; text-align: right; font-variant-numeric: tabular-nums;";
const ADVERT_LINK: &str = "cursor: pointer; text-decoration: underline; background: none; border: none; padding: 0; font-size: inherit; color: var(--color-link);";

#[component]
fn AdvertTab(
    no_order: Vec<WbDayCloseAdvertNoOrderLineDto>,
    order_accrual: Vec<WbDayCloseAdvertOrderAccrualLineDto>,
    tabs_store: AppGlobalContext,
) -> impl IntoView {
    let advert_sub_tab = RwSignal::new("no_order".to_string());

    // Группировка один раз
    let no_order_grouped = group_no_order(&no_order);
    let order_accrual_grouped = group_order_accrual(&order_accrual);

    let no_order_total: f64 = no_order_grouped.iter().map(|r| r.amount).sum();
    let order_accrual_total: f64 = order_accrual_grouped.iter().map(|r| r.amount).sum();
    let no_order_count = no_order_grouped.len();
    let order_accrual_count = order_accrual_grouped.len();

    // Состояние сортировки: (поле, asc)
    let no_order_sort = RwSignal::new(("amount".to_string(), false));
    let order_accrual_sort = RwSignal::new(("amount".to_string(), false));

    view! {
        <div style="padding: var(--spacing-md); display: flex; flex-direction: column; gap: var(--spacing-md);">
            <div style="border-bottom: 1px solid var(--color-border);">
                <TabList selected_value=advert_sub_tab>
                    <Tab value="no_order".to_string()>
                        {format!("Клики (без заказов) ({})", no_order_count)}
                    </Tab>
                    <Tab value="with_order".to_string()>
                        {format!("Клики (с заказами) ({})", order_accrual_count)}
                    </Tab>
                </TabList>
            </div>

            // ── Клики (без заказов) — p911 ──────────────────────────────────
            {move || {
                if advert_sub_tab.get() != "no_order" {
                    return view! { <span /> }.into_any();
                }
                let rows = no_order_grouped.clone();
                if rows.is_empty() {
                    return view! {
                        <div style="color: var(--color-text-secondary); padding: var(--spacing-md);">
                            "Нет данных по рекламным кликам без заказов"
                        </div>
                    }.into_any();
                }

                let toggle_sort = move |field: &'static str| {
                    no_order_sort.update(|(f, asc)| {
                        if f == field { *asc = !*asc; } else { *f = field.to_string(); *asc = false; }
                    });
                };

                view! {
                    <div style="overflow-x: auto;">
                        <table style="width: 100%; border-collapse: collapse; font-size: 0.88em;">
                            <thead>
                                <tr style="background: var(--color-bg-elevated);">
                                    <th style=ADVERT_TH on:click=move |_| toggle_sort("sa_name")>
                                        "Номенклатура"
                                        <span class=move || {
                                            let (f, _) = no_order_sort.get();
                                            if f == "sa_name" { " sort-icon active" } else { " sort-icon" }
                                        }>
                                            {move || {
                                                let (f, asc) = no_order_sort.get();
                                                if f == "sa_name" { if asc { " ▲" } else { " ▼" } } else { "" }
                                            }}
                                        </span>
                                    </th>
                                    <th style=ADVERT_TH_R on:click=move |_| toggle_sort("amount")>
                                        "Сумма"
                                        <span class=move || {
                                            let (f, _) = no_order_sort.get();
                                            if f == "amount" { " sort-icon active" } else { " sort-icon" }
                                        }>
                                            {move || {
                                                let (f, asc) = no_order_sort.get();
                                                if f == "amount" { if asc { " ▲" } else { " ▼" } } else { " ▼" }
                                            }}
                                        </span>
                                    </th>
                                    <th style=ADVERT_TH>"Кампании"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    let mut sorted = rows.clone();
                                    let (f, asc) = no_order_sort.get();
                                    match f.as_str() {
                                        "sa_name" => sorted.sort_by(|a, b| if asc { a.sa_name.cmp(&b.sa_name) } else { b.sa_name.cmp(&a.sa_name) }),
                                        _ => sorted.sort_by(|a, b| if asc { a.amount.partial_cmp(&b.amount).unwrap_or(std::cmp::Ordering::Equal) } else { b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal) }),
                                    }
                                    sorted.into_iter().map(|row| {
                                        let sa = row.sa_name.clone();
                                        let nref = row.nomenclature_ref.clone();
                                        let tabs2 = tabs_store.clone();
                                        let sa2 = sa.clone();
                                        let campaigns = row.campaigns.clone();
                                        view! {
                                            <tr style="transition: background 0.1s;" class="data-table__row">
                                                <td style=ADVERT_TD>
                                                    {match nref {
                                                        Some(id) => view! {
                                                            <button style=ADVERT_LINK
                                                                on:click=move |_| {
                                                                    let key = format!("a004_nomenclature_details_{}", id);
                                                                    tabs2.open_tab(&key, &sa2);
                                                                }
                                                            >{sa}</button>
                                                        }.into_any(),
                                                        None => view! { <span style="color: var(--color-text-secondary);">{sa}</span> }.into_any(),
                                                    }}
                                                </td>
                                                <td style=ADVERT_TD_R>
                                                    <strong>{format_money(row.amount)}</strong>
                                                </td>
                                                <td style=ADVERT_TD>
                                                    <div style="display: flex; flex-wrap: wrap; gap: 4px;">
                                                        {campaigns.into_iter().map(|(code, cref, amt)| {
                                                            let tabs3 = tabs_store.clone();
                                                            let code_label = code.clone();
                                                            let code_display = code.clone();
                                                            let title = format!("{} — {}", code, format_money(amt));
                                                            let tab_key = cref
                                                                .map(|r| format!("a030_wb_advert_campaign_details_{}", r))
                                                                .unwrap_or_else(|| format!("a030_wb_advert_campaign_details_{}", code));
                                                            view! {
                                                                <button
                                                                    style="cursor: pointer; background: var(--color-bg-elevated); border: 1px solid var(--color-border); border-radius: 4px; padding: 1px 6px; font-size: 0.85em; color: var(--color-link); white-space: nowrap;"
                                                                    title=title
                                                                    on:click=move |_| {
                                                                        tabs3.open_tab(&tab_key, &code_label);
                                                                    }
                                                                >{code_display}</button>
                                                            }
                                                        }).collect_view()}
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()
                                }}
                            </tbody>
                            <tfoot>
                                <tr style="font-weight: 700; border-top: 2px solid var(--color-border); background: var(--color-bg-elevated);">
                                    <td style=ADVERT_TD>"Итого"</td>
                                    <td style=ADVERT_TD_R>{format_money(no_order_total)}</td>
                                    <td style=ADVERT_TD></td>
                                </tr>
                            </tfoot>
                        </table>
                    </div>
                }.into_any()
            }}

            // ── Клики (с заказами) — p913 ────────────────────────────────────
            {move || {
                if advert_sub_tab.get() != "with_order" {
                    return view! { <span /> }.into_any();
                }
                let rows = order_accrual_grouped.clone();
                if rows.is_empty() {
                    return view! {
                        <div style="color: var(--color-text-secondary); padding: var(--spacing-md);">
                            "Нет данных по рекламным кликам с заказами"
                        </div>
                    }.into_any();
                }

                let toggle_sort = move |field: &'static str| {
                    order_accrual_sort.update(|(f, asc)| {
                        if f == field { *asc = !*asc; } else { *f = field.to_string(); *asc = false; }
                    });
                };

                view! {
                    <div style="overflow-x: auto;">
                        <table style="width: 100%; border-collapse: collapse; font-size: 0.88em;">
                            <thead>
                                <tr style="background: var(--color-bg-elevated);">
                                    <th style=ADVERT_TH on:click=move |_| toggle_sort("sa_name")>
                                        "Номенклатура"
                                        <span class=move || {
                                            let (f, _) = order_accrual_sort.get();
                                            if f == "sa_name" { " sort-icon active" } else { " sort-icon" }
                                        }>
                                            {move || {
                                                let (f, asc) = order_accrual_sort.get();
                                                if f == "sa_name" { if asc { " ▲" } else { " ▼" } } else { "" }
                                            }}
                                        </span>
                                    </th>
                                    <th style=ADVERT_TH_R on:click=move |_| toggle_sort("amount")>
                                        "Сумма"
                                        <span class=move || {
                                            let (f, _) = order_accrual_sort.get();
                                            if f == "amount" { " sort-icon active" } else { " sort-icon" }
                                        }>
                                            {move || {
                                                let (f, asc) = order_accrual_sort.get();
                                                if f == "amount" { if asc { " ▲" } else { " ▼" } } else { " ▼" }
                                            }}
                                        </span>
                                    </th>
                                    <th style=ADVERT_TH>"Кампании"</th>
                                    <th style=ADVERT_TH>"Заказы"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    let mut sorted = rows.clone();
                                    let (f, asc) = order_accrual_sort.get();
                                    match f.as_str() {
                                        "sa_name" => sorted.sort_by(|a, b| if asc { a.sa_name.cmp(&b.sa_name) } else { b.sa_name.cmp(&a.sa_name) }),
                                        _ => sorted.sort_by(|a, b| if asc { a.amount.partial_cmp(&b.amount).unwrap_or(std::cmp::Ordering::Equal) } else { b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal) }),
                                    }
                                    sorted.into_iter().map(|row| {
                                        let sa = row.sa_name.clone();
                                        let nref = row.nomenclature_ref.clone();
                                        let campaigns = row.campaigns.clone();
                                        let orders = row.orders.clone();
                                        let tabs_nom = tabs_store.clone();
                                        let tabs_cam = tabs_store.clone();
                                        let tabs_ord = tabs_store.clone();
                                        let sa2 = sa.clone();
                                        view! {
                                            <tr class="data-table__row">
                                                <td style=ADVERT_TD>
                                                    {match nref {
                                                        Some(id) => view! {
                                                            <button style=ADVERT_LINK
                                                                on:click=move |_| {
                                                                    let key = format!("a004_nomenclature_details_{}", id);
                                                                    tabs_nom.open_tab(&key, &sa2);
                                                                }
                                                            >{sa}</button>
                                                        }.into_any(),
                                                        None => view! { <span style="color: var(--color-text-secondary);">{sa}</span> }.into_any(),
                                                    }}
                                                </td>
                                                <td style=ADVERT_TD_R>
                                                    <strong>{format_money(row.amount)}</strong>
                                                </td>
                                                <td style=ADVERT_TD>
                                                    <div style="display: flex; flex-wrap: wrap; gap: 4px;">
                                                        {campaigns.into_iter().map(|(code, cref, amt)| {
                                                            let tabs_c = tabs_cam.clone();
                                                            let code_label = code.clone();
                                                            let code_display = code.clone();
                                                            let title = format!("{} — {}", code, format_money(amt));
                                                            let tab_key = cref
                                                                .map(|r| format!("a030_wb_advert_campaign_details_{}", r))
                                                                .unwrap_or_else(|| format!("a030_wb_advert_campaign_details_{}", code));
                                                            view! {
                                                                <button
                                                                    style="cursor: pointer; background: var(--color-bg-elevated); border: 1px solid var(--color-border); border-radius: 4px; padding: 1px 6px; font-size: 0.85em; color: var(--color-link); white-space: nowrap;"
                                                                    title=title
                                                                    on:click=move |_| {
                                                                        tabs_c.open_tab(&tab_key, &code_label);
                                                                    }
                                                                >{code_display}</button>
                                                            }
                                                        }).collect_view()}
                                                    </div>
                                                </td>
                                                <td style=ADVERT_TD>
                                                    <div style="display: flex; flex-wrap: wrap; gap: 4px;">
                                                        {orders.into_iter().map(|(order_key, order_id, order_date)| {
                                                            let tabs_o = tabs_ord.clone();
                                                            let label = order_date
                                                                .as_deref()
                                                                .map(format_date)
                                                                .unwrap_or_else(|| order_key.chars().take(12).collect());
                                                            match order_id {
                                                                Some(oid) => view! {
                                                                    <button
                                                                        style="cursor: pointer; background: var(--color-bg-elevated); border: 1px solid var(--color-border); border-radius: 4px; padding: 1px 6px; font-size: 0.85em; color: var(--color-link); white-space: nowrap;"
                                                                        on:click=move |_| {
                                                                            let key = format!("a015_wb_orders_details_{}", oid);
                                                                            tabs_o.open_tab(&key, "Заказ WB");
                                                                        }
                                                                    >{label}</button>
                                                                }.into_any(),
                                                                None => view! {
                                                                    <span style="font-size: 0.85em; color: var(--color-text-secondary);">{label}</span>
                                                                }.into_any(),
                                                            }
                                                        }).collect_view()}
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()
                                }}
                            </tbody>
                            <tfoot>
                                <tr style="font-weight: 700; border-top: 2px solid var(--color-border); background: var(--color-bg-elevated);">
                                    <td style=ADVERT_TD>"Итого"</td>
                                    <td style=ADVERT_TD_R>{format_money(order_accrual_total)}</td>
                                    <td style=ADVERT_TD></td>
                                    <td style=ADVERT_TD></td>
                                </tr>
                            </tfoot>
                        </table>
                    </div>
                }.into_any()
            }}
        </div>
    }
}
