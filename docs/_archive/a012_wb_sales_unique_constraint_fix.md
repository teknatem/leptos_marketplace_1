# Исправление UNIQUE Constraint для a012_wb_sales

## Проблема

При импорте данных из Wildberries API возникала ошибка:

```
Failed to process WB sale
Execution Error: error returned from database: (code: 2067) 
UNIQUE constraint failed: a012_wb_sales.document_no, a012_wb_sales.event_type, a012_wb_sales.supplier_article
```

## Причина

**Несоответствие между схемой БД и кодом:**

- **В реальной БД**: `UNIQUE (document_no, event_type, supplier_article)` - композитный constraint
- **В коде миграции** (db.rs): `document_no TEXT NOT NULL UNIQUE` - только на одном поле

**Бизнес-логика:**
- `document_no` содержит SRID (уникальный идентификатор строки из WB API)
- Один SRID может встречаться для разных:
  - `event_type`: "sale" (продажа) или "return" (возврат)
  - `supplier_article`: разных артикулов поставщика
- Поэтому уникальность должна быть по **комбинации трех полей**

**Проблема дедупликации:**
- Функция `repository::upsert_document()` искала существующую запись только по:
  1. `sale_id` (если есть)
  2. `document_no` (если sale_id нет)
- Но не проверяла композитный ключ `(document_no, event_type, supplier_article)`
- Это приводило к попыткам INSERT вместо UPDATE при наличии дубликатов

## Решение

### 1. Обновлена схема БД в коде (db.rs)

```rust
// Было:
document_no TEXT NOT NULL UNIQUE,

// Стало:
document_no TEXT NOT NULL,
...
-- Composite unique constraint
UNIQUE (document_no, event_type, supplier_article)
```

### 2. Добавлена функция поиска по композитному ключу (repository.rs)

```rust
pub async fn get_by_composite_key(
    document_no: &str,
    event_type: &str,
    supplier_article: &str,
) -> Result<Option<WbSales>>
```

### 3. Обновлена логика upsert (repository.rs)

Теперь проверяется в следующем порядке:
1. `sale_id` (наиболее надежный идентификатор от WB)
2. Композитный ключ `(document_no, event_type, supplier_article)` - соответствует UNIQUE constraint
3. Только `document_no` (fallback для обратной совместимости)

### 4. Добавлена диагностика (processors/sales.rs)

При ошибке UNIQUE constraint violation выводится детальная информация:
- document_no (SRID)
- event_type (sale/return)
- supplier_article
- sale_id
- sale_date
- quantity

Это помогает быстро идентифицировать причину дубликатов.

### 5. Создана миграция (migrate_a012_wb_sales_fix_unique.sql)

Скрипт для обновления существующих баз данных:
- Создает новую таблицу с правильным constraint
- Копирует данные (оставляет только последнюю версию при дубликатах)
- Удаляет старую таблицу
- Пересоздает индексы

## Применение исправлений

### Для новых баз данных

Изменения в `db.rs` автоматически применятся при создании новой БД.

### Для существующих баз данных

Выполнить миграцию:

```powershell
sqlite3 "E:\dev\rust\leptos_marketplace_1\data\app.db" < migrate_a012_wb_sales_fix_unique.sql
```

Или из другого расположения:

```powershell
sqlite3 "путь_к_вашей_БД\app.db" < migrate_a012_wb_sales_fix_unique.sql
```

### Проверка перед миграцией

Проверить наличие дубликатов:

```sql
SELECT document_no, event_type, supplier_article, COUNT(*) as duplicates
FROM a012_wb_sales
GROUP BY document_no, event_type, supplier_article
HAVING COUNT(*) > 1;
```

### Проверка после миграции

```sql
-- Должно вернуть 0 строк (нет дубликатов)
SELECT document_no, event_type, supplier_article, COUNT(*) as cnt 
FROM a012_wb_sales 
GROUP BY document_no, event_type, supplier_article 
HAVING cnt > 1;

-- Проверить индексы
PRAGMA index_list('a012_wb_sales');
```

## Влияние на работу

### Положительные эффекты

1. **Корректная дедупликация**: теперь система правильно определяет существующие записи
2. **Детальная диагностика**: при ошибках выводится полная информация для анализа
3. **Соответствие схемы**: код и БД используют одинаковый constraint
4. **Производительность**: добавлен индекс на `document_no` для быстрого поиска

### Возможные побочные эффекты

1. **Старые дубликаты**: при миграции будет оставлена только последняя версия дубликатов
2. **Увеличение строк**: один SRID теперь может генерировать несколько строк (для разных event_type/articles)
   - Это **корректное поведение**, соответствующее бизнес-логике WB

## Бизнес-логика WB Sales

### Поля уникального ключа

1. **document_no** (SRID):
   - Уникальный идентификатор строки продажи от Wildberries
   - Может повторяться для разных событий/артикулов

2. **event_type**:
   - `"sale"` - продажа товара
   - `"return"` - возврат товара
   - Определяется знаком quantity (отрицательное = возврат)

3. **supplier_article**:
   - Артикул поставщика
   - Один SRID может относиться к разным артикулам

### Примеры допустимых комбинаций

```
SRID: WB_12345, event_type: sale,   supplier_article: ART-001  ✓
SRID: WB_12345, event_type: return, supplier_article: ART-001  ✓
SRID: WB_12345, event_type: sale,   supplier_article: ART-002  ✓
```

Все три записи **легальны и корректны**.

### Дубликаты (недопустимо)

```
SRID: WB_12345, event_type: sale, supplier_article: ART-001
SRID: WB_12345, event_type: sale, supplier_article: ART-001  ✗ DUPLICATE
```

## Дальнейшие улучшения

1. **Добавить поле raw_payload_hash** для проверки изменений в данных
2. **Логировать все попытки дедупликации** для аналитики
3. **Dashboard дубликатов** в UI для мониторинга
4. **Автоматическое разрешение конфликтов** на основе last_change_date

## Связанные файлы

- `crates/backend/src/shared/data/db.rs` - схема БД
- `crates/backend/src/domain/a012_wb_sales/repository.rs` - работа с БД
- `crates/backend/src/usecases/u504_import_from_wildberries/processors/sales.rs` - обработка данных WB
- `migrate_a012_wb_sales_fix_unique.sql` - SQL миграция

## История изменений

- **2026-01-21**: Исправлен UNIQUE constraint, добавлена диагностика, создана миграция
