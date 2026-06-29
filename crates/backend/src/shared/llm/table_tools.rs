//! Инструменты агента-построителя таблиц (навык `table-builder`).
//!
//! Таблица = один hybrid-плагин: `server_script` тянет данные SELECT'ом, а
//! `client_script` отдаёт строки + компактный table-spec в `window.PluginTables.render`
//! (тема-aware HTML-таблица без зависимостей — см. `frontend/static/plugin-tables.js`).
//!
//! Эти инструменты дают только заготовки (шаблон/примеры/контракт). Валидация,
//! smoke-тест, сохранение и запуск выполняются существующими `plugin_*`-инструментами.

use super::types::ToolDefinition;
use serde_json::{json, Value};

/// Имена инструментов построителя таблиц (для guard'а в диспетчере).
pub const TABLE_TOOL_NAMES: &[&str] =
    &["table_template", "table_examples", "get_table_ui_contract"];

/// Клиентский ES-модуль таблицы: грузит данные и рисует через PluginTables.
fn table_client_script(spec_json: &str) -> String {
    format!(
        r##"let _table = null;
export async function mount(root, host) {{
  root.replaceChildren();
  const loading = document.createElement("div");
  loading.className = "status";
  loading.textContent = "Загрузка…";
  root.appendChild(loading);
  try {{
    const rows = await host.invoke("data", {{}});
    const spec = {spec};
    _table = PluginTables.render(root, spec, rows);
  }} catch (e) {{
    root.replaceChildren();
    const box = document.createElement("div");
    box.className = "status status--error";
    box.textContent = e && e.message ? e.message : String(e);
    root.appendChild(box);
  }}
}}
export async function unmount() {{
  if (_table) {{ try {{ _table.destroy(); }} catch (e) {{}} _table = null; }}
}}
"##,
        spec = spec_json
    )
}

/// Серверный метод без параметров (демо-SQL).
const TABLE_SERVER: &str = r##"export async function data(args, host) {
  return await host.db.queryResource("rows", []);
}
"##;

/// Серверный метод с периодом из контекста (для примеров на реальных таблицах).
const TABLE_SERVER_CTX: &str = r##"export async function data(args, host) {
  const c = host.context || {};
  const p = c.params || {};
  const today = new Date();
  const fromDefault = new Date(today);
  fromDefault.setDate(today.getDate() - 30);
  const iso = d => d.toISOString().slice(0, 10);
  const from = c.date_from || p.date_from || args.date_from || iso(fromDefault);
  const to = c.date_to || p.date_to || args.date_to || iso(today);
  return await host.db.queryResource("rows", [from, to]);
}
"##;

/// Собрать bundle-таблицу. `with_ctx` — серверный метод читает период из контекста.
fn build_bundle(
    code: &str,
    title: &str,
    description: &str,
    capabilities: Value,
    spec: Value,
    sql: &str,
    with_ctx: bool,
) -> Value {
    let params = if with_ctx {
        json!([
            {
                "key": "date_from",
                "param_type": "date",
                "label": "Дата с",
                "required": false,
                "global_filter_key": "date_from"
            },
            {
                "key": "date_to",
                "param_type": "date",
                "label": "Дата по",
                "required": false,
                "global_filter_key": "date_to"
            }
        ])
    } else {
        json!([])
    };
    json!({
        "manifest": {
            "code": code,
            "title": title,
            "runtime": "hybrid",
            "api_version": "2",
            "description": description,
            "capabilities": capabilities,
        },
        "params": params,
        "client_script": table_client_script(&spec.to_string()),
        "server_script": if with_ctx { TABLE_SERVER_CTX } else { TABLE_SERVER },
        "sql_resources": { "rows": sql },
        "styles": ".ptables{padding:4px;}",
    })
}

// ─── Демо-SQL без зависимости от схемы (шаблон сразу валиден и запускается) ───

const DEMO_BASIC_SQL: &str = "SELECT 'Куртка' AS name, 5200 AS revenue, 42 AS qty UNION ALL SELECT 'Джинсы', 3100, 88 UNION ALL SELECT 'Кроссовки', 7400, 31 UNION ALL SELECT 'Футболка', 1800, 120";
const DEMO_FIN_SQL: &str = "SELECT 'WB' AS channel, 520000 AS revenue, 0.34 AS margin UNION ALL SELECT 'OZON', 310000, 0.21 UNION ALL SELECT 'YM', 180000, -0.05";
const DEMO_PIVOT_SQL: &str = "SELECT 'Одежда' AS category, 'Май' AS period, 5200 AS revenue UNION ALL SELECT 'Обувь', 'Май', 3100 UNION ALL SELECT 'Одежда', 'Июнь', 6100 UNION ALL SELECT 'Обувь', 'Июнь', 2700";

/// Минимальный валидный bundle-таблица по типу (basic | financial | pivot-lite).
fn table_template(args: &Value) -> Value {
    let kind = args.get("kind").and_then(Value::as_str).unwrap_or("basic");
    let caps = json!(["network:none"]);
    let bundle = match kind {
        "financial" => build_bundle(
            "MY-TABLE-FIN",
            "Моя таблица (финансовая)",
            "Шаблон финансовой таблицы: каналы, выручка, маржа с условным форматированием.",
            caps,
            json!({
                "title": "Финансовый срез",
                "columns": [
                    { "key": "channel", "label": "Канал", "type": "text" },
                    { "key": "revenue", "label": "Выручка", "type": "money" },
                    { "key": "margin", "label": "Маржа", "type": "percent" }
                ],
                "sort": { "key": "revenue", "dir": "desc" },
                "filters": { "global": true, "perColumn": true },
                "conditionalFormat": [
                    { "column": "revenue", "kind": "dataBar", "color": "primary" },
                    { "column": "margin", "kind": "threshold", "rules": [
                        { "op": "<", "value": 0, "color": "error", "target": "text" },
                        { "op": ">=", "value": 0.3, "color": "success", "target": "bg" }
                    ] }
                ],
                "totals": { "enabled": true, "agg": { "revenue": "sum", "margin": "avg" } },
                "pagination": { "enabled": true, "pageSize": 50 },
                "export": { "csv": true, "clipboard": true }
            }),
            DEMO_FIN_SQL,
            false,
        ),
        "pivot-lite" => build_bundle(
            "MY-TABLE-PIVOT",
            "Моя таблица (срез)",
            "Шаблон таблицы-среза: категория × период × мера (плоская форма).",
            caps,
            json!({
                "title": "Срез по категориям",
                "columns": [
                    { "key": "category", "label": "Категория", "type": "text" },
                    { "key": "period", "label": "Период", "type": "text" },
                    { "key": "revenue", "label": "Выручка", "type": "money" }
                ],
                "sort": { "key": "category", "dir": "asc" },
                "filters": { "global": true, "perColumn": true },
                "conditionalFormat": [
                    { "column": "revenue", "kind": "heatmap", "min": "error", "mid": "warning", "max": "success" }
                ],
                "totals": { "enabled": true, "agg": { "revenue": "sum" } },
                "pagination": { "enabled": true, "pageSize": 50 },
                "export": { "csv": true, "clipboard": true }
            }),
            DEMO_PIVOT_SQL,
            false,
        ),
        _ => build_bundle(
            "MY-TABLE-BASIC",
            "Моя таблица",
            "Базовый шаблон: текст + меры, сортировка/фильтры/итоги/экспорт.",
            caps,
            json!({
                "title": "Заголовок",
                "columns": [
                    { "key": "name", "label": "Наименование", "type": "text" },
                    { "key": "revenue", "label": "Выручка", "type": "money" },
                    { "key": "qty", "label": "Кол-во", "type": "int" }
                ],
                "sort": { "key": "revenue", "dir": "desc" },
                "filters": { "global": true, "perColumn": true },
                "conditionalFormat": [
                    { "column": "revenue", "kind": "dataBar", "color": "primary" }
                ],
                "totals": { "enabled": true, "agg": { "revenue": "sum", "qty": "sum" } },
                "pagination": { "enabled": true, "pageSize": 50 },
                "export": { "csv": true, "clipboard": true }
            }),
            DEMO_BASIC_SQL,
            false,
        ),
    };
    json!({
        "bundle": bundle,
        "kind": kind,
        "hint": "Минимальный валидный плагин-таблица. Замени code/title и SQL в sql_resources.rows на реальный \
                 SELECT (проверь через execute_query), синхронизируй column.key с алиасами SELECT, затем \
                 plugin_smoke_test → plugin_upsert(status=active)."
    })
}

/// Готовые рабочие примеры на реальной таблице a012_wb_sales (db:read:wb).
fn table_examples() -> Value {
    let wb_caps = json!(["db:read:wb", "network:none"]);

    let articles = build_bundle(
        "EXAMPLE-TABLE-TOP-ARTICLES",
        "Пример: артикулы WB по выручке",
        "Таблица топ-артикулов: выручка и кол-во продаж за период из контекста.",
        wb_caps.clone(),
        json!({
            "title": "Артикулы WB по выручке",
            "columns": [
                { "key": "article", "label": "Артикул", "type": "text", "width": "200px" },
                { "key": "revenue", "label": "Выручка", "type": "money" },
                { "key": "qty", "label": "Продаж, шт", "type": "int" }
            ],
            "sort": { "key": "revenue", "dir": "desc" },
            "filters": { "global": true, "perColumn": true },
            "conditionalFormat": [
                { "column": "revenue", "kind": "dataBar", "color": "primary" }
            ],
            "totals": { "enabled": true, "agg": { "revenue": "sum", "qty": "sum" } },
            "pagination": { "enabled": true, "pageSize": 50 },
            "export": { "csv": true, "clipboard": true }
        }),
        "SELECT article, SUM(total_price) AS revenue, COUNT(*) AS qty FROM a012_wb_sales WHERE is_deleted = 0 AND date BETWEEN ? AND ? GROUP BY article ORDER BY revenue DESC LIMIT 200",
        true,
    );

    let daily = build_bundle(
        "EXAMPLE-TABLE-DAILY",
        "Пример: продажи WB по дням",
        "Таблица выручки и числа продаж по дням за период из контекста.",
        wb_caps,
        json!({
            "title": "Продажи WB по дням",
            "columns": [
                { "key": "d", "label": "Дата", "type": "date" },
                { "key": "revenue", "label": "Выручка", "type": "money" },
                { "key": "qty", "label": "Продаж, шт", "type": "int" }
            ],
            "sort": { "key": "d", "dir": "asc" },
            "filters": { "global": true, "perColumn": true },
            "conditionalFormat": [
                { "column": "revenue", "kind": "heatmap", "min": "error", "mid": "warning", "max": "success" }
            ],
            "totals": { "enabled": true, "agg": { "revenue": "sum", "qty": "sum" } },
            "pagination": { "enabled": true, "pageSize": 50 },
            "export": { "csv": true, "clipboard": true }
        }),
        "SELECT date AS d, SUM(total_price) AS revenue, COUNT(*) AS qty FROM a012_wb_sales WHERE is_deleted = 0 AND date BETWEEN ? AND ? GROUP BY date ORDER BY date",
        true,
    );

    json!({
        "examples": [
            { "title": "Топ артикулов (dataBar)", "bundle": articles },
            { "title": "По дням (heatmap)", "bundle": daily }
        ],
        "hint": "Скопируй ближайший по форме данных пример, замени таблицу/колонки на свои (проверь SELECT \
                 через execute_query), выровняй table-spec, затем plugin_smoke_test → plugin_upsert."
    })
}

/// Контракт рантайма таблиц: как звать PluginTables.render и какой table-spec.
fn table_ui_contract() -> Value {
    json!({
        "runtime": "В iframe доступна глобаль window.PluginTables. В client_script внутри mount(root, host): \
                    const rows = await host.invoke(\"data\", {}); PluginTables.render(root, spec, rows).",
        "server": "Серверный метод data(args, host) возвращает массив объектов-строк через \
                   host.db.queryResource(\"rows\", params). Период бери из host.context.date_from/date_to \
                   или host.context.params.date_from/date_to; если контекст пустой, шаблон берёт последние 30 дней.",
        "client_side": "Сортировка/фильтры/условное форматирование/итоги/пагинация/экспорт работают \
                        КЛИЕНТ-САЙД над уже загруженным массивом строк — без новых запросов к серверу. \
                        Держи объём в разумных пределах (≈ до нескольких тысяч строк): используй \
                        GROUP BY / LIMIT в SELECT.",
        "column_types": ["text", "number", "int", "money", "percent", "date"],
        "spec": {
            "title": "string",
            "columns": [{
                "key": "имя колонки = алиас в SELECT",
                "label": "подпись",
                "type": "text|number|int|money|percent|date",
                "align": "left|right|center (необяз.; числа по умолчанию right)",
                "width": "напр. '180px' (необяз.)",
                "format": "money|int|percent|number (необяз.; по умолчанию = type)",
                "hidden": "bool (скрыта по умолчанию)"
            }],
            "sort": { "key": "колонка", "dir": "asc|desc" },
            "filters": { "global": "bool — строка поиска по всем колонкам", "perColumn": "bool — фильтр под каждой колонкой" },
            "conditionalFormat": [{
                "column": "ключ колонки",
                "kind": "threshold|dataBar|heatmap",
                "rules": "[для threshold] [{ op:'>|<|>=|<=|=|!=', value:N, color:'success|warning|error|<hex>', target:'text|bg' }]",
                "color": "[для dataBar] success|warning|error|primary|<hex>",
                "min": "[для heatmap] цвет нижней границы (success|warning|error|<hex>)",
                "mid": "[для heatmap] цвет середины",
                "max": "[для heatmap] цвет верхней границы"
            }],
            "totals": { "enabled": "bool", "agg": { "<колонка>": "sum|avg|count|min|max" } },
            "pagination": { "enabled": "bool (по умолчанию true)", "pageSize": "число (по умолчанию 50)" },
            "export": { "csv": "bool (по умолчанию true)", "clipboard": "bool (по умолчанию true)" }
        },
        "rules": [
            "Тема (свет/тёмная) и цвета подхватываются автоматически — НЕ хардкодь палитру; для cond-format \
             используй семантические имена success|warning|error|primary.",
            "column.key обязан совпадать с алиасом колонки в SELECT.",
            "Числовые операции (сортировка/итоги/dataBar/heatmap/числовые фильтры) работают только для \
             числовых типов (number|int|money|percent).",
            "Свой CSS — по минимуму; PluginTables сам рисует шапку/фильтры/итоги/пагинацию/тулбар."
        ]
    })
}

/// Определения инструментов построителя таблиц.
pub fn table_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "table_template".into(),
            description: "Минимальный ВАЛИДНЫЙ скелет плагина-таблицы (hybrid) по типу: \
                          basic | financial | pivot-lite. Возвращает { bundle, hint } с готовыми \
                          client/server скриптами и демо-SQL. Начинай новую таблицу с шаблона."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["basic", "financial", "pivot-lite"], "description": "Тип шаблона таблицы. По умолчанию basic." }
                }
            }),
        },
        ToolDefinition {
            name: "table_examples".into(),
            description: "Готовые рабочие примеры таблиц (топ-артикулы / по дням) на реальной \
                          таблице a012_wb_sales — образец структуры, SQL и table-spec \
                          (dataBar, heatmap, итоги). Возвращает { examples:[{ title, bundle }] }."
                .into(),
            parameters: json!({ "type": "object", "properties": {} }),
        },
        ToolDefinition {
            name: "get_table_ui_contract".into(),
            description: "Контракт рантайма таблиц: как звать PluginTables.render, полная форма \
                          table-spec (columns/sort/filters/conditionalFormat/totals/pagination/export), \
                          типы колонок и правила темы/условного форматирования."
                .into(),
            parameters: json!({ "type": "object", "properties": {} }),
        },
    ]
}

/// Диспетчер инструментов построителя таблиц (без БД — чистые заготовки).
pub fn execute_table_tool(name: &str, arguments: &str) -> Value {
    let args: Value = serde_json::from_str(arguments).unwrap_or_else(|_| json!({}));
    match name {
        "table_template" => table_template(&args),
        "table_examples" => table_examples(),
        "get_table_ui_contract" => table_ui_contract(),
        _ => json!({ "error": format!("Unknown table tool: '{}'", name) }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::plugins::PluginBundle;

    #[test]
    fn templates_are_valid_bundles() {
        for kind in ["basic", "financial", "pivot-lite"] {
            let out = table_template(&json!({ "kind": kind }));
            let bundle: PluginBundle = serde_json::from_value(out["bundle"].clone())
                .unwrap_or_else(|e| panic!("template {kind} bundle parse: {e}"));
            bundle
                .validate()
                .unwrap_or_else(|e| panic!("template {kind} invalid: {e}"));
        }
    }

    #[test]
    fn examples_are_valid_bundles() {
        let out = table_examples();
        let arr = out["examples"].as_array().expect("examples array");
        assert_eq!(arr.len(), 2);
        for ex in arr {
            let bundle: PluginBundle =
                serde_json::from_value(ex["bundle"].clone()).expect("example bundle parse");
            bundle.validate().expect("example bundle invalid");
        }
    }

    #[test]
    fn default_kind_is_basic() {
        let out = table_template(&json!({}));
        assert_eq!(out["kind"], json!("basic"));
    }
}
