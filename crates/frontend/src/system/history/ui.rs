use chrono::{DateTime, FixedOffset};
use contracts::system::history::PageHistoryDto;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::modal_frame::ModalFrame;

use super::api;

/// Moscow timezone offset (UTC+3). Timestamps are stored in UTC and shown in MSK.
const MSK_OFFSET_SECONDS: i32 = 3 * 3600;

/// Parse a stored RFC3339 timestamp and convert it to Moscow time.
fn to_msk(raw: &str) -> Option<DateTime<FixedOffset>> {
    let offset = FixedOffset::east_opt(MSK_OFFSET_SECONDS)?;
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&offset))
}

/// "HH:MM" in MSK (falls back to the raw string when unparseable).
fn time_label(raw: &str) -> String {
    match to_msk(raw) {
        Some(dt) => dt.format("%H:%M").to_string(),
        None => raw.to_string(),
    }
}

/// "YYYY-MM-DD" day key in MSK, used to group rows and draw day separators.
fn day_label(raw: &str) -> String {
    match to_msk(raw) {
        Some(dt) => dt.format("%Y-%m-%d").to_string(),
        None => raw.chars().take(10).collect(),
    }
}

/// Whether the tab key denotes an aggregate *list* page (e.g. "a012_wb_sales",
/// "a004_nomenclature_list") as opposed to a single-record detail, a "new"
/// form, a dashboard view or a tree.
fn is_aggregate_list_key(key: &str) -> bool {
    // Aggregate keys look like "aNNN_...".
    let bytes = key.as_bytes();
    let is_aggregate = bytes.len() > 4
        && bytes[0] == b'a'
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'_';
    if !is_aggregate {
        return false;
    }
    // Single-record / non-list pages.
    if key.contains("_details_")
        || key.ends_with("_details")
        || key.contains("_new")
        || key.contains("_view_")
    {
        return false;
    }
    // Tree views are not flat lists.
    !matches!(key, "a003_counterparty" | "a004_nomenclature")
}

/// Title shown in the history list, prefixed for aggregate list pages.
fn display_title(tab_key: &str, title: &str) -> String {
    if is_aggregate_list_key(tab_key) {
        format!("Список: {}", title)
    } else {
        title.to_string()
    }
}

#[component]
pub fn HistoryHeaderButton() -> impl IntoView {
    let modal_open = RwSignal::new(false);
    let modal_closing = RwSignal::new(false);
    let close_modal = Callback::new(move |_| {
        if modal_closing.get_untracked() {
            return;
        }
        modal_closing.set(true);
        spawn_local(async move {
            TimeoutFuture::new(180).await;
            modal_open.set(false);
            modal_closing.set(false);
        });
    });

    view! {
        <button
            class="app-header__icon-button"
            on:click=move |_| {
                modal_closing.set(false);
                modal_open.set(true);
            }
            title="История открытых страниц"
        >
            {icon("clock")}
        </button>
        <Show when=move || modal_open.get()>
            <HistoryDrawer on_close=close_modal closing=modal_closing />
        </Show>
    }
}

#[component]
fn HistoryDrawer(on_close: Callback<()>, closing: RwSignal<bool>) -> impl IntoView {
    let items = RwSignal::new(Vec::<PageHistoryDto>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(None::<String>);
    let clearing = RwSignal::new(false);
    let tabs = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let reload = move || {
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match api::list_history(None).await {
                Ok(value) => items.set(value),
                Err(err) => error.set(Some(err)),
            }
            loading.set(false);
        });
    };
    Effect::new(move |_| reload());

    let clear_all = move |_| {
        if clearing.get_untracked() {
            return;
        }
        clearing.set(true);
        spawn_local(async move {
            match api::clear_history().await {
                Ok(_) => items.set(Vec::new()),
                Err(err) => error.set(Some(err)),
            }
            clearing.set(false);
        });
    };

    view! {
        <ModalFrame
            on_close=on_close
            overlay_style="align-items: stretch; justify-content: flex-end; padding: 0;".to_string()
            overlay_class_signal=Signal::derive(move || {
                if closing.get() {
                    "favorite-drawer-overlay favorite-drawer-overlay--closing".to_string()
                } else {
                    "favorite-drawer-overlay".to_string()
                }
            })
            modal_style="width: min(560px, 100vw); max-width: min(560px, 100vw); height: 100vh; max-height: 100vh; border-radius: 0; overflow: hidden;".to_string()
            modal_class_signal=Signal::derive(move || {
                if closing.get() {
                    "favorite-modal favorite-modal--list favorite-drawer favorite-drawer--closing".to_string()
                } else {
                    "favorite-modal favorite-modal--list favorite-drawer".to_string()
                }
            })
        >
            <div class="favorite-modal__header">
                <h3>"История открытых страниц"</h3>
                <div style="display: flex; align-items: center; gap: 8px;">
                    <button
                        class="button button--secondary"
                        on:click=clear_all
                        disabled=move || clearing.get() || items.get().is_empty()
                        title="Очистить историю"
                    >
                        {move || if clearing.get() { "Очистка..." } else { "Очистить" }}
                    </button>
                    <button class="favorite-modal__close" on:click=move |_| on_close.run(())>"x"</button>
                </div>
            </div>
            <div class="favorite-modal__body">
                <Show when=move || loading.get()>
                    <div class="favorite-modal__loading">"Загрузка..."</div>
                </Show>
                <Show when=move || error.get().is_some()>
                    <div class="favorite-modal__error">{move || error.get().unwrap_or_default()}</div>
                </Show>
                <Show when=move || !loading.get() && items.get().is_empty()>
                    <div class="favorite-modal__empty">"История пуста"</div>
                </Show>
                <div class="windows-list__items">
                    {move || {
                        let rows = items.get();
                        let mut prev_day: Option<String> = None;
                        rows.into_iter()
                            .map(|item| {
                                let open_item = item.clone();
                                let time = time_label(&item.opened_at);
                                let day = day_label(&item.opened_at);
                                let title = display_title(&item.tab_key, &item.title);
                                // Divider (top border) only when the day changes.
                                let new_day = prev_day.as_deref() != Some(day.as_str());
                                prev_day = Some(day.clone());
                                let row_style = if new_day {
                                    "display: flex; align-items: center; gap: 10px; width: 100%; \
                                     padding: 3px 10px; text-align: left; background: transparent; \
                                     border: none; border-top: 1px solid var(--color-border); cursor: pointer; \
                                     color: var(--color-text-primary); font-size: var(--font-size-sm);"
                                } else {
                                    "display: flex; align-items: center; gap: 10px; width: 100%; \
                                     padding: 3px 10px; text-align: left; background: transparent; \
                                     border: none; cursor: pointer; \
                                     color: var(--color-text-primary); font-size: var(--font-size-sm);"
                                };
                                view! {
                                    <button
                                        class="windows-list__item"
                                        style=row_style
                                        title=day.clone()
                                        on:click=move |_| {
                                            tabs.open_tab(&open_item.tab_key, &open_item.title);
                                            on_close.run(());
                                        }
                                    >
                                        <span style="flex: 0 0 auto; font-variant-numeric: tabular-nums; white-space: nowrap; color: var(--color-text-muted);">
                                            {time}
                                        </span>
                                        <span style="flex: 1 1 auto; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                            {title}
                                        </span>
                                    </button>
                                }
                            })
                            .collect_view()
                    }}
                </div>
            </div>
        </ModalFrame>
    }
}
