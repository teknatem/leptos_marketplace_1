use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons;
use leptos::prelude::*;

#[derive(Debug, Clone)]
pub struct MegaMenuItem {
    pub key: &'static str,
    pub title: &'static str,
    pub icon_name: &'static str,
}

#[component]
pub fn MegaMenuCategory(
    label: &'static str,
    items: Vec<MegaMenuItem>,
    #[prop(default = 2)] columns: usize,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    let grid_cols_class = match columns {
        1 => "mega-menu-grid-1",
        2 => "mega-menu-grid-2",
        3 => "mega-menu-grid-3",
        _ => "mega-menu-grid-2",
    };

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
                <span>{label}</span>
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
                <div class=format!("mega-menu-content {}", grid_cols_class)>
                    {items.into_iter().map(|item| {
                        let key = item.key;
                        let title = item.title;
                        let icon_name = item.icon_name;
                        
                        view! {
                            <button
                                class="mega-menu-card"
                                on:click=move |_| {
                                    tabs_store.open_tab(key, title);
                                    set_is_open.set(false);
                                }
                            >
                                <div class="mega-menu-card-icon">
                                    {icons::icon(icon_name)}
                                </div>
                                <div class="mega-menu-card-title">
                                    {title}
                                </div>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn MegaMenuBar() -> impl IntoView {
    // Справочники
    let directories = vec![
        MegaMenuItem {
            key: "a002_organization",
            title: "Организации",
            icon_name: "building",
        },
        MegaMenuItem {
            key: "a003_counterparty",
            title: "Контрагенты",
            icon_name: "contact",
        },
        MegaMenuItem {
            key: "a004_nomenclature",
            title: "Номенклатура",
            icon_name: "list",
        },
        MegaMenuItem {
            key: "a004_nomenclature_list",
            title: "Номенклатура (список)",
            icon_name: "table",
        },
        MegaMenuItem {
            key: "a005_marketplace",
            title: "Маркетплейсы",
            icon_name: "store",
        },
        MegaMenuItem {
            key: "a007_marketplace_product",
            title: "Товары МП",
            icon_name: "package",
        },
        MegaMenuItem {
            key: "a008_marketplace_sales",
            title: "Продажи МП",
            icon_name: "dollar-sign",
        },
        MegaMenuItem {
            key: "a009_ozon_returns",
            title: "Возвраты OZON",
            icon_name: "package-x",
        },
    ];

    // Документы
    let documents = vec![
        MegaMenuItem {
            key: "a010_ozon_fbs_posting",
            title: "OZON FBS Posting",
            icon_name: "file-text",
        },
        MegaMenuItem {
            key: "a011_ozon_fbo_posting",
            title: "OZON FBO Posting",
            icon_name: "file-text",
        },
        MegaMenuItem {
            key: "a015_wb_orders",
            title: "WB Orders",
            icon_name: "file-text",
        },
        MegaMenuItem {
            key: "a012_wb_sales",
            title: "WB Sales",
            icon_name: "file-text",
        },
        MegaMenuItem {
            key: "a013_ym_order",
            title: "YM Orders",
            icon_name: "file-text",
        },
        MegaMenuItem {
            key: "a014_ozon_transactions",
            title: "Транзакции OZON",
            icon_name: "credit-card",
        },
    ];

    // Интеграции
    let integrations = vec![
        MegaMenuItem {
            key: "a001_connection_1c",
            title: "Подключения 1С",
            icon_name: "database",
        },
        MegaMenuItem {
            key: "a006_connection_mp",
            title: "Подключения МП",
            icon_name: "plug",
        },
        MegaMenuItem {
            key: "u501_import_from_ut",
            title: "Импорт из УТ 11",
            icon_name: "import",
        },
        MegaMenuItem {
            key: "u502_import_from_ozon",
            title: "Импорт из OZON",
            icon_name: "import",
        },
        MegaMenuItem {
            key: "u503_import_from_yandex",
            title: "Импорт из Yandex",
            icon_name: "import",
        },
        MegaMenuItem {
            key: "u504_import_from_wildberries",
            title: "Импорт из Wildberries",
            icon_name: "import",
        },
        MegaMenuItem {
            key: "u506_import_from_lemanapro",
            title: "Импорт из ЛеманаПро",
            icon_name: "import",
        },
    ];

    // Операции
    let operations = vec![MegaMenuItem {
        key: "u505_match_nomenclature",
        title: "Сопоставление",
        icon_name: "layers",
    }];

    // Регистры
    let registers = vec![
        MegaMenuItem {
            key: "p900_sales_register",
            title: "Регистр продаж",
            icon_name: "database",
        },
        MegaMenuItem {
            key: "p901_barcodes",
            title: "Штрихкоды номенклатуры",
            icon_name: "barcode",
        },
        MegaMenuItem {
            key: "p902_ozon_finance_realization",
            title: "OZON Finance Realization",
            icon_name: "dollar-sign",
        },
        MegaMenuItem {
            key: "p903_wb_finance_report",
            title: "WB Finance Report",
            icon_name: "dollar-sign",
        },
        MegaMenuItem {
            key: "p904_sales_data",
            title: "Sales Data",
            icon_name: "dollar-sign",
        },
        MegaMenuItem {
            key: "p905_commission_history",
            title: "WB Commission History",
            icon_name: "percent",
        },
    ];

    view! {
        <nav class="mega-menu-bar">
            <MegaMenuCategory label="Справочники" items=directories columns=2 />
            <MegaMenuCategory label="Документы" items=documents columns=2 />
            <MegaMenuCategory label="Интеграции" items=integrations columns=2 />
            <MegaMenuCategory label="Операции" items=operations columns=1 />
            <MegaMenuCategory label="Регистры" items=registers columns=2 />
        </nav>
    }
}

