//! Layout tab — дерево категорий+индикаторов с DnD и IndicatorPicker drawer

use super::super::view_model::BiDashboardDetailsVm;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use leptos::prelude::*;
use std::collections::HashMap;
use thaw::*;
use wasm_bindgen::JsCast;

// ── Data structures ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ItemEdit {
    pub indicator_id: String,
    /// Display name cached from the picker (not part of contracts, serde-ignored by backend)
    #[serde(default)]
    pub indicator_name: String,
    pub sort_order: i32,
    pub col_class: String,
    #[serde(default)]
    pub param_overrides: HashMap<String, String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct GroupEdit {
    pub id: String,
    pub title: String,
    pub sort_order: i32,
    pub items: Vec<ItemEdit>,
    pub subgroups: Vec<GroupEdit>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct LayoutEdit {
    pub groups: Vec<GroupEdit>,
}

#[derive(Clone, Debug, PartialEq)]
enum DragItem {
    Category(usize),
    Indicator { group_idx: usize, item_idx: usize },
}

#[derive(Clone, Debug, serde::Deserialize)]
struct IndicatorRow {
    pub id: String,
    pub code: String,
    pub description: String,
    pub status: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_layout(json: &str) -> LayoutEdit {
    serde_json::from_str(json).unwrap_or(LayoutEdit { groups: vec![] })
}

fn serialize_layout(layout: &LayoutEdit) -> String {
    serde_json::to_string(layout).unwrap_or_else(|_| r#"{"groups":[]}"#.to_string())
}

fn new_group_id() -> String {
    let id = uuid::Uuid::new_v4().to_string();
    format!("g-{}", &id[..8])
}

/// Short display label: indicator_name if set, else first 8 chars of UUID
fn item_display(item: &ItemEdit) -> String {
    if !item.indicator_name.is_empty() {
        item.indicator_name.clone()
    } else if item.indicator_id.len() > 8 {
        format!("{}…", &item.indicator_id[..8])
    } else {
        item.indicator_id.clone()
    }
}

// ── Drop application ─────────────────────────────────────────────────────────

fn apply_drop(layout: &mut LayoutEdit, from: &DragItem, onto: &DragItem) {
    fn renumber_items(group: &mut GroupEdit) {
        for (i, item) in group.items.iter_mut().enumerate() {
            item.sort_order = i as i32;
        }
    }

    match (from, onto) {
        // Reorder categories
        (DragItem::Category(fi), DragItem::Category(ti)) => {
            let fi = *fi;
            let ti = *ti;
            if fi != ti && fi < layout.groups.len() && ti < layout.groups.len() {
                layout.groups.swap(fi, ti);
                for (i, g) in layout.groups.iter_mut().enumerate() {
                    g.sort_order = i as i32;
                }
            }
        }
        // Indicator → indicator (same or different group)
        (
            DragItem::Indicator {
                group_idx: sg,
                item_idx: si,
            },
            DragItem::Indicator {
                group_idx: tg,
                item_idx: ti,
            },
        ) => {
            let sg = *sg;
            let si = *si;
            let tg = *tg;
            let ti = *ti;
            if sg == tg {
                if si != ti && sg < layout.groups.len() {
                    let g = &mut layout.groups[sg];
                    if si < g.items.len() && ti < g.items.len() {
                        let item = g.items.remove(si);
                        let insert_at = if si < ti { ti - 1 } else { ti };
                        g.items.insert(insert_at, item);
                        renumber_items(g);
                    }
                }
            } else if sg < layout.groups.len()
                && tg < layout.groups.len()
                && si < layout.groups[sg].items.len()
            {
                let item = layout.groups[sg].items.remove(si);
                let insert_at = ti.min(layout.groups[tg].items.len());
                layout.groups[tg].items.insert(insert_at, item);
                renumber_items(&mut layout.groups[sg]);
                if sg != tg {
                    renumber_items(&mut layout.groups[tg]);
                }
            }
        }
        // Indicator → category header → append to that category
        (
            DragItem::Indicator {
                group_idx: sg,
                item_idx: si,
            },
            DragItem::Category(tg),
        ) => {
            let sg = *sg;
            let si = *si;
            let tg = *tg;
            if sg != tg
                && sg < layout.groups.len()
                && tg < layout.groups.len()
                && si < layout.groups[sg].items.len()
            {
                let item = layout.groups[sg].items.remove(si);
                layout.groups[tg].items.push(item);
                renumber_items(&mut layout.groups[sg]);
                if sg != tg {
                    renumber_items(&mut layout.groups[tg]);
                }
            }
        }
        _ => {}
    }
}

// ── a024 API fetch ────────────────────────────────────────────────────────────

async fn fetch_indicators(q: &str) -> Result<Vec<IndicatorRow>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let mut url = format!(
        "{}/api/a024-bi-indicator/list?limit=200&offset=0&sort_by=code&sort_desc=false",
        api_base()
    );
    let q_trimmed = q.trim();
    if q_trimmed.len() >= 2 {
        let encoded = js_sys::encode_uri_component(q_trimmed)
            .as_string()
            .unwrap_or_default();
        url.push_str(&format!("&q={}", encoded));
    }

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text_str = text.as_string().unwrap_or_default();

    let parsed: serde_json::Value = serde_json::from_str(&text_str).map_err(|e| e.to_string())?;
    let items = parsed["items"].as_array().cloned().unwrap_or_default();

    Ok(items
        .iter()
        .filter_map(|v| {
            Some(IndicatorRow {
                id: v["id"].as_str()?.to_string(),
                code: v["code"].as_str().unwrap_or("").to_string(),
                description: v["description"].as_str().unwrap_or("").to_string(),
                status: v["status"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect())
}

// ── LayoutTab ─────────────────────────────────────────────────────────────────

#[component]
pub fn LayoutTab(vm: BiDashboardDetailsVm) -> impl IntoView {
    let layout_json = vm.layout_json;
    let layout: RwSignal<LayoutEdit> = RwSignal::new(parse_layout(&layout_json.get_untracked()));

    // Sync inbound JSON changes
    Effect::new(move |_| {
        let json = layout_json.get();
        if json != serialize_layout(&layout.get_untracked()) {
            layout.set(parse_layout(&json));
        }
    });

    // Backfill indicator_name for items loaded from DB that don't have it yet.
    // Runs once on mount; fetches the full indicator list and fills in any
    // blank indicator_name fields so the tree shows names instead of UUIDs.
    Effect::new(move |_| {
        let needs_backfill = layout.with(|l| {
            l.groups
                .iter()
                .any(|g| g.items.iter().any(|i| i.indicator_name.is_empty()))
        });
        if !needs_backfill {
            return;
        }
        leptos::task::spawn_local(async move {
            if let Ok(rows) = fetch_indicators("").await {
                let name_map: std::collections::HashMap<String, String> = rows
                    .iter()
                    .map(|r| (r.id.clone(), format!("{} — {}", r.code, r.description)))
                    .collect();
                let mut changed = false;
                layout.update(|l| {
                    for g in &mut l.groups {
                        for item in &mut g.items {
                            if item.indicator_name.is_empty() {
                                if let Some(name) = name_map.get(&item.indicator_id) {
                                    item.indicator_name = name.clone();
                                    changed = true;
                                }
                            }
                        }
                    }
                });
                if changed {
                    layout_json.set(serialize_layout(&layout.get_untracked()));
                }
            }
        });
    });

    // DnD state
    let dragging: RwSignal<Option<DragItem>> = RwSignal::new(None);
    let drag_over: RwSignal<Option<DragItem>> = RwSignal::new(None);

    // Drawer state
    let drawer_open: RwSignal<bool> = RwSignal::new(false);
    let drawer_group_idx: RwSignal<Option<usize>> = RwSignal::new(None);

    let on_add_group = move |_| {
        layout.update(|l| {
            l.groups.push(GroupEdit {
                id: new_group_id(),
                title: format!("Категория {}", l.groups.len() + 1),
                sort_order: l.groups.len() as i32,
                items: vec![],
                subgroups: vec![],
            });
        });
        layout_json.set(serialize_layout(&layout.get_untracked()));
    };

    view! {
        <div class="details-tabs__content" style="width: 100%; display: flex; flex-direction: row; justify-content: center;">
                <div class="details-section">
                    // Header + "New category" button
                    <div class="details-section__header ltree-section-header">
                        <Button
                            appearance=ButtonAppearance::Primary
                            size=ButtonSize::Small
                            on_click=on_add_group
                        >
                            {icon("folder-plus")} " Новая категория"
                        </Button>
                    </div>

                    <div class="ltree">
                        {move || {
                            let groups = layout.with(|l| l.groups.clone());

                            if groups.is_empty() {
                                return view! {
                                    <div class="ltree__empty">
                                        "Нет категорий. Нажмите «Новая категория»."
                                    </div>
                                }.into_any();
                            }

                            groups.into_iter().enumerate().map(|(gi, group)| {
                                let cat_class = move || {
                                    let is_dragging = dragging.with(|d| *d == Some(DragItem::Category(gi)));
                                    let is_over    = drag_over.with(|d| *d == Some(DragItem::Category(gi)));
                                    match (is_dragging, is_over) {
                                        (true, _) => "ltree-cat ltree-cat--dragging",
                                        (_, true) => "ltree-cat ltree-cat--drop-target",
                                        _         => "ltree-cat",
                                    }
                                };

                                view! {
                                    <div class="ltree-cat-wrap">
                                        // ── Category header ─────────────────
                                        <div
                                            class=cat_class
                                            draggable="true"
                                            on:dragstart=move |ev: web_sys::DragEvent| {
                                                if let Some(dt) = ev.data_transfer() {
                                                    let _ = dt.set_data("text/plain", &gi.to_string());
                                                }
                                                dragging.set(Some(DragItem::Category(gi)));
                                            }
                                            on:dragend=move |_| {
                                                dragging.set(None);
                                                drag_over.set(None);
                                            }
                                            on:dragover=move |ev: web_sys::DragEvent| {
                                                ev.prevent_default();
                                                drag_over.set(Some(DragItem::Category(gi)));
                                            }
                                            on:dragleave=move |_| {
                                                drag_over.update(|d| {
                                                    if *d == Some(DragItem::Category(gi)) { *d = None; }
                                                });
                                            }
                                            on:drop=move |ev: web_sys::DragEvent| {
                                                ev.prevent_default();
                                                if let Some(src) = dragging.get_untracked() {
                                                    layout.update(|l| apply_drop(l, &src, &DragItem::Category(gi)));
                                                    layout_json.set(serialize_layout(&layout.get_untracked()));
                                                }
                                                dragging.set(None);
                                                drag_over.set(None);
                                            }
                                        >
                                            <span class="ltree-cat__grip">{icon("grab")}</span>
                                            <span class="ltree-cat__icon">{icon("layers")}</span>

                                            <input
                                                type="text"
                                                class="ltree-cat__title-input"
                                                value=group.title.clone()
                                                placeholder="Название категории"
                                                draggable="false"
                                                on:input=move |ev: web_sys::Event| {
                                                    let val = ev.target().unwrap()
                                                        .unchecked_into::<web_sys::HtmlInputElement>()
                                                        .value();
                                                    layout.update(|l| {
                                                        if let Some(g) = l.groups.get_mut(gi) {
                                                            g.title = val;
                                                        }
                                                    });
                                                    layout_json.set(serialize_layout(&layout.get_untracked()));
                                                }
                                                on:dragstart=|ev: web_sys::DragEvent| ev.prevent_default()
                                            />

                                            // Item count badge
                                            <span class="ltree-cat__count">
                                                {move || layout.with(|l| {
                                                    l.groups.get(gi).map(|g| g.items.len()).unwrap_or(0)
                                                })}
                                            </span>

                                            // Add indicators (green)
                                            <button
                                                class="ltree-btn ltree-btn--green"
                                                on:click=move |_| {
                                                    drawer_group_idx.set(Some(gi));
                                                    drawer_open.set(true);
                                                }
                                            >
                                                // Plus icon
                                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                                                    <line x1="12" y1="5" x2="12" y2="19"/>
                                                    <line x1="5" y1="12" x2="19" y2="12"/>
                                                </svg>
                                                " Добавить"
                                            </button>

                                            // Delete category (red)
                                            <button
                                                class="ltree-btn ltree-btn--red"
                                                on:click=move |_| {
                                                    layout.update(|l| {
                                                        if gi < l.groups.len() { l.groups.remove(gi); }
                                                    });
                                                    layout_json.set(serialize_layout(&layout.get_untracked()));
                                                }
                                            >
                                                // Trash icon
                                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                    <polyline points="3 6 5 6 21 6"/>
                                                    <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/>
                                                    <path d="M10 11v6"/>
                                                    <path d="M14 11v6"/>
                                                    <path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"/>
                                                </svg>
                                            </button>
                                        </div>

                                        // ── Indicator rows ──────────────────
                                        <div class="ltree-items">
                                            {group.items.into_iter().enumerate().map(|(ii, item)| {
                                                let display = item_display(&item);

                                                let item_class = move || {
                                                    let this = DragItem::Indicator { group_idx: gi, item_idx: ii };
                                                    let is_dragging = dragging.with(|d| *d == Some(this.clone()));
                                                    let is_over    = drag_over.with(|d| *d == Some(this.clone()));
                                                    match (is_dragging, is_over) {
                                                        (true, _) => "ltree-item ltree-item--dragging",
                                                        (_, true) => "ltree-item ltree-item--drop-target",
                                                        _         => "ltree-item",
                                                    }
                                                };

                                                view! {
                                                    <div
                                                        class=item_class
                                                        draggable="true"
                                                        on:dragstart=move |ev: web_sys::DragEvent| {
                                                            if let Some(dt) = ev.data_transfer() {
                                                                let _ = dt.set_data("text/plain", "item");
                                                            }
                                                            dragging.set(Some(DragItem::Indicator { group_idx: gi, item_idx: ii }));
                                                        }
                                                        on:dragend=move |_| {
                                                            dragging.set(None);
                                                            drag_over.set(None);
                                                        }
                                                        on:dragover=move |ev: web_sys::DragEvent| {
                                                            ev.prevent_default();
                                                            drag_over.set(Some(DragItem::Indicator { group_idx: gi, item_idx: ii }));
                                                        }
                                                        on:dragleave=move |_| {
                                                            drag_over.update(|d| {
                                                                if *d == Some(DragItem::Indicator { group_idx: gi, item_idx: ii }) {
                                                                    *d = None;
                                                                }
                                                            });
                                                        }
                                                        on:drop=move |ev: web_sys::DragEvent| {
                                                            ev.prevent_default();
                                                            if let Some(src) = dragging.get_untracked() {
                                                                layout.update(|l| apply_drop(l, &src, &DragItem::Indicator { group_idx: gi, item_idx: ii }));
                                                                layout_json.set(serialize_layout(&layout.get_untracked()));
                                                            }
                                                            dragging.set(None);
                                                            drag_over.set(None);
                                                        }
                                                    >
                                                        <span class="ltree-item__grip">{icon("grab")}</span>
                                                        <span class="ltree-item__icon">{icon("activity")}</span>

                                                        // Indicator name (read-only)
                                                        <span class="ltree-item__name">{display}</span>

                                                        // Remove (red ×)
                                                        <button
                                                            class="ltree-btn ltree-btn--ghost-red ltree-item__remove"
                                                            draggable="false"
                                                            on:click=move |_| {
                                                                layout.update(|l| {
                                                                    if let Some(g) = l.groups.get_mut(gi) {
                                                                        if ii < g.items.len() {
                                                                            g.items.remove(ii);
                                                                        }
                                                                    }
                                                                });
                                                                layout_json.set(serialize_layout(&layout.get_untracked()));
                                                            }
                                                            on:dragstart=|ev: web_sys::DragEvent| ev.prevent_default()
                                                        >
                                                            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                                                                <line x1="18" y1="6" x2="6" y2="18"/>
                                                                <line x1="6" y1="6" x2="18" y2="18"/>
                                                            </svg>
                                                        </button>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}

                                            {if layout.with(|l| l.groups.get(gi).map(|g| g.items.is_empty()).unwrap_or(true)) {
                                                view! {
                                                    <div class="ltree-items__empty">
                                                        "Нет индикаторов — нажмите «Добавить» или перетащите сюда"
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <div></div> }.into_any()
                                            }}
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>().into_any()
                        }}
                        </div>
                        // Muted hint outside the tree
                                        <p class="ltree__hint" style="margin-top: 4px;">
                                        "Перетаскивайте строки за ≡ · нажмите "
                                        <span class="ltree__hint-badge ltree__hint-badge--green">"＋"</span>
                                        " чтобы добавить индикаторы · "
                                        <span class="ltree__hint-badge ltree__hint-badge--red">"✕"</span>
                                        " чтобы удалить"
                                    </p>


                </div>

            // ── Indicator Picker Drawer ──────────────────────────────────────
            <OverlayDrawer
                open=drawer_open
                position=DrawerPosition::Right
                size=DrawerSize::Medium
                close_on_esc=true
            >
                <DrawerHeader>
                    <DrawerHeaderTitle>
                        {move || {
                            if let Some(gi) = drawer_group_idx.get() {
                                layout.with(|l| {
                                    format!(
                                        "Добавить в «{}»",
                                        l.groups.get(gi).map(|g| g.title.as_str()).unwrap_or("…")
                                    )
                                })
                            } else {
                                "Добавить индикаторы".to_string()
                            }
                        }}
                    </DrawerHeaderTitle>
                </DrawerHeader>
                <DrawerBody>
                    <IndicatorPicker
                        layout=layout
                        layout_json=layout_json
                        drawer_group_idx=drawer_group_idx
                        drawer_open=drawer_open
                    />
                </DrawerBody>
            </OverlayDrawer>
        </div>
    }
}

// ── IndicatorPicker ───────────────────────────────────────────────────────────

#[component]
fn IndicatorPicker(
    layout: RwSignal<LayoutEdit>,
    layout_json: RwSignal<String>,
    drawer_group_idx: RwSignal<Option<usize>>,
    drawer_open: RwSignal<bool>,
) -> impl IntoView {
    // Full list fetched once per drawer open
    let all_indicators: RwSignal<Vec<IndicatorRow>> = RwSignal::new(vec![]);
    let search_q: RwSignal<String> = RwSignal::new(String::new());
    let loading: RwSignal<bool> = RwSignal::new(false);
    let fetch_error: RwSignal<Option<String>> = RwSignal::new(None);
    let selected_ids: RwSignal<Vec<String>> = RwSignal::new(vec![]);

    // Fetch ALL indicators once when drawer opens; reset state on each open.
    // Uses prev return value (bool) to detect the open→true transition.
    Effect::new(move |was_open: Option<bool>| {
        let open = drawer_open.get();
        if open && was_open != Some(true) {
            selected_ids.set(vec![]);
            search_q.set(String::new());
            leptos::task::spawn_local(async move {
                loading.set(true);
                fetch_error.set(None);
                match fetch_indicators("").await {
                    Ok(rows) => all_indicators.set(rows),
                    Err(e) => fetch_error.set(Some(e)),
                }
                loading.set(false);
            });
        }
        open
    });

    // Client-side substring filter — instant, no flicker
    let filtered = Signal::derive(move || {
        let q = search_q.get();
        let q = q.trim().to_lowercase();
        all_indicators.with(|rows| {
            if q.is_empty() {
                rows.clone()
            } else {
                rows.iter()
                    .filter(|r| {
                        r.code.to_lowercase().contains(&q)
                            || r.description.to_lowercase().contains(&q)
                    })
                    .cloned()
                    .collect()
            }
        })
    });

    // IDs already present in the target category (shown as "already added")
    let already_in_cat = Signal::derive(move || {
        drawer_group_idx
            .get()
            .map(|gi| {
                layout.with(|l| {
                    l.groups
                        .get(gi)
                        .map(|g| {
                            g.items
                                .iter()
                                .map(|i| i.indicator_id.clone())
                                .collect::<std::collections::HashSet<String>>()
                        })
                        .unwrap_or_default()
                })
            })
            .unwrap_or_default()
    });

    let on_confirm = move |_| {
        let ids = selected_ids.get_untracked();
        let indicator_map: std::collections::HashMap<String, String> = all_indicators
            .with_untracked(|rows| {
                rows.iter()
                    .map(|r| (r.id.clone(), format!("{} — {}", r.code, r.description)))
                    .collect()
            });

        if let Some(gi) = drawer_group_idx.get_untracked() {
            layout.update(|l| {
                if let Some(group) = l.groups.get_mut(gi) {
                    let existing: std::collections::HashSet<String> =
                        group.items.iter().map(|i| i.indicator_id.clone()).collect();
                    for id in &ids {
                        if !existing.contains(id) {
                            let so = group.items.len() as i32;
                            let name = indicator_map.get(id).cloned().unwrap_or_default();
                            group.items.push(ItemEdit {
                                indicator_id: id.clone(),
                                indicator_name: name,
                                sort_order: so,
                                col_class: "1x1".to_string(),
                                param_overrides: HashMap::new(),
                            });
                        }
                    }
                }
            });
            layout_json.set(serialize_layout(&layout.get_untracked()));
        }
        selected_ids.set(vec![]);
        drawer_open.set(false);
    };

    let on_cancel = move |_| {
        selected_ids.set(vec![]);
        drawer_open.set(false);
    };

    view! {
        <div class="ind-picker">
            // Search — filters client-side, no API call
            <div class="ind-picker__search">
                <input
                    type="text"
                    class="form__input"
                    placeholder="Поиск по коду или названию…"
                    // prop:value keeps the DOM in sync when we reset on reopen
                    prop:value=move || search_q.get()
                    on:input=move |ev: web_sys::Event| {
                        let val = ev.target().unwrap()
                            .unchecked_into::<web_sys::HtmlInputElement>()
                            .value();
                        search_q.set(val);
                    }
                />
            </div>

            // Summary bar
            <div class="ind-picker__summary">
                {move || {
                    let n     = selected_ids.with(|v| v.len());
                    let shown = filtered.with(|v| v.len());
                    let total = all_indicators.with(|v| v.len());
                    if n == 0 {
                        view! {
                            <span class="ind-picker__summary-text">
                                {if shown == total {
                                    format!("Всего: {}", total)
                                } else {
                                    format!("Найдено: {} из {}", shown, total)
                                }}
                            </span>
                        }.into_any()
                    } else {
                        view! {
                            <span class="ind-picker__summary-text ind-picker__summary-text--active">
                                "Выбрано: " {n}
                            </span>
                            <button
                                class="ind-picker__clear-btn"
                                on:click=move |_| selected_ids.set(vec![])
                            >
                                "Снять всё"
                            </button>
                        }.into_any()
                    }
                }}
            </div>

            // Loading / error
            {move || loading.get().then(|| view! {
                <div class="ind-picker__loading">"Загрузка…"</div>
            })}
            {move || fetch_error.get().map(|e| view! {
                <div class="ind-picker__error">{e}</div>
            })}

            // List
            <div class="ind-picker__list">
                {move || {
                    let rows   = filtered.get();
                    let sel    = selected_ids.get();
                    let in_cat = already_in_cat.get();

                    if rows.is_empty() && !loading.get_untracked() {
                        return view! {
                            <div class="ind-picker__list-empty">"Нет индикаторов по запросу"</div>
                        }.into_any();
                    }

                    rows.into_iter().map(|row| {
                        let is_in_cat = in_cat.contains(&row.id);
                        let is_sel    = sel.contains(&row.id);
                        let row_id    = row.id.clone();

                        let row_class = if is_sel {
                            "ind-picker__row ind-picker__row--selected"
                        } else {
                            "ind-picker__row"
                        };

                        let status_class = match row.status.as_str() {
                            "active"   => "ind-picker__status ind-picker__status--active",
                            "archived" => "ind-picker__status ind-picker__status--archived",
                            _          => "ind-picker__status ind-picker__status--draft",
                        };
                        let status_label = match row.status.as_str() {
                            "active"   => "активен",
                            "archived" => "архив",
                            _          => "черновик",
                        };

                        view! {
                            <label class=row_class>
                                // prop:checked — sets DOM property, not HTML attribute.
                                // This is required for checkboxes to reflect reactive state
                                // correctly after user interaction (attribute is ignored by
                                // browsers once the element has been touched).
                                <input
                                    type="checkbox"
                                    class="ind-picker__checkbox"
                                    prop:checked=is_sel
                                    on:change=move |_| {
                                        selected_ids.update(|v| {
                                            if let Some(pos) = v.iter().position(|x| x == &row_id) {
                                                v.remove(pos);
                                            } else {
                                                v.push(row_id.clone());
                                            }
                                        });
                                    }
                                />
                                <span class="ind-picker__row-body">
                                    <span class="ind-picker__code">{row.code.clone()}</span>
                                    <span class="ind-picker__desc">{row.description.clone()}</span>
                                </span>
                                {if is_in_cat {
                                    view! {
                                        <span class="ind-picker__already-badge">"в списке"</span>
                                    }.into_any()
                                } else {
                                    view! {
                                        <span class=status_class>{status_label}</span>
                                    }.into_any()
                                }}
                            </label>
                        }
                    }).collect::<Vec<_>>().into_any()
                }}
            </div>

            // Footer
            <div class="ind-picker__footer">
                <Button
                    appearance=ButtonAppearance::Primary
                    on_click=on_confirm
                    disabled=Signal::derive(move || selected_ids.with(|v| v.is_empty()))
                >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="20 6 9 17 4 12"/>
                    </svg>
                    " Добавить в категорию"
                </Button>
                <Button appearance=ButtonAppearance::Secondary on_click=on_cancel>
                    "Отмена"
                </Button>
            </div>
        </div>
    }
}
