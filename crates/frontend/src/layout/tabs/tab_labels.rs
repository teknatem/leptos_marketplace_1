//! Tab labels - единственный источник правды для заголовков табов.
//!
//! Для агрегатов с metadata_gen.rs используются константы из contracts.
//! Для остальных (проекции, юзкейсы, системные) — хардкод.

use contracts::domain::a001_connection_1c::ENTITY_METADATA as A001;
use contracts::domain::a002_organization::ENTITY_METADATA as A002;
use contracts::domain::a004_nomenclature::ENTITY_METADATA as A004;
use contracts::domain::a005_marketplace::ENTITY_METADATA as A005;
use contracts::domain::a006_connection_mp::ENTITY_METADATA as A006;
use contracts::domain::a012_wb_sales::ENTITY_METADATA as A012;
use contracts::domain::a013_ym_order::ENTITY_METADATA as A013;
use contracts::domain::a017_llm_agent::ENTITY_METADATA as A017;
use contracts::domain::a018_llm_chat::ENTITY_METADATA as A018;
use contracts::domain::a019_llm_artifact::ENTITY_METADATA as A019;
use contracts::domain::a020_wb_promotion::ENTITY_METADATA as A020;

/// Возвращает читаемый заголовок таба для данного ключа.
///
/// Для агрегатов с metadata_gen берёт `list_name` из contracts.
/// Для остальных — хардкод. Fallback: сам ключ.
pub fn tab_label_for_key(key: &str) -> &'static str {
    match key {
        // ── Aggregates с metadata_gen ─────────────────────────────────────
        "a001_connection_1c" => A001.ui.list_name,
        "a002_organization" => A002.ui.list_name,
        "a004_nomenclature" => A004.ui.list_name,
        "a004_nomenclature_list" => "Номенклатура (список)",
        "a005_marketplace" => A005.ui.list_name,
        "a006_connection_mp" => A006.ui.list_name,
        "a012_wb_sales" => A012.ui.list_name,
        "a013_ym_order" => A013.ui.list_name,
        "a017_llm_agent" => A017.ui.list_name,
        "a018_llm_chat" => A018.ui.list_name,
        "a019_llm_artifact" => A019.ui.list_name,
        "a020_wb_promotion" => A020.ui.list_name,

        // ── Aggregates без metadata_gen ───────────────────────────────────
        "a003_counterparty" => "Контрагенты",
        "a007_marketplace_product" => "Товары МП",
        "a008_marketplace_sales" => "Продажи МП",
        "a009_ozon_returns" => "Возвраты OZON",
        "a010_ozon_fbs_posting" => "OZON FBS Posting",
        "a011_ozon_fbo_posting" => "OZON FBO Posting",
        "a014_ozon_transactions" => "Транзакции OZON",
        "a015_wb_orders" => "WB Orders",
        "a016_ym_returns" => "Возвраты Yandex",

        // ── Use Cases (u5xx) ──────────────────────────────────────────────
        "u501_import_from_ut" => "Импорт из УТ 11",
        "u502_import_from_ozon" => "Импорт из OZON",
        "u503_import_from_yandex" => "Импорт из Yandex",
        "u504_import_from_wildberries" => "Импорт из Wildberries",
        "u505_match_nomenclature" => "Сопоставление",
        "u506_import_from_lemanapro" => "Импорт из ЛеманаПро",

        // ── Projections (p9xx) ────────────────────────────────────────────
        "p900_sales_register" => "Регистр продаж",
        "p901_barcodes" => "Штрихкоды номенклатуры",
        "p902_ozon_finance_realization" => "OZON Finance Realization",
        "p903_wb_finance_report" => "WB Finance Report",
        "p904_sales_data" => "Sales Data",
        "p905_commission_history" => "WB Commission History",
        "p906_nomenclature_prices" => "Дилерские цены (УТ)",
        "p907_ym_payment_report" => "YM Отчёт по платежам",
        "p908_wb_goods_prices" => "WB Цены товаров",

        // ── Dashboards (d4xx) ─────────────────────────────────────────────
        "d400_monthly_summary" => "Сводка за месяц",
        "d401_metadata_dashboard" => "Метаданные",
        "d401_wb_finance" => "WB Finance",
        "d403_indicators" => "Показатели",

        // ── System (sys_*, dom_*) ─────────────────────────────────────────
        "sys_users" => "Пользователи",
        "sys_scheduled_tasks" => "Регламентные задания",
        "sys_thaw_test" => "Тест Thaw UI",
        "dom_inspector" => "DOM Inspector",

        // ── Features ─────────────────────────────────────────────────────
        "universal_dashboard" => "Универсальный дашборд",
        "all_reports" => "Все отчеты",
        "schema_browser" => "Схемы данных",

        // ── Fallback ──────────────────────────────────────────────────────
        _ => "",
    }
}

/// Возвращает первый непустой идентификатор из цепочки fallback.
///
/// Порядок приоритета: document_no → article → description → id
pub fn pick_identifier<'a>(
    document_no: Option<&'a str>,
    article: Option<&'a str>,
    description: Option<&'a str>,
    id: &'a str,
) -> &'a str {
    [document_no, article, description]
        .into_iter()
        .flatten()
        .find(|s| !s.is_empty())
        .unwrap_or(id)
}

/// Формирует заголовок detail-таба: «<entity> · <identifier>».
///
/// Пример: `detail_tab_label("Чат LLM", "abc-123")` → `"Чат LLM · abc-123"`
pub fn detail_tab_label(entity_label: &'static str, identifier: &str) -> String {
    format!("{} · {}", entity_label, identifier)
}

/// Возвращает element_name для агрегата по ключу (для detail-табов).
pub fn entity_element_name(aggregate_key: &str) -> &'static str {
    match aggregate_key {
        "a001_connection_1c" => A001.ui.element_name,
        "a002_organization" => A002.ui.element_name,
        "a004_nomenclature" => A004.ui.element_name,
        "a005_marketplace" => A005.ui.element_name,
        "a006_connection_mp" => A006.ui.element_name,
        "a012_wb_sales" => A012.ui.element_name,
        "a013_ym_order" => A013.ui.element_name,
        "a017_llm_agent" => A017.ui.element_name,
        "a018_llm_chat" => A018.ui.element_name,
        "a019_llm_artifact" => A019.ui.element_name,
        "a020_wb_promotion" => A020.ui.element_name,
        _ => "",
    }
}
