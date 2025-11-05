use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons;
use leptos::prelude::*;

#[component]
pub fn Navbar() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    fn icon_for(kind: &str) -> AnyView {
        icons::icon(kind)
    }

    // Справочники
    let directories = vec![
        ("a002_organization", "Организации", "building"),
        ("a003_counterparty", "Контрагенты", "contact"),
        ("a004_nomenclature", "Номенклатура", "list"),
        ("a005_marketplace", "Маркетплейсы", "store"),
        ("a007_marketplace_product", "Товары МП", "package"),
        ("a008_marketplace_sales", "Продажи МП", "cash"),
        ("a009_ozon_returns", "Возвраты OZON", "package-x"),
    ];

    // Документы (Aggregates)
    let documents = vec![
        ("a010_ozon_fbs_posting", "OZON FBS Posting", "file-text"),
        ("a011_ozon_fbo_posting", "OZON FBO Posting", "file-text"),
        ("a012_wb_sales", "WB Sales", "file-text"),
        ("a013_ym_order", "YM Orders", "file-text"),
    ];

    // Интеграции: подключения + импорты
    let integrations = vec![
        // Подключения
        ("a001_connection_1c", "Подключения 1С", "database"),
        ("a006_connection_mp", "Подключения МП", "plug"),
        // Импорты
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
    ];

    // Операции
    let operations = vec![("u505_match_nomenclature", "Сопоставление", "layers")];

    // Регистры (Projections)
    let registers = vec![("p900_sales_register", "Регистр продаж", "database")];

    view! {
        <nav class="main-nav-bar">
            <>
                // Справочники
                <div class="main-nav-bar-header">
                    "Справочники"
                </div>
                <ul>
                    {directories.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </ul>

                // Документы
                <div class="main-nav-bar-header">
                    "Документы"
                </div>
                <ul>
                    {documents.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </ul>

                // Интеграции
                <div class="main-nav-bar-header">
                    "Интеграции"
                </div>
                <ul>
                    {integrations.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </ul>

                // Операции
                <div class="main-nav-bar-header">
                    "Операции"
                </div>
                <ul>
                    {operations.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </ul>

                // Регистры
                <div class="main-nav-bar-header">
                    "Регистры"
                </div>
                <ul>
                    {registers.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            </>
        </nav>
    }
}
