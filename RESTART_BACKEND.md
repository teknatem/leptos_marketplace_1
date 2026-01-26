# ПЕРЕЗАПУСК BACKEND С ДИАГНОСТИКОЙ

## ⚠️ КРИТИЧЕСКИ ВАЖНО

Новый код с детальной диагностикой **УЖЕ СКОМПИЛИРОВАН**, но backend нужно **ПЕРЕЗАПУСТИТЬ**, чтобы изменения вступили в силу!

## Шаги

### 1. Остановить текущий backend

В терминале, где запущен backend, нажмите:
```
Ctrl + C
```

### 2. Запустить backend с DEBUG логами

```powershell
$env:RUST_LOG="debug"
cargo run --bin backend
```

Или в двух отдельных терминалах:

**Терминал 1 - сборка:**
```powershell
cargo build --bin backend
```

**Терминал 2 - запуск:**
```powershell
$env:RUST_LOG="debug"
.\target\debug\backend.exe
```

### 3. Запустить импорт WB Sales

В UI: **UseCase → Import from Wildberries → Sales**

### 4. Смотреть логи в терминале backend

При возникновении ошибки вы увидите **ДЕТАЛЬНЫЕ ЛОГИ**:

```
[REPOSITORY] upsert_document called with: document_no='...', event_type='...', supplier_article='...', sale_id=...
[REPOSITORY] No sale_id, looking up by composite key: ...
[REPOSITORY] INSERTING new record: ...

⚠️ CRITICAL: Record EXISTS in DB but was not found by lookup!
    <- Если видите это, значит проблема в lookup-логике

❌ INSERT FAILED: error returned from database: (code: 2067)
Details:
- document_no: '...'      <- ТОЧНОЕ ЗНАЧЕНИЕ
- event_type: Some("...") <- ТОЧНОЕ ЗНАЧЕНИЕ
- supplier_article: Some("...") <- ТОЧНОЕ ЗНАЧЕНИЕ
- sale_id: ...
- id: ...
```

## Что делать с логами

**Скопируйте ВСЕ логи** начиная с `[REPOSITORY] upsert_document called` до конца ошибки.

Это покажет:
1. **Точные значения** полей, которые вызывают конфликт
2. **Был ли найден** существующий документ
3. **Какой метод поиска** использовался
4. **Есть ли расхождение** между lookup и реальным состоянием БД

## Проверка состояния БД (уже выполнено)

✅ Всего записей: 12,702
✅ NULL значений в ключевых полях: 0
✅ Дубликатов по (document_no, event_type, supplier_article): 0
✅ UNIQUE constraint применен: да

База данных в порядке! Проблема, скорее всего, в логике lookup или race condition при параллельном импорте.

## Возможная причина

Если вы видите `⚠️ CRITICAL: Record EXISTS in DB but was not found by lookup!`, это означает:

1. **При первой проверке** запись НЕ найдена функцией `get_by_composite_key()`
2. **При double-check** прямо перед INSERT запись УЖЕ существует
3. **Между проверками** кто-то успел вставить эту же запись (race condition)

**Решение:** Добавить транзакцию или retry логику.

## Другая возможная причина

Различие в значениях полей:
- Лидирующие/trailing пробелы
- Разный регистр (если БД case-sensitive)
- Empty string vs NULL (хотя мы проверили - NULL нет)

Логи покажут **точные значения**, которые можно будет сравнить с БД:

```sql
SELECT document_no, event_type, supplier_article,
       length(document_no) as len_doc,
       length(event_type) as len_event, 
       length(supplier_article) as len_art,
       quote(document_no) as doc_quoted,
       quote(event_type) as event_quoted,
       quote(supplier_article) as art_quoted
FROM a012_wb_sales 
WHERE document_no = '<значение_из_лога>'
LIMIT 5;
```

Функция `quote()` покажет скрытые символы (пробелы, переносы строк и т.д.).
