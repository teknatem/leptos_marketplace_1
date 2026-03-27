//! FilterRegistry — страница просмотра глобального реестра фильтров.

use crate::data_view::api;
use crate::data_view::types::{FilterDef, FilterKind};
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use thaw::*;

// ── Kind badge ────────────────────────────────────────────────────────────────

fn kind_badge(kind: &FilterKind) -> AnyView {
    let (color, label) = match kind {
        FilterKind::DateRange { .. } => (BadgeColor::Informative, "DateRange"),
        FilterKind::MultiSelect { .. } => (BadgeColor::Success, "MultiSelect"),
        FilterKind::Select { .. } => (BadgeColor::Warning, "Select"),
        FilterKind::Text => (BadgeColor::Subtle, "Text"),
    };
    view! { <Badge color=color>{label}</Badge> }.into_any()
}

fn kind_detail(kind: &FilterKind) -> AnyView {
    match kind {
        FilterKind::DateRange { from_id, to_id } => {
            let f = from_id.clone();
            let t = to_id.clone();
            view! {
                <span class="filter-reg__detail">
                    "from: "
                    <code class="filter-reg__code">{f}</code>
                    " / to: "
                    <code class="filter-reg__code">{t}</code>
                </span>
            }
            .into_any()
        }
        FilterKind::MultiSelect { source } => {
            let src = source.clone();
            view! {
                <span class="filter-reg__detail">
                    "source: "
                    <code class="filter-reg__code">{src}</code>
                </span>
            }
            .into_any()
        }
        FilterKind::Select { options } => {
            let opts = options.clone();
            view! {
                <div class="filter-reg__options">
                    {opts.into_iter().map(|o| view! {
                        <span class="filter-reg__option">
                            <code class="filter-reg__code">{o.value}</code>
                            " — " {o.label}
                        </span>
                    }).collect_view()}
                </div>
            }
            .into_any()
        }
        FilterKind::Text => view! { <span /> }.into_any(),
    }
}

// ── FilterRegistryPage ────────────────────────────────────────────────────────

#[component]
#[allow(non_snake_case)]
pub fn FilterRegistryPage() -> impl IntoView {
    let (filters, set_filters) = signal::<Vec<FilterDef>>(vec![]);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let search = RwSignal::new(String::new());

    let load = move || {
        set_loading.set(true);
        set_error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match api::fetch_global_filters().await {
                Ok(mut data) => {
                    data.sort_by(|a, b| a.id.cmp(&b.id));
                    set_filters.set(data);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| load());

    let filtered = Signal::derive(move || {
        let q = search.get().to_lowercase();
        filters
            .get()
            .into_iter()
            .filter(|f| {
                q.is_empty()
                    || f.id.to_lowercase().contains(&q)
                    || f.label.to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
    });

    view! {
        <PageFrame page_id="filter-registry" category="list">

            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {icon("filter")}
                        " Реестр фильтров"
                    </h1>
                    <span class="text-muted" style="font-size: 13px;">
                        "Глобальный реестр всех допустимых фильтров системы"
                    </span>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load()
                        disabled=loading
                    >
                        {icon("refresh")} " Обновить"
                    </Button>
                </div>
            </div>

            <div class="page__toolbar">
                <Input value=search placeholder="Поиск по id или метке..." />
                {move || {
                    let total = filters.get().len();
                    let shown = filtered.get().len();
                    if total > 0 {
                        view! {
                            <span class="text-muted" style="font-size: 12px; align-self: center;">
                                {if shown == total {
                                    format!("{total} фильтров")
                                } else {
                                    format!("{shown} из {total}")
                                }}
                            </span>
                        }.into_any()
                    } else {
                        view! { <span /> }.into_any()
                    }
                }}
            </div>

            {move || error.get().map(|e| view! {
                <div class="warning-box warning-box--error">
                    <span class="warning-box__icon">"⚠"</span>
                    <span class="warning-box__text">{e}</span>
                </div>
            })}

            <div class="page__content">
                {move || {
                    if loading.get() {
                        return view! {
                            <div class="placeholder">"Загрузка реестра фильтров..."</div>
                        }.into_any();
                    }
                    let items = filtered.get();
                    if items.is_empty() {
                        return view! {
                            <div class="placeholder">"Нет фильтров, соответствующих запросу."</div>
                        }.into_any();
                    }
                    view! {
                        <div class="filter-reg__table">
                            // ── Header ──────────────────────────────────────
                            <div class="filter-reg__row filter-reg__row--header">
                                <span class="filter-reg__col filter-reg__col--id">"ID"</span>
                                <span class="filter-reg__col filter-reg__col--label">"Метка"</span>
                                <span class="filter-reg__col filter-reg__col--kind">"Тип"</span>
                                <span class="filter-reg__col filter-reg__col--details">"Детали"</span>
                            </div>
                            // ── Rows ─────────────────────────────────────────
                            {items.into_iter().map(|f| {
                                let id   = f.id.clone();
                                let lbl  = f.label.clone();
                                let kind_badge_view = kind_badge(&f.kind);
                                let kind_detail_view = kind_detail(&f.kind);
                                view! {
                                    <div class="filter-reg__row">
                                        <span class="filter-reg__col filter-reg__col--id">
                                            <code class="filter-reg__id">{id}</code>
                                        </span>
                                        <span class="filter-reg__col filter-reg__col--label">{lbl}</span>
                                        <span class="filter-reg__col filter-reg__col--kind">
                                            {kind_badge_view}
                                        </span>
                                        <span class="filter-reg__col filter-reg__col--details">
                                            {kind_detail_view}
                                        </span>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_any()
                }}
            </div>

        </PageFrame>
    }
}
