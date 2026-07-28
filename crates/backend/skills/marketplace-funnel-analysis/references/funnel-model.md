# Каноническая модель воронки

Этапы расположены в порядке:

`impressions → clicks → product_views → cart_adds → orders → deliveries`.

Обязательные поля: `orders`. Остальные этапы могут отсутствовать, но должны быть
неотрицательными числами. `revenue` является денежным показателем и не участвует
в расчёте конверсии.

Для каждого доступного этапа рассчитываются:

- `conversion_from_previous = value / previous_value`;
- `conversion_from_start = value / first_available_value`;
- `drop_off = 1 - conversion_from_previous`.

Доли возвращаются числами от 0 до 1, без округления для отображения.

## Соответствие каноническим этапам для Wildberries

Прежде чем подавать WB-данные в задачи `calculate-funnel`/`compare-*`, приведи реальные поля
p916 к каноническим этапам (детали и SQL — `wildberries-mapping.md`):

| Канонический этап | WB (поле p916) |
|---|---|
| `impressions` | `show_free_count + show_paid_count`, только если доступны обе компоненты; иначе общий этап `N/A` |
| `clicks` | `paid_open_count` (платные переходы) — часто отсутствует, это не ошибка |
| `product_views` | `open_count` (переходы в карточку) |
| `cart_adds` | `cart_count` |
| `orders` | `order_count` (**фактические** заказы a015, не `funnel_order_count`) |
| `deliveries` | `buyout_count` (выкупы) |
| `revenue` | `order_sum` |

WB-специфичные ветки (отмены, возвраты, канал paid/free) в generic-модели отсутствуют — для них
используй задачу `wb-funnel`. Отсутствующий этап (`N/A`) не заменяй нулём.
