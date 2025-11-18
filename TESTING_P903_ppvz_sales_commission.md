# Проверка загрузки поля ppvz_sales_commission в P903

## Описание проблемы

Поле `ppvz_sales_commission` должно правильно загружаться из API Wildberries и соответствовать расчету на основе `commission_percent`, но это не происходит.

## Добавленное логирование

### 1. В API клиенте (`wildberries_api_client.rs`)

После получения и парсинга данных из API логируются первые 3 записи:

- `rrd_id`
- `commission_percent`
- `ppvz_sales_commission`
- `retail_price_withdisc_rub`
- `retail_amount`

Логи сохраняются в:

- Консоль backend (через `tracing::info!`)
- Файл `target/logs/wildberries_api_requests.log`

### 2. В executor (`executor.rs`)

Перед сохранением в БД логируются первые 5 записей:

- `rrd_id`
- `commission_percent`
- `ppvz_sales_commission`
- `retail_price_withdisc_rub`

## Как проверить

### Шаг 1: Запустите импорт WB Finance Report

1. Откройте форму "Import from Wildberries (U504)"
2. Выберите подключение к Wildberries
3. Установите галочку "p903_wb_finance_report - Финансовый отчет WB"
4. Выберите период (например, последние 2-3 дня)
5. Запустите импорт

### Шаг 2: Проверьте логи

#### Лог файл API запросов:

```
target/logs/wildberries_api_requests.log
```

Найдите секцию с образцами записей:

```
=== Sample Record 1 ===
rrd_id: Some(123456)
commission_percent: Some(24.0)
ppvz_sales_commission: Some(23.74)
retail_price_withdisc_rub: Some(399.68)
retail_amount: Some(367.0)
```

#### Backend console лог:

Смотрите вывод backend приложения, найдите строки:

```
WB Finance Report sample 1: rrd_id=Some(123456), commission_percent=Some(24.0), ppvz_sales_commission=Some(23.74)
WB Finance Report row 1: rrd_id=123456, commission_percent=Some(24.0), ppvz_sales_commission=Some(23.74), ...
```

### Шаг 3: Проверьте данные в БД

Выполните SQL запрос для проверки сохраненных данных:

```sql
-- Первые 10 записей с обоими полями
SELECT
    rr_dt,
    rrd_id,
    nm_id,
    sa_name,
    retail_price_withdisc_rub,
    commission_percent,
    ppvz_sales_commission,
    retail_amount,
    -- Расчет ожидаемой комиссии: цена * процент / 100
    ROUND(retail_price_withdisc_rub * commission_percent / 100, 2) as calculated_commission,
    -- Разница между фактической и рассчитанной комиссией
    ROUND(ppvz_sales_commission - (retail_price_withdisc_rub * commission_percent / 100), 2) as difference
FROM p903_wb_finance_report
WHERE commission_percent IS NOT NULL
  AND ppvz_sales_commission IS NOT NULL
  AND retail_price_withdisc_rub IS NOT NULL
ORDER BY rr_dt DESC, rrd_id
LIMIT 10;

-- Статистика по заполненности полей
SELECT
    COUNT(*) as total_records,
    COUNT(commission_percent) as has_commission_percent,
    COUNT(ppvz_sales_commission) as has_ppvz_sales_commission,
    COUNT(CASE WHEN commission_percent IS NOT NULL AND ppvz_sales_commission IS NOT NULL THEN 1 END) as has_both
FROM p903_wb_finance_report;

-- Проверка расхождений
SELECT
    COUNT(*) as records_with_difference,
    AVG(ABS(ppvz_sales_commission - (retail_price_withdisc_rub * commission_percent / 100))) as avg_difference,
    MAX(ABS(ppvz_sales_commission - (retail_price_withdisc_rub * commission_percent / 100))) as max_difference
FROM p903_wb_finance_report
WHERE commission_percent IS NOT NULL
  AND ppvz_sales_commission IS NOT NULL
  AND retail_price_withdisc_rub IS NOT NULL
  AND ABS(ppvz_sales_commission - (retail_price_withdisc_rub * commission_percent / 100)) > 0.01;
```

### Шаг 4: Проверьте в UI

1. Откройте "WB Finance Report (P903)" в левом меню
2. Отсортируйте по дате (DESC)
3. Проверьте колонки:
   - **Commission%** - процент комиссии
   - **Sales Comm** - сумма комиссии от Wildberries (ppvz_sales_commission)
4. Откройте детальную форму любой записи
5. Найдите поля:
   - "Процент комиссии Wildberries с продажи" (commission_percent)
   - "Комиссия WB за продажу" (ppvz_sales_commission)
6. Проверьте вкладку "Raw JSON" - там должны быть ВСЕ поля из API

## Ожидаемое поведение

### Поле `ppvz_sales_commission`

- Это **абсолютная сумма** комиссии в рублях, которую Wildberries удерживает с продавца
- Приходит из API как отдельное поле
- **НЕ всегда равно** `retail_price_withdisc_rub * commission_percent / 100`
- Может отличаться из-за:
  - Округлений
  - Специальных условий для продавца
  - Корректировок Wildberries
  - Промо-акций и скидок

### Поле `commission_percent`

- Это **процент** комиссии
- Используется для информации и расчетов

### Пример из реального API:

```json
{
  "commission_percent": 24,
  "retail_price_withdisc_rub": 399.68,
  "ppvz_sales_commission": 23.74
}
```

Расчет: `399.68 * 24 / 100 = 95.92` ❌ НЕ совпадает с 23.74!

**Причина:** `ppvz_sales_commission` - это фактическая комиссия, которую рассчитал Wildberries с учетом всех условий. Она может отличаться от простого расчета по проценту.

## Возможные проблемы

### 1. Поле `ppvz_sales_commission` всегда `NULL`

**Причина:** Поле не приходит из API или неправильное имя поля
**Решение:** Проверьте логи API, секцию "Sample Record" - там должно быть значение

### 2. Поле приходит, но не сохраняется в БД

**Причина:** Ошибка в mapping или сохранении
**Решение:** Проверьте логи executor - должны быть значения перед сохранением

### 3. Поле есть в БД, но не отображается в UI

**Причина:** Ошибка в frontend
**Решение:** Проверьте console браузера на ошибки, откройте вкладку Network

## Дополнительные SQL запросы

### Посмотреть raw JSON для проверки всех полей из API:

```sql
SELECT
    rr_dt,
    rrd_id,
    json_extract(extra, '$.commission_percent') as api_commission_percent,
    json_extract(extra, '$.ppvz_sales_commission') as api_ppvz_sales_commission,
    commission_percent as db_commission_percent,
    ppvz_sales_commission as db_ppvz_sales_commission
FROM p903_wb_finance_report
WHERE extra IS NOT NULL
LIMIT 5;
```

### Найти записи где поле отсутствует:

```sql
SELECT
    COUNT(*) as total,
    COUNT(ppvz_sales_commission) as with_value,
    COUNT(*) - COUNT(ppvz_sales_commission) as without_value
FROM p903_wb_finance_report;
```

## Итоговые файлы

### Измененные файлы:

1. `crates/backend/src/usecases/u504_import_from_wildberries/wildberries_api_client.rs`

   - Добавлено логирование первых 3 записей после получения из API

2. `crates/backend/src/usecases/u504_import_from_wildberries/executor.rs`
   - Добавлено логирование первых 5 записей перед сохранением в БД

### Логи для анализа:

- `target/logs/wildberries_api_requests.log` - подробные логи API запросов
- Backend console - основные события импорта
- `target/logs/backend.log` - все логи backend (если настроено)

## Контакты

Если после проверки логов проблема не ясна, предоставьте:

1. Фрагмент из `wildberries_api_requests.log` с "Sample Record"
2. Результат SQL запроса проверки данных
3. Screenshot из UI с проблемной записью
