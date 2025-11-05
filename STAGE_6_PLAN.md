# Этап 6: Интеграция в UI, тестирование, мониторинг качества данных

## 📋 Контекст: что уже реализовано (этапы 1-5)

### ✅ Этап 1: Схемы БД

**Созданы таблицы:**

- `document_raw_storage` - хранение сырых JSON из API маркетплейсов
- `p900_sales_register` - унифицированный регистр продаж с 22 полями
- `a010_ozon_fbs_posting` - документы OZON FBS
- `a011_ozon_fbo_posting` - документы OZON FBO
- `a012_wb_sales` - документы Wildberries
- `a013_ym_order` - документы Yandex Market
- **8 индексов** для Sales Register (по датам, организациям, кабинетам, товарам, статусам)

### ✅ Этап 2: Contracts для агрегатов

**Созданы 4 документа-агрегата** в `crates/contracts/src/domain/`:

- a010_ozon_fbs_posting (Header, Lines, State, Monetary, SourceMeta)
- a011_ozon_fbo_posting (аналогично)
- a012_wb_sales (Header, Line, State, SourceMeta)
- a013_ym_order (Header, Lines, State, SourceMeta)

### ✅ Этап 3: Repository и Service

**Реализованы для каждого агрегата:**

- Repository: insert, update, get, list, soft_delete, upsert
- Service: validate, store_document_with_raw (сохраняет raw JSON + проекцию)

### ✅ Этап 4: API Clients - методы получения продаж

**Добавлены методы:**

- `u502_import_from_ozon`: `fetch_fbs_postings()`, `fetch_fbo_postings()`
- `u504_import_from_wildberries`: `fetch_sales()`
- `u503_import_from_yandex`: `fetch_orders()`, `fetch_order_details()`

### ✅ Этап 5: Projection p900

**Реализована автоматическая проекция:**

- `projection_builder.rs`: маппинг из 4 типов документов в Sales Register
- `service.rs`: orchestration проекции при сохранении документа
- `repository.rs`: upsert в p900_sales_register с идемпотентностью

### ✅ Дополнительные улучшения структуры

**Добавлены UUID ссылки:**

- `connection_mp_ref` - на a006_connection_mp (кабинет)
- `organization_ref` - на a002_organization
- `marketplace_product_ref` - на a007_marketplace_product (пока NULL)
- `registrator_ref` - на документ-регистратор (raw JSON)
- `sale_date` - отдельное поле с датой реализации

---

## 🎯 Этап 6: Интеграция в UI, тестирование, мониторинг

### Цели этапа:

1. Создать UI для просмотра регистра продаж
2. Добавить фильтры и группировки
3. Интегрировать вызовы fetch_sales в существующие import flows
4. Реализовать мониторинг качества данных
5. Протестировать end-to-end flow

---

## 📝 План реализации

### 6.1. Contracts: DTO для API

#### 6.1.0. Создать общие DTO структуры

**Файл:** `crates/contracts/src/projections/p900_mp_sales_register/dto.rs`

**Структуры:**

```rust
// Общие DTO, используемые frontend и backend
pub struct SalesRegisterListRequest {
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    pub marketplace: Option<String>,
    pub organization_ref: Option<String>,
    pub connection_mp_ref: Option<String>,
    pub status_norm: Option<String>,
    pub seller_sku: Option<String>,
    pub limit: i32,
    pub offset: i32,
}

pub struct SalesRegisterListResponse {
    pub items: Vec<SalesRegisterDto>,
    pub total_count: i32,
    pub has_more: bool,
}

pub struct SalesRegisterDto {
    pub marketplace: String,
    pub document_no: String,
    pub line_id: String,
    pub sale_date: NaiveDate,
    pub seller_sku: Option<String>,
    pub title: Option<String>,
    pub qty: f64,
    pub amount_line: Option<f64>,
    pub status_norm: String,
    // ... остальные поля
}
```

**Файл:** `crates/contracts/src/projections/p900_mp_sales_register/mod.rs`

```rust
pub mod dto;
pub use dto::*;
```

---

### 6.2. Frontend: UI для Sales Register

#### 6.2.1. Создать компонент SalesRegisterTable

**Файл:** `crates/frontend/src/projections/p900_mp_sales_register/table.rs`

**Функционал:**

- Таблица с продажами из p900_sales_register
- Колонки: дата, маркетплейс, организация, кабинет, товар, кол-во, сумма, статус
- Пагинация (по 50 записей)
- Сортировка по колонкам
- Экспорт в CSV

#### 6.2.2. Создать компонент SalesRegisterFilters

**Файл:** `crates/frontend/src/projections/p900_mp_sales_register/filters.rs`

**Фильтры:**

- Период (от/до) по `sale_date`
- Маркетплейс (OZON/WB/YM)
- Организация (dropdown из a002)
- Кабинет МП (dropdown из a006)
- Статус (dropdown: все, completed, cancelled, etc.)
- Артикул продавца (seller_sku)

#### 6.2.3. Создать компонент SalesRegisterCharts

**Файл:** `crates/frontend/src/projections/p900_mp_sales_register/charts.rs`

**Графики:**

- Динамика продаж по дням (line chart)
- Распределение по маркетплейсам (pie chart)
- Топ-10 товаров по выручке (bar chart)
- Группировка по организациям (bar chart)

**Файл:** `crates/frontend/src/projections/p900_mp_sales_register/mod.rs`

```rust
pub mod table;
pub mod filters;
pub mod charts;

pub use table::SalesRegisterTable;
pub use filters::SalesRegisterFilters;
pub use charts::SalesRegisterCharts;
```

---

### 6.3. Backend: API Endpoints

#### 6.3.1. Создать handler для списка продаж

**Файл:** `crates/backend/src/handlers/sales_register.rs`

**Endpoints:**

```rust
GET /api/sales-register/list
Query params:
- date_from: NaiveDate
- date_to: NaiveDate
- marketplace?: String
- organization_ref?: String
- connection_mp_ref?: String
- status_norm?: String
- seller_sku?: String
- limit: i32 (default 50)
- offset: i32 (default 0)

Response: {
  items: Vec<SalesRegisterEntry>,
  total_count: i32,
  has_more: bool
}
```

#### 6.3.2. Создать handler для агрегированных данных

**Endpoints:**

```rust
GET /api/sales-register/stats/by-date
GET /api/sales-register/stats/by-marketplace
GET /api/sales-register/stats/by-organization
GET /api/sales-register/stats/by-product
```

**Response example (by-date):**

```json
{
  "data": [
    {
      "date": "2025-01-15",
      "sales_count": 150,
      "total_qty": 320,
      "total_revenue": 145000.5
    }
  ]
}
```

#### 6.3.3. Добавить методы в repository

**Файл:** `crates/backend/src/projections/p900_mp_sales_register/repository.rs`

**Новые методы:**

```rust
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
) -> Result<(Vec<Model>, i32)>

pub async fn get_stats_by_date(
    date_from: NaiveDate,
    date_to: NaiveDate,
    marketplace: Option<String>,
) -> Result<Vec<DailyStat>>

pub async fn get_stats_by_marketplace(
    date_from: NaiveDate,
    date_to: NaiveDate,
) -> Result<Vec<MarketplaceStat>>

pub async fn get_stats_by_organization(
    date_from: NaiveDate,
    date_to: NaiveDate,
) -> Result<Vec<OrganizationStat>>
```

---

### 6.4. Интеграция fetch_sales в import flows

#### 6.4.1. Обновить u502_import_from_ozon/executor.rs

**Добавить методы:**

```rust
pub async fn import_fbs_postings(
    connection: &ConnectionMP,
    date_from: NaiveDate,
    date_to: NaiveDate,
) -> Result<ImportSalesResult>

pub async fn import_fbo_postings(
    connection: &ConnectionMP,
    date_from: NaiveDate,
    date_to: NaiveDate,
) -> Result<ImportSalesResult>
```

**Логика:**

1. Вызвать `api_client.fetch_fbs_postings()`
2. Для каждого posting создать OzonFbsPosting aggregate
3. Вызвать `a010_service::store_document_with_raw()` (автоматически проецируется)
4. Вернуть статистику импорта

#### 6.4.2. Обновить u504_import_from_wildberries/executor.rs

**Добавить метод:**

```rust
pub async fn import_sales(
    connection: &ConnectionMP,
    date_from: NaiveDate,
) -> Result<ImportSalesResult>
```

#### 6.4.3. Обновить u503_import_from_yandex/executor.rs

**Добавить метод:**

```rust
pub async fn import_orders(
    connection: &ConnectionMP,
    status: Option<String>,
    updated_from: Option<NaiveDate>,
) -> Result<ImportSalesResult>
```

#### 6.4.4. Создать единый handler для импорта продаж

**Файл:** `crates/backend/src/handlers/import_sales.rs`

**Endpoint:**

```rust
POST /api/import/sales
Body: {
  connection_mp_id: String,
  marketplace: String, // "OZON_FBS", "OZON_FBO", "WB", "YM"
  date_from: String,
  date_to: String
}

Response: {
  success: bool,
  imported_count: i32,
  projected_count: i32,
  errors: Vec<String>
}
```

---

### 6.5. Мониторинг качества данных

#### 6.5.1. Создать service для проверки качества

**Файл:** `crates/backend/src/services/sales_data_quality.rs`

**Проверки:**

```rust
pub struct DataQualityReport {
    pub total_records: i32,
    pub missing_organization_ref: i32,
    pub missing_connection_mp_ref: i32,
    pub missing_marketplace_product_ref: i32,
    pub missing_seller_sku: i32,
    pub negative_amounts: i32,
    pub zero_qty: i32,
    pub future_sale_dates: i32,
    pub duplicate_documents: i32,
}

pub async fn check_data_quality(
    date_from: NaiveDate,
    date_to: NaiveDate,
) -> Result<DataQualityReport>
```

#### 6.5.2. Создать UI для мониторинга

**Компонент:** `DataQualityDashboard`

**Показатели:**

- Общее количество продаж
- Процент записей без ссылок
- Процент сопоставленных товаров
- Список ошибок с возможностью исправления

#### 6.5.3. Добавить логирование в projection

**Обновить:** `crates/backend/src/projections/p900_mp_sales_register/service.rs`

```rust
// Логировать успешные проекции
tracing::info!(
    "Projected {} lines from {} to Sales Register",
    entries.len(),
    document_type
);

// Логировать ошибки с деталями
tracing::error!(
    "Failed to project {}: {} - {:?}",
    document_type,
    document_no,
    error
);
```

---

### 6.6. Тестирование

#### 6.6.1. Unit tests для projection_builder

**Файл:** `crates/backend/src/projections/p900_mp_sales_register/projection_builder.rs`

**Тесты:**

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_from_ozon_fbs_mapping() { }

    #[test]
    fn test_from_ozon_fbo_mapping() { }

    #[test]
    fn test_from_wb_sales_mapping() { }

    #[test]
    fn test_from_ym_order_mapping() { }

    #[test]
    fn test_sale_date_extraction() { }
}
```

#### 6.6.2. Integration tests для end-to-end flow

**Файл:** `crates/backend/tests/sales_register_integration.rs`

**Сценарии:**

1. Import OZON FBS → проверка в Sales Register
2. Import WB → проверка в Sales Register
3. Фильтрация по датам
4. Фильтрация по организации
5. Идемпотентность (повторный импорт тех же данных)

#### 6.6.3. UI E2E tests

**Сценарии:**

1. Открыть Sales Register → загрузка данных
2. Применить фильтры → обновление таблицы
3. Экспорт в CSV
4. Просмотр графиков

---

### 6.7. Автоматическое сопоставление с товарами МП

#### 6.7.1. Создать service для сопоставления

**Файл:** `crates/backend/src/services/product_matching.rs`

**Логика:**

```rust
pub async fn match_sales_to_products(
    date_from: NaiveDate,
    date_to: NaiveDate,
) -> Result<MatchingReport> {
    // 1. Получить записи Sales Register без marketplace_product_ref
    // 2. Для каждой записи:
    //    - Найти a007 по seller_sku + marketplace
    //    - Если найден → обновить marketplace_product_ref
    // 3. Вернуть статистику (matched, unmatched)
}
```

#### 6.7.2. Добавить scheduled job

**Файл:** `crates/backend/src/jobs/product_matching_job.rs`

**Запуск:** Каждые 10 минут или после каждого импорта продаж

---

## 🗂️ Структура файлов для создания

```
crates/
├── contracts/src/
│   └── projections/
│       └── p900_mp_sales_register/
│           ├── mod.rs                  ✨ NEW
│           └── dto.rs                  ✨ NEW (DTO для API)
├── backend/src/
│   ├── handlers/
│   │   ├── sales_register.rs          ✨ NEW
│   │   └── import_sales.rs            ✨ NEW
│   ├── services/
│   │   ├── sales_data_quality.rs      ✨ NEW
│   │   └── product_matching.rs        ✨ NEW
│   ├── jobs/
│   │   └── product_matching_job.rs    ✨ NEW
│   └── tests/
│       └── sales_register_integration.rs  ✨ NEW
└── frontend/src/
    └── projections/
        └── p900_mp_sales_register/
            ├── mod.rs                  ✨ NEW
            ├── table.rs                ✨ NEW
            ├── filters.rs              ✨ NEW
            └── charts.rs               ✨ NEW
```

---

## 📊 Критерии готовности этапа 6

### Backend:

- ✅ 4 API endpoints для списка и статистики
- ✅ Методы list_with_filters в repository
- ✅ Интеграция fetch_sales в import flows
- ✅ Service для проверки качества данных
- ✅ Service для автоматического сопоставления
- ✅ Unit tests (покрытие >80%)
- ✅ Integration tests

### Frontend:

- ✅ Компонент SalesRegisterTable (в `projections/p900_mp_sales_register/`)
- ✅ Фильтры и поиск
- ✅ Графики и визуализация
- ✅ Экспорт в CSV
- ✅ Data Quality Dashboard

### Contracts:

- ✅ DTO для Sales Register API (в `contracts/src/projections/p900_mp_sales_register/`)

### Функциональность:

- ✅ End-to-end импорт: API → Document → Projection → UI
- ✅ Фильтрация по всем ключевым полям
- ✅ Автоматическое сопоставление товаров
- ✅ Мониторинг качества данных
- ✅ Все маркетплейсы работают (OZON/WB/YM)

---

## 🔧 Технические детали

### Используемые технологии:

- **Backend:** Rust, Axum, Sea-ORM, SQLite
- **Frontend:** Leptos (Rust WASM)
- **Графики:** plotly.rs или charming
- **CSV Export:** csv crate

### Существующие зависимости:

- `a002_organization` - организации
- `a006_connection_mp` - кабинеты маркетплейсов
- `a007_marketplace_product` - товары МП
- `u502/u503/u504` - usecases импорта

---

## 📅 Примерная оценка времени

| Задача                      | Время     |
| --------------------------- | --------- |
| 6.1 Contracts DTO           | 1-2 часа  |
| 6.2 Frontend UI             | 6-8 часов |
| 6.3 Backend API             | 4-6 часов |
| 6.4 Интеграция import flows | 3-4 часа  |
| 6.5 Мониторинг качества     | 2-3 часа  |
| 6.6 Тестирование            | 4-5 часов |
| 6.7 Автосопоставление       | 2-3 часа  |

**Итого:** 22-31 часов (3-4 рабочих дня)

---

## 💡 Рекомендации для нового чата

1. **Начать с Contracts DTO** (6.1) - создать общие структуры для API
2. **Backend API** (6.3) - создать endpoints и repository methods
3. **Интеграция** (6.4) - подключить fetch_sales к import flows
4. **Тестирование backend** (6.6.1, 6.6.2) - убедиться что всё работает
5. **Frontend UI** (6.2) - создать компоненты для отображения
6. **Мониторинг** (6.5) - добавить проверки качества
7. **Автосопоставление** (6.7) - реализовать связь с товарами

---

## 📚 Полезные ссылки на существующий код

**Repository pattern:**

- `crates/backend/src/domain/a006_connection_mp/repository.rs`

**Handler pattern:**

- `crates/backend/src/handlers/connection_mp.rs`

**Frontend projection pattern:**

- `crates/frontend/src/projections/` (аналогично backend)

**Contracts DTO pattern:**

- `crates/contracts/src/projections/` (общие структуры для frontend/backend)

**Import executor pattern:**

- `crates/backend/src/usecases/u502_import_from_ozon/executor.rs`

---

## ✅ Текущий статус (после этапов 1-5)

✅ Backend полностью реализован  
✅ Projection работает автоматически  
✅ API clients могут получать данные  
✅ Структура БД оптимизирована  
✅ Компиляция без ошибок

**Готово к этапу 6!** 🚀
