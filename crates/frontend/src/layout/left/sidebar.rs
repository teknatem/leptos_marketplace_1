//! Sidebar component with collapsible menu items
//! Based on bolt-mpi-ui-redesign/src/components/Sidebar.tsx

use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::system::auth::context::use_auth;
use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq)]
struct MenuGroup {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    items: Vec<(&'static str, &'static str, &'static str)>, // (id, label, icon)
    admin_only: bool,
}

fn get_menu_groups() -> Vec<MenuGroup> {
    vec![
        MenuGroup {
            id: "dashboards",
            label: "Дашборды",
            icon: "bar-chart",
            items: vec![
                ("d400_monthly_summary", "Сводка за месяц", "bar-chart"),
                ("d401_metadata_dashboard", "Метаданные", "layout-dashboard"),
                (
                    "universal_dashboard",
                    "Универсальный дашборд",
                    "table-pivot",
                ),
                ("schema_browser", "Схемы данных", "database-cog"),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "references",
            label: "Справочники",
            icon: "database",
            items: vec![
                ("a002_organization", "Организации", "building"),
                ("a003_counterparty", "Контрагенты", "contact"),
                ("a004_nomenclature", "Номенклатура", "list"),
                ("a004_nomenclature_list", "Номенклатура (список)", "table"),
                ("a005_marketplace", "Маркетплейсы", "store"),
                ("a007_marketplace_product", "Товары МП", "package"),
            ],
            admin_only: false,
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
                ("a009_ozon_returns", "Возвраты OZON", "package-x"),
                ("a016_ym_returns", "Возвраты Yandex", "package-x"),
                ("a014_ozon_transactions", "Транзакции OZON", "credit-card"),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "integrations",
            label: "Интеграции",
            icon: "plug",
            items: vec![
                ("a001_connection_1c", "Подключения 1С", "database"),
                ("a006_connection_mp", "Подключения МП", "plug"),
                ("u501_import_from_ut", "Импорт из УТ 11", "import"),
                ("u502_import_from_ozon", "Импорт из OZON", "import"),
                ("u503_import_from_yandex", "Импорт из Yandex", "import"),
                (
                    "u504_import_from_wildberries",
                    "Импорт из Wildberries",
                    "import",
                ),
                (
                    "u506_import_from_lemanapro",
                    "Импорт из ЛеманаПро",
                    "import",
                ),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "operations",
            label: "Операции",
            icon: "layers",
            items: vec![
                ("u505_match_nomenclature", "Сопоставление", "layers"),
                ("a018_llm_chat", "LLM Чат", "message-square"),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "information",
            label: "Информация",
            icon: "database",
            items: vec![
                ("p900_sales_register", "Регистр продаж", "database"),
                ("p901_barcodes", "Штрихкоды номенклатуры", "barcode"),
                (
                    "p902_ozon_finance_realization",
                    "OZON Finance Realization",
                    "dollar-sign",
                ),
                ("p903_wb_finance_report", "WB Finance Report", "dollar-sign"),
                ("p904_sales_data", "Sales Data", "dollar-sign"),
                (
                    "p905_commission_history",
                    "WB Commission History",
                    "percent",
                ),
                (
                    "p906_nomenclature_prices",
                    "Дилерские цены (УТ)",
                    "dollar-sign",
                ),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "settings",
            label: "Настройки",
            icon: "settings",
            items: vec![
                ("sys_users", "Пользователи", "users"),
                ("sys_scheduled_tasks", "Регламентные задания", "calendar"),
                ("a017_llm_agent", "Агенты LLM", "robot"),
                ("sys_thaw_test", "Тест Thaw UI", "test-tube"),
            ],
            admin_only: true,
        },
    ]
}

#[component]
pub fn Sidebar() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (auth_state, _) = use_auth();

    let is_admin = move || {
        let state = auth_state.get();
        state
            .user_info
            .as_ref()
            .map(|u| u.is_admin)
            .unwrap_or(false)
    };

    // Check admin status once, untracked, for filtering menu groups
    let is_admin_untracked = auth_state.with_untracked(|state| {
        state
            .user_info
            .as_ref()
            .map(|u| u.is_admin)
            .unwrap_or(false)
    });

    // Initially expanded groups (matching the sample)
    /*let expanded_groups = RwSignal::new(vec![
        "dashboards".to_string(),
        "references".to_string(),
        "documents".to_string(),
        "integrations".to_string(),
        "operations".to_string(),
        "information".to_string(),
    ]);*/

    let expanded_groups = RwSignal::new(vec![]);

    let groups = get_menu_groups();

    view! {
        <div class="app-sidebar__content">
            {groups.into_iter().filter_map(|group| {
                    let is_admin_only = group.admin_only;

                    // Skip admin-only groups if user is not admin
                    if is_admin_only && !is_admin_untracked {
                        return None;
                    }

                    let group_id = group.id.to_string();
                    let has_children = !group.items.is_empty();

                    let group_id_stored = StoredValue::new(group_id.clone());
                    let group_id_for_exp = group_id.clone();
                    let group_id_for_click = group_id.clone();

                    Some(view! {
                        <div>
                            // Parent item
                            <div
                                class="app-sidebar__item"
                                class:app-sidebar__item--active=move || {
                                    let gid = group_id_stored.get_value();
                                    !has_children && ctx.active.get().as_ref().map(|a| a == &gid).unwrap_or(false)
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
                                <div class="app-sidebar__item-content">
                                    {icon(group.icon)}
                                    <span>{group.label}</span>
                                </div>
                                {has_children.then(|| {
                                    let gid_exp = group_id_for_exp.clone();
                                    view! {
                                        <div
                                            class="app-sidebar__chevron"
                                            class:app-sidebar__chevron--expanded=move || expanded_groups.get().contains(&gid_exp)
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
                                        <div class="app-sidebar__children">
                                            {items_stored.get_value().into_iter().map(|(id, label, icon_name)| {
                                                let item_id = StoredValue::new(id.to_string());
                                                view! {
                                                    <div
                                                        class="app-sidebar__item"
                                                        class:app-sidebar__item--active=move || {
                                                            let iid = item_id.get_value();
                                                            ctx.active.get().as_ref().map(|a| a == &iid).unwrap_or(false)
                                                        }
                                                        style:padding-left="10px"
                                                        on:click=move |_| {
                                                            ctx.open_tab(id, label);
                                                        }
                                                    >
                                                        <div class="app-sidebar__item-content">
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
                    })
                }).collect_view()}
        </div>
    }
}
