//! Marketplace navigator page.
//!
//! Container component that owns the shared state (search query, view mode)
//! and dispatches rendering to one of three sibling modules.

mod brief;
mod data;
mod detailed;
mod table;

pub use self::data::{BLOCKS, COLUMNS};

use crate::layout::global_context::AppGlobalContext;
use crate::navigator::shared::prefs::NavigatorPagePrefs;
use crate::navigator::shared::types::{NavLink, ViewMode};
use crate::shared::icons::icon;
use crate::system::auth::context::{has_read_access, use_auth, AuthState};
use leptos::prelude::*;

const PAGE_ID: &str = "marketplace";

/// Lower-case substring search helper used by all three modes.
pub fn link_matches(link: &NavLink, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let lower = needle.to_lowercase();
    link.label.to_lowercase().contains(&lower) || link.annotation.to_lowercase().contains(&lower)
}

/// Returns true when the current user is allowed to follow the link.
pub fn link_visible(auth_state: ReadSignal<AuthState>, link: &NavLink) -> bool {
    match link.scope_id {
        None => true,
        Some(scope) => has_read_access(auth_state, scope),
    }
}

#[component]
#[allow(non_snake_case)]
pub fn MarketplaceNavigator() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (auth_state, _) = use_auth();

    let prefs = NavigatorPagePrefs::load(PAGE_ID);
    let view_mode = RwSignal::new(prefs.view_mode);
    let search = RwSignal::new(String::new());

    Effect::new(move |_| {
        let mode = view_mode.get();
        let mut p = NavigatorPagePrefs::load(PAGE_ID);
        if p.view_mode != mode {
            p.view_mode = mode;
            p.save(PAGE_ID);
        }
    });

    view! {
        <div class="navigator">
            <div class="navigator__header">
                <div class="navigator__title">"Маркетплейсы"</div>
                <NavigatorSearch search=search />
                <div class="navigator__view-switch-slot">
                    <ViewSwitch view_mode=view_mode />
                </div>
            </div>

            {move || match view_mode.get() {
                ViewMode::Brief => view! {
                    <brief::MarketplaceBrief
                        search=search.read_only()
                        ctx=ctx
                        auth_state=auth_state
                    />
                }.into_any(),
                ViewMode::Detailed => view! {
                    <detailed::MarketplaceDetailed
                        search=search.read_only()
                        ctx=ctx
                        auth_state=auth_state
                    />
                }.into_any(),
                ViewMode::Table => view! {
                    <table::MarketplaceTable
                        search=search.read_only()
                        ctx=ctx
                        auth_state=auth_state
                    />
                }.into_any(),
            }}
        </div>
    }
}

#[component]
fn NavigatorSearch(search: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="navigator__search">
            <span class="navigator__search-icon">{icon("search")}</span>
            <input
                class="navigator__search-input"
                type="search"
                placeholder="Поиск по разделам и ссылкам…"
                prop:value=move || search.get()
                on:input=move |ev| {
                    let v = event_target_value(&ev);
                    search.set(v);
                }
            />
            <Show when=move || !search.get().is_empty()>
                <button
                    class="navigator__search-clear"
                    type="button"
                    title="Очистить"
                    on:click=move |_| search.set(String::new())
                >
                    {icon("x")}
                </button>
            </Show>
        </div>
    }
}

#[component]
fn ViewSwitch(view_mode: RwSignal<ViewMode>) -> impl IntoView {
    view! {
        <div class="navigator__view-switch" role="tablist" aria-label="Режим просмотра">
            {ViewMode::ALL
                .iter()
                .copied()
                .map(|mode| {
                    view! {
                        <button
                            class="navigator__view-btn"
                            class:navigator__view-btn--active=move || view_mode.get() == mode
                            type="button"
                            role="tab"
                            aria-selected=move || (view_mode.get() == mode).to_string()
                            on:click=move |_| view_mode.set(mode)
                        >
                            {icon(mode.icon())}
                            <span>{mode.label()}</span>
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}
