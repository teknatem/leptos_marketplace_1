//! Sidebar component with collapsible menu items
//! Based on bolt-mpi-ui-redesign/src/components/Sidebar.tsx

use crate::layout::global_context::AppGlobalContext;
use crate::layout::tabs::tab_label_for_key;
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
                ("d403_indicators", tab_label_for_key("d403_indicators"), "activity"),
                ("d400_monthly_summary", tab_label_for_key("d400_monthly_summary"), "bar-chart"),
                ("d401_metadata_dashboard", tab_label_for_key("d401_metadata_dashboard"), "layout-dashboard"),
                ("a024_bi_indicator", tab_label_for_key("a024_bi_indicator"), "activity"),
                ("universal_dashboard", tab_label_for_key("universal_dashboard"), "table-pivot"),
                ("all_reports", tab_label_for_key("all_reports"), "table"),
                ("schema_browser", tab_label_for_key("schema_browser"), "database-cog"),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "references",
            label: "Справочники",
            icon: "database",
            items: vec![
                ("a002_organization", tab_label_for_key("a002_organization"), "building"),
                ("a003_counterparty", tab_label_for_key("a003_counterparty"), "contact"),
                ("a004_nomenclature", tab_label_for_key("a004_nomenclature"), "list"),
                ("a004_nomenclature_list", tab_label_for_key("a004_nomenclature_list"), "table"),
                ("a005_marketplace", tab_label_for_key("a005_marketplace"), "store"),
                ("a007_marketplace_product", tab_label_for_key("a007_marketplace_product"), "package"),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "documents",
            label: "Документы",
            icon: "file-text",
            items: vec![
                ("a015_wb_orders", tab_label_for_key("a015_wb_orders"), "file-text"),
                ("a021_production_output", tab_label_for_key("a021_production_output"), "package"),
                ("a022_kit_variant", tab_label_for_key("a022_kit_variant"), "layers"),
                ("a023_purchase_of_goods", tab_label_for_key("a023_purchase_of_goods"), "shopping-cart"),
                ("a020_wb_promotion", tab_label_for_key("a020_wb_promotion"), "tag"),
                ("a013_ym_order", tab_label_for_key("a013_ym_order"), "file-text"),
                ("a010_ozon_fbs_posting", tab_label_for_key("a010_ozon_fbs_posting"), "file-text"),
                ("a011_ozon_fbo_posting", tab_label_for_key("a011_ozon_fbo_posting"), "file-text"),
                ("a012_wb_sales", tab_label_for_key("a012_wb_sales"), "file-text"),
                ("a009_ozon_returns", tab_label_for_key("a009_ozon_returns"), "package-x"),
                ("a016_ym_returns", tab_label_for_key("a016_ym_returns"), "package-x"),
                ("a008_marketplace_sales", tab_label_for_key("a008_marketplace_sales"), "cash"),
                ("a014_ozon_transactions", tab_label_for_key("a014_ozon_transactions"), "credit-card"),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "integrations",
            label: "Интеграции",
            icon: "plug",
            items: vec![
                ("a001_connection_1c", tab_label_for_key("a001_connection_1c"), "database"),
                ("a006_connection_mp", tab_label_for_key("a006_connection_mp"), "plug"),
                ("u501_import_from_ut", tab_label_for_key("u501_import_from_ut"), "import"),
                ("u502_import_from_ozon", tab_label_for_key("u502_import_from_ozon"), "import"),
                ("u503_import_from_yandex", tab_label_for_key("u503_import_from_yandex"), "import"),
                ("u504_import_from_wildberries", tab_label_for_key("u504_import_from_wildberries"), "import"),
                ("u506_import_from_lemanapro", tab_label_for_key("u506_import_from_lemanapro"), "import"),
                ("u507_import_from_erp", tab_label_for_key("u507_import_from_erp"), "import"),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "operations",
            label: "Операции",
            icon: "layers",
            items: vec![
                ("u505_match_nomenclature", tab_label_for_key("u505_match_nomenclature"), "layers"),
                ("a018_llm_chat", tab_label_for_key("a018_llm_chat"), "message-square"),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "information",
            label: "Информация",
            icon: "database",
            items: vec![
                ("p900_sales_register", tab_label_for_key("p900_sales_register"), "database"),
                ("p901_barcodes", tab_label_for_key("p901_barcodes"), "barcode"),
                ("p902_ozon_finance_realization", tab_label_for_key("p902_ozon_finance_realization"), "dollar-sign"),
                ("p903_wb_finance_report", tab_label_for_key("p903_wb_finance_report"), "dollar-sign"),
                ("p904_sales_data", tab_label_for_key("p904_sales_data"), "dollar-sign"),
                ("p905_commission_history", tab_label_for_key("p905_commission_history"), "percent"),
                ("p906_nomenclature_prices", tab_label_for_key("p906_nomenclature_prices"), "dollar-sign"),
                ("p907_ym_payment_report", tab_label_for_key("p907_ym_payment_report"), "receipt"),
                ("p908_wb_goods_prices", tab_label_for_key("p908_wb_goods_prices"), "tag"),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "settings",
            label: "Настройки",
            icon: "settings",
            items: vec![
                ("sys_users", tab_label_for_key("sys_users"), "users"),
                ("sys_scheduled_tasks", tab_label_for_key("sys_scheduled_tasks"), "calendar"),
                ("a017_llm_agent", tab_label_for_key("a017_llm_agent"), "robot"),
                ("sys_thaw_test", tab_label_for_key("sys_thaw_test"), "test-tube"),
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
