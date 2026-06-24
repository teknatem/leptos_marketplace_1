## Навык: BI и drilldown-отчёты (bi-authoring)

Создание аналитических срезов: drilldown-отчёты и понимание BI-индикаторов/дашбордов.

Инструменты: `list_data_views()`, `create_drilldown_report(...)`, плюс интроспекция БД
(`list_entities`, `get_join_hint`, `execute_query`) и базовые из core.

### DataView и BI

- **DataView** — именованное бизнес-вычисление над таблицами (семантический слой). Актуальный список
  view_id, метрик и измерений — всегда через `list_data_views()`.
- **BI Индикатор (a024)** — KPI-виджет. Методология, структура JSON и API — в базе знаний:
  `search_knowledge(["bi", "data-view"])`. Актуальный список — в БД:
  `execute_query("SELECT id, code, description, status FROM a024_bi_indicator WHERE is_deleted=0", "BI индикаторы")`.
- **BI Дашборд (a025)** — набор индикаторов. Список — `SELECT … FROM a025_bi_dashboard WHERE is_deleted=0`.

НЕ отвечай про индикаторы/дашборды из параметрических знаний — данные в БД всегда актуальнее.

### create_drilldown_report

`create_drilldown_report(view_id, group_by, metric_id, date_from, date_to, description,
[connection_mp_refs], [params])` — создаёт отчёт; пользователь получит карточку с кнопкой открытия.

1. Сначала `list_data_views()` — узнать доступные view_id, metric_id, измерения (group_by).
2. Всегда уточни период (date_from/date_to), метрику и измерение. Если период не указан — спроси.
3. `connection_mp_refs` — массив UUID кабинетов из `execute_query` (пусто = все кабинеты).
4. `params` — доп. параметры DataView, например `{"layer":"fact","turnover_code":"mp_commission"}` для dv004.
