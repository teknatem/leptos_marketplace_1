---
title: p916 — универсальная воронка продаж маркетплейсов (движения)
tags: [marketplaces, wildberries, funnel, воронка, p916, analytics, sales, orders, cancellations, returns, cohort]
related: [a036_wb_sales_funnel_daily, a015_wb_orders, a012_wb_sales, p909_mp_order_line_turnovers, p915_mp_order_events]
updated: 2026-07-20
---

# p916_mp_sales_funnel_turnovers — воронка продаж «от просмотра до завершения»

`p916` — универсальная проекция-накопитель воронки продаж маркетплейсов, сшивающая весь
путь товара: **показы → переходы → корзина → заказ → выкуп**, а также **отмены и возвраты**.
Стартовая реализация — только Wildberries, но ключи спроектированы под YM и OZON без
переделки.

Модель — **движения-обороты** (как `p909`/`p914`/`p915`): каждый регистратор при
проведении/импорте удаляет свои строки (delete-by-registrator) и вставляет заново;
агрегация метрик — `SUM` на чтении. Строки «широкие» (все метрики стадии — колонками одной
строки), пустые строки не пишутся (разреженность — контроль размера).

## Две стадии воронки (поле `stage`)

Стадии разделены сознательно, потому что данные разной природы, а «заказ» в них — **разные
числа** (маркетинговый счётчик воронки ≠ фактические строки заказов).

- **`marketing`** (верх воронки, источники `a036`/`a040`/`a026`) — дневной агрегат
  `nm_id × дата`, без идентичности заказа. Метрики: `show_free_count` (органические показы,
  из a040), `show_paid_count` (рекламные показы, из a026), `open_count` (переходы в карточку,
  a036), `cart_count`, `wishlist_count`, `funnel_order_count`, `funnel_order_sum`. «Всего
  показов» не хранится — считается на чтении `COALESCE(free,0)+COALESCE(paid,0)`.
- **`fulfillment`** (заказ → завершение, источники `a015_wb_orders` + `a012_wb_sales`) —
  уровень заказа. Метрики: `order_count`/`order_sum`, `cancel_count`/`cancel_sum`,
  `buyout_count`/`buyout_sum`, `return_count`/`return_sum`.

## Две оси дат (ключевая особенность)

Каждая строка несёт **две даты**:

- **`cohort_date`** — ось когорты = **дата заказа** (винтаж: «из заказов дня N выкуплено /
  отменено / возвращено столько-то»). Для стадии `marketing` — это день воронки.
- **`event_date`** — ось потока = **дата транзакции** самого события (касса/период). Для
  стадии `marketing` совпадает с `cohort_date`.

Пример: отменённый заказ порождает **две** строки от одного регистратора:
1) «заказ» — `order_count=1`, `cohort_date=event_date=`дата заказа;
2) «отмена» — `cancel_count=1`, `cohort_date=`дата заказа, `event_date=`дата отмены.

Так одна проекция строит и когортную (конверсия заказов), и потоковую (движение за период)
картину.

## Гранулярность и измерения

Ключ строки: `connection_mp_ref × товар × дата`. Товар — универсально
`marketplace_product_ref` (a007); `nm_id` — WB-native мост (для стыковки стадий, т.к. у
`a036` нет `marketplace_product_ref`). Имена/бренд/предмет в строке **не хранятся** — их
джойнят из `a004_nomenclature` / `a007_marketplace_product` на чтении.

## Регистраторы и хуки

| stage | Регистратор (`registrator_type`) | Источник | Где формируется |
|---|---|---|---|
| marketing | `a036_wb_sales_funnel_daily` | a036 (импорт) | `a036::repository::replace_for_period` (delete-by-period + insert): переходы/корзина/заказы-воронки |
| marketing | `a040_wb_search_analytics_daily` | a040 (импорт) | `a040::repository::replace_for_period` (delete-by-period + insert): `show_free_count` |
| marketing | `a026_wb_advert_daily` | a026 (импорт) | `a026::repository::replace_for_period_scoped` (delete-by-registrator + insert): `show_paid_count` |
| fulfillment | `a015_wb_orders` | a015 (проведение) | `a015::posting` (заказ + отмена) |
| fulfillment | `a012_wb_sales` | a012 (проведение) | `a012::posting` (выкуп/возврат) |

Репост через `u508` пересобирает стадию 2 автоматически (переиспользует `post_document`).
Разовый бэкфилл стадии 1 из накопленной истории a036:
`POST /api/a036/wb-sales-funnel/rebuild-funnel-projection`.

## Как читать / считать

- Метрики аддитивны → `SUM(...)`, группировка по нужной оси (`cohort_date` или `event_date`).
- Показы nullable → `SUM(COALESCE(show_free_count,0))`, `SUM(COALESCE(show_paid_count,0))`;
  всего = сумма обоих.
- Конверсии **не хранятся**, считаются на чтении: конверсия в корзину = `cart/open`,
  в заказ = `order/cart`, выкуп = `buyout/order`, доля отмен = `cancel/order`.
- Готовая агрегирующая функция: `p916::repository::aggregate_by_product(request)` с выбором
  оси (`FunnelDateAxis::Cohort | Event`).

## Когортная привязка выкупов/возвратов (srid → a015.order_dt)

У `a012_wb_sales` нет даты заказа на строке. При проведении a012 хук резолвит дату заказа по
`srid` (`a012.header.document_no` → `a015::repository::order_date_by_srid`) и передаёт её в
`builder::from_wb_sales(..., order_cohort_date)` как `cohort_date`. `event_date` остаётся датой
продажи (`sale_dt`). Если заказ в a015 не найден — фолбэк на дату продажи (для срезов без
исходного заказа). Отмены (`a015`) атрибутируются к дате заказа напрямую.

## Идемпотентность (детерминированный id + upsert)

`id` строки движения — детерминированный `uuid v5` от натурального ключа
`(registrator_type, registrator_ref, stage, kind, cohort_date, event_date, connection_mp_ref, nm_id|mp_ref)`,
где `kind` ∈ {`marketing`,`order`,`cancel`,`buyout`,`return`} (различает заказ/отмену одного
srid в один день). Вставки идут через `INSERT ... ON CONFLICT(id) DO UPDATE` — повтор той же
строки перезаписывает метрики, а не задваивает обороты. Delete-by-period/registrator оставлен
для удаления исчезнувших строк (разреженность). Менять `P916_ID_NAMESPACE` или состав ключа
нельзя — иначе `id` перестанут совпадать между прогонами (нужен полный пересбор проекции).

## Доступность показов (N/A ≠ 0)

`show_free_count`/`show_paid_count` — nullable (NULL = данных нет). На чтении
`aggregate_by_product`/`funnel_period_summary` рядом с суммами считают
`SUM(CASE WHEN ... IS NOT NULL THEN 1 ELSE 0 END)` → флаги `show_*_available` в
`MpFunnelAggRow`/`FunnelPeriodSummary`. Дашборд `d406` показывает `N/A` (не `0`), если источник
показов недоступен (нет подписки «Джем»/рекламы).

## Показы (важно для ответов про «верх воронки»)

Верх воронки собирает **три показателя показов**: `show_free_count` (бесплатные/органические,
из поисковой аналитики `a040`, «Товары по контенту», подписка «Джем»), `show_paid_count`
(платные, из рекламного отчёта `a026`, `metrics.views`) и «всего» = сумма обоих (на чтении).
Переходы (`open_count`, `a036`) — отдельная стадия ниже показов. Разделение платных и органических
показов сделано осознанно: смешивать трафик разной природы нельзя.

Оговорка: точная семантика `a040` (сырой счётчик показов vs доля/`visibility` в % выдачи) не
верифицирована офлайн — при первом живом прогоне `task024` сверить `parse_search_report_row`,
чтобы `show_free_count` был счётчиком, а не процентом (см. `a040/llm.md`).
