# Миграция таблицы p902_ozon_finance_realization

## Проблема
При попытке импорта данных из Ozon возникает ошибка:
```
UNIQUE constraint failed: p902_ozon_finance_realization.posting_number, p902_ozon_finance_realization.sku
```

Это происходит потому, что старая схема БД имела PRIMARY KEY из 2 полей `(posting_number, sku)`,
а новая версия кода требует PRIMARY KEY из 3 полей `(posting_number, sku, operation_type)`.

## Решение

### Вариант 1: Пересоздать БД (ПОТЕРЯ ВСЕХ ДАННЫХ)

1. Остановите backend сервер
2. Удалите файл базы данных:
   ```bash
   rm target/db/app.db
   # или на Windows:
   del target\db\app.db
   ```
3. Запустите backend - таблица будет создана с новой схемой автоматически

### Вариант 2: Применить миграцию (СОХРАНЕНИЕ ДАННЫХ)

1. Остановите backend сервер
2. Примените SQL миграцию:
   ```bash
   sqlite3 target/db/app.db < migrate_p902.sql
   ```
3. Запустите backend

## Что делает миграция

1. Создает новую таблицу `p902_ozon_finance_realization_new` с правильной схемой
2. Копирует все существующие данные, устанавливая `operation_type='delivery'` и `is_return=0`
3. Удаляет старую таблицу
4. Переименовывает новую таблицу
5. Создает индексы заново

## Изменения в схеме

**Старая схема:**
- PRIMARY KEY: `(posting_number, sku)`

**Новая схема:**
- PRIMARY KEY: `(posting_number, sku, operation_type)`
- Новая колонка: `is_return INTEGER NOT NULL DEFAULT 0`

## Что нового

После миграции система будет:
- ✅ Обрабатывать `delivery_commission` (продажи) с `operation_type='delivery'` и `is_return=0`
- ✅ Обрабатывать `return_commission` (возвраты) с `operation_type='return'` и `is_return=1`
- ✅ Хранить отрицательные суммы для возвратов
- ✅ Показывать возвраты с желтым фоном в UI
