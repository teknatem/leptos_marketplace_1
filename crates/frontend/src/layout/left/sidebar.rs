//! Sidebar component with collapsible menu items
//! Based on bolt-mpi-ui-redesign/src/components/Sidebar.tsx

use crate::layout::global_context::AppGlobalContext;
use crate::layout::tabs::tab_label_for_key;
use crate::shared::icons::icon;
use crate::system::auth::context::{has_read_access, use_auth};
use leptos::prelude::*;

/// A single sidebar navigation item.
#[derive(Clone, Debug, PartialEq)]
struct SidebarItem {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    /// Optional access scope. When set, the item is hidden unless the user
    /// has at least `read` access to this scope. `None` = always visible.
    scope_id: Option<&'static str>,
}

impl SidebarItem {
    fn new(id: &'static str, label: &'static str, icon: &'static str) -> Self {
        Self {
            id,
            label,
            icon,
            scope_id: None,
        }
    }

    fn with_scope(id: &'static str, label: &'static str, icon: &'static str) -> Self {
        // For aggregates the scope_id equals the tab key (folder name).
        Self {
            id,
            label,
            icon,
            scope_id: Some(id),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct MenuGroup {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    items: Vec<SidebarItem>,
    admin_only: bool,
}

fn get_menu_groups() -> Vec<MenuGroup> {
    vec![
        MenuGroup {
            id: "dashboards",
            label: "Дашборды",
            icon: "bar-chart",
            items: vec![
                SidebarItem::with_scope(
                    "a024_bi_indicator",
                    tab_label_for_key("a024_bi_indicator"),
                    "activity",
                ),
                SidebarItem::with_scope(
                    "a025_bi_dashboard",
                    tab_label_for_key("a025_bi_dashboard"),
                    "layout-dashboard",
                ),
                SidebarItem::new("data_view", tab_label_for_key("data_view"), "layers"),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "settings",
            label: "Настройки",
            icon: "settings",
            items: vec![
                SidebarItem::new(
                    "d400_monthly_summary",
                    tab_label_for_key("d400_monthly_summary"),
                    "bar-chart",
                ),
                SidebarItem::new(
                    "d401_metadata_dashboard",
                    tab_label_for_key("d401_metadata_dashboard"),
                    "layout-dashboard",
                ),
                SidebarItem::new(
                    "drilldown__new",
                    tab_label_for_key("drilldown__new"),
                    "zoom-in",
                ),
                SidebarItem::new(
                    "universal_dashboard",
                    tab_label_for_key("universal_dashboard"),
                    "table-pivot",
                ),
                SidebarItem::new("all_reports", tab_label_for_key("all_reports"), "table"),
                SidebarItem::new(
                    "schema_browser",
                    tab_label_for_key("schema_browser"),
                    "database-cog",
                ),
            ],
            admin_only: true,
        },
        MenuGroup {
            id: "references",
            label: "Справочники",
            icon: "database",
            items: vec![
                SidebarItem::with_scope(
                    "a002_organization",
                    tab_label_for_key("a002_organization"),
                    "building",
                ),
                SidebarItem::with_scope(
                    "a003_counterparty",
                    tab_label_for_key("a003_counterparty"),
                    "contact",
                ),
                SidebarItem::with_scope(
                    "a004_nomenclature",
                    tab_label_for_key("a004_nomenclature"),
                    "list",
                ),
                // a004_nomenclature_list is a view variant of a004, same scope.
                SidebarItem {
                    id: "a004_nomenclature_list",
                    label: tab_label_for_key("a004_nomenclature_list"),
                    icon: "table",
                    scope_id: Some("a004_nomenclature"),
                },
                SidebarItem::with_scope(
                    "a005_marketplace",
                    tab_label_for_key("a005_marketplace"),
                    "store",
                ),
                SidebarItem::with_scope(
                    "a007_marketplace_product",
                    tab_label_for_key("a007_marketplace_product"),
                    "package",
                ),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "documents",
            label: "Документы",
            icon: "file-text",
            items: vec![
                SidebarItem::with_scope(
                    "a015_wb_orders",
                    tab_label_for_key("a015_wb_orders"),
                    "file-text",
                ),
                SidebarItem::with_scope(
                    "a026_wb_advert_daily",
                    tab_label_for_key("a026_wb_advert_daily"),
                    "activity",
                ),
                SidebarItem::with_scope(
                    "a021_production_output",
                    tab_label_for_key("a021_production_output"),
                    "package",
                ),
                SidebarItem::with_scope(
                    "a022_kit_variant",
                    tab_label_for_key("a022_kit_variant"),
                    "layers",
                ),
                SidebarItem::with_scope(
                    "a023_purchase_of_goods",
                    tab_label_for_key("a023_purchase_of_goods"),
                    "shopping-cart",
                ),
                SidebarItem::with_scope(
                    "a020_wb_promotion",
                    tab_label_for_key("a020_wb_promotion"),
                    "tag",
                ),
                SidebarItem::with_scope(
                    "a013_ym_order",
                    tab_label_for_key("a013_ym_order"),
                    "file-text",
                ),
                SidebarItem::with_scope(
                    "a010_ozon_fbs_posting",
                    tab_label_for_key("a010_ozon_fbs_posting"),
                    "file-text",
                ),
                SidebarItem::with_scope(
                    "a011_ozon_fbo_posting",
                    tab_label_for_key("a011_ozon_fbo_posting"),
                    "file-text",
                ),
                SidebarItem::with_scope(
                    "a012_wb_sales",
                    tab_label_for_key("a012_wb_sales"),
                    "file-text",
                ),
                SidebarItem::with_scope(
                    "a009_ozon_returns",
                    tab_label_for_key("a009_ozon_returns"),
                    "package-x",
                ),
                SidebarItem::with_scope(
                    "a016_ym_returns",
                    tab_label_for_key("a016_ym_returns"),
                    "package-x",
                ),
                SidebarItem::with_scope(
                    "a008_marketplace_sales",
                    tab_label_for_key("a008_marketplace_sales"),
                    "cash",
                ),
                SidebarItem::with_scope(
                    "a014_ozon_transactions",
                    tab_label_for_key("a014_ozon_transactions"),
                    "credit-card",
                ),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "integrations",
            label: "Интеграции",
            icon: "plug",
            items: vec![
                SidebarItem::with_scope(
                    "a001_connection_1c",
                    tab_label_for_key("a001_connection_1c"),
                    "database",
                ),
                SidebarItem::with_scope(
                    "a006_connection_mp",
                    tab_label_for_key("a006_connection_mp"),
                    "plug",
                ),
                SidebarItem::new(
                    "u501_import_from_ut",
                    tab_label_for_key("u501_import_from_ut"),
                    "import",
                ),
                SidebarItem::new(
                    "u502_import_from_ozon",
                    tab_label_for_key("u502_import_from_ozon"),
                    "import",
                ),
                SidebarItem::new(
                    "u503_import_from_yandex",
                    tab_label_for_key("u503_import_from_yandex"),
                    "import",
                ),
                SidebarItem::new(
                    "u504_import_from_wildberries",
                    tab_label_for_key("u504_import_from_wildberries"),
                    "import",
                ),
                SidebarItem::new(
                    "u506_import_from_lemanapro",
                    tab_label_for_key("u506_import_from_lemanapro"),
                    "import",
                ),
                SidebarItem::new(
                    "u507_import_from_erp",
                    tab_label_for_key("u507_import_from_erp"),
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
                SidebarItem::new(
                    "general_ledger",
                    tab_label_for_key("general_ledger"),
                    "book-open",
                ),
                SidebarItem::new(
                    "u505_match_nomenclature",
                    tab_label_for_key("u505_match_nomenclature"),
                    "layers",
                ),
                SidebarItem::new(
                    "u508_repost_documents",
                    tab_label_for_key("u508_repost_documents"),
                    "refresh-cw",
                ),
                SidebarItem::with_scope(
                    "a018_llm_chat",
                    tab_label_for_key("a018_llm_chat"),
                    "message-square",
                ),
            ],
            admin_only: true,
        },
        MenuGroup {
            id: "information",
            label: "Информация",
            icon: "database",
            items: vec![
                SidebarItem::new(
                    "p900_sales_register",
                    tab_label_for_key("p900_sales_register"),
                    "database",
                ),
                SidebarItem::new(
                    "p901_barcodes",
                    tab_label_for_key("p901_barcodes"),
                    "barcode",
                ),
                SidebarItem::new(
                    "p902_ozon_finance_realization",
                    tab_label_for_key("p902_ozon_finance_realization"),
                    "dollar-sign",
                ),
                SidebarItem::new(
                    "p903_wb_finance_report",
                    tab_label_for_key("p903_wb_finance_report"),
                    "dollar-sign",
                ),
                SidebarItem::new(
                    "p904_sales_data",
                    tab_label_for_key("p904_sales_data"),
                    "dollar-sign",
                ),
                SidebarItem::new(
                    "p909_mp_order_line_turnovers",
                    tab_label_for_key("p909_mp_order_line_turnovers"),
                    "list-ordered",
                ),
                SidebarItem::new(
                    "p910_mp_unlinked_turnovers",
                    tab_label_for_key("p910_mp_unlinked_turnovers"),
                    "file-stack",
                ),
                SidebarItem::new(
                    "p911_wb_advert_by_items",
                    tab_label_for_key("p911_wb_advert_by_items"),
                    "list-ordered",
                ),
                SidebarItem::new(
                    "p905_commission_history",
                    tab_label_for_key("p905_commission_history"),
                    "percent",
                ),
                SidebarItem::new(
                    "p906_nomenclature_prices",
                    tab_label_for_key("p906_nomenclature_prices"),
                    "dollar-sign",
                ),
                SidebarItem::new(
                    "p907_ym_payment_report",
                    tab_label_for_key("p907_ym_payment_report"),
                    "receipt",
                ),
                SidebarItem::new(
                    "p908_wb_goods_prices",
                    tab_label_for_key("p908_wb_goods_prices"),
                    "tag",
                ),
            ],
            admin_only: false,
        },
        MenuGroup {
            id: "administration",
            label: "Администрирование",
            icon: "shield",
            items: vec![
                SidebarItem::new("sys_users", tab_label_for_key("sys_users"), "users"),
                SidebarItem::new("sys_roles", tab_label_for_key("sys_roles"), "shield"),
                SidebarItem::new(
                    "sys_roles_matrix",
                    tab_label_for_key("sys_roles_matrix"),
                    "table",
                ),
                SidebarItem::new(
                    "sys_scheduled_tasks",
                    tab_label_for_key("sys_scheduled_tasks"),
                    "calendar",
                ),
                SidebarItem::with_scope(
                    "a017_llm_agent",
                    tab_label_for_key("a017_llm_agent"),
                    "robot",
                ),
                SidebarItem::new(
                    "filter_registry",
                    tab_label_for_key("filter_registry"),
                    "filter",
                ),
                SidebarItem::new(
                    "sys_thaw_test",
                    tab_label_for_key("sys_thaw_test"),
                    "test-tube",
                ),
            ],
            admin_only: true,
        },
    ]
}

#[component]
pub fn Sidebar() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (auth_state, _) = use_auth();

    // Check admin status once, untracked, for filtering menu groups.
    // Scopes are also read untracked, they are stable for the session lifetime.
    let is_admin_untracked = auth_state.with_untracked(|state| {
        state
            .user_info
            .as_ref()
            .map(|u| u.is_admin)
            .unwrap_or(false)
    });

    let expanded_groups = RwSignal::new(vec![]);
    let groups = get_menu_groups();

    view! {
        <div class="app-sidebar__content">
            {groups
                .into_iter()
                .filter_map(|group| {
                    let is_admin_only = group.admin_only;

                    if is_admin_only && !is_admin_untracked {
                        return None;
                    }

                    let visible_items: Vec<SidebarItem> = group
                        .items
                        .into_iter()
                        .filter(|item| match item.scope_id {
                            None => true,
                            Some(scope) => has_read_access(auth_state, scope),
                        })
                        .collect();

                    // Keep the user-facing settings group visible even if it becomes empty.
                    if visible_items.is_empty() && group.id != "settings" {
                        return None;
                    }

                    let group_id = group.id.to_string();
                    let has_children = !visible_items.is_empty();

                    let group_id_stored = StoredValue::new(group_id.clone());
                    let group_id_for_exp = group_id.clone();
                    let group_id_for_click = group_id.clone();

                    Some(view! {
                        <div>
                            <div
                                class="app-sidebar__item"
                                class:app-sidebar__item--active=move || {
                                    let gid = group_id_stored.get_value();
                                    !has_children
                                        && ctx.active.get().as_ref().map(|a| a == &gid).unwrap_or(false)
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

                            {has_children.then(|| {
                                let gid_show = group_id.clone();
                                let items_stored = StoredValue::new(visible_items);
                                view! {
                                    <Show when=move || expanded_groups.get().contains(&gid_show)>
                                        <div class="app-sidebar__children">
                                            {items_stored
                                                .get_value()
                                                .into_iter()
                                                .map(|item| {
                                                    let item_id = StoredValue::new(item.id.to_string());
                                                    view! {
                                                        <div
                                                            class="app-sidebar__item"
                                                            class:app-sidebar__item--active=move || {
                                                                let iid = item_id.get_value();
                                                                ctx.active.get().as_ref().map(|a| a == &iid).unwrap_or(false)
                                                            }
                                                            style:padding-left="10px"
                                                            on:click=move |_| {
                                                                ctx.open_tab(item.id, item.label);
                                                            }
                                                        >
                                                            <div class="app-sidebar__item-content">
                                                                {icon(item.icon)}
                                                                <span>{item.label}</span>
                                                            </div>
                                                        </div>
                                                    }
                                                })
                                                .collect_view()}
                                        </div>
                                    </Show>
                                }
                            })}
                        </div>
                    })
                })
                .collect_view()}
        </div>
    }
}
