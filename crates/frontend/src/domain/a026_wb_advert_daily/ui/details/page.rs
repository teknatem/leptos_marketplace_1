//! Main page component for WB Advert Daily details (MVVM Standard)
//!
//! Thin wrapper that creates the ViewModel, renders the styled header with
//! post/unpost actions, the tab bar, and routes to tab components.

use super::tabs::{AttributionTab, GeneralTab, JournalTab, LinesTab, ProjectionsTab};
use super::view_model::WbAdvertDailyDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::more_actions_menu::{use_more_actions_close, MoreActionsMenu};
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use crate::system::favorites::ui::FavoriteButton;
use leptos::prelude::*;
use thaw::*;

/// Main component for WB Advert Daily details
#[component]
pub fn WbAdvertDailyDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let vm = WbAdvertDailyDetailsVm::new(tabs);
    let stored_id = StoredValue::new(id.clone());

    vm.load(id);

    // Lazy-load nested data when its tab becomes active.
    Effect::new({
        let vm = vm.clone();
        move || match vm.active_tab.get() {
            "projections" if !vm.projections_loaded.get() => vm.load_projections(),
            "journal" if !vm.general_ledger_entries_loaded.get() => {
                vm.load_general_ledger_entries()
            }
            _ => {}
        }
    });

    let vm_header = vm.clone();
    let vm_tabs = vm.clone();
    let vm_content = vm.clone();

    view! {
        <PageFrame page_id="a026_wb_advert_daily--detail" category=PAGE_CAT_DETAIL class="page--wide">
            <Header vm=vm_header favorite_target_id=stored_id.get_value() on_close=on_close />

            <TabBar vm=vm_tabs />

            <div class="page__content">
                {move || {
                    if vm.loading.get() {
                        view! {
                            <Flex gap=FlexGap::Small style="align-items:center;justify-content:center;padding:var(--spacing-4xl);">
                                <Spinner />
                                <span>"Загрузка..."</span>
                            </Flex>
                        }.into_any()
                    } else if let Some(err) = vm.error.get() {
                        view! { <div class="alert alert--error">{err}</div> }.into_any()
                    } else if vm.doc.get().is_some() {
                        view! { <TabContent vm=vm_content.clone() /> }.into_any()
                    } else {
                        view! { <div class="alert">"Документ не найден."</div> }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}

// ── Header ────────────────────────────────────────────────────────────────────

#[component]
fn Header(
    vm: WbAdvertDailyDetailsVm,
    favorite_target_id: String,
    on_close: Callback<()>,
) -> impl IntoView {
    let is_posted = vm.is_posted();
    let title = vm.header_title();
    let favorite_title = vm.tab_label();
    let doc = vm.doc;
    let tab_key = format!("a026_wb_advert_daily_details_{}", favorite_target_id);

    // Поднятое состояние меню «Ещё», чтобы открывать его и кнопкой, и правым кликом по заголовку.
    let more_open = RwSignal::new(false);
    let more_pos = RwSignal::new((0.0_f64, 0.0_f64));

    // Правый клик по заголовку открывает «Ещё» у курсора (для проведённого документа).
    let on_header_contextmenu = move |ev: web_sys::MouseEvent| {
        if !is_posted.get() {
            return;
        }
        ev.prevent_default();
        let vw = web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(1024.0);
        more_pos.set((ev.client_y() as f64 + 4.0, vw - ev.client_x() as f64));
        more_open.set(true);
    };

    view! {
        <div class="page__header" on:contextmenu=on_header_contextmenu>
            <div class="page__header-left">
                <FavoriteButton
                    target_kind="a026_wb_advert_daily_details".to_string()
                    target_id=favorite_target_id
                    target_title=favorite_title
                    tab_key=tab_key
                />
                <h1 class="page__title">{move || title.get()}</h1>
                <Show when=move || doc.get().is_some()>
                    {move || {
                        let posted = is_posted.get();
                        view! {
                            <Badge
                                appearance=BadgeAppearance::Filled
                                color=if posted { BadgeColor::Success } else { BadgeColor::Warning }
                            >
                                {if posted { "Проведен" } else { "Не проведен" }}
                            </Badge>
                        }
                    }}
                </Show>
            </div>
            <div class="page__header-right">
                <PostButtons vm=vm.clone() more_open=more_open more_pos=more_pos />
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
    }
}

// ── Post / Unpost ─────────────────────────────────────────────────────────────

#[component]
fn PostButtons(
    vm: WbAdvertDailyDetailsVm,
    more_open: RwSignal<bool>,
    more_pos: RwSignal<(f64, f64)>,
) -> impl IntoView {
    let is_posted = vm.is_posted();
    let posting = vm.posting;
    let doc = vm.doc;

    let on_post = {
        let vm = vm.clone();
        Callback::new(move |_: ()| vm.post())
    };
    let on_unpost = {
        let vm = vm;
        Callback::new(move |_: ()| vm.unpost())
    };

    view! {
        <Show when=move || doc.get().is_some()>
            // «Провести» всегда в заголовке: для непроведённого — проведение, для проведённого —
            // идемпотентное перепроведение. Спиннер в кнопке во время операции.
            <Button
                appearance=ButtonAppearance::Primary
                size=ButtonSize::Medium
                on_click=move |_| on_post.run(())
                disabled=Signal::derive(move || posting.get())
            >
                <span class="page-action-button__content">
                    <span class="page-action-button__icon">
                        {move || if posting.get() {
                            view! { <Spinner size=SpinnerSize::Tiny /> }.into_any()
                        } else {
                            icon("check").into_any()
                        }}
                    </span>
                    <span class="page-action-button__text">
                        {move || match (is_posted.get(), posting.get()) {
                            (_, true) => "Проведение...",
                            (true, false) => "Перепровести",
                            (false, false) => "Провести",
                        }}
                    </span>
                </span>
            </Button>
            // Проведённый документ — отмена проведения в подменю «Ещё».
            <Show when=move || is_posted.get()>
                <MoreActionsMenu open=more_open pos=more_pos>
                    <button
                        class="theme-dropdown__item"
                        disabled=move || posting.get()
                        on:click=move |_| {
                            use_more_actions_close();
                            on_unpost.run(());
                        }
                    >
                        <span style="display: flex; align-items: center; gap: 8px;">
                            {move || if posting.get() {
                                view! { <Spinner size=SpinnerSize::Tiny /> }.into_any()
                            } else {
                                icon("x").into_any()
                            }}
                            {move || if posting.get() { "Отмена проведения..." } else { "Отменить проведение" }}
                        </span>
                    </button>
                </MoreActionsMenu>
            </Show>
        </Show>
    }
}

// ── Tab bar ───────────────────────────────────────────────────────────────────

#[component]
fn TabBar(vm: WbAdvertDailyDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let projections_count = vm.projections_count();
    let journal_count = vm.general_ledger_entries_count();

    view! {
        <div class="page__tabs">
            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "general"
                on:click={ let vm = vm.clone(); move |_| vm.set_tab("general") }
            >
                "Общие"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "lines"
                on:click={ let vm = vm.clone(); move |_| vm.set_tab("lines") }
            >
                "Позиции"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "linked_orders"
                on:click={ let vm = vm.clone(); move |_| vm.set_tab("linked_orders") }
            >
                "Атрибуция"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "journal"
                on:click={ let vm = vm.clone(); move |_| vm.set_tab("journal") }
            >
                "Журнал"
                <Badge
                    appearance=BadgeAppearance::Tint
                    color=Signal::derive(move || if active_tab.get() == "journal" { BadgeColor::Brand } else { BadgeColor::Success })
                    attr:style="margin-left: 6px;"
                >
                    {move || journal_count.get().to_string()}
                </Badge>
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "projections"
                on:click={ let vm = vm.clone(); move |_| vm.set_tab("projections") }
            >
                "Проекции"
                <Badge
                    appearance=BadgeAppearance::Tint
                    color=Signal::derive(move || if active_tab.get() == "projections" { BadgeColor::Brand } else { BadgeColor::Informative })
                    attr:style="margin-left: 6px;"
                >
                    {move || projections_count.get().to_string()}
                </Badge>
            </button>
        </div>
    }
}

// ── Tab content ───────────────────────────────────────────────────────────────

#[component]
fn TabContent(vm: WbAdvertDailyDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let vm_general = vm.clone();
    let vm_lines = vm.clone();
    let vm_attr = vm.clone();
    let vm_journal = vm.clone();
    let vm_projections = vm.clone();

    view! {
        {move || match active_tab.get() {
            "general"       => view! { <GeneralTab     vm=vm_general.clone()     /> }.into_any(),
            "lines"         => view! { <LinesTab       vm=vm_lines.clone()       /> }.into_any(),
            "linked_orders" => view! { <AttributionTab vm=vm_attr.clone()        /> }.into_any(),
            "journal"       => view! { <JournalTab     vm=vm_journal.clone()     /> }.into_any(),
            "projections"   => view! { <ProjectionsTab vm=vm_projections.clone() /> }.into_any(),
            _               => view! { <GeneralTab     vm=vm_general.clone()     /> }.into_any(),
        }}
    }
}
