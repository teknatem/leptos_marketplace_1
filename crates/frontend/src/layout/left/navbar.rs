use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons;
use crate::system::auth::context::use_auth;
use leptos::prelude::*;
use std::collections::HashMap;

#[component]
pub fn Navbar() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");
    let (auth_state, _) = use_auth();
    let is_admin = move || {
        let state = auth_state.get();
        state
            .user_info
            .as_ref()
            .map(|u| u.is_admin)
            .unwrap_or(false)
    };

    // Состояние collapsed/expanded для каждого раздела
    let (collapsed_sections, set_collapsed_sections) = signal({
        let mut map = HashMap::new();

        // Загружаем состояние из localStorage
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(stored)) = storage.get_item("navbar_collapsed_sections") {
                    if let Ok(parsed) = serde_json::from_str::<HashMap<String, bool>>(&stored) {
                        map = parsed;
                    }
                }
            }
        }

        // По умолчанию все разделы раскрыты
        map.entry("dashboards".to_string()).or_insert(false);
        map.entry("directories".to_string()).or_insert(false);
        map.entry("documents".to_string()).or_insert(false);
        map.entry("integrations".to_string()).or_insert(false);
        map.entry("operations".to_string()).or_insert(false);
        map.entry("information".to_string()).or_insert(false);
        map.entry("settings".to_string()).or_insert(false);

        map
    });

    // Функция для переключения состояния раздела
    let toggle_section = move |section_id: &str| {
        set_collapsed_sections.update(|map| {
            let current = *map.get(section_id).unwrap_or(&false);
            map.insert(section_id.to_string(), !current);

            // Сохраняем в localStorage
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(json) = serde_json::to_string(&*map) {
                        let _ = storage.set_item("navbar_collapsed_sections", &json);
                    }
                }
            }
        });
    };

    fn icon_for(kind: &str) -> AnyView {
        icons::icon(kind)
    }

    // Дашборды
    let dashboards = vec![("d400_monthly_summary", "Сводка за месяц", "bar-chart")];

    // Справочники
    let directories = vec![
        ("a002_organization", "Организации", "building"),
        ("a003_counterparty", "Контрагенты", "contact"),
        ("a004_nomenclature", "Номенклатура", "list"),
        ("a004_nomenclature_list", "Номенклатура (список)", "table"),
        ("a005_marketplace", "Маркетплейсы", "store"),
        ("a007_marketplace_product", "Товары МП", "package"),
    ];

    // Документы (Aggregates)
    let documents = vec![
        ("a008_marketplace_sales", "Продажи МП", "cash"),
        ("a010_ozon_fbs_posting", "OZON FBS Posting", "file-text"),
        ("a011_ozon_fbo_posting", "OZON FBO Posting", "file-text"),
        ("a015_wb_orders", "WB Orders", "file-text"),
        ("a012_wb_sales", "WB Sales", "file-text"),
        ("a013_ym_order", "YM Orders", "file-text"),
        ("a009_ozon_returns", "Возвраты OZON", "package-x"),
        ("a016_ym_returns", "Возвраты Yandex", "package-x"),
        ("a014_ozon_transactions", "Транзакции OZON", "credit-card"),
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

    // Информация (Projections) - переименовано из "Регистры"
    let information = vec![
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
        ("p906_nomenclature_prices", "Плановые цены", "dollar-sign"),
    ];

    // Настройки (admin only)
    let settings = [("sys_users", "Пользователи", "users")];

    view! {
        <nav class="main-nav-bar">
            <>
                // Дашборды
                <CollapsibleSection
                    title="Дашборды"
                    is_collapsed=move || collapsed_sections.get().get("dashboards").copied().unwrap_or(false)
                    on_toggle=move || toggle_section("dashboards")
                >
                    {dashboards.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </CollapsibleSection>

                // Справочники
                <CollapsibleSection
                    title="Справочники"
                    is_collapsed=move || collapsed_sections.get().get("directories").copied().unwrap_or(false)
                    on_toggle=move || toggle_section("directories")
                >
                    {directories.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </CollapsibleSection>

                // Документы
                <CollapsibleSection
                    title="Документы"
                    is_collapsed=move || collapsed_sections.get().get("documents").copied().unwrap_or(false)
                    on_toggle=move || toggle_section("documents")
                >
                    {documents.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </CollapsibleSection>

                // Интеграции
                <CollapsibleSection
                    title="Интеграции"
                    is_collapsed=move || collapsed_sections.get().get("integrations").copied().unwrap_or(false)
                    on_toggle=move || toggle_section("integrations")
                >
                    {integrations.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </CollapsibleSection>

                // Операции
                <CollapsibleSection
                    title="Операции"
                    is_collapsed=move || collapsed_sections.get().get("operations").copied().unwrap_or(false)
                    on_toggle=move || toggle_section("operations")
                >
                    {operations.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </CollapsibleSection>

                // Информация (Projections)
                <CollapsibleSection
                    title="Информация"
                    is_collapsed=move || collapsed_sections.get().get("information").copied().unwrap_or(false)
                    on_toggle=move || toggle_section("information")
                >
                    {information.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </CollapsibleSection>

                // Настройки (admin only)
                <Show when=is_admin>
                    <CollapsibleSection
                        title="Настройки"
                        is_collapsed=move || collapsed_sections.get().get("settings").copied().unwrap_or(false)
                        on_toggle=move || toggle_section("settings")
                    >
                        {settings.iter().map(|(key, title, icon_name)| {
                            let key = key.to_string();
                            let title = title.to_string();
                            let icon_name = icon_name.to_string();
                            view! {
                                <li on:click=move |_| tabs_store.open_tab(&key, &title)>
                                    {icon_for(&icon_name)}
                                    <span>{title.clone()}</span>
                                </li>
                            }
                        }).collect_view()}
                    </CollapsibleSection>
                </Show>
            </>
        </nav>
    }
}

#[component]
fn CollapsibleSection(
    title: &'static str,
    is_collapsed: impl Fn() -> bool + 'static + Copy + Send,
    on_toggle: impl Fn() + 'static,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="nav-section">
            <div
                class="main-nav-bar-header collapsible"
                on:click=move |_| on_toggle()
            >
                <span class="nav-section-chevron" class:collapsed=move || is_collapsed()>
                    {icons::icon("chevron-right")}
                </span>
                <span>{title}</span>
            </div>
            <ul class="nav-section-content" class:collapsed=move || is_collapsed()>
                {children()}
            </ul>
        </div>
    }
}
