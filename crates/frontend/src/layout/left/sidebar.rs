//! Sidebar component with collapsible menu items
//! Based on bolt-mpi-ui-redesign/src/components/Sidebar.tsx

use leptos::prelude::*;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;

#[derive(Clone, Debug, PartialEq)]
struct MenuGroup {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    items: Vec<(&'static str, &'static str, &'static str)>, // (id, label, icon)
}

fn get_menu_groups() -> Vec<MenuGroup> {
    vec![
        MenuGroup {
            id: "dashboard",
            label: "Сводка за месяц",
            icon: "layout-dashboard",
            items: vec![],
        },
        MenuGroup {
            id: "references",
            label: "Справочники",
            icon: "database",
            items: vec![
                ("a002_organization", "Организации", "building"),
                ("a003_counterparty", "Контрагенты", "contact"),
                ("a004_nomenclature", "Номенклатура", "package"),
            ],
        },
        MenuGroup {
            id: "documents",
            label: "Документы",
            icon: "file-text",
            items: vec![
                ("a008_marketplace_sales", "Продажи МП", "cash"),
                ("a010_ozon_fbs_posting", "OZON FBS Posting", "file-text"),
                ("a011_ozon_fbo_posting", "OZON FBO Posting", "file-text"),
                ("a015_wb_orders", "WB Orders", "file-text"),
                ("a012_wb_sales", "WB Sales", "file-text"),
                ("a013_ym_order", "YM Orders", "file-text"),
                ("a009_ozon_returns", "Возвраты OZON", "return"),
                ("a016_ym_returns", "Возвраты Yandex", "return"),
            ],
        },
        MenuGroup {
            id: "integrations",
            label: "Интеграции",
            icon: "settings",
            items: vec![
                ("a001_connection_1c", "Подключения 1С", "database"),
                ("a006_connection_mp", "Подключения МП", "plug"),
                ("u501_import_from_ut", "Импорт из УТ 11", "import"),
                ("u504_import_from_wildberries", "Импорт из Wildberries", "import"),
                ("u505_import_from_ozon", "Импорт из OZON", "import"),
                ("u506_import_from_yandex", "Импорт из Yandex", "import"),
            ],
        },
    ]
}

#[component]
pub fn Sidebar() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    
    // Initially expanded groups (matching the sample)
    let expanded_groups = RwSignal::new(vec![
        "references".to_string(),
        "documents".to_string(),
        "integrations".to_string(),
    ]);
    
    let groups = get_menu_groups();
    
    view! {
        <div class="sidebar">
            <div class="sidebar-content">
                {groups.into_iter().map(|group| {
                    let group_id = group.id.to_string();
                    let has_children = !group.items.is_empty();
                    
                    let group_id_for_exp = group_id.clone();
                    let group_id_for_click = group_id.clone();
                    let group_id_for_active = group_id.clone();
                    
                    view! {
                        <div>
                            // Parent item
                            <div
                                class="sidebar-item"
                                class:active=move || {
                                    !has_children && ctx.active.get().as_ref().map(|a| a == &group_id_for_active).unwrap_or(false)
                                }
                                style:padding-left="12px"
                                on:click=move |_| {
                                    if has_children {
                                        let gid = group_id_for_click.clone();
                                        expanded_groups.update(move |items| {
                                            if let Some(pos) = items.iter().position(|x| x == &gid) {
                                                items.remove(pos);
                                            } else {
                                                items.push(gid);
                                            }
                                        });
                                    } else {
                                        ctx.open_tab(group.id, group.label);
                                    }
                                }
                            >
                                <div class="sidebar-item-content">
                                    {icon(group.icon)}
                                    <span>{group.label}</span>
                                </div>
                                {has_children.then(|| {
                                    let gid_exp = group_id_for_exp.clone();
                                    view! {
                                        <div 
                                            class="sidebar-chevron"
                                            class:expanded=move || expanded_groups.get().contains(&gid_exp)
                                        >
                                            {icon("chevron-right")}
                                        </div>
                                    }
                                })}
                            </div>
                            
                            // Children
                            {has_children.then(|| {
                                let gid_show = group_id.clone();
                                let items_stored = StoredValue::new(group.items.clone());
                                view! {
                                    <Show when=move || expanded_groups.get().contains(&gid_show)>
                                        <div class="sidebar-children">
                                            {items_stored.get_value().into_iter().map(|(id, label, icon_name)| {
                                                let item_id = id.to_string();
                                                view! {
                                                    <div
                                                        class="sidebar-item"
                                                        class:active=move || ctx.active.get().as_ref().map(|a| a == &item_id).unwrap_or(false)
                                                        style:padding-left="28px"
                                                        on:click=move |_| {
                                                            ctx.open_tab(id, label);
                                                        }
                                                    >
                                                        <div class="sidebar-item-content">
                                                            {icon(icon_name)}
                                                            <span>{label}</span>
                                                        </div>
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    </Show>
                                }
                            })}
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}
