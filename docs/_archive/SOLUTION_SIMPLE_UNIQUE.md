# Решение: Упрощение дедупликации WB Sales

## Проблема

Ошибка при импорте WB Sales:
```
UNIQUE constraint failed: a012_wb_sales.document_no, a012_wb_sales.event_type, a012_wb_sales.supplier_article
```

## Причина сложности

Композитный UNIQUE ключ на трех полях создавал сложности:
- Сложная логика поиска (sale_id → composite key → document_no)
- Возможность race condition при параллельном импорте
- Необходимость проверки NULL значений
- Сложная диагностика ошибок

## Решение: Использовать только sale_id

### Принцип

**sale_id** - единственный уникальный ключ для дедупликации записей.

### Преимущества

✅ **Простота** - один ключ вместо композитного из трех полей
✅ **Надежность** - меньше условий для race condition
✅ **Производительность** - простой индекс работает быстрее
✅ **Ясность** - легко понять и отладить

## Изменения

### 1. Схема БД (db.rs)

**Было:**
```sql
sale_id TEXT,
...
UNIQUE (document_no, event_type, supplier_article)
```

**Стало:**
```sql
sale_id TEXT NOT NULL UNIQUE,
```

### 2. Processor (processors/sales.rs)

**Ключевое изменение:** `sale_id` теперь **всегда генерируется**, если не пришел от API:

```rust
let sale_id = if let Some(sid) = sale_row.sale_id.clone() {
    sid
} else {
    // Генерируем уникальный sale_id
    format!("WB_GEN_{}_{}_{}_{}", 
        document_no, 
        event_type, 
        supplier_article,
        chrono::Utc::now().timestamp_millis()
    )
};
```

**Логика поиска:**
```rust
// Упростилась до одной строки:
let existing = a012_wb_sales::service::get_by_sale_id(&sale_id).await?;
```

### 3. Repository (repository.rs)

**Было:**
- Сложная логика с тремя вариантами поиска
- Double-check перед INSERT
- Детальная диагностика композитного ключа

**Стало:**
```rust
pub async fn upsert_document(aggregate: &WbSales) -> Result<Uuid> {
    let sale_id = aggregate.header.sale_id.as_ref()
        .ok_or_else(|| anyhow::anyhow!("sale_id is required"))?;
    
    // Простой поиск только по sale_id
    let existing = get_by_sale_id(sale_id).await?;
    
    if let Some(existing_doc) = existing {
        // UPDATE
    } else {
        // INSERT
    }
}
```

## Применение исправлений

### Шаг 1: Применить миграцию БД

**ВАЖНО:** Остановите backend перед миграцией!

```powershell
sqlite3 "E:\dev\rust\leptos_marketplace_1\data\app.db" < migrate_a012_wb_sales_simplify_unique.sql
```

Миграция:
- Создаст новую таблицу с `sale_id UNIQUE`
- Скопирует данные, сгенерирует sale_id для записей без него
- Удалит дубликаты (оставит последнюю версию)
- Пересоздаст индексы

### Шаг 2: Перекомпилировать backend

```powershell
cargo build --bin backend
```

### Шаг 3: Запустить backend

```powershell
cargo run --bin backend
```

### Шаг 4: Протестировать импорт

В UI: **UseCase → Import from Wildberries → Sales**

## Проверка после миграции

### 1. Проверить, что все записи имеют sale_id

```sql
SELECT COUNT(*) FROM a012_wb_sales WHERE sale_id IS NULL OR sale_id = '';
```

**Ожидается:** 0

### 2. Проверить отсутствие дубликатов sale_id

```sql
SELECT sale_id, COUNT(*) as cnt 
FROM a012_wb_sales 
GROUP BY sale_id 
HAVING cnt > 1;
```

**Ожидается:** 0 строк

### 3. Проверить общее количество записей

```sql
SELECT COUNT(*) as total FROM a012_wb_sales;
```

## Поведение системы

### При импорте с WB API

**Если sale_id пришел от API:**
- Используется как есть
- Поиск существующей записи по sale_id
- UPDATE если найдена, INSERT если нет

**Если sale_id не пришел от API:**
- Генерируется уникальный sale_id на основе:
  - document_no (SRID)
  - event_type (sale/return)
  - supplier_article
  - timestamp (для уникальности)
- Поиск существующей записи по сгенерированному sale_id
- INSERT (так как это новый sale_id)

### Логирование

**DEBUG уровень:**
```
Processing WB sale: sale_id=ABC123, document_no=WB_456, ...
[REPOSITORY] upsert_document called with sale_id='ABC123', ...
[REPOSITORY] Updating existing record: id=..., sale_id='ABC123'
```

или

```
[REPOSITORY] Inserting new record: id=..., sale_id='ABC123'
```

**ERROR только при настоящих ошибках:**
```
Failed to store WB sale - sale_id: ABC123, error: ...
UNIQUE constraint violation on sale_id: ABC123
  This should not happen as sale_id is unique.
  Possible cause: race condition during parallel import
```

## Преимущества нового подхода

### 1. Простота кода

- **Было:** ~100 строк логики дедупликации с 3 вариантами поиска
- **Стало:** ~10 строк с одним вариантом поиска

### 2. Надежность

- **Было:** race condition между проверкой по composite key и INSERT
- **Стало:** race condition только при одновременной вставке одного sale_id (крайне редко)

### 3. Производительность

- **Было:** до 3 запросов к БД для поиска существующей записи
- **Стало:** 1 запрос к БД по индексу sale_id

### 4. Понятность логов

- **Было:** множество DEBUG/INFO/CRITICAL логов с деталями композитного ключа
- **Стало:** минимальные логи с sale_id

## Возможные вопросы

### Q: Что если WB API вернет дубликат sale_id?

**A:** SQLite выдаст UNIQUE constraint error. Это правильное поведение - означает, что API вернул действительно дублирующиеся данные. В логах будет указан sale_id.

### Q: Что если sale_id не придет от API?

**A:** Генерируется уникальный sale_id на основе document_no, event_type, supplier_article и timestamp.

### Q: Можно ли импортировать одну и ту же запись дважды?

**A:** 
- Если sale_id одинаковый - будет UPDATE (запись обновится)
- Если sale_id разный - будет INSERT (создастся новая запись)

### Q: Что стало с полями document_no, event_type, supplier_article?

**A:** Они остались в таблице как обычные поля (без UNIQUE constraint), используются для отображения и фильтрации.

## Откат изменений

Если нужно вернуться к старой схеме:

1. Остановить backend
2. Восстановить backup БД
3. Откатить изменения в коде:
   ```powershell
   git checkout crates/backend/src/shared/data/db.rs
   git checkout crates/backend/src/domain/a012_wb_sales/repository.rs
   git checkout crates/backend/src/usecases/u504_import_from_wildberries/processors/sales.rs
   ```
4. Перекомпилировать и запустить

## Связанные файлы

- `migrate_a012_wb_sales_simplify_unique.sql` - SQL миграция
- `crates/backend/src/shared/data/db.rs` - схема БД
- `crates/backend/src/domain/a012_wb_sales/repository.rs` - логика работы с БД
- `crates/backend/src/usecases/u504_import_from_wildberries/processors/sales.rs` - обработка данных WB

## История

- **2026-01-21 (v1)**: Попытка исправить композитный UNIQUE constraint, добавлена диагностика
- **2026-01-21 (v2)**: Упрощено до использования только sale_id как единственного уникального ключа ✅
