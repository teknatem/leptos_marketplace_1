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
