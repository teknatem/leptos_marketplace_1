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
        ("u504_import_from_wildberries", "Импорт из Wildberries", "import"),
    ];

    // Операции
    let operations = vec![
        ("u505_match_nomenclature", "Сопоставление", "layers"),
    ];

    view! {
        <nav class="main-nav-bar">
            <>
                // Справочники
                <div style="padding: 10px; padding-top: 30px; font-weight: bold; color: #888; font-size: 12px; text-transform: uppercase;">
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

                // Интеграции
                <div style="padding: 10px; font-weight: bold; color: #888; font-size: 12px; text-transform: uppercase; margin-top: 20px;">
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
                <div style="padding: 10px; font-weight: bold; color: #888; font-size: 12px; text-transform: uppercase; margin-top: 20px;">
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
            </>
        </nav>
    }
}
