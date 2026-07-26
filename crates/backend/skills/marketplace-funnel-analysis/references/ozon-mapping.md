# OZON mapping

| OZON | Каноническое поле |
|---|---|
| `impressions`, `hits_view` | `impressions` |
| `clicks`, `hits_tocart` | `clicks` / `cart_adds` по смыслу источника |
| `product_views`, `hits_view_pdp` | `product_views` |
| `cart_adds`, `hits_tocart` | `cart_adds` |
| `orders`, `ordered_units` | `orders` |
| `deliveries`, `delivered_units` | `deliveries` |
| `revenue`, `ordered_amount` | `revenue` |

Не смешивать `hits_tocart` с clicks, если источник уже содержит отдельный этап clicks.
