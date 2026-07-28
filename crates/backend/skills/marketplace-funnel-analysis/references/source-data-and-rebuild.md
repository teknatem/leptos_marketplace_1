# Первичные данные и пересбор воронки WB

`p916` — производная проекция. Пустая или неполная проекция ещё не означает, что бизнес-данных
нет. Перед выводом о нехватке данных проверь документы-источники на том же периоде и кабинете.

## Источники и движения

| Документ | Что хранит | Что создаёт в p916 |
|---|---|---|
| `a036_wb_sales_funnel_daily` | дневная воронка по `nm_id`: карточка, корзина, заказы-воронки | `marketing`: `open_count`, `cart_count`, `wishlist_count`, `funnel_order_*` |
| `a026_wb_advert_daily` | дневная реклама | `marketing`: `show_paid_count`, `paid_open_count`, `paid_cart_count` |
| `a015_wb_orders` | заказ и отмена на уровне `srid` | `fulfillment`: `order_*`, `cancel_*` |
| `a012_wb_sales` | выкуп и возврат на уровне `srid` | `fulfillment`: `buyout_*`, `return_*` |

## Обязательная диагностика первички

Сначала зафиксируй `date_from`, `date_to` и `connection_mp_ref`. Выполни read-only проверки:

```sql
SELECT COUNT(*) AS docs,
       MIN(document_date) AS min_date,
       MAX(document_date) AS max_date,
       SUM(total_open_count) AS opens,
       SUM(total_cart_count) AS carts,
       SUM(total_order_count) AS funnel_orders
FROM a036_wb_sales_funnel_daily
WHERE document_date BETWEEN ? AND ?
  AND connection_id = ?
  AND is_deleted = 0
```

```sql
SELECT COUNT(*) AS docs,
       MIN(document_date) AS min_date,
       MAX(document_date) AS max_date,
       SUM(total_views) AS paid_shows,
       SUM(total_clicks) AS paid_clicks,
       SUM(total_orders) AS advert_orders
FROM a026_wb_advert_daily
WHERE document_date BETWEEN ? AND ?
  AND connection_id = ?
  AND is_deleted = 0
```

```sql
SELECT COUNT(*) AS docs,
       MIN(document_date) AS min_date,
       MAX(document_date) AS max_date,
       SUM(CASE WHEN COALESCE(is_cancel,0) = 1 THEN 1 ELSE 0 END) AS cancelled_docs,
       SUM(CASE WHEN nomenclature_ref IS NULL OR nomenclature_ref = '' THEN 1 ELSE 0 END) AS unmapped_docs
FROM a015_wb_orders
WHERE substr(document_date,1,10) BETWEEN ? AND ?
  AND json_extract(header_json, '$.connection_id') = ?
  AND is_deleted = 0
```

Для `a012` нельзя ограничиваться только `sale_date <= date_to`: `u508` берёт `srid` заказов
когорты из `a015`, затем выбирает связанные продажи/возвраты начиная с `date_from` без верхней
границы, чтобы захватить поздние выкупы.

```sql
WITH cohort_orders AS (
  SELECT DISTINCT document_no AS srid
  FROM a015_wb_orders
  WHERE substr(document_date,1,10) BETWEEN ? AND ?
    AND json_extract(header_json, '$.connection_id') = ?
    AND is_deleted = 0
)
SELECT COUNT(*) AS docs,
       MIN(substr(s.sale_date,1,10)) AS min_sale_date,
       MAX(substr(s.sale_date,1,10)) AS max_sale_date,
       SUM(CASE WHEN COALESCE(s.is_customer_return,0) = 1 THEN 1 ELSE 0 END) AS returns,
       SUM(CASE WHEN s.nomenclature_ref IS NULL OR s.nomenclature_ref = ''
                THEN 1 ELSE 0 END) AS unmapped_docs
FROM a012_wb_sales s
JOIN cohort_orders o ON o.srid = s.document_no
WHERE substr(s.sale_date,1,10) >= ?
  AND s.is_deleted = 0
```

После первички сравни покрытие проекции:

```sql
SELECT registrator_type,
       COUNT(*) AS projection_rows,
       MIN(cohort_date) AS min_cohort_date,
       MAX(cohort_date) AS max_cohort_date,
       SUM(open_count) AS opens,
       SUM(cart_count) AS carts,
       SUM(order_count) AS orders,
       SUM(buyout_count) AS buyouts,
       SUM(COALESCE(show_paid_count,0)) AS paid_shows
FROM p916_mp_sales_funnel_turnovers
WHERE cohort_date BETWEEN ? AND ?
  AND connection_mp_ref = ?
GROUP BY registrator_type
ORDER BY registrator_type
```

## Как интерпретировать

- Источник пуст → пересбор не создаст отсутствующие события. Сообщи, какой импорт и период
  отсутствуют. Историю `a036` нельзя восстановить из `a026`, `a015` или `a012`.
- Источник заполнен, а соответствующих строк `p916` нет/мало → проекция не проведена или
  устарела; это кандидат на `u508`.
- `a036` есть, но `open_count/cart_count` в `p916` нет → пересобери stage marketing.
- `a015/a012` есть, но fulfillment пуст → перепроведи нижнюю часть воронки.
- `a026` есть, но платные показы отсутствуют → перепроведи рекламные документы.

## `u508_repost_documents`

UI: **u508 — «Перепроведение документов и проекций» → «Пересбор воронки за период»**.

Сценарий выполняет четыре шага:

1. перепроводит `a015` — заказы/отмены;
2. перепроводит связанные `a012` — выкупы/возвраты когорты;
3. перепроводит `a026` — платные показы/переходы/корзина;
4. пересобирает stage marketing из сохранённых `a036`.

API запуска: `POST /api/u508/repost/funnel/start`; прогресс:
`GET /api/u508/repost/{session_id}/progress`; сводка после пересбора:
`GET /api/u508/repost/funnel/diagnostics`.

`u508` меняет данные. Аналитический агент не запускает его без явного поручения и разрешения:
он диагностирует, показывает расхождение «первичка → p916» и рекомендует оператору пересбор.
