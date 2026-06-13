//! Динамическая категория мега-меню «Плагины».
//!
//! Список строится из `GET /api/plugin` (включённые плагины). Клик открывает
//! вкладку с ключом `plugin__<id>` — её рендерит `PluginHost` через `render_tab_content`.
//! Видна только администратору (плагины — admin-only).

use crate::layout::global_context::AppGlobalContext;
use crate::plugins::api;
use crate::shared::icons;
use crate::shared::icons::icon;
use contracts::plugins::PluginListItem;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn PluginsMenuCategory() -> impl IntoView {
    let (is_open, set_is_open) = signal(false);
    let (items, set_items) = signal(Vec::<PluginListItem>::new());

    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    // Загрузка списка плагинов при монтировании.
    spawn_local(async move {
        if let Ok(list) = api::list_enabled().await {
            set_items.set(list);
        }
    });

    view! {
        <div
            class="mega-menu-category"
            on:mouseenter=move |_| set_is_open.set(true)
            on:mouseleave=move |_| set_is_open.set(false)
        >
            <button
                class="mega-menu-btn"
                class:mega-menu-btn-active=move || is_open.get()
            >
                <span>"Плагины"</span>
                <span
                    class="mega-menu-chevron"
                    class:mega-menu-chevron-open=move || is_open.get()
                >
                    {icons::icon("chevron-down")}
                </span>
            </button>

            <div
                class="mega-menu-panel"
                class:mega-menu-panel-open=move || is_open.get()
            >
                <div class="mega-menu-content mega-menu-grid-1">
                    <button
                        class="mega-menu-card"
                        on:click=move |_| {
                            tabs_store.open_tab("plugins", "Плагины — реестр");
                            set_is_open.set(false);
                        }
                    >
                        <div class="mega-menu-card-icon">
                            {icons::icon("table")}
                        </div>
                        <div class="mega-menu-card-title">
                            "Реестр плагинов"
                        </div>
                    </button>
                    {move || {
                        let list = items.get();
                        if list.is_empty() {
                            view! {
                                <div class="mega-menu-empty">"Нет доступных плагинов"</div>
                            }.into_any()
                        } else {
                            list.into_iter().map(|p| {
                                let tabs_store = tabs_store.clone();
                                let key = format!("plugin__{}", p.id);
                                let title = p.title.clone();
                                view! {
                                    <button
                                        class="mega-menu-card"
                                        on:click=move |_| {
                                            tabs_store.open_tab(&key, &title);
                                            set_is_open.set(false);
                                        }
                                    >
                                        <div class="mega-menu-card-icon">
                                            {icons::icon("package")}
                                        </div>
                                        <div class="mega-menu-card-title">
                                            {p.title.clone()}
                                        </div>
                                    </button>
                                }
                            }).collect_view().into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

/// Динамическая группа левого сайдбара «Плагины» (admin-only).
///
/// Сайдбар построен на статических `&'static str`-ключах; плагины динамические,
/// поэтому это отдельный компонент, который сам тянет `GET /api/plugin` и рендерит
/// сворачиваемую группу теми же CSS-классами, что и статические группы.
#[component]
pub fn PluginsSidebarGroup() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (items, set_items) = signal(Vec::<PluginListItem>::new());
    let expanded = RwSignal::new(false);

    spawn_local(async move {
        if let Ok(list) = api::list_enabled().await {
            set_items.set(list);
        }
    });

    view! {
        <div>
            <div
                class="app-sidebar__item"
                style:padding-left="12px"
                on:click=move |_| expanded.update(|v| *v = !*v)
            >
                <div class="app-sidebar__item-content">
                    {icon("box")}
                    <span>"Плагины"</span>
                </div>
                <div
                    class="app-sidebar__chevron"
                    class:app-sidebar__chevron--expanded=move || expanded.get()
                >
                    {icon("chevron-right")}
                </div>
            </div>

            <Show when=move || expanded.get()>
                <div class="app-sidebar__children">
                    <div
                        class="app-sidebar__item"
                        style:padding-left="10px"
                        on:click=move |_| ctx.open_tab("plugins", "Плагины — реестр")
                    >
                        <div class="app-sidebar__item-content">
                            {icon("table")}
                            <span>"Реестр плагинов"</span>
                        </div>
                    </div>
                    {move || {
                        items.get().into_iter().map(|p| {
                            let key = format!("plugin__{}", p.id);
                            let title = p.title.clone();
                            let title_label = p.title.clone();
                            view! {
                                <div
                                    class="app-sidebar__item"
                                    style:padding-left="10px"
                                    on:click=move |_| ctx.open_tab(&key, &title)
                                >
                                    <div class="app-sidebar__item-content">
                                        {icon("package")}
                                        <span>{title_label}</span>
                                    </div>
                                </div>
                            }
                        }).collect_view()
                    }}
                </div>
            </Show>
        </div>
    }
}
