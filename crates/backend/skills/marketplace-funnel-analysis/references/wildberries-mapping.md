# Wildberries mapping

| Wildberries | Каноническое поле |
|---|---|
| `openCardCount`, `views` | `product_views` |
| `addToCartCount`, `add_to_cart` | `cart_adds` |
| `ordersCount`, `orders` | `orders` |
| `buyoutsCount`, `buyouts` | `deliveries` |
| `ordersSumRub`, `revenue` | `revenue` |

У WB рекламные impressions/clicks могут приходить из другого отчёта. Если их нет,
воронка начинается с `product_views`; это не ошибка.
