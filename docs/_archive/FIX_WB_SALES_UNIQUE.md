# Быстрое исправление UNIQUE constraint для a012_wb_sales

## Проблема

```
UNIQUE constraint failed: a012_wb_sales.document_no, a012_wb_sales.event_type, a012_wb_sales.supplier_article
```

## Причина

База данных имеет композитный UNIQUE constraint на `(document_no, event_type, supplier_article)`, но код проверял дубликаты только по `document_no`.

## Быстрое решение

### 1. Применить миграцию БД

```powershell
sqlite3 "E:\dev\rust\leptos_marketplace_1\data\app.db" < migrate_a012_wb_sales_fix_unique.sql
```

**ВАЖНО:** Перед миграцией остановите backend!

### 2. Перекомпилировать проект

```powershell
cargo build --bin backend
```

### 3. Запустить backend

```powershell
cargo run --bin backend
```

## Что исправлено

✅ **Схема БД** - обновлена в коде (db.rs)
✅ **Дедупликация** - добавлен поиск по композитному ключу (repository.rs)
✅ **Диагностика** - детальные логи при ошибках (processors/sales.rs)
✅ **Миграция** - SQL скрипт для существующих БД

## Проверка после применения

### 1. Проверить отсутствие дубликатов

```powershell
sqlite3 "E:\dev\rust\leptos_marketplace_1\data\app.db" "SELECT document_no, event_type, supplier_article, COUNT(*) FROM a012_wb_sales GROUP BY document_no, event_type, supplier_article HAVING COUNT(*) > 1;"
```

Должно вернуть **0 строк**.

### 2. Запустить импорт WB Sales

В UI перейти в **UseCase → Import from Wildberries → Sales**.

### 3. Проверить логи

При импорте должны появиться сообщения:

```
Processing WB sale: document_no=..., event_type=..., supplier_article=...
Successfully stored WB sale: ...
```

При ошибке будет детальная диагностика с указанием причины.

## Детальная документация

См. `docs/a012_wb_sales_unique_constraint_fix.md` для подробного описания проблемы и решения.

## Откат (если что-то пошло не так)

1. Остановить backend
2. Восстановить backup БД (если делали)
3. Вернуть изменения в коде:

```powershell
git checkout crates/backend/src/shared/data/db.rs
git checkout crates/backend/src/domain/a012_wb_sales/repository.rs
git checkout crates/backend/src/usecases/u504_import_from_wildberries/processors/sales.rs
```

4. Перекомпилировать и запустить
