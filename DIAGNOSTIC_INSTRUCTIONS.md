# Диагностика ошибки UNIQUE constraint для a012_wb_sales

## Текущая ситуация

Ошибка продолжается:
```
UNIQUE constraint failed: a012_wb_sales.document_no, a012_wb_sales.event_type, a012_wb_sales.supplier_article
```

## Добавлена расширенная диагностика

В код добавлено детальное логирование на **всех уровнях**:

### 1. Уровень процессора (processors/sales.rs)
- INFO логи перед сохранением записи
- ERROR логи с полной информацией при ошибке
- Объяснение возможных причин

### 2. Уровень repository (repository.rs)
- DEBUG логи при поиске существующих записей
- INFO логи перед INSERT/UPDATE
- Проверка наличия записи в БД перед INSERT (double-check)
- **CRITICAL** лог, если запись существует, но не найдена lookup-функцией
- Детальный ERROR лог при неудачном INSERT с точными значениями полей

## Действия для диагностики

### Шаг 1: Перезапустить backend

**ВАЖНО:** Остановите текущий backend (Ctrl+C в терминале) и запустите заново:

```powershell
cargo run --bin backend
```

Или просто:

```powershell
cargo build --bin backend
# затем в отдельном терминале:
target\debug\backend.exe
```

### Шаг 2: Установить уровень логирования DEBUG

Добавьте в начало `crates/backend/src/main.rs` (если еще нет):

```rust
std::env::set_var("RUST_LOG", "debug");
```

Или запустите backend с переменной окружения:

```powershell
$env:RUST_LOG="debug"
cargo run --bin backend
```

### Шаг 3: Запустить импорт WB Sales

В UI перейти: **UseCase → Import from Wildberries → Sales**

### Шаг 4: Проверить логи

При возникновении ошибки вы увидите **детальную цепочку логов**:

```
[REPOSITORY] upsert_document called with: document_no='WB_123', event_type='sale', supplier_article='ART-001', sale_id=None

[REPOSITORY] No sale_id, looking up by composite key: document_no='WB_123', event_type='sale', supplier_article='ART-001'

[REPOSITORY] Not found by composite key, trying fallback by document_no only: 'WB_123'

[REPOSITORY] INSERTING new record: id=..., document_no='WB_123', event_type=Some("sale"), supplier_article=Some("ART-001")

⚠️ CRITICAL: Record EXISTS in DB but was not found by lookup!
    document_no='WB_123', event_type='sale', supplier_article='ART-001'. 
    This indicates a bug in the lookup logic.

❌ INSERT FAILED: error returned from database: (code: 2067) UNIQUE constraint failed...
Details:
- document_no: 'WB_123'
- event_type: Some("sale")
- supplier_article: Some("ART-001")
- sale_id: None
- id: <uuid>
```

## Что даст диагностика

1. **Точные значения полей**, которые вызывают конфликт
2. **Путь поиска** существующей записи (по sale_id → composite key → document_no)
3. **Обнаружение расхождения** между lookup и реальным состоянием БД
4. **Подтверждение** применения миграции БД

## Возможные причины ошибки

### Причина 1: Миграция БД не применена

**Проверка:**
```powershell
sqlite3 "E:\dev\rust\leptos_marketplace_1\data\app.db" "SELECT sql FROM sqlite_master WHERE type='table' AND name='a012_wb_sales';"
```

**Ожидаемый результат:** должна быть строка `UNIQUE (document_no, event_type, supplier_article)`

**Решение:** Применить миграцию
```powershell
# Остановить backend!
sqlite3 "E:\dev\rust\leptos_marketplace_1\data\app.db" < migrate_a012_wb_sales_fix_unique.sql
```

### Причина 2: Ошибка в lookup-логике

**Симптомы:**
- Логи показывают `[REPOSITORY] Not found by composite key`
- Затем сразу `⚠️ CRITICAL: Record EXISTS in DB`

**Это значит:**
- Запись существует в БД
- Но `get_by_composite_key()` её не находит
- Возможно из-за различий в значениях полей (пробелы, регистр, NULL vs empty string)

**Решение:** Посмотреть на точные значения в логах и проверить в БД:
```sql
SELECT document_no, event_type, supplier_article, 
       length(document_no), length(event_type), length(supplier_article)
FROM a012_wb_sales 
WHERE document_no = '<значение_из_лога>';
```

### Причина 3: NULL значения в полях

**Проверка в БД:**
```sql
SELECT COUNT(*) FROM a012_wb_sales 
WHERE event_type IS NULL OR supplier_article IS NULL;
```

**Если есть NULL:**
- `get_by_composite_key()` не найдет такие записи (filter не сработает на NULL)
- Но UNIQUE constraint считает NULL = NULL

**Решение:** Обновить NULL на пустые строки или специальное значение:
```sql
UPDATE a012_wb_sales 
SET event_type = 'unknown' 
WHERE event_type IS NULL;

UPDATE a012_wb_sales 
SET supplier_article = '' 
WHERE supplier_article IS NULL;
```

## Следующие шаги

1. **Перезапустите backend** с новым кодом
2. **Включите DEBUG логи** (`RUST_LOG=debug`)
3. **Запустите импорт** WB Sales
4. **Скопируйте логи** с ошибкой и пришлите для анализа
5. **Проверьте БД** на наличие UNIQUE constraint и NULL значений

## Помощь

После запуска с диагностикой, пришлите:
1. Логи из терминала (с момента `[REPOSITORY] upsert_document called` до ошибки)
2. Результат проверки схемы БД (SELECT sql...)
3. Результат проверки на NULL (SELECT COUNT...)

Это позволит точно определить причину проблемы.
