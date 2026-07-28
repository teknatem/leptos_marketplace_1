# Wildberries — диагностический playbook воронки

Таблица «симптом → вероятная причина → чем подтвердить». Все SQL — один SELECT по p916/исходникам
(`execute_query`, bind `?`). Термины и поля — из `wildberries-mapping.md`.

## Триаж по стадиям (где просадка)

Сначала посчитай конверсии на чтении и найди наименьшую относительно нормы кабинета:

| Конверсия | Формула | Куда копать при просадке |
|---|---|---|
| `open_to_cart` | `cart_count / open_count` | карточка/контент/цена: трафик есть, корзина не растёт |
| `cart_to_order` | `order_count / cart_count` | цена/наличие/сроки доставки на этапе оформления |
| `order_to_buyout` | `buyout_count / order_count` | качество/логистика/размер: выкуп низкий |
| `cancel_rate` | `cancel_count / order_count` | отмены (наличие, дубли, длинная доставка) |

Наибольший `drop_off` (=`1 − conversion_from_previous`) — основной кандидат на диагностику.

## Симптомы и причины

| Симптом | Вероятная причина | Подтверждение |
|---|---|---|
| Верхние этапы `N/A`, а не `0` | нет источника показов: нет подписки «Джем» (a040 всё равно не питает воронку) и/или нет рекламы a026 | проверь наличие marketing-строк с `show_paid_count IS NOT NULL` (флаг доступности); нет — верхние показы недоступны |
| `funnel_order_count` ≫ `order_count` | это РАЗНЫЕ числа: маркетинговый счётчик воронки (a036) vs фактические заказы (a015) | не баг; сравнивай заказы только по `order_count` |
| Цифры «скачут» между запросами | перепутаны оси: `cohort_date` vs `event_date` | зафиксируй ось; когорта — по дате заказа, поток — по дате транзакции |
| Числа не сходятся с d406 | другой канал/ось/период, либо d406 применил фильтр Платные/Бесплатные | воспроизведи фильтр d406 (см. paid/free шаблон в mapping) на тех же датах/кабинете |
| Нижний этап больше верхнего (немонотонность) | стадии из разных источников (a036 vs a015/a012) склеены | пометь `non_monotonic_funnel`; не интерпретируй как ошибку данных без проверки источников |
| Рост показов/переходов при падении заказов | ухудшение конверсии на нижних этапах | посчитай `cart_to_order`/`order_to_buyout` за оба периода |
| Заказ «не попал в платные» | p913-членство = только заказы со **значимой** долей рекламных затрат (`is_allocated && is_significant_amount`) | проверь вхождение `srid` в p913 (см. ниже) |
| Выкуп «пропал»/привязан не к той дате | у a012 нет даты заказа; когорта резолвится по `srid → a015.order_dt`, при отсутствии заказа — фолбэк на дату продажи | сверь `cohort_date` строки выкупа с датой заказа в a015 |

## SQL для расследования

**Доступность показов/рекламы в срезе:**

```sql
SELECT SUM(CASE WHEN show_paid_count IS NOT NULL THEN 1 ELSE 0 END) AS paid_shows_rows,
       SUM(CASE WHEN show_free_count IS NOT NULL THEN 1 ELSE 0 END) AS free_shows_rows
FROM p916_mp_sales_funnel_turnovers
WHERE stage = 'marketing'
  AND cohort_date BETWEEN ? AND ?
  AND connection_mp_ref = ?
```

**Разрыв маркетинговых и фактических заказов по товару:**

```sql
SELECT nm_id,
       SUM(COALESCE(funnel_order_count,0)) AS funnel_orders,
       SUM(COALESCE(order_count,0))        AS real_orders
FROM p916_mp_sales_funnel_turnovers
WHERE cohort_date BETWEEN ? AND ? AND connection_mp_ref = ?
GROUP BY nm_id
HAVING funnel_orders <> real_orders
ORDER BY ABS(funnel_orders - real_orders) DESC
```

**Строки p916 по конкретному заказу (srid) + канал:**

```sql
SELECT f.stage, f.registrator_type, f.cohort_date, f.event_date,
       f.order_count, f.cancel_count, f.buyout_count, f.return_count,
       CASE WHEN pj.order_key IS NOT NULL THEN 'paid' ELSE 'free' END AS channel
FROM p916_mp_sales_funnel_turnovers f
LEFT JOIN (
  SELECT DISTINCT order_key FROM p913_wb_advert_order_attr
  WHERE turnover_code = 'advert_clicks_order_accrual'
) pj ON pj.order_key = f.order_key
WHERE f.order_key = ?
ORDER BY f.event_date
```

Дальше — исходники: `a015_wb_orders` (заказ/отмена) и `a012_wb_sales` (выкуп/возврат) по тому же
srid для первичных полей.

## Правила интерпретации

- Отсутствующий этап (`NULL`/нет строки) — это **нет данных**, а не ноль. Не считай конверсию
  через отсутствующий этап.
- Сравнение периодов — только при одинаковом наборе фильтров (кабинет/ось/канал) и разных labels.
- Сегменты с предупреждениями качества не ставь в рейтинг без оговорки.
- Всегда указывай, какую ось и канал использовал в ответе.
