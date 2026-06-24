## Навык: аналитика данных (data-analytics)

Работа с данными маркетплейсов через SQL и семантический слой.

Инструменты: `list_entities([category])`, `get_join_hint(from, to)`, `execute_query(sql, description)`,
`list_data_views()`, `get_chart_of_accounts()`, `list_gl_turnovers([report_group])` (+ базовые из core:
`get_architecture_overview`, `get_entity_schema`, `search_knowledge`/`get_knowledge`).

### Правила работы с SQL

1. Если знаешь entity_index — сразу `get_entity_schema`, НЕ вызывай `list_entities`.
   Если индекс неизвестен — `list_entities` с нужным category (wb/ozon/ym/ref/llm/bi), не без фильтра.
2. Имена таблиц и колонок должны ТОЧНО совпадать со схемой. Только SELECT (INSERT/UPDATE/DELETE запрещены).
3. Пиши SQL в блоках ```sql … ```. Давай краткое объяснение результата (2–3 предложения).
4. Если вопрос касается бизнес-метрик/терминов/методологии — вызови `search_knowledge`.

### Известные схемы (без get_entity_schema)

- `a006_connection_mp`: id (UUID), code, description (магазин), `marketplace` (FK→a005, UUID; именно
  `marketplace`, не `marketplace_id`), organization_ref (FK→a002), is_used (0/1), planned_commission_percent.
  Для WB: `WHERE marketplace = (SELECT id FROM a005_marketplace WHERE code = 'mp-wb')`.
- `a005_marketplace`: id, code (mp-wb/mp-ozon/mp-ym), description.
- `a002_organization`: id, code, description.

### Поиск UUID в справочниках

а) `get_entity_schema("a006")` — точные имена колонок;
б) `execute_query("SELECT id, code, description FROM a006_connection_mp WHERE …", "…")`;
в) из rows берёшь нужные id (UUID).

### General Ledger

Перед SQL к `sys_general_ledger` вызови `list_gl_turnovers` (точные turnover_code) и при необходимости
`get_chart_of_accounts` (план счетов, что дебетуется/кредитуется: 7609/76YA/9001/9002 и т.д.).

Если вопрос про BI-индикаторы/дашборды или нужно СОЗДАТЬ drilldown-отчёт — активируй навык `bi-authoring`.
