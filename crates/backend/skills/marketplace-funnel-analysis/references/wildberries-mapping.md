# Wildberries — карта пайплайна воронки (источники → p916 → d406)

Этот файл — источник истины по WB-воронке в **этой** системе. Не путай с абстрактной
`funnel-model.md`: здесь реальные таблицы, поля и SQL. Полный нарратив проекции —
`projections/p916_mp_sales_funnel_turnovers/llm.md` (в коде).

## Пайплайн целиком

```
a036 (funnel report)  ┐
a026 (реклама)         ├─ стадия marketing ─┐
                       ┘                     ├─→ p916_mp_sales_funnel_turnovers ─→ d406 (дашборд)
a015 (заказы)          ┐                     │
a012 (продажи/выкупы)  ├─ стадия fulfillment ┘
                       ┘
a040 (поиск/«Джем»)  ✗ НЕ подключён к воронке (см. ниже)
```

- **p916** — проекция-накопитель (движения-обороты): каждый регистратор при
  проведении/импорте удаляет свои строки и вставляет заново; на чтении метрики
  агрегируются `SUM`. Метрики аддитивны, конверсии **не хранятся** — считаются на чтении.
- **d406** (`GET /api/dashboards/wb-sales-funnel`) — витрина поверх p916 (имена товаров из
  a004, канальный фильтр). Это **не** источник данных для LLM — данные бери из p916 через
  `execute_query`, а d406 используй как эталон для сверки чисел.
- Если движения p916 отсутствуют или выглядят неполными, сначала сравни их с сохранёнными
  документами-источниками, затем используй сценарий пересбора `u508`; точный порядок и SQL —
  `source-data-and-rebuild.md`.
- Товарные разрезы строй по основной номенклатуре 1С (`a004`) через маппинг `a007`;
  шесть измерений и устойчивый join описаны в `nomenclature-mapping.md`.

## Две стадии (`stage`) — «заказ» в них это РАЗНЫЕ числа

| stage | Источники | Что несёт |
|---|---|---|
| `marketing` | a036, a026 | верх воронки, дневной агрегат `nm_id × дата`, без идентичности заказа |
| `fulfillment` | a015, a012 | низ воронки, уровень заказа (заказ → выкуп/отмена/возврат) |

`funnel_order_count` (маркетинговый счётчик воронки из a036) ≠ `order_count` (фактические
строки заказов из a015). Не смешивай их и не выдавай одно за другое.

## Таблица соответствия: источник → поле p916 → канонический этап

| Источник (поле) | Поле p916 | Стадия воронки WB | Канонический этап (funnel-model) |
|---|---|---|---|
| a026 `metrics.views` | `show_paid_count` | платные показы | часть `impressions` |
| — (нет источника) | `show_free_count` | органические показы | часть `impressions` — **всегда `NULL`/`N/A`** |
| _вычисляется на чтении_ | `show_free_count + show_paid_count`, только при доступности обеих компонент | всего показов | `impressions`; иначе `N/A` |
| a026 `metrics.clicks` | `paid_open_count` | платные переходы | `clicks` (платный трек) |
| a036 openCard | `open_count` | переходы в карточку | `product_views` |
| a026 `metrics.atbs` | `paid_cart_count` | платные добавления в корзину | `cart_adds` (платный трек) |
| a036 addToCart | `cart_count` | корзина | `cart_adds` |
| a036 addToWishlist | `wishlist_count` | избранное | (вне линейной воронки) |
| a036 orders | `funnel_order_count` / `funnel_order_sum` | заказы-воронки (маркетинг) | — (счётчик, не факт) |
| a015 (заказ) | `order_count` / `order_sum` | заказы (факт) | `orders` |
| a015 (отмена) | `cancel_count` / `cancel_sum` | отмены | (ветка) |
| a012 (выкуп) | `buyout_count` / `buyout_sum` | выкупы | `deliveries` |
| a012 (возврат) | `return_count` / `return_sum` | возвраты | (ветка) |

## Две оси дат (ключевая особенность)

Каждая строка несёт **две даты** — выбирай ось под вопрос:

- `cohort_date` — **дата заказа** (винтаж): «из заказов дня N выкуплено/отменено столько-то».
  Для стадии `marketing` — день воронки.
- `event_date` — **дата транзакции** самого события (касса/период отмены/выкупа). Для
  `marketing` совпадает с `cohort_date`.

Пример: отменённый заказ порождает две строки от одного регистратора: «заказ»
(`cohort_date=event_date=`дата заказа) и «отмена» (`cohort_date=`дата заказа,
`event_date=`дата отмены).

## Доступность (`N/A ≠ 0`)

`show_free_count`/`show_paid_count` — nullable (`NULL` = данных нет, **не** ноль). На чтении
рядом с суммами считай `SUM(CASE WHEN ... IS NOT NULL THEN 1 ELSE 0 END)` → флаг доступности.
Если источника показов/рекламы нет (нет подписки «Джем» / нет a026), верхние этапы — `N/A`, а
не `0`. Дашборд d406 так и показывает.

## a040 исключён из воронки (частый вопрос)

`a040_wb_search_analytics_daily` (поиск/«Джем») **не** питает p916. Живой WB-эндпоинт
`/table/details` отдаёт только `visibility` (% показов в поиске), а не счётчик показов
(`impressions` всегда 0). Проценты в `show_free_count` писать нельзя (SUM смешал бы штуки и
проценты), поэтому органические показы сейчас `N/A` до появления реального источника счётчика.
a040 полезен для поисковой аналитики (позиции, топ-запросы, видимость), но это отдельный слой,
не воронка.

## Канальный сплит paid/free (фильтр d406: Все / Платные / Бесплатные)

Ответ по каждой стадии несёт и total, и `paid_*`; фильтр применяется на клиенте: All=total,
Paid=`paid_*`, Free=`total − paid` (обрезка ≥0). Природа платного трека разная:

- **Верх (переходы/корзина)** — из собственных счётчиков рекламы a026 (`show_paid_count`,
  `paid_open_count`, `paid_cart_count`). Free = total(a036) − paid.
- **Низ (заказы/выкупы/отмены/возвраты)** — делится по вхождению `srid` заказа в атрибуцию
  рекламы `p913_wb_advert_order_attr` (`turnover_code = 'advert_clicks_order_accrual'`).
  fulfillment-строки p916 несут `order_key` (srid); на чтении делается
  `LEFT JOIN` и `paid_* = SUM(CASE WHEN pj.order_key IS NOT NULL THEN <metric> END)`.
- Оговорка: p913-членство = «заказы со **значимой** долей рекламных затрат» → заказ с ~0
  расхода может не попасть в платные.

**Запрет гибридной конверсии:** `show_paid_count` нельзя делить на общие `open_count`,
`cart_count`, `order_count` или `buyout_count`. Общая и платная ветки имеют разную область
наблюдения. При недоступных органических показах общие `show → ...` конверсии равны `N/A`.
Платные конверсии рассчитывай только между соседними `paid_*`; если нужного `paid_*` нет,
результат также `N/A`.

## Готовые SQL-шаблоны (`execute_query`, один SELECT, bind `?`)

**0. Найти кабинет по отображаемому имени:**

```sql
SELECT id, code, description
FROM a006_connection_mp
WHERE description LIKE ?
  AND marketplace = (SELECT id FROM a005_marketplace WHERE code = 'mp-wb')
```

**1. Общая и платная воронка за период (когортная ось).** Один запрос возвращает совместимые
total- и paid-метрики. Не добавляй SQL-комментарии `--`: `execute_query` их отклоняет.

```sql
SELECT
  SUM(COALESCE(f.show_free_count,0)) AS show_free,
  SUM(COALESCE(f.show_paid_count,0)) AS show_paid,
  CASE
    WHEN SUM(CASE WHEN f.show_free_count IS NOT NULL THEN 1 ELSE 0 END) > 0
     AND SUM(CASE WHEN f.show_paid_count IS NOT NULL THEN 1 ELSE 0 END) > 0
    THEN SUM(COALESCE(f.show_free_count,0)) + SUM(COALESCE(f.show_paid_count,0))
    ELSE NULL
  END AS show_total,
  SUM(COALESCE(f.open_count,0)) AS opens,
  SUM(COALESCE(f.cart_count,0)) AS carts,
  SUM(COALESCE(f.order_count,0)) AS orders,
  SUM(COALESCE(f.cancel_count,0)) AS cancels,
  SUM(COALESCE(f.buyout_count,0)) AS buyouts,
  SUM(COALESCE(f.return_count,0)) AS returns,
  SUM(COALESCE(f.paid_open_count,0)) AS paid_opens,
  SUM(COALESCE(f.paid_cart_count,0)) AS paid_carts,
  SUM(CASE WHEN pj.order_key IS NOT NULL THEN COALESCE(f.order_count,0) ELSE 0 END) AS paid_orders,
  SUM(CASE WHEN pj.order_key IS NOT NULL THEN COALESCE(f.buyout_count,0) ELSE 0 END) AS paid_buyouts
FROM p916_mp_sales_funnel_turnovers f
LEFT JOIN (
  SELECT DISTINCT order_key
  FROM p913_wb_advert_order_attr
  WHERE turnover_code = 'advert_clicks_order_accrual'
) pj ON pj.order_key = f.order_key
WHERE f.cohort_date BETWEEN ? AND ?
  AND f.connection_mp_ref = ?
```

**2. Агрегат воронки по товару за период (когортная ось).** Показы/переходы/корзина
берём из marketing-строк, заказы/выкупы — из fulfillment; поэтому суммируем по всей проекции
и группируем по `nm_id`:

```sql
SELECT nm_id,
       SUM(COALESCE(show_free_count,0)) AS show_free,
       SUM(COALESCE(show_paid_count,0)) AS show_paid,
       CASE
         WHEN SUM(CASE WHEN show_free_count IS NOT NULL THEN 1 ELSE 0 END) > 0
          AND SUM(CASE WHEN show_paid_count IS NOT NULL THEN 1 ELSE 0 END) > 0
         THEN SUM(COALESCE(show_free_count,0)) + SUM(COALESCE(show_paid_count,0))
         ELSE NULL
       END AS show_total,
       SUM(CASE WHEN show_free_count IS NOT NULL THEN 1 ELSE 0 END) AS free_shows_avail,
       SUM(CASE WHEN show_paid_count IS NOT NULL THEN 1 ELSE 0 END) AS paid_shows_avail,
       SUM(COALESCE(open_count,0))   AS opens,
       SUM(COALESCE(cart_count,0))   AS carts,
       SUM(COALESCE(order_count,0))  AS orders,
       SUM(COALESCE(cancel_count,0)) AS cancels,
       SUM(COALESCE(buyout_count,0)) AS buyouts,
       SUM(COALESCE(return_count,0)) AS returns,
       SUM(COALESCE(order_sum,0))    AS order_sum
FROM p916_mp_sales_funnel_turnovers
WHERE cohort_date BETWEEN ? AND ?
  AND connection_mp_ref = ?
GROUP BY nm_id
ORDER BY orders DESC
```

Для потоковой картины (движение за период) замени `cohort_date` на `event_date`.

**3. Конверсии считаются на чтении** (не хранятся):
`open_to_cart = carts/opens`, `cart_to_order = orders/carts`,
`order_to_buyout = buyouts/orders`, `cancel_rate = cancels/orders`. Если знаменатель 0 —
конверсия `null`, а не 0.

**4. Разбивка paid/free низа воронки** (заказы по каналу через p913):

```sql
SELECT
  SUM(order_count) AS orders_total,
  SUM(CASE WHEN pj.order_key IS NOT NULL THEN order_count ELSE 0 END) AS orders_paid
FROM p916_mp_sales_funnel_turnovers f
LEFT JOIN (
  SELECT DISTINCT order_key
  FROM p913_wb_advert_order_attr
  WHERE turnover_code = 'advert_clicks_order_accrual'
) pj ON pj.order_key = f.order_key
WHERE f.stage = 'fulfillment'
  AND f.cohort_date BETWEEN ? AND ?
  AND f.connection_mp_ref = ?
```

`orders_free = orders_total − orders_paid`.

**5. Расследование конкретного заказа** — drilldown в исходники: строки p916 по `order_key`
(= srid), затем `a015_wb_orders` (заказ/отмена) и `a012_wb_sales` (выкуп/возврат) по тому же
srid; канал — вхождение srid в p913.
