# Aggregate a008_marketplace_sales

## Описание

Агрегат для хранения данных по продажам с маркетплейсов. Записи уникальны по связке: подключение (a006), позиция (a007), дата начисления (NaiveDate).

## Поля

- connection_id: String (a006_connection_mp)
- organization_id: String (a002_organization)
- marketplace_id: String (a005_marketplace)
- accrual_date: NaiveDate (YYYY-MM-DD)
- product_id: String (a007_marketplace_product)
- quantity: i32
- revenue: f64
- operation_type: String (тип операции источника, например sale/return/commission)

## Таблица

`a008_marketplace_sales` создаётся автоматически при инициализации БД.

```sql
CREATE TABLE a008_marketplace_sales (
  id TEXT PRIMARY KEY NOT NULL,
  code TEXT NOT NULL DEFAULT '',
  description TEXT NOT NULL,
  comment TEXT,
  connection_id TEXT NOT NULL,
  organization_id TEXT NOT NULL,
  marketplace_id TEXT NOT NULL,
  accrual_date TEXT NOT NULL,
  product_id TEXT NOT NULL,
  quantity INTEGER NOT NULL,
  revenue REAL NOT NULL,
  operation_type TEXT NOT NULL DEFAULT '',
  is_deleted INTEGER NOT NULL DEFAULT 0,
  is_posted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT,
  updated_at TEXT,
  version INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_a008_sales_unique
  ON a008_marketplace_sales (connection_id, product_id, accrual_date, operation_type);
```

## REST API

- GET `/api/marketplace_sales` — список
- GET `/api/marketplace_sales/:id` — по ID
- POST `/api/marketplace_sales` — создать/обновить (upsert) по DTO
- DELETE `/api/marketplace_sales/:id` — мягкое удаление

## Frontend

- Компонент списка: `crates/frontend/src/domain/a008_marketplace_sales/ui/list/mod.rs`
- Навигация: ключ вкладки `a008_marketplace_sales`
