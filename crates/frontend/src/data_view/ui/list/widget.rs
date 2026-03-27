//! DataView list page — карточки всех зарегистрированных DataView.

use crate::data_view::api;
use crate::data_view::types::DataViewMeta;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use thaw::*;

fn category_badge(category: &str) -> AnyView {
    let (color, label) = match category {
        "revenue" => (BadgeColor::Success, "Выручка".to_string()),
        "orders" => (BadgeColor::Informative, "Заказы".to_string()),
        "costs" => (BadgeColor::Warning, "Затраты".to_string()),
        "returns" => (BadgeColor::Danger, "Возвраты".to_string()),
        other => (BadgeColor::Subtle, other.to_string()),
    };
    view! { <Badge color=color>{label}</Badge> }.into_any()
}

#[component]
#[allow(non_snake_case)]
pub fn DataViewList() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let (items, set_items) = signal::<Vec<DataViewMeta>>(vec![]);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let filter = RwSignal::new(String::new());

    Effect::new(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match api::fetch_list().await {
                Ok(data) => {
                    set_items.set(data);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    });

    let filtered = Signal::derive(move || {
        let q = filter.get().to_lowercase();
        items
            .get()
            .into_iter()
            .filter(|m| {
                q.is_empty()
                    || m.id.to_lowercase().contains(&q)
                    || m.name.to_lowercase().contains(&q)
                    || m.category.to_lowercase().contains(&q)
                    || m.description.to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
    });

    view! {
        <PageFrame page_id="data_view--list" category="list">
            // ── Header ──────────────────────────────────────────────────────
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">{icon("layers")} " DataView — Семантический слой"</h1>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| {
                            wasm_bindgen_futures::spawn_local(async move {
                                set_loading.set(true);
                                match api::fetch_list().await {
                                    Ok(data) => { set_items.set(data); set_loading.set(false); }
                                    Err(e)   => { set_error.set(Some(e)); set_loading.set(false); }
                                }
                            });
                        }
                    >
                        {icon("refresh")} " Обновить"
                    </Button>
                </div>
            </div>

            // ── Toolbar ─────────────────────────────────────────────────────
            <div class="page__toolbar">
                <Input value=filter placeholder="Поиск по id, названию, категории..." />
            </div>

            // ── Error ────────────────────────────────────────────────────────
            {move || error.get().map(|e| view! {
                <div class="warning-box warning-box--error">
                    <span class="warning-box__icon">"⚠"</span>
                    <span class="warning-box__text">{e}</span>
                </div>
            })}

            // ── Content ──────────────────────────────────────────────────────
            <div class="page__content">
                {move || {
                    if loading.get() {
                        view! {
                            <div class="placeholder">"Загрузка DataView каталога..."</div>
                        }.into_any()
                    } else if filtered.get().is_empty() {
                        view! {
                            <div class="placeholder">"Нет DataView, соответствующих фильтру."</div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="dv-card-grid">
                                {filtered.get().into_iter().map(|meta| {
                                    let id = meta.id.clone();
                                    let tab_key   = format!("data_view_details_{}", id);
                                    let tab_title = format!("DataView · {}", id);
                                    let ctx_open  = ctx.clone();

                                    view! {
                                        <div class="dv-card" on:click=move |_| {
                                            ctx_open.open_tab(&tab_key, &tab_title);
                                        }>
                                            <div class="dv-card__header">
                                                <span class="dv-card__id">{meta.id.clone()}</span>
                                                {category_badge(&meta.category)}
                                                <span class="dv-card__version">"v"{meta.version}</span>
                                            </div>
                                            <div class="dv-card__name">{meta.name.clone()}</div>
                                            <div class="dv-card__description">{meta.description.clone()}</div>
                                            <div class="dv-card__sources">
                                                {icon("database")}
                                                {meta.data_sources.join(", ")}
                                            </div>
                                            <div class="dv-card__dims">
                                                {icon("layers")}
                                                {format!("{} измерений", meta.available_dimensions.len())}
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}
