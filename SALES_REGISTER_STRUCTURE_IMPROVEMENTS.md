# Sales Register - Улучшения структуры

## Выполненные изменения

### 1. ✅ Добавлены UUID ссылки на агрегаты

**Новые поля в таблице `p900_sales_register`:**

- **`connection_mp_ref`** (TEXT NOT NULL) - UUID ссылка на a006_connection_mp (кабинет маркетплейса)
- **`organization_ref`** (TEXT NOT NULL) - UUID ссылка на a002_organization (организация)
- **`marketplace_product_ref`** (TEXT, nullable) - UUID ссылка на a007_marketplace_product (товар МП)

**Преимущества:**
- Прямая связь с агрегатами без JOIN через строковые поля
- Возможность быстрой группировки и фильтрации по организациям и кабинетам
- Подготовка к автоматическому сопоставлению с товарами МП (a007)

### 2. ✅ Добавлено поле даты реализации

**Новое поле:**
- **`sale_date`** (TEXT NOT NULL, формат: YYYY-MM-DD) - дата реализации товара (без времени)

**Преимущества:**
- Быстрая группировка по датам для отчетов
- Удобная фильтрация по периодам продаж
- Отдельное поле для бизнес-логики (без времени)
- Индекс для быстрых запросов по датам

**Заполнение:**
- Извлекается из `event_time_source.date_naive()`
- OZON: из `delivered_at`
- Wildberries: из `sale_dt`
- Yandex Market: из `status_changed_at` (когда DELIVERED)

### 3. ✅ Переименованы поля с единым суффиксом _ref

**Изменения имен:**
- `source_ref` → **`registrator_ref`** - ссылка на документ-регистратор (raw JSON)

**Консистентность именования:**
Все ссылки на агрегаты теперь имеют суффикс `_ref`:
- `connection_mp_ref` - ссылка на кабинет
- `organization_ref` - ссылка на организацию
- `marketplace_product_ref` - ссылка на товар МП
- `registrator_ref` - ссылка на сырой JSON документа

### 4. ✅ Улучшена структура таблицы БД

**Добавлены комментарии в SQL:**
```sql
CREATE TABLE p900_sales_register (
    -- NK (Natural Key)
    marketplace TEXT NOT NULL,
    document_no TEXT NOT NULL,
    line_id TEXT NOT NULL,
    
    -- Metadata
    scheme TEXT,
    document_type TEXT NOT NULL,
    document_version INTEGER NOT NULL DEFAULT 1,
    
    -- References to aggregates (UUID)
    connection_mp_ref TEXT NOT NULL,
    organization_ref TEXT NOT NULL,
    marketplace_product_ref TEXT,
    registrator_ref TEXT NOT NULL,
    
    -- Timestamps and status
    event_time_source TEXT NOT NULL,
    sale_date TEXT NOT NULL,
    source_updated_at TEXT,
    status_source TEXT NOT NULL,
    status_norm TEXT NOT NULL,
    
    -- Product identification
    seller_sku TEXT,
    mp_item_id TEXT NOT NULL,
    barcode TEXT,
    title TEXT,
    
    -- Quantities and money
    qty REAL NOT NULL,
    price_list REAL,
    discount_total REAL,
    price_effective REAL,
    amount_line REAL,
    currency_code TEXT,
    
    -- Technical fields
    loaded_at_utc TEXT NOT NULL,
    payload_version INTEGER NOT NULL DEFAULT 1,
    extra TEXT,
    
    PRIMARY KEY (marketplace, document_no, line_id)
);
```

### 5. ✅ Добавлены новые индексы

**Обновленный набор индексов:**

1. **`idx_sales_register_sale_date`** - по дате реализации (для отчетов)
2. **`idx_sales_register_event_time`** - по времени события (сортировка)
3. **`idx_sales_register_connection_mp`** - по кабинету (группировка)
4. **`idx_sales_register_organization`** - по организации (группировка)
5. **`idx_sales_register_product`** - по товару МП (связь с a007)
6. **`idx_sales_register_seller_sku`** - по артикулу продавца
7. **`idx_sales_register_mp_item_id`** - по ID товара на МП
8. **`idx_sales_register_status_norm`** - по статусу

**Итого: 8 индексов** для быстрого доступа ко всем ключевым полям

### 6. ✅ Обновлены структуры в коде

**Обновлен `SalesRegisterEntry` (repository.rs):**
```rust
pub struct SalesRegisterEntry {
    // NK
    pub marketplace: String,
    pub document_no: String,
    pub line_id: String,
    
    // Metadata
    pub scheme: Option<String>,
    pub document_type: String,
    pub document_version: i32,
    
    // References to aggregates (UUID as String)
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub marketplace_product_ref: Option<String>,
    pub registrator_ref: String,
    
    // Timestamps and status
    pub event_time_source: DateTime<Utc>,
    pub sale_date: chrono::NaiveDate,
    pub source_updated_at: Option<DateTime<Utc>>,
    pub status_source: String,
    pub status_norm: String,
    
    // ... остальные поля
}
```

### 7. ✅ Обновлен маппинг в projection_builder

**Все 4 функции обновлены:**

```rust
// OZON FBS/FBO
connection_mp_ref: document.header.connection_id.clone(),
organization_ref: document.header.organization_id.clone(),
marketplace_product_ref: None, // TODO: заполняется при сопоставлении
registrator_ref: document.source_meta.raw_payload_ref.clone(),
sale_date: event_time.date_naive(),

// Wildberries
connection_mp_ref: document.header.connection_id.clone(),
organization_ref: document.header.organization_id.clone(),
marketplace_product_ref: None,
registrator_ref: document.source_meta.raw_payload_ref.clone(),
sale_date: event_time.date_naive(),

// Yandex Market  
connection_mp_ref: document.header.connection_id.clone(),
organization_ref: document.header.organization_id.clone(),
marketplace_product_ref: None,
registrator_ref: document.source_meta.raw_payload_ref.clone(),
sale_date: event_time.date_naive(),
```

---

## Дополнительные улучшения

### Группировка полей
Поля в таблице логически сгруппированы с комментариями:
- NK (Natural Key)
- Metadata
- References to aggregates
- Timestamps and status
- Product identification
- Quantities and money
- Technical fields

### Консистентность именования
- Все ссылки на UUID: суффикс `_ref`
- Все временные поля: суффикс `_at` или `_date`
- Все поля статусов: префикс `status_`

### TODO для будущей реализации

**marketplace_product_ref** помечен как NULL с комментарием:
```rust
marketplace_product_ref: None, // TODO: должно заполняться при сопоставлении с a007
```

**Будущая функциональность:**
1. После импорта продаж запустить процесс сопоставления
2. По `seller_sku` или `mp_item_id` + `marketplace` найти соответствующий a007_marketplace_product
3. Обновить `marketplace_product_ref` в Sales Register
4. Это позволит строить отчеты с полной информацией о товарах

---

## Преимущества новой структуры

### Производительность
✅ 8 индексов для быстрого доступа к данным  
✅ Отдельное поле `sale_date` для группировки по датам  
✅ UUID ссылки для быстрого JOIN с агрегатами  

### Аналитика
✅ Группировка по организациям (один кабинет может иметь несколько организаций)  
✅ Группировка по кабинетам маркетплейсов  
✅ Связь с товарами МП для детальной аналитики  
✅ Отдельная дата реализации для бизнес-отчетов  

### Поддерживаемость
✅ Единая система именования (_ref для ссылок)  
✅ Логическая группировка полей в SQL  
✅ Четкое разделение NK, metadata, ссылок и данных  
✅ Комментарии в коде и SQL  

### Масштабируемость
✅ Готовность к автоматическому сопоставлению с товарами  
✅ Возможность добавления новых маркетплейсов  
✅ Гибкая система индексов  

---

## Статус компиляции

✅ **Backend компилируется без ошибок**
✅ Все модули обновлены
✅ Все индексы созданы
✅ Маппинг работает корректно

---

## Примеры использования

### Выборка продаж по организации
```sql
SELECT * FROM p900_sales_register 
WHERE organization_ref = 'uuid-организации'
AND sale_date BETWEEN '2025-01-01' AND '2025-01-31'
ORDER BY sale_date DESC;
```

### Группировка по кабинетам
```sql
SELECT connection_mp_ref, marketplace, 
       COUNT(*) as sales_count,
       SUM(amount_line) as total_amount
FROM p900_sales_register
WHERE sale_date = '2025-01-15'
GROUP BY connection_mp_ref, marketplace;
```

### Группировка по датам
```sql
SELECT sale_date, 
       COUNT(*) as sales_count,
       SUM(qty) as total_qty,
       SUM(amount_line) as total_amount
FROM p900_sales_register
WHERE marketplace = 'OZON'
AND sale_date >= '2025-01-01'
GROUP BY sale_date
ORDER BY sale_date;
```

### Поиск по товару МП (после сопоставления)
```sql
SELECT * FROM p900_sales_register
WHERE marketplace_product_ref = 'uuid-товара'
ORDER BY sale_date DESC;
```

---

## Файлы изменены

1. `crates/backend/src/shared/data/db.rs` - схема БД и индексы
2. `crates/backend/src/projections/p900_mp_sales_register/repository.rs` - модель и upsert
3. `crates/backend/src/projections/p900_mp_sales_register/projection_builder.rs` - маппинг всех 4 типов документов

**Всего: 3 файла, ~300 строк кода обновлено**

