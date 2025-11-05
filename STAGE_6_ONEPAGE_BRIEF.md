# 📄 Этап 6: One-Page Brief

## ✅ Готово (этапы 1-5)

- **БД:** `p900_sales_register` (22 поля, 8 индексов), 4 таблицы агрегатов (a010-a013)
- **Backend:** Repository, Service, Projection для OZON FBS/FBO, WB, YM
- **API Clients:** fetch_fbs_postings, fetch_fbo_postings, fetch_sales, fetch_orders
- **Автопроекция:** Документ → `store_document_with_raw()` → Sales Register

## 🎯 Задача этапа 6

Создать **UI + интеграция в import flows + мониторинг**

## 📝 План (6 шагов)

### 1️⃣ Backend API (4-6 часов)

```
📂 handlers/sales_register.rs
  GET /api/sales-register/list (фильтры: даты, МП, орг, кабинет, статус)
  GET /api/sales-register/stats/by-date
  GET /api/sales-register/stats/by-marketplace

📂 projections/p900_mp_sales_register/repository.rs
  + list_with_filters()
  + get_stats_by_date()
  + get_stats_by_marketplace()

📂 handlers/import_sales.rs
  POST /api/import/sales (connection_id, marketplace, dates)
```

### 2️⃣ Интеграция import flows (3-4 часа)

```
📂 usecases/u502_import_from_ozon/executor.rs
  + import_fbs_postings() → вызывает fetch + store_document_with_raw

📂 usecases/u504_import_from_wildberries/executor.rs
  + import_sales()

📂 usecases/u503_import_from_yandex/executor.rs
  + import_orders()
```

### 3️⃣ Frontend UI (6-8 часов)

```
📂 projections/p900_mp_sales_register/
  table.rs - таблица с продажами
  filters.rs - фильтры (даты, МП, орг, статус)
  charts.rs - графики (динамика, pie charts)

📂 contracts/src/projections/p900_mp_sales_register/
  dto.rs - общие DTO для API

+ Роутинг /sales-register
+ Экспорт в CSV
```

### 4️⃣ Мониторинг качества (2-3 часа)

```
📂 services/sales_data_quality.rs
  check_data_quality() → DataQualityReport
  - missing refs, negative amounts, duplicates

📂 components/data_quality_dashboard.rs
  UI для отображения метрик качества
```

### 5️⃣ Автосопоставление товаров (2-3 часа)

```
📂 services/product_matching.rs
  match_sales_to_products() → MatchingReport
  Логика: seller_sku + marketplace → a007 → обновить marketplace_product_ref

📂 jobs/product_matching_job.rs
  Scheduled: каждые 10 мин или после импорта
```

### 6️⃣ Тестирование (4-5 часов)

```
tests/projection_builder_test.rs - unit tests для маппинга
tests/sales_register_integration.rs - end-to-end:
  - Import OZON → проверка в Sales Register
  - Import WB → проверка
  - Фильтры работают
  - Идемпотентность
```

---

## 💻 Код-примеры

### Repository method

```rust
pub async fn list_with_filters(
    date_from: NaiveDate, date_to: NaiveDate,
    marketplace: Option<String>,
    organization_ref: Option<String>,
    limit: i32, offset: i32,
) -> Result<(Vec<Model>, i32)> {
    let mut query = Entity::find()
        .filter(Column::SaleDate.gte(date_from.to_string()))
        .filter(Column::SaleDate.lte(date_to.to_string()));
    if let Some(mp) = marketplace {
        query = query.filter(Column::Marketplace.eq(mp));
    }
    // ... остальные фильтры
    let total = query.clone().count(conn()).await? as i32;
    let items = query.order_by_desc(Column::SaleDate)
        .limit(limit as u64).offset(offset as u64).all(conn()).await?;
    Ok((items, total))
}
```

### Import executor

```rust
pub async fn import_fbs_postings(
    connection: &ConnectionMP,
    date_from: NaiveDate, date_to: NaiveDate,
) -> Result<ImportSalesResult> {
    let api_client = OzonApiClient::new();
    let response = api_client.fetch_fbs_postings(connection, date_from, date_to, 100, 0).await?;
    for posting_json in response.postings {
        let raw = serde_json::to_string(&posting_json)?;
        let doc = map_to_aggregate(posting_json, connection)?;
        a010_service::store_document_with_raw(doc, &raw).await?; // ← auto-projects!
    }
    Ok(ImportSalesResult { imported: response.postings.len(), ... })
}
```

---

## 🏁 Критерии готовности

- [ ] **Backend:** 3+ endpoints работают, фильтры работают
- [ ] **Import:** Все 4 МП импортируют → данные в Sales Register
- [ ] **Frontend:** Таблица + фильтры + графики
- [ ] **Качество:** Dashboard показывает метрики
- [ ] **Тесты:** Unit + Integration проходят
- [ ] **Автосопоставление:** marketplace_product_ref заполняется

---

## 🚀 Старт

**Начать с:** `handlers/sales_register.rs` + `repository::list_with_filters()`  
**Проверить:** `cargo check` → `cargo test` → запустить frontend  
**Время:** 21-29 часов (3-4 дня)

**Готово к реализации!** ⚡
