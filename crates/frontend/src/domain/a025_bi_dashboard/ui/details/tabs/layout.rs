//! Layout tab — tree editor for DashboardLayout (groups + items)

use super::super::view_model::BiDashboardDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;

/// Simple local structs for editing
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ItemEdit {
    pub indicator_id: String,
    pub sort_order: i32,
    pub col_class: String,
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

fn parse_layout(json: &str) -> LayoutEdit {
    serde_json::from_str(json).unwrap_or(LayoutEdit { groups: vec![] })
}

fn serialize_layout(layout: &LayoutEdit) -> String {
    serde_json::to_string_pretty(layout).unwrap_or_else(|_| r#"{"groups":[]}"#.to_string())
}

fn new_group_id() -> String {
    let id = uuid::Uuid::new_v4().to_string();
    format!("g-{}", &id[..8])
}

/// Renders a single indicator item row
#[component]
fn ItemRow(
    group_idx: usize,
    item_idx: usize,
    layout: RwSignal<LayoutEdit>,
    layout_json: RwSignal<String>,
) -> impl IntoView {
    let col_class_options = vec!["1x1", "2x1", "1x2", "2x2", "3x1", "full"];

    view! {
        <div class="layout-item">
            <div class="layout-item__indicator">
                <label class="form__label--sm">"ID индикатора"</label>
                <input
                    type="text"
                    class="form__input form__input--sm"
                    value=move || {
                        layout.with(|l| {
                            l.groups.get(group_idx)
                                .and_then(|g| g.items.get(item_idx))
                                .map(|i| i.indicator_id.clone())
                                .unwrap_or_default()
                        })
                    }
                    placeholder="UUID индикатора a024"
                    on:input=move |ev| {
                        use wasm_bindgen::JsCast;
                        let val = ev.target().unwrap()
                            .unchecked_into::<web_sys::HtmlInputElement>()
                            .value();
                        layout.update(|l| {
                            if let Some(g) = l.groups.get_mut(group_idx) {
                                if let Some(item) = g.items.get_mut(item_idx) {
                                    item.indicator_id = val;
                                }
                            }
                        });
                        layout_json.set(serialize_layout(&layout.get_untracked()));
                    }
                />
            </div>
            <div class="layout-item__col">
                <label class="form__label--sm">"Размер"</label>
                <select
                    class="form__select form__select--sm"
                    on:change=move |ev| {
                        use wasm_bindgen::JsCast;
                        let val = ev.target().unwrap()
                            .unchecked_into::<web_sys::HtmlSelectElement>()
                            .value();
                        layout.update(|l| {
                            if let Some(g) = l.groups.get_mut(group_idx) {
                                if let Some(item) = g.items.get_mut(item_idx) {
                                    item.col_class = val;
                                }
                            }
                        });
                        layout_json.set(serialize_layout(&layout.get_untracked()));
                    }
                >
                    {col_class_options.iter().map(|&opt| {
                        let is_selected = move || {
                            layout.with(|l| {
                                l.groups.get(group_idx)
                                    .and_then(|g| g.items.get(item_idx))
                                    .map(|i| i.col_class == opt)
                                    .unwrap_or(false)
                            })
                        };
                        view! {
                            <option value=opt selected=is_selected>{opt}</option>
                        }
                    }).collect::<Vec<_>>()}
                </select>
            </div>
            <Button
                size=ButtonSize::Small
                appearance=ButtonAppearance::Secondary
                on_click=move |_| {
                    layout.update(|l| {
                        if let Some(g) = l.groups.get_mut(group_idx) {
                            g.items.remove(item_idx);
                        }
                    });
                    layout_json.set(serialize_layout(&layout.get_untracked()));
                }
            >
                {icon("x")}
            </Button>
        </div>
    }
}

#[component]
pub fn LayoutTab(vm: BiDashboardDetailsVm) -> impl IntoView {
    let layout_json = vm.layout_json;

    let layout_state: RwSignal<LayoutEdit> =
        RwSignal::new(parse_layout(&layout_json.get_untracked()));

    // Keep layout_json in sync when layout_state is modified externally
    Effect::new(move |_| {
        let json = layout_json.get();
        let current = layout_state.get_untracked();
        let current_json = serialize_layout(&current);
        if json != current_json {
            layout_state.set(parse_layout(&json));
        }
    });

    let on_add_group = {
        let ls = layout_state.clone();
        let lj = layout_json.clone();
        move |_| {
            ls.update(|l| {
                l.groups.push(GroupEdit {
                    id: new_group_id(),
                    title: format!("Группа {}", l.groups.len() + 1),
                    sort_order: l.groups.len() as i32,
                    items: vec![],
                    subgroups: vec![],
                });
            });
            lj.set(serialize_layout(&ls.get_untracked()));
        }
    };

    view! {
        <div class="details-tabs__content">
            <CardAnimated delay_ms=0>
                <div class="details-section">
                    <div class="details-section__header">
                        <h4 class="details-section__title">"Структура дашборда"</h4>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            size=ButtonSize::Small
                            on_click=on_add_group
                        >
                            {icon("folder-plus")} " Добавить группу"
                        </Button>
                    </div>
                    <p class="form__hint">
                        "Организуйте индикаторы по группам (категориям). Каждая группа может содержать вложенные подгруппы. "
                        "Размер карточки задаётся как «ширина x высота» (например «2x1» = 2 колонки, 1 ряд)."
                    </p>

                    {move || {
                        let groups = layout_state.with(|l| l.groups.clone());
                        if groups.is_empty() {
                            return view! {
                                <div class="placeholder placeholder--small">
                                    "Нет групп. Нажмите «Добавить группу»."
                                </div>
                            }.into_any();
                        }

                        groups.into_iter().enumerate().map(|(gi, group)| {
                            let ls = layout_state.clone();
                            let lj = layout_json.clone();
                            let ls_del = ls.clone();
                            let lj_del = lj.clone();
                            let ls_item = ls.clone();
                            let lj_item = lj.clone();

                            view! {
                                <div class="layout-group">
                                    <div class="layout-group__header">
                                        <input
                                            type="text"
                                            class="form__input layout-group__title-input"
                                            value=group.title.clone()
                                            placeholder="Название группы"
                                            on:input=move |ev| {
                                                use wasm_bindgen::JsCast;
                                                let val = ev.target().unwrap()
                                                    .unchecked_into::<web_sys::HtmlInputElement>()
                                                    .value();
                                                ls.update(|l| {
                                                    if let Some(g) = l.groups.get_mut(gi) {
                                                        g.title = val;
                                                    }
                                                });
                                                lj.set(serialize_layout(&ls.get_untracked()));
                                            }
                                        />
                                        <Button
                                            size=ButtonSize::Small
                                            appearance=ButtonAppearance::Secondary
                                            on_click=move |_| {
                                                ls_item.update(|l| {
                                                    if let Some(g) = l.groups.get_mut(gi) {
                                                        g.items.push(ItemEdit {
                                                            indicator_id: String::new(),
                                                            sort_order: g.items.len() as i32,
                                                            col_class: "1x1".to_string(),
                                                        });
                                                    }
                                                });
                                                lj_item.set(serialize_layout(&ls_item.get_untracked()));
                                            }
                                        >
                                            {icon("plus")} " Индикатор"
                                        </Button>
                                        <Button
                                            size=ButtonSize::Small
                                            appearance=ButtonAppearance::Secondary
                                            on_click=move |_| {
                                                ls_del.update(|l| { l.groups.remove(gi); });
                                                lj_del.set(serialize_layout(&ls_del.get_untracked()));
                                            }
                                        >
                                            {icon("trash-2")}
                                        </Button>
                                    </div>

                                    <div class="layout-group__items">
                                        {group.items.iter().enumerate().map(|(ii, _item)| {
                                            view! {
                                                <ItemRow
                                                    group_idx=gi
                                                    item_idx=ii
                                                    layout=layout_state
                                                    layout_json=layout_json
                                                />
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>

                                    {if group.items.is_empty() {
                                        view! {
                                            <div class="placeholder placeholder--small">
                                                "Нет индикаторов. Нажмите «+ Индикатор»."
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }}
                                </div>
                            }
                        }).collect::<Vec<_>>().into_any()
                    }}
                </div>
            </CardAnimated>

            // Raw JSON editor (for advanced users)
            <CardAnimated delay_ms=50>
                <div class="details-section">
                    <h4 class="details-section__title">"JSON (раскладка)"</h4>
                    <p class="form__hint">"Прямое редактирование JSON структуры раскладки."</p>
                    <textarea
                        class="form__textarea form__textarea--mono"
                        rows=10
                        prop:value=move || layout_json.get()
                        on:input=move |ev| {
                            use wasm_bindgen::JsCast;
                            let val = ev.target().unwrap()
                                .unchecked_into::<web_sys::HtmlTextAreaElement>()
                                .value();
                            layout_json.set(val.clone());
                            if let Ok(parsed) = serde_json::from_str::<LayoutEdit>(&val) {
                                layout_state.set(parsed);
                            }
                        }
                    />
                </div>
            </CardAnimated>
        </div>
    }
}
