use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons;
use crate::system::auth::context::use_auth;
use leptos::prelude::*;

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

    fn icon_for(kind: &str) -> AnyView {
        icons::icon(kind)
    }

    // Дашборды
    let dashboards = vec![
        ("d400_monthly_summary", "Сводка за месяц", "bar-chart"),
    ];

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

    // Регистры (Projections)
    let registers = vec![
        ("p900_sales_register", "Регистр продаж", "database"),
        ("p901_barcodes", "Штрихкоды номенклатуры", "barcode"),
        (
            "p902_ozon_finance_realization",
            "OZON Finance Realization (Postings)",
            "dollar-sign",
        ),
        (
            "p903_wb_finance_report",
            "WB Finance Report (P903)",
            "dollar-sign",
        ),
        ("p904_sales_data", "Sales Data (P904)", "dollar-sign"),
        (
            "p905_commission_history",
            "WB Commission History (P905)",
            "percent",
        ),
        (
            "p906_nomenclature_prices",
            "Плановые цены (P906)",
            "dollar-sign",
        ),
    ];

    // Настройки (admin only)
    let settings = [("sys_users", "Пользователи", "users")];

    view! {
        <nav class="main-nav-bar">
            <>
                // Дашборды
                <div class="main-nav-bar-header">
                    "Дашборды"
                </div>
                <ul>
                    {dashboards.into_iter().map(|(key, title, icon_name)| {
                        view! {
                            <li on:click=move |_| tabs_store.open_tab(key, title)>
                                {icon_for(icon_name)}
                                <span>{title}</span>
                            </li>
                        }
                    }).collect_view()}
                </ul>

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

                // Настройки (admin only)
                <Show when=is_admin>
                    <div class="main-nav-bar-header">
                        "Настройки"
                    </div>
                    <ul>
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
                    </ul>
                </Show>
            </>
        </nav>
    }
}
