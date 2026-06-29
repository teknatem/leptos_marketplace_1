//! Инструменты агента-построителя графиков (навык `chart-builder`).
//!
//! График = один hybrid-плагин: `server_script` тянет данные SELECT'ом, а
//! `client_script` отдаёт строки + компактный chart-spec в `window.PluginCharts.render`
//! (Chart.js, тема-aware, инжектится в iframe — см. `frontend/static/plugin-charts.js`).
//!
//! Эти инструменты дают только заготовки (шаблон/примеры/контракт). Валидация,
//! smoke-тест, сохранение и запуск выполняются существующими `plugin_*`-инструментами.

use super::types::ToolDefinition;
use serde_json::{json, Value};

/// Имена инструментов построителя графиков (для guard'а в диспетчере).
pub const CHART_TOOL_NAMES: &[&str] =
    &["chart_template", "chart_examples", "get_chart_ui_contract"];

/// Клиентский ES-модуль графика: грузит данные и рисует через PluginCharts.
fn chart_client_script(spec_json: &str) -> String {
    format!(
        r##"export async function mount(root, host) {{
  root.innerHTML = '<div class="status">Загрузка…</div>';
  try {{
    const rows = await host.invoke("data", {{}});
    const spec = {spec};
    PluginCharts.render(root, spec, rows);
  }} catch (e) {{
    root.innerHTML = '<div class="status status--error">' + (e && e.message ? e.message : String(e)) + '</div>';
  }}
}}
"##,
        spec = spec_json
    )
}

/// Серверный метод без параметров (демо-SQL).
const CHART_SERVER: &str = r##"export async function data(args, host) {
  return await host.db.queryResource("series", []);
}
"##;

/// Серверный метод с периодом из контекста (для примеров на реальных таблицах).
const CHART_SERVER_CTX: &str = r##"export async function data(args, host) {
  const c = host.context || {};
  const from = c.date_from || "1970-01-01";
  const to = c.date_to || "2999-12-31";
  return await host.db.queryResource("series", [from, to]);
}
"##;

/// Собрать bundle-график. `with_ctx` — серверный метод читает период из контекста.
fn build_bundle(
    code: &str,
    title: &str,
    description: &str,
    capabilities: Value,
    spec: Value,
    sql: &str,
    with_ctx: bool,
) -> Value {
    json!({
        "manifest": {
            "code": code,
            "title": title,
            "runtime": "hybrid",
            "api_version": "2",
            "description": description,
            "capabilities": capabilities,
        },
        "client_script": chart_client_script(&spec.to_string()),
        "server_script": if with_ctx { CHART_SERVER_CTX } else { CHART_SERVER },
        "sql_resources": { "series": sql },
        "styles": ".pcharts{padding:8px;}",
    })
}

// ─── Демо-SQL без зависимости от схемы (шаблон сразу валиден и запускается) ───

const DEMO_TS_SQL: &str = "SELECT '2024-05-01' AS d, 1200 AS revenue UNION ALL SELECT '2024-05-02', 1450 UNION ALL SELECT '2024-05-03', 1310 UNION ALL SELECT '2024-05-04', 1680";
const DEMO_BAR_SQL: &str =
    "SELECT 'WB' AS name, 5200 AS value UNION ALL SELECT 'OZON', 3100 UNION ALL SELECT 'YM', 1800";
const DEMO_PIE_SQL: &str = "SELECT 'Одежда' AS name, 5200 AS value UNION ALL SELECT 'Обувь', 3100 UNION ALL SELECT 'Аксессуары', 1800";

/// Минимальный валидный bundle-график по типу (line | bar | pie).
fn chart_template(args: &Value) -> Value {
    let kind = args.get("type").and_then(Value::as_str).unwrap_or("line");
    let caps = json!(["network:none"]);
    let bundle = match kind {
        "bar" => build_bundle(
            "MY-CHART-BAR",
            "Мой график (столбцы)",
            "Шаблон bar-графика: категория + мера.",
            caps,
            json!({
                "type": "bar", "title": "Заголовок", "x": "name",
                "series": [{ "y": "value", "label": "Значение" }],
                "format": "money", "alternatives": ["stacked-bar", "line"]
            }),
            DEMO_BAR_SQL,
            false,
        ),
        "pie" => build_bundle(
            "MY-CHART-PIE",
            "Мой график (доли)",
            "Шаблон круговой/кольцевой диаграммы: доля от целого.",
            caps,
            json!({
                "type": "doughnut", "title": "Заголовок",
                "category": "name", "value": "value", "format": "money"
            }),
            DEMO_PIE_SQL,
            false,
        ),
        _ => build_bundle(
            "MY-CHART-LINE",
            "Мой график (линия)",
            "Шаблон time-series: дата/период + мера(ы).",
            caps,
            json!({
                "type": "line", "title": "Заголовок", "x": "d",
                "series": [{ "y": "revenue", "label": "Выручка" }],
                "format": "money", "alternatives": ["area", "bar"]
            }),
            DEMO_TS_SQL,
            false,
        ),
    };
    json!({
        "bundle": bundle,
        "type": kind,
        "hint": "Минимальный валидный график. Замени code/title и SQL в sql_resources на реальный SELECT \
                 (проверь через execute_query), синхронизируй имена колонок в chart-spec (x/series.y или \
                 category/value), затем plugin_smoke_test → plugin_upsert(status=active)."
    })
}

/// Готовые рабочие примеры на реальной таблице a012_wb_sales (db:read:wb).
fn chart_examples() -> Value {
    let wb_caps = json!(["db:read:wb", "network:none"]);

    let time_series = build_bundle(
        "EXAMPLE-CHART-REVENUE-TS",
        "Пример: выручка WB по дням",
        "Time-series: сумма продаж по дням за период из контекста.",
        wb_caps.clone(),
        json!({
            "type": "line", "title": "Выручка WB по дням", "x": "d",
            "series": [{ "y": "revenue", "label": "Выручка" }],
            "format": "money", "alternatives": ["area", "bar"]
        }),
        "SELECT date AS d, SUM(total_price) AS revenue FROM a012_wb_sales WHERE is_deleted = 0 AND date BETWEEN ? AND ? GROUP BY date ORDER BY date",
        true,
    );

    let bar = build_bundle(
        "EXAMPLE-CHART-TOP-ARTICLES",
        "Пример: топ артикулов WB по выручке",
        "Bar: 10 артикулов с наибольшей выручкой за период.",
        wb_caps.clone(),
        json!({
            "type": "bar", "title": "Топ-10 артикулов по выручке", "x": "name",
            "series": [{ "y": "value", "label": "Выручка" }],
            "format": "money", "horizontal": true, "alternatives": ["stacked-bar"]
        }),
        "SELECT article AS name, SUM(total_price) AS value FROM a012_wb_sales WHERE is_deleted = 0 AND date BETWEEN ? AND ? GROUP BY article ORDER BY value DESC LIMIT 10",
        true,
    );

    let pie = build_bundle(
        "EXAMPLE-CHART-ARTICLE-SHARE",
        "Пример: доля топ-артикулов в выручке",
        "Doughnut: доля топ-6 артикулов в общей выручке за период.",
        wb_caps,
        json!({
            "type": "doughnut", "title": "Доля артикулов в выручке",
            "category": "name", "value": "value", "format": "money"
        }),
        "SELECT article AS name, SUM(total_price) AS value FROM a012_wb_sales WHERE is_deleted = 0 AND date BETWEEN ? AND ? GROUP BY article ORDER BY value DESC LIMIT 6",
        true,
    );

    json!({
        "examples": [
            { "title": "Time-series: выручка по дням", "bundle": time_series },
            { "title": "Bar: топ артикулов", "bundle": bar },
            { "title": "Doughnut: доля артикулов", "bundle": pie }
        ],
        "hint": "Скопируй ближайший по форме данных пример, замени таблицу/колонки на свои (проверь SELECT \
                 через execute_query), выровняй chart-spec, затем plugin_smoke_test → plugin_upsert."
    })
}

/// Контракт рантайма графиков: как звать PluginCharts.render и какой chart-spec.
fn chart_ui_contract() -> Value {
    json!({
        "runtime": "В iframe доступны глобали window.Chart (Chart.js) и window.PluginCharts. \
                    В client_script внутри mount(root, host): const rows = await host.invoke(\"data\", {}); \
                    PluginCharts.render(root, spec, rows).",
        "server": "Серверный метод data(args, host) возвращает массив объектов-строк через \
                   host.db.queryResource(\"series\", params). Период бери из host.context.date_from/date_to.",
        "types": ["line", "area", "bar", "stacked-bar", "pie", "doughnut"],
        "spec_cartesian": {
            "type": "line|area|bar|stacked-bar",
            "title": "string",
            "x": "имя колонки оси категорий/времени",
            "series": [{ "y": "имя колонки меры", "label": "подпись" }],
            "stacked": "bool (для накопления)",
            "horizontal": "bool (горизонтальные столбцы / top-N)",
            "format": "money|int|percent|number",
            "alternatives": ["типы для чипов-переключателя у пользователя"]
        },
        "spec_pie": {
            "type": "pie|doughnut",
            "title": "string",
            "category": "колонка-категория (метки)",
            "value": "колонка-мера (значения)",
            "format": "money|int|percent|number"
        },
        "rules": [
            "Тема (свет/тёмная) и цвета осей подхватываются автоматически — НЕ хардкодь цвета.",
            "Имена колонок в chart-spec (x/series.y или category/value) должны совпадать с алиасами в SELECT.",
            "alternatives рисует чипы для мгновенной смены типа пользователем — перечисли совместимые типы.",
            "Свой CSS — по минимуму; PluginCharts сам форматирует оси/легенду/тултипы."
        ]
    })
}

/// Определения инструментов построителя графиков.
pub fn chart_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "chart_template".into(),
            description: "Минимальный ВАЛИДНЫЙ скелет графика-плагина (hybrid) по типу: \
                          line | bar | pie. Возвращает { bundle, hint } с готовыми client/server \
                          скриптами и демо-SQL. Начинай новый график с шаблона."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "type": { "type": "string", "enum": ["line", "bar", "pie"], "description": "Базовый тип графика. По умолчанию line." }
                }
            }),
        },
        ToolDefinition {
            name: "chart_examples".into(),
            description: "Готовые рабочие примеры графиков (time-series / bar / doughnut) на реальной \
                          таблице a012_wb_sales — образец структуры, SQL и chart-spec. \
                          Возвращает { examples:[{ title, bundle }] }."
                .into(),
            parameters: json!({ "type": "object", "properties": {} }),
        },
        ToolDefinition {
            name: "get_chart_ui_contract".into(),
            description: "Контракт рантайма графиков: как звать PluginCharts.render, форма chart-spec \
                          (картезианские типы и pie/doughnut), доступные типы и правила темы/альтернатив."
                .into(),
            parameters: json!({ "type": "object", "properties": {} }),
        },
    ]
}

/// Диспетчер инструментов построителя графиков (без БД — чистые заготовки).
pub fn execute_chart_tool(name: &str, arguments: &str) -> Value {
    let args: Value = serde_json::from_str(arguments).unwrap_or_else(|_| json!({}));
    match name {
        "chart_template" => chart_template(&args),
        "chart_examples" => chart_examples(),
        "get_chart_ui_contract" => chart_ui_contract(),
        _ => json!({ "error": format!("Unknown chart tool: '{}'", name) }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::plugins::PluginBundle;

    #[test]
    fn templates_are_valid_bundles() {
        for kind in ["line", "bar", "pie"] {
            let out = chart_template(&json!({ "type": kind }));
            let bundle: PluginBundle = serde_json::from_value(out["bundle"].clone())
                .unwrap_or_else(|e| panic!("template {kind} bundle parse: {e}"));
            bundle
                .validate()
                .unwrap_or_else(|e| panic!("template {kind} invalid: {e}"));
        }
    }

    #[test]
    fn examples_are_valid_bundles() {
        let out = chart_examples();
        let arr = out["examples"].as_array().expect("examples array");
        assert_eq!(arr.len(), 3);
        for ex in arr {
            let bundle: PluginBundle =
                serde_json::from_value(ex["bundle"].clone()).expect("example bundle parse");
            bundle.validate().expect("example bundle invalid");
        }
    }

    #[test]
    fn default_type_is_line() {
        let out = chart_template(&json!({}));
        assert_eq!(out["type"], json!("line"));
    }
}
