---
title: BI Индикаторы (a024)
tags: [a024, bi, индикатор, дашборд, data-view, kpi]
related: [a025, dv001_revenue, p904]
updated: 2026-03-12
---

# BI Индикаторы (a024_bi_indicator)

BI Индикатор — единица отображения данных на дашборде. Каждый индикатор:
- привязан к источнику данных (DataView + метрика)
- имеет настройки отображения (HTML-шаблон, CSS, формат числа)
- может иметь пороговые значения (зелёный / красный)
- может быть drill-down кликабельным

## Структура DataSpec (источник данных)

**Новый путь (рекомендуется):**
```json
{
  "view_id": "dv001_revenue",
  "metric_id": "revenue"
}
```

**Приоритеты вычисления:**
1. `view_id` → DataViewRegistry (новый, основной)
2. `data_source_config` → schema_executor (универсальный fallback)
3. `schema_query` → legacy p904 путь
4. `schema_id` → IndicatorRegistry (deprecated)

## Создание индикатора через API

```
POST /api/a024/bi_indicator
```

**Минимальный запрос:**
```json
{
  "description": "Выручка WB за период",
  "owner_user_id": "<UUID пользователя>",
  "data_spec": {
    "schema_id": "",
    "query_config": { "data_source": "", "selected_fields": [], "groupings": [], "filters": {}, "display_fields": [], "sort": { "field": "", "ascending": true }, "enabled_fields": [] },
    "view_id": "dv001_revenue",
    "metric_id": "revenue"
  },
  "params": [
    { "key": "date_from",      "param_type": "date", "label": "Начало периода",  "required": false, "global_filter_key": "date_from" },
    { "key": "date_to",        "param_type": "date", "label": "Конец периода",   "required": false, "global_filter_key": "date_to" },
    { "key": "connection_ids", "param_type": "ref",  "label": "Кабинеты МП",     "required": false, "global_filter_key": "connection_ids" }
  ],
  "view_spec": {
    "style_name": "custom",
    "custom_html": "<div class=\"kpi\"><div class=\"kpi__label\">{{title}}</div><div class=\"kpi__value\">{{value}}</div><div class=\"kpi__delta\">{{delta}}</div></div>",
    "format": { "kind": "Money", "currency": "RUB" },
    "thresholds": []
  },
  "status": "active",
  "is_public": true
}
```

## ViewSpec — отображение индикатора

### Переменные шаблона

| Переменная  | Значение                              |
|-------------|---------------------------------------|
| `{{value}}` | Текущее значение (форматированное)    |
| `{{delta}}` | Изменение к предыдущему периоду (±%)  |
| `{{title}}` | Название индикатора (description)     |

### Форматы числа (format.kind)

| kind       | Описание                         | Пример       |
|------------|----------------------------------|--------------|
| `Money`    | Денежная сумма с валютой         | 1 234 567 ₽  |
| `Integer`  | Целое число                      | 42 000       |
| `Percent`  | Процент с десятичными знаками    | 23.5%        |

### Пороговые значения

```json
"thresholds": [
  { "condition": "> 25", "color": "rgb(34,197,94)", "label": "Высокая" },
  { "condition": "< 10", "color": "rgb(239,68,68)",  "label": "Низкая"  }
]
```

## Список существующих индикаторов

| Код             | Название          | DataView         | Метрика     | Статус  |
|-----------------|-------------------|------------------|-------------|---------|
| IND-REVENUE-WB  | Выручка WB        | dv001_revenue    | revenue     | active  |
| IND-MARGIN      | Маржинальность    | —                | —           | active  |
| IND-ORDERS      | Кол-во заказов    | dv001_revenue    | (legacy)    | active  |

## Получение списка индикаторов

```
GET /api/a024/bi_indicator
```

## Вычисление значения индикатора

```
POST /api/a024/bi_indicator/{id}/compute
{
  "date_from": "2026-01-01",
  "date_to": "2026-01-31",
  "connection_mp_refs": []
}
```

## Связанные концепции

- **BI Дашборд (a025)** — компонует несколько индикаторов в сетку
- **DataView (dv001_revenue)** — источник данных для индикаторов продаж
- **p904_sales_data** — основная таблица сводных данных продаж
