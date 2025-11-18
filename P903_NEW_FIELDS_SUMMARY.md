# Добавлены новые поля в P903 WB Finance Report

## Дата: 2025-11-17

## Добавленные поля

### 1. cashback_amount (REAL)
**Описание:** Сумма кэшбэка  
**Тип:** Число с плавающей точкой  
**В UI Details:** "Сумма кэшбэка"

### 2. ppvz_for_pay (REAL)
**Описание:** К перечислению за товар  
**Тип:** Число с плавающей точкой  
**В UI Details:** "К перечислению за товар"  
**Примечание:** Сумма, которую WB должен перечислить продавцу за товар

### 3. ppvz_kvw_prc (REAL)
**Описание:** Процент комиссии (ppvz_kvw_prc)  
**Тип:** Число с плавающей точкой  
**В UI Details:** "Процент комиссии (ppvz_kvw_prc)"

### 4. ppvz_kvw_prc_base (REAL)
**Описание:** Базовый процент комиссии  
**Тип:** Число с плавающей точкой  
**В UI Details:** "Базовый процент комиссии"

### 5. srv_dbs (INTEGER)
**Описание:** Доставка силами продавца (DBS - Delivery by Seller)  
**Тип:** Целое число (0 или 1)  
**Значения:** 
- `1` = Да (доставка силами продавца)
- `0` = Нет (доставка силами WB)
**В UI Details:** "Доставка силами продавца (DBS)" - отображается как "Да"/"Нет"

## Измененные файлы

### Backend
1. **`crates/backend/src/shared/data/db.rs`**
   - Добавлены поля в CREATE TABLE
   - Добавлена автоматическая миграция для существующих таблиц

2. **`crates/backend/src/projections/p903_wb_finance_report/repository.rs`**
   - Добавлены поля в `Model` (SeaORM entity)
   - Добавлены поля в `WbFinanceReportEntry`
   - Добавлены в `upsert_entry()` (обе ветки: update и insert)

3. **`crates/backend/src/usecases/u504_import_from_wildberries/executor.rs`**
   - Добавлена загрузка полей из API при импорте
   - Для `srv_dbs`: конвертация `bool -> i32` (true -> 1, false -> 0)

4. **`crates/contracts/src/projections/p903_wb_finance_report/dto.rs`**
   - Добавлены поля в `WbFinanceReportDto`

5. **`crates/backend/src/handlers/p903_wb_finance_report.rs`**
   - Добавлены поля в `model_to_dto` конвертацию

### Frontend
6. **`crates/frontend/src/projections/p903_wb_finance_report/ui/details/mod.rs`**
   - Добавлены поля в `WbFinanceReportDto`
   - Добавлены 5 новых `FieldRow` в `get_field_rows()`
   - Для `srv_dbs`: отображение 1/0 как "Да"/"Нет"

7. **`crates/frontend/src/projections/p903_wb_finance_report/ui/list/mod.rs`**
   - Добавлены поля в `WbFinanceReportDto`

### SQL Migration
8. **`migrate_p903_wb_finance_report.sql`**
   - Добавлены поля в CREATE TABLE скрипт

## Автоматическая миграция

При запуске backend автоматически проверяется наличие новых полей и добавляет их если отсутствуют:
```rust
let new_fields = vec![
    ("cashback_amount", "REAL"),
    ("ppvz_for_pay", "REAL"),
    ("ppvz_kvw_prc", "REAL"),
    ("ppvz_kvw_prc_base", "REAL"),
    ("srv_dbs", "INTEGER"),
];
```

## Источник данных

Все поля загружаются из API Wildberries:
- Endpoint: `https://statistics-api.wildberries.ru/api/v5/supplier/reportDetailByPeriod`
- Эти поля уже были определены в структуре `WbFinanceReportRow` как дополнительные поля

## Отображение в UI

### Details форма
Все 5 полей отображаются в таблице на вкладке "Fields":

| Описание | Идентификатор | Значение |
|----------|--------------|----------|
| Сумма кэшбэка | cashback_amount | {число с 2 знаками} |
| К перечислению за товар | ppvz_for_pay | {число с 2 знаками} |
| Процент комиссии (ppvz_kvw_prc) | ppvz_kvw_prc | {число с 2 знаками} |
| Базовый процент комиссии | ppvz_kvw_prc_base | {число с 2 знаками} |
| Доставка силами продавца (DBS) | srv_dbs | Да/Нет |

### List форма
Поля доступны в данных, но не отображаются в основной таблице (можно добавить при необходимости).

## Проверка

### SQL запрос для проверки наличия полей:
```sql
PRAGMA table_info(p903_wb_finance_report);
```

### Проверка данных:
```sql
SELECT 
    rr_dt,
    rrd_id,
    cashback_amount,
    ppvz_for_pay,
    ppvz_kvw_prc,
    ppvz_kvw_prc_base,
    srv_dbs
FROM p903_wb_finance_report
WHERE cashback_amount IS NOT NULL 
   OR ppvz_for_pay IS NOT NULL
LIMIT 10;
```

## Примечания

1. **Автоматическая миграция:** При первом запуске backend после обновления кода, новые поля будут добавлены автоматически
2. **Nullable поля:** Все новые поля имеют тип `Option<>` и могут быть `NULL`
3. **srv_dbs конвертация:** В API приходит как `bool`, в БД сохраняется как `INTEGER` (0/1)
4. **Raw JSON:** Все поля также присутствуют в поле `extra` в полном JSON ответе от API

## Статус
✅ Реализовано полностью
✅ Backend компилируется без ошибок
✅ Frontend компилируется без ошибок
✅ Автоматическая миграция БД реализована

