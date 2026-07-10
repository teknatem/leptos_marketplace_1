## Навык: аналитика данных (data-analytics)

Работа с данными маркетплейсов через SQL и семантический слой.

Инструменты: `list_data_sources([kind])`, `query_data_schema(...)`, `run_data_view_scalar(...)`,
`run_data_view_drilldown(...)`, `execute_query(sql, params, description)`, `list_entities([category])`,
`get_join_hint(from, to)`, `get_chart_of_accounts()`, `list_gl_turnovers([report_group])`
(+ базовые из core: `get_architecture_overview`, `get_entity_schema`, `search_knowledge`/`get_knowledge`).

### Источники данных: три роли (выбирай осознанно)

Доступ к аналитике — три независимых движка (подробно: ADR-0010):
- **DataView (`dvXX`)** — курируемые «виртуальные таблицы»: благословлённые метрики, сравнение
  **2 периодов**, кэш. Обнаруживай через `list_data_sources("dataview")`, читай данные через
  `run_data_view_scalar` или `run_data_view_drilldown`. Это **источник истины определений**
  (выручка, себестоимость и т.п.) — НЕ переизобретай их в сыром SQL.
- **Схемы таблиц (`dsXX`, kind=base)** — декларативное описание таблиц БД (поля/типы/связи) для
  гибкого ad-hoc (группировки/фильтры/агрегаты по «сырым» полям). В UI: «Схемы таблиц» (каталог) и
  «Конструктор запросов» (построитель). Обнаруживай через `list_data_sources("base")`, читай через
  `query_data_schema`. Сюда входят безопасные metadata-проекции справочников, например `a006`
  без API-ключей.
- **Сырой SQL (`execute_query`)** — только fallback для нестандартного и разового; один SELECT/WITH,
  bind-параметры через `params`. `a006_connection_mp` доступен (можно JOIN по `marketplace`/кабинету);
  `a001_connection_1c` остаётся недоступной.

Дерево выбора: нужен благословлённый показатель / 2 периода / составная метрика → DataView; нужен
произвольный разрез одной таблицы → схема/SQL; остальное → сырой SQL. Если одна таблица доступна и как
схема, и как DataView (напр. `p904`: ds03 гибкий, dv001 курируемый) — для официальных цифр бери DataView.

Воронка продаж WB (просмотры/корзина/заказы/выкуп и конверсии между ними, `a036_wb_sales_funnel_daily`) —
это `dv008_wb_sales_funnel`. Если пользователь прикрепил страницу плагина «Воронка продаж WB» или просит
анализ/рекомендации по воронке — читай через `run_data_view_drilldown(view_id="dv008_wb_sales_funnel",
group_by="nm_id"|"date"|"connection_mp_ref", metric_ids=[...])` (можно сразу несколько метрик — так
получаешь всю таблицу воронки за один вызов) или `run_data_view_scalar` для сводной цифры. Метрики:
`open_count, cart_count, order_count, order_sum, buyout_count, buyout_sum, cart_conv_pct, order_conv_pct,
buyout_pct`. Не переизобретай json_each по `lines_json` в сыром SQL — этот разбор уже в dv008.

### Правила работы с SQL

1. Для получения аналитических строк сначала используй `list_data_sources`, DataView и base-схемы.
   `get_entity_schema`/`list_entities` нужны для нестандартного Raw SQL.
   Если индекс неизвестен — `list_entities` с нужным category (wb/ozon/ym/ref/llm/bi), не без фильтра.
2. Имена таблиц и колонок должны ТОЧНО совпадать со схемой. Только SELECT (INSERT/UPDATE/DELETE запрещены).
3. Поля base-схемы (напр. `dim1` = категория, `marketplace`) — это ИЗМЕРЕНИЯ схемы, а НЕ колонки
   таблицы: они джойнятся из справочников. В `query_data_schema` используй их как `group_by`/`filters`.
   Если нужен сырой SQL — НЕ выдумывай такие колонки, а возьми готовый `generated_sql` из ответа
   `query_data_schema` (там реальные JOIN-ы и колонки) и адаптируй его.
4. Пиши SQL в блоках ```sql … ```. Давай краткое объяснение результата (2–3 предложения).
5. Если вопрос касается бизнес-метрик/терминов/методологии — вызови `search_knowledge`.

### Термины → сущности (глоссарий)

- **товар / номенклатура / позиция / SKU / артикул / карточка** без уточнения площадки → всегда
  `a004_nomenclature` (справочник 1С:УТ; товары при `is_folder = 0`, категории-папки при `is_folder = 1`).
- **позиция/карточка на конкретном маркетплейсе** (nmId WB, offerId/shop_sku YM) → `a007_marketplace_product`;
  связь: `a004_nomenclature.id = a007_marketplace_product.nomenclature_ref`.
- Сомневаешься в терминах/связях сущности — `get_entity_schema("a004")` (там синонимы, поля и путь к МП).

### Известные схемы (без get_entity_schema)

- `a006_connection_mp`: id (UUID), code, description (магазин), `marketplace` (FK→a005, UUID; именно
  `marketplace`, не `marketplace_id`), organization_ref (FK→a002), is_used (0/1), planned_commission_percent.
  Для WB: `WHERE marketplace = (SELECT id FROM a005_marketplace WHERE code = 'mp-wb')`.
- `a005_marketplace`: id, code (mp-wb/mp-ozon/mp-ym), description.
- `a002_organization`: id, code, description.

### Поиск UUID в справочниках

а) `list_data_sources("base")` — найди безопасную схему `a006`;
б) `query_data_schema` с `fields=["id","code","description"]` и фильтром `is_used = 1`;
в) из `rows` возьми нужные id (UUID). Raw SQL к `a006_connection_mp` теперь разрешён — можно
   джойнить его напрямую (напр. по `marketplace`), не вытаскивая id отдельно.

### General Ledger

Перед SQL к `sys_general_ledger` вызови `list_gl_turnovers` (точные turnover_code) и при необходимости
`get_chart_of_accounts` (план счетов, что дебетуется/кредитуется: 7609/76YA/9001/9002 и т.д.).

Если вопрос про BI-индикаторы/дашборды или нужно СОЗДАТЬ drilldown-отчёт — активируй навык `bi-authoring`.
