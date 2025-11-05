# 🚀 Quick Start: Этап 6 - Интеграция в UI

## 📌 Краткая суммаризация

### Что уже сделано (этапы 1-5):

✅ **БД:** Создана таблица `p900_sales_register` с 22 полями + 8 индексов  
✅ **Агрегаты:** 4 документа (a010-a013) для OZON FBS/FBO, WB, YM  
✅ **Repository:** Полный CRUD + upsert для всех агрегатов  
✅ **Service:** `store_document_with_raw()` + автоматическая проекция  
✅ **API Clients:** Методы fetch_sales для всех маркетплейсов  
✅ **Projection:** Автоматический маппинг документов → Sales Register

### Ключевые поля Sales Register:

- **UUID ссылки:** `connection_mp_ref`, `organization_ref`, `marketplace_product_ref`, `registrator_ref`
- **Даты:** `event_time_source`, `sale_date` (отдельное поле!)
- **Товар:** `seller_sku`, `mp_item_id`, `barcode`, `title`
- **Деньги:** `qty`, `price_list`, `discount_total`, `price_effective`, `amount_line`
- **Статусы:** `status_source`, `status_norm`

---

## 🎯 Что нужно сделать в этапе 6

### Приоритет 1: Backend API (начать с этого!)

```
1. Создать handlers/sales_register.rs
   - GET /api/sales-register/list (с фильтрами)
   - GET /api/sales-register/stats/by-date
   - GET /api/sales-register/stats/by-marketplace

2. Добавить методы в repository.rs
   - list_with_filters()
   - get_stats_by_date()
   - get_stats_by_marketplace()

3. Создать handlers/import_sales.rs
   - POST /api/import/sales (единый endpoint для всех МП)
```

### Приоритет 2: Интеграция import flows

```
4. Обновить u502/executor.rs - добавить import_fbs_postings()
5. Обновить u504/executor.rs - добавить import_sales()
6. Обновить u503/executor.rs - добавить import_orders()
```

### Приоритет 3: Frontend UI

```
7. Создать projections/p900_mp_sales_register/table.rs
8. Создать projections/p900_mp_sales_register/filters.rs
9. Добавить роутинг и страницу /sales-register
```

### Приоритет 4: Мониторинг и автосопоставление

```
10. Создать services/sales_data_quality.rs
11. Создать services/product_matching.rs
12. Добавить scheduled job для автосопоставления
```

---

## 💻 Примеры кода для старта

### 1. Handler для списка продаж

```rust
// crates/backend/src/handlers/sales_register.rs
use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;

#[derive(Debug, Deserialize)]
pub struct SalesListQuery {
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    pub marketplace: Option<String>,
    pub organization_ref: Option<String>,
    pub connection_mp_ref: Option<String>,
    pub status_norm: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SalesListResponse {
    pub items: Vec<SalesRegisterEntry>,
    pub total_count: i32,
    pub has_more: bool,
}

pub async fn list_sales(
    Query(query): Query<SalesListQuery>,
) -> Result<Json<SalesListResponse>, AppError> {
    let (items, total) = repository::list_with_filters(
        query.date_from,
        query.date_to,
        query.marketplace,
        query.organization_ref,
        query.connection_mp_ref,
        query.status_norm,
        None, // seller_sku
        query.limit.unwrap_or(50),
        query.offset.unwrap_or(0),
    ).await?;

    Ok(Json(SalesListResponse {
        has_more: total > (query.offset.unwrap_or(0) + items.len() as i32),
        total_count: total,
        items,
    }))
}
```

### 2. Repository method с фильтрами

```rust
// crates/backend/src/projections/p900_mp_sales_register/repository.rs
pub async fn list_with_filters(
    date_from: NaiveDate,
    date_to: NaiveDate,
    marketplace: Option<String>,
    organization_ref: Option<String>,
    connection_mp_ref: Option<String>,
    status_norm: Option<String>,
    seller_sku: Option<String>,
    limit: i32,
    offset: i32,
) -> Result<(Vec<Model>, i32)> {
    let mut query = Entity::find()
        .filter(Column::SaleDate.gte(date_from.format("%Y-%m-%d").to_string()))
        .filter(Column::SaleDate.lte(date_to.format("%Y-%m-%d").to_string()));

    if let Some(mp) = marketplace {
        query = query.filter(Column::Marketplace.eq(mp));
    }
    if let Some(org) = organization_ref {
        query = query.filter(Column::OrganizationRef.eq(org));
    }
    if let Some(conn) = connection_mp_ref {
        query = query.filter(Column::ConnectionMpRef.eq(conn));
    }
    if let Some(status) = status_norm {
        query = query.filter(Column::StatusNorm.eq(status));
    }
    if let Some(sku) = seller_sku {
        query = query.filter(Column::SellerSku.eq(sku));
    }

    // Count total
    let total = query.clone().count(conn()).await? as i32;

    // Get page
    let items = query
        .order_by_desc(Column::SaleDate)
        .limit(limit as u64)
        .offset(offset as u64)
        .all(conn())
        .await?;

    Ok((items, total))
}
```

### 3. Import executor для OZON FBS

```rust
// crates/backend/src/usecases/u502_import_from_ozon/executor.rs
use crate::domain::a010_ozon_fbs_posting::service as a010_service;

pub async fn import_fbs_postings(
    connection: &ConnectionMP,
    date_from: NaiveDate,
    date_to: NaiveDate,
) -> Result<ImportSalesResult> {
    let api_client = OzonApiClient::new();

    let mut imported = 0;
    let mut errors = Vec::new();
    let mut offset = 0;
    const LIMIT: i32 = 100;

    loop {
        let response = api_client.fetch_fbs_postings(
            connection,
            date_from,
            date_to,
            LIMIT,
            offset,
        ).await?;

        if response.postings.is_empty() {
            break;
        }

        for posting_json in response.postings {
            let raw_json = serde_json::to_string(&posting_json)?;

            // Создать OzonFbsPosting aggregate
            let document = map_ozon_posting_to_aggregate(posting_json, connection)?;

            // Сохранить (автоматически проецируется в Sales Register)
            match a010_service::store_document_with_raw(document, &raw_json).await {
                Ok(_) => imported += 1,
                Err(e) => errors.push(format!("Error: {}", e)),
            }
        }

        if !response.has_next {
            break;
        }
        offset += LIMIT;
    }

    Ok(ImportSalesResult {
        success: true,
        imported_count: imported,
        projected_count: imported, // 1:1 для FBS
        errors,
    })
}
```

---

## 📁 Файлы для использования из проекта

### Эталонные примеры:

- **Repository:** `crates/backend/src/domain/a006_connection_mp/repository.rs`
- **Handler:** `crates/backend/src/handlers/connection_mp.rs`
- **Executor:** `crates/backend/src/usecases/u502_import_from_ozon/executor.rs`
- **Frontend Projection:** `crates/frontend/src/projections/` (аналогично backend)
- **Contracts DTO:** `crates/contracts/src/projections/` (общие структуры)

### Основные модули для расширения:

- `crates/backend/src/projections/p900_mp_sales_register/repository.rs` - добавить методы
- `crates/backend/src/projections/p900_mp_sales_register/service.rs` - если нужна доп. логика
- `crates/backend/src/usecases/u502_import_from_ozon/executor.rs` - добавить import методы
- `crates/backend/src/usecases/u503_import_from_yandex/executor.rs` - добавить import методы
- `crates/backend/src/usecases/u504_import_from_wildberries/executor.rs` - добавить import методы

---

## ⚡ Быстрый старт (пошагово)

```bash
# 1. Начать с backend API
cursor "Создай handlers/sales_register.rs с endpoint GET /api/sales-register/list"

# 2. Добавить метод в repository
cursor "Добавь метод list_with_filters в p900_mp_sales_register/repository.rs"

# 3. Интегрировать в import flow
cursor "Добавь метод import_fbs_postings в u502_import_from_ozon/executor.rs"

# 4. Протестировать
cursor "Создай integration test для импорта OZON FBS → проверка в Sales Register"

# 5. Frontend
cursor "Создай компонент SalesRegisterTable в frontend/projections/p900_mp_sales_register/"
```

---

## 🔍 Проверочный чек-лист

### Backend готов, если:

- [ ] GET /api/sales-register/list возвращает данные с фильтрами
- [ ] GET /api/sales-register/stats/by-date возвращает агрегаты
- [ ] POST /api/import/sales импортирует данные для всех МП
- [ ] После импорта данные появляются в p900_sales_register
- [ ] Unit tests проходят
- [ ] Integration tests проходят

### Frontend готов, если:

- [ ] Таблица показывает продажи из Sales Register
- [ ] Фильтры работают (даты, МП, организация, статус)
- [ ] Можно экспортировать в CSV
- [ ] Графики отображают динамику продаж
- [ ] UI responsive и удобен

### Система работает end-to-end, если:

- [ ] Импорт из OZON → данные в UI
- [ ] Импорт из WB → данные в UI
- [ ] Импорт из YM → данные в UI
- [ ] Автосопоставление заполняет marketplace_product_ref
- [ ] Data Quality Dashboard показывает метрики

---

## 📚 Документация из этапов 1-5

1. `SALES_REGISTER_IMPLEMENTATION_SUMMARY.md` - итоги этапов 1-5
2. `SALES_REGISTER_STRUCTURE_IMPROVEMENTS.md` - детали структуры
3. `SALES_REGISTER_STRUCTURE_BEFORE_AFTER.md` - сравнение ДО/ПОСЛЕ
4. `STAGE_6_PLAN.md` - полный план этапа 6 (этот файл)

---

## 🎯 Главное

**Цель:** Создать полный end-to-end flow:

```
API МП → fetch_sales → Document Aggregate → store_with_raw →
→ Projection → p900_sales_register → Backend API → Frontend UI
```

**Текущий статус:** Backend готов, Projection работает, осталось UI + интеграция

**Время:** 3-4 рабочих дня

**Начать с:** Backend API handlers + repository methods ✨

---

Удачи! 🚀
