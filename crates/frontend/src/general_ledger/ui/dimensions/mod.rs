use std::collections::{HashMap, HashSet};

use crate::general_ledger::api::fetch_gl_dimensions_catalog;
use crate::layout::global_context::AppGlobalContext;
use crate::layout::tabs::tab_label_for_key;
use crate::shared::clipboard::copy_to_clipboard;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_LIST;
use contracts::general_ledger::{GlDimensionCatalogItem, GlDimensionDef};
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

#[derive(Clone)]
pub struct DimensionPreviewGroup {
    pub root: GlDimensionDef,
    pub children: Vec<GlDimensionDef>,
}

pub fn dimension_search_text(dimensions: &[GlDimensionDef]) -> String {
    dimensions
        .iter()
        .flat_map(|dimension| {
            [
                dimension.id.clone(),
                dimension.label.clone(),
                dimension.code.clone(),
                dimension.code_main.clone(),
                dimension.code_suffix.clone().unwrap_or_default(),
                dimension.parent_id.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn group_dimension_preview(dimensions: &[GlDimensionDef]) -> Vec<DimensionPreviewGroup> {
    let mut roots = Vec::new();
    let mut children_by_parent: HashMap<String, Vec<GlDimensionDef>> = HashMap::new();

    for dimension in dimensions {
        if let Some(parent_id) = dimension.parent_id.as_deref() {
            children_by_parent
                .entry(parent_id.to_string())
                .or_default()
                .push(dimension.clone());
        } else {
            roots.push(dimension.clone());
        }
    }

    let mut groups = roots
        .into_iter()
        .map(|root| {
            let mut children = children_by_parent.remove(&root.id).unwrap_or_default();
            children.sort_by(|left, right| left.code.cmp(&right.code));
            DimensionPreviewGroup { root, children }
        })
        .collect::<Vec<_>>();

    for (_, mut orphan_children) in children_by_parent {
        orphan_children.sort_by(|left, right| left.code.cmp(&right.code));
        for child in orphan_children {
            groups.push(DimensionPreviewGroup {
                root: child,
                children: Vec::new(),
            });
        }
    }

    groups
}

fn catalog_search_text(item: &GlDimensionCatalogItem) -> String {
    let usage_text = item
        .used_by_turnovers
        .iter()
        .flat_map(|usage| {
            [
                usage.turnover_code.clone(),
                usage.turnover_name.clone(),
                usage.report_group.clone(),
            ]
        })
        .collect::<Vec<_>>()
        .join(" ");

    [
        item.id.clone(),
        item.label.clone(),
        item.code.clone(),
        item.code_main.clone(),
        item.code_suffix.clone().unwrap_or_default(),
        item.parent_id.clone().unwrap_or_default(),
        item.root_id.clone(),
        item.path_codes.join(" "),
        item.db_field.clone(),
        usage_text,
    ]
    .join(" ")
}

fn matches_catalog_filters(
    item: &GlDimensionCatalogItem,
    search: &str,
    turnover_filter: Option<&str>,
) -> bool {
    if let Some(turnover_code) = turnover_filter {
        if !item
            .used_by_turnovers
            .iter()
            .any(|usage| usage.turnover_code == turnover_code)
        {
            return false;
        }
    }

    if search.is_empty() {
        return true;
    }

    catalog_search_text(item).to_lowercase().contains(search)
}

#[component]
pub fn DimensionPreview(dimensions: Vec<GlDimensionDef>) -> impl IntoView {
    let groups = group_dimension_preview(&dimensions);

    if groups.is_empty() {
        return view! {
            <div class="gldim-preview">
                <span style="opacity: 0.65;">"—"</span>
            </div>
        }
        .into_any();
    }

    view! {
        <div class="gldim-preview">
            <For
                each=move || groups.clone()
                key=|group| group.root.id.clone()
                children=move |group| {
                    view! {
                        <div class="gldim-preview__group">
                            <span class="gldim-preview__root">{group.root.code.clone()}</span>
                            {if group.children.is_empty() {
                                view! { <></> }.into_any()
                            } else {
                                view! {
                                    <span class="gldim-preview__children">
                                        {group
                                            .children
                                            .into_iter()
                                            .map(|child| {
                                                view! {
                                                    <span class="gldim-preview__child">
                                                        {child.code_suffix.unwrap_or_default()}
                                                    </span>
                                                }
                                            })
                                            .collect_view()}
                                    </span>
                                }
                                .into_any()
                            }}
                        </div>
                    }
                }
            />
        </div>
    }
    .into_any()
}

#[component]
pub fn GeneralLedgerDimensionsPage(initial_turnover_code: Option<String>) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let (items, set_items) = signal(Vec::<GlDimensionCatalogItem>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    let search_query = RwSignal::new(String::new());
    let turnover_filter = RwSignal::new(initial_turnover_code.unwrap_or_default());
    let selected_id = RwSignal::new(String::new());
    let loaded = RwSignal::new(false);

    let load_catalog = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            match fetch_gl_dimensions_catalog().await {
                Ok(response) => {
                    set_items.set(response.items);
                    loaded.set(true);
                }
                Err(err) => set_error.set(Some(err)),
            }

            set_loading.set(false);
        });
    };

    Effect::new(move |_| {
        if !loaded.get() {
            load_catalog();
        }
    });

    let direct_matches = Signal::derive(move || {
        let search = search_query.get().trim().to_lowercase();
        let turnover_filter = turnover_filter.get().trim().to_string();
        let turnover_filter = if turnover_filter.is_empty() {
            None
        } else {
            Some(turnover_filter)
        };

        items
            .get()
            .into_iter()
            .filter(|item| matches_catalog_filters(item, &search, turnover_filter.as_deref()))
            .collect::<Vec<_>>()
    });

    let visible_items = Signal::derive(move || {
        let matched = direct_matches.get();
        let mut visible_ids = HashSet::new();

        for item in &matched {
            visible_ids.insert(item.id.clone());
            for path_id in &item.path_ids {
                visible_ids.insert(path_id.clone());
            }
        }

        items
            .get()
            .into_iter()
            .filter(|item| visible_ids.contains(&item.id))
            .collect::<Vec<_>>()
    });

    let visible_roots = Signal::derive(move || {
        visible_items
            .get()
            .into_iter()
            .filter(|item| item.parent_id.is_none())
            .collect::<Vec<_>>()
    });

    let filtered_count = Signal::derive(move || direct_matches.get().len());
    let total_count = Signal::derive(move || items.get().len());

    Effect::new(move |_| {
        let current_selected = selected_id.get();
        let visible = visible_items.get();
        let matches = direct_matches.get();

        if visible.is_empty() {
            if !current_selected.is_empty() {
                selected_id.set(String::new());
            }
            return;
        }

        if current_selected.is_empty() || !visible.iter().any(|item| item.id == current_selected) {
            if let Some(next) = matches.first().or_else(|| visible.first()) {
                if current_selected != next.id {
                    selected_id.set(next.id.clone());
                }
            }
        }
    });

    let selected_item = Signal::derive(move || {
        let selected = selected_id.get();
        visible_items
            .get()
            .into_iter()
            .find(|item| item.id == selected)
            .or_else(|| direct_matches.get().into_iter().next())
    });

    view! {
        <PageFrame page_id="general_ledger_dimensions--catalog" category=PAGE_CAT_LIST class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Измерения GL"</h1>
                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                        {move || filtered_count.get().to_string()}
                    </Badge>
                    <span style="font-size: 12px; opacity: 0.75;">
                        {move || format!("из {}", total_count.get())}
                    </span>
                    <Show when=move || !turnover_filter.get().trim().is_empty()>
                        <Badge appearance=BadgeAppearance::Outline color=BadgeColor::Success>
                            {move || format!("turnover: {}", turnover_filter.get())}
                        </Badge>
                    </Show>
                </div>

                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| {
                            tabs_store.open_tab("general_ledger_turnovers", tab_label_for_key("general_ledger_turnovers"));
                        }
                    >
                        "Обороты GL"
                    </Button>

                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load_catalog()
                        disabled=Signal::derive(move || loading.get())
                    >
                        {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                    </Button>
                </div>
            </div>

            <div class="page__content gldim-page">
                {move || error.get().map(|err| view! {
                    <div class="alert alert--error">{err}</div>
                })}

                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div class="filter-panel-header__left">
                            <span class="filter-panel__title">"Фильтры"</span>
                        </div>
                    </div>

                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End style="flex-wrap: wrap;">
                            <div style="width: 320px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Поиск"</Label>
                                    <Input value=search_query placeholder="code, id, db_field, turnover_code..." />
                                </Flex>
                            </div>

                            <div style="width: 240px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Фильтр по обороту"</Label>
                                    <Input value=turnover_filter placeholder="customer_revenue, advert_clicks_order_accrual..." />
                                </Flex>
                            </div>

                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| turnover_filter.set(String::new())
                            >
                                "Сбросить"
                            </Button>
                        </Flex>
                    </div>
                </div>

                <div class="gldim-split">
                    // ── Левая панель: дерево ──────────────────────────────────────────
                    <div class="gldim-panel">
                        <div class="gldim-panel__header">
                            <div class="gldim-panel__title">"Измерения"</div>
                            <Badge appearance=BadgeAppearance::Tint>
                                {move || visible_items.get().len().to_string()}
                            </Badge>
                        </div>

                        <div class="gldim-tree">
                            <Show
                                when=move || !visible_roots.get().is_empty()
                                fallback=move || view! {
                                    <div class="gldim-tree__empty">
                                        {if loading.get() { "Загрузка..." } else { "Нет измерений по текущему фильтру." }}
                                    </div>
                                }
                            >
                                <For
                                    each=move || visible_roots.get()
                                    key=|item| item.id.clone()
                                    children=move |root| {
                                        let root_code = root.code.clone();
                                        let root_label = root.label.clone();
                                        let root_id_for_children = root.id.clone();
                                        let root_selected_id = root.id.clone();
                                        let root_id_for_active = root.id.clone();
                                        view! {
                                            <div class="gldim-tree__group">
                                                <button
                                                    type="button"
                                                    class="gldim-tree__btn"
                                                    class:gldim-tree__btn--active=move || selected_id.get() == root_id_for_active
                                                    on:click=move |_| selected_id.set(root_selected_id.clone())
                                                >
                                                    <span class="gldim-tree__code">{root_code}</span>
                                                    <span class="gldim-tree__label">{root_label}</span>
                                                </button>
                                                <For
                                                    each=move || {
                                                        visible_items
                                                            .get()
                                                            .into_iter()
                                                            .filter(|item| item.parent_id.as_deref() == Some(root_id_for_children.as_str()))
                                                            .collect::<Vec<_>>()
                                                    }
                                                    key=|item| item.id.clone()
                                                    children=move |child| {
                                                        let child_code = child.code.clone();
                                                        let child_label = child.label.clone();
                                                        let child_id = child.id.clone();
                                                        let child_id_for_active = child.id.clone();
                                                        view! {
                                                            <button
                                                                type="button"
                                                                class="gldim-tree__btn gldim-tree__btn--child"
                                                                class:gldim-tree__btn--active=move || selected_id.get() == child_id_for_active
                                                                on:click=move |_| selected_id.set(child_id.clone())
                                                            >
                                                                <span class="gldim-tree__code">{child_code}</span>
                                                                <span class="gldim-tree__label">{child_label}</span>
                                                            </button>
                                                        }
                                                    }
                                                />
                                            </div>
                                        }
                                    }
                                />
                            </Show>
                        </div>
                    </div>

                    // ── Правая панель: карточка измерения ─────────────────────────────
                    <div class="gldim-panel">
                        {move || {
                            selected_item
                                .get()
                                .map(|item| {
                                    let code = item.code.clone();
                                    let label = item.label.clone();
                                    let id = item.id.clone();
                                    let db_field = item.db_field.clone();
                                    let parent_id = item.parent_id.clone();
                                    let path_codes = item.path_codes.clone();
                                    let turnover_count = item.turnover_count;
                                    let used_by_turnovers = item.used_by_turnovers.clone();

                                    let code_copy = code.clone();
                                    let id_copy = id.clone();
                                    let db_field_copy = db_field.clone();
                                    let has_parent = parent_id.is_some();
                                    let has_usage = !used_by_turnovers.is_empty();
                                    let path_codes_for_show = path_codes.clone();
                                    let has_path = path_codes_for_show.len() > 1;

                                    view! {
                                        <div class="gldim-details">
                                            // Hero
                                            <div class="gldim-hero">
                                                <span class="gldim-hero__code">{code.clone()}</span>
                                                <div class="gldim-hero__body">
                                                    <div class="gldim-hero__label">{label}</div>
                                                    <div class="gldim-hero__field">
                                                        <span class="gldim-hero__field-icon">"⊞"</span>
                                                        <code>{db_field.clone()}</code>
                                                        <button
                                                            type="button"
                                                            class="gldim-copy-btn"
                                                            title="Copy db_field"
                                                            on:click=move |_| copy_to_clipboard(&db_field_copy)
                                                        >
                                                            "⎘"
                                                        </button>
                                                    </div>
                                                </div>
                                            </div>

                                            // Идентификаторы
                                            <div class="gldim-section">
                                                <div class="gldim-section__title">"Идентификаторы"</div>
                                                <div class="gldim-kv">
                                                    <div class="gldim-kv__row">
                                                        <span class="gldim-kv__key">"code"</span>
                                                        <span class="gldim-kv__val">
                                                            <code>{code.clone()}</code>
                                                            <button
                                                                type="button"
                                                                class="gldim-copy-btn"
                                                                title="Copy code"
                                                                on:click=move |_| copy_to_clipboard(&code_copy)
                                                            >
                                                                "⎘"
                                                            </button>
                                                        </span>
                                                    </div>
                                                    <div class="gldim-kv__row">
                                                        <span class="gldim-kv__key">"id"</span>
                                                        <span class="gldim-kv__val">
                                                            <code>{id.clone()}</code>
                                                            <button
                                                                type="button"
                                                                class="gldim-copy-btn"
                                                                title="Copy id"
                                                                on:click=move |_| copy_to_clipboard(&id_copy)
                                                            >
                                                                "⎘"
                                                            </button>
                                                        </span>
                                                    </div>
                                                    <Show when=move || has_parent>
                                                        <div class="gldim-kv__row">
                                                            <span class="gldim-kv__key">"parent"</span>
                                                            <span class="gldim-kv__val">
                                                                <code>{parent_id.clone().unwrap_or_default()}</code>
                                                            </span>
                                                        </div>
                                                    </Show>
                                                    <Show when=move || has_path>
                                                        <div class="gldim-kv__row">
                                                            <span class="gldim-kv__key">"path"</span>
                                                            <span class="gldim-kv__val">
                                                                <span class="gldim-breadcrumb">
                                                                    {path_codes
                                                                        .iter()
                                                                        .enumerate()
                                                                        .map(|(idx, code)| {
                                                                            let is_last = idx == path_codes.len() - 1;
                                                                            view! {
                                                                                <span class="gldim-breadcrumb__item" class:gldim-breadcrumb__item--active=is_last>
                                                                                    {code.clone()}
                                                                                </span>
                                                                                {if !is_last {
                                                                                    view! { <span class="gldim-breadcrumb__sep">" › "</span> }.into_any()
                                                                                } else {
                                                                                    view! { <></> }.into_any()
                                                                                }}
                                                                            }
                                                                        })
                                                                        .collect_view()}
                                                                </span>
                                                            </span>
                                                        </div>
                                                    </Show>
                                                </div>
                                            </div>

                                            // Обороты
                                            <div class="gldim-section">
                                                <div class="gldim-section__title">
                                                    "Обороты"
                                                    <span class="gldim-section__count">
                                                        {turnover_count.to_string()}
                                                    </span>
                                                </div>
                                                {if has_usage {
                                                    view! {
                                                        <div class="gldim-tags">
                                                            {used_by_turnovers
                                                                .into_iter()
                                                                .map(|usage| {
                                                                    let turnover_code = usage.turnover_code.clone();
                                                                    let label = usage.turnover_code.clone();
                                                                    let name = usage.turnover_name.clone();
                                                                    view! {
                                                                        <button
                                                                            type="button"
                                                                            class="gldim-tag"
                                                                            title=name
                                                                            on:click=move |_| {
                                                                                tabs_store.open_tab(
                                                                                    &format!("general_ledger_turnover_details_{}", turnover_code),
                                                                                    &label,
                                                                                );
                                                                            }
                                                                        >
                                                                            {label.clone()}
                                                                        </button>
                                                                    }
                                                                })
                                                                .collect_view()}
                                                        </div>
                                                    }
                                                    .into_any()
                                                } else {
                                                    view! {
                                                        <div class="gldim-tree__empty">"Нет связанных оборотов."</div>
                                                    }
                                                    .into_any()
                                                }}
                                            </div>
                                        </div>
                                    }
                                    .into_any()
                                })
                                .unwrap_or_else(|| {
                                    view! {
                                        <div class="page__placeholder">"Выберите измерение слева."</div>
                                    }
                                    .into_any()
                                })
                        }}
                    </div>
                </div>
            </div>
        </PageFrame>
    }
}
