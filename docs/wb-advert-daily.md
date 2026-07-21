## Реклама (дневная): `GET /api/ext/v1/wb-advert-daily`

Дневные показатели рекламы Wildberries (`a026_wb_advert_daily`) — одна строка на
кампанию × товар × дата. Источник — `repository::product_rows_for_period`.

| Параметр        | Обяз. | Описание                                                |
|-----------------|-------|---------------------------------------------------------|
| `date_from`     | да    | Начало периода, `YYYY-MM-DD`                            |
| `date_to`       | да    | Конец периода, `YYYY-MM-DD` (включительно)              |
| `connection_id` | нет   | Фильтр по кабинету WB (UUID). Пусто → все кабинеты      |
| `limit`         | нет   | Макс. строк (по умолчанию 5000, потолок 50000)          |
| `offset`        | нет   | Смещение для постраничной выгрузки                      |

Без `date_from`/`date_to` → `400`.

Пример ответа:

```json
{
  "items": [
    {
      "date": "2026-07-14",
      "connection_id": "1386a311-…",
      "connection_name": "WB - SANSTAR",
      "organization_name": "ООО …",
      "advert_id": 12345678,
      "nm_id": 316700126,
      "nm_name": "…",
      "nomenclature_ref": "…",
      "app_types": [1, 32],
      "placements": ["recom", "search"],
      "views": 1420,
      "clicks": 37,
      "ctr": 2.6,
      "cpc": 8.1,
      "atbs": 12,
      "orders": 4,
      "shks": 5,
      "sum": 300.0,
      "sum_price": 5990.0,
      "cr": 10.8,
      "canceled": 1
    }
  ],
  "total": 2740
}
```

Метрики: `views`, `clicks`, `ctr`, `cpc` (воронка показов) → `atbs`, `orders`, `shks`,
`cr`, `canceled` (корзина/заказы) → `sum` (расход на рекламу), `sum_price` (сумма заказов).
`total` — общее число строк за период (до пагинации).

```powershell
curl.exe -H "X-Api-Key: <КЛЮЧ>" `
  "http://localhost:3000/api/ext/v1/wb-advert-daily?date_from=2026-07-08&date_to=2026-07-14"
```