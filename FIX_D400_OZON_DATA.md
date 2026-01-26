# Исправление отсутствия данных OZON в d400_monthly_summary

## Проблема
За период 2025-12 в дашборде d400_monthly_summary не попадают данные по OZON, хотя в регистре p900_sales_register они есть.

## Причина
Дашборд **d400_monthly_summary** строится на основе регистра **p904_sales_data**, а не p900_sales_register.

Для OZON документов (FBS/FBO/Returns) отсутствовали проекции в p904_sales_data:
- ✓ WB Sales → p900 + p904 
- ✓ YM Order → p900 + p904
- ✓ YM Returns → p900 + p904
- ✗ OZON FBS → p900 только
- ✗ OZON FBO → p900 только
- ✗ OZON Returns → p900 только

## Решение
Добавлены проекции p904 для OZON FBS, FBO и Returns документов:

### Изменённые файлы:
1. `crates/backend/src/projections/p904_sales_data/projection_builder.rs`
   - Добавлены функции `from_ozon_fbs()`, `from_ozon_fbo()` и `from_ozon_returns()`

2. `crates/backend/src/projections/p904_sales_data/service.rs`
   - Добавлены функции `project_ozon_fbs()`, `project_ozon_fbo()` и `project_ozon_returns()`

3. `crates/backend/src/domain/a010_ozon_fbs_posting/posting.rs`
   - Обновлён `post_document()` для вызова p904 проекций
   - Обновлён `unpost_document()` для удаления p904 проекций

4. `crates/backend/src/domain/a011_ozon_fbo_posting/posting.rs`
   - Обновлён `post_document()` для вызова p904 проекций
   - Обновлён `unpost_document()` для удаления p904 проекций

5. `crates/backend/src/domain/a009_ozon_returns/posting.rs`
   - Обновлён `post_document()` для вызова p904 проекций
   - Обновлён `unpost_document()` для удаления p904 проекций

## Шаги для применения исправления

### 1. Перезапустить backend
```bash
cd e:\dev\rust\2\leptos_marketplace_1
cargo run --bin backend --release
```

### 2. Перепровести OZON FBO и OZON Returns документы

**Текущее состояние:**
- OZON FBS: 968 документов (все проведены ✓)
- OZON FBO: 299 документов (**не проведены** ✗)
- OZON Returns: 141 документов (все проведены, но **без p904** ✗)

**Способ 1: Через SQL (для FBO и Returns)**
```sql
-- Отменить проведение OZON FBO
UPDATE a011_ozon_fbo_posting SET is_posted = 0 WHERE is_posted = 1;

-- Отменить проведение OZON Returns
UPDATE a009_ozon_returns SET is_posted = 0 WHERE is_posted = 1;
```

Затем через API или UI провести документы заново.

**Способ 2: Через UI**
1. Открыть список документов OZON FBO
2. Выбрать все непроведённые документы
3. Нажать "Провести"
4. Повторить для OZON Returns

### 3. Проверить результат

После перепроведения проверьте:

```sql
-- Проверка p904_sales_data
SELECT 
    registrator_type, 
    COUNT(*) as cnt,
    SUM(customer_in) as total
FROM p904_sales_data 
WHERE date LIKE '2025-12%'
GROUP BY registrator_type;
```

Ожидаемый результат:
- WB_Sales: ~5400 записей
- YM_Order: ~1100 записей
- YM_Returns: ~34 записей
- **OZON_FBS: ~900+ записей** (новые!)
- **OZON_FBO: ~300+ записей** (новые!)
- **OZON_Returns: ~141 записей** (новые!)

### 4. Проверить дашборд

Откройте d400_monthly_summary для 2025-12 и убедитесь, что данные OZON отображаются.

## Техническая информация

### Структура p904_sales_data
Проекции создаются на основе:
- **Дата**: `delivered_at` или `updated_at_source`
- **Выручка**: `customer_in` = `amount_line`
- **Связи**: автоматически создаются `marketplace_product_ref` через `find_or_create_for_sale()`

### Условия проекции
- **OZON FBS**: только статус `DELIVERED`
- **OZON FBO**: все статусы
- **OZON Returns**: все возвраты (customer_out с минусом)

## Проверка
```sql
-- До исправления
SELECT marketplace, COUNT(*) FROM p900_sales_register WHERE sale_date LIKE '2025-12%' GROUP BY marketplace;
-- OZON: 1042, WB: 5375, YM: 1141

SELECT registrator_type, COUNT(*) FROM p904_sales_data WHERE date LIKE '2025-12%' GROUP BY registrator_type;
-- WB_Sales: 5436, YM_Order: 1133, YM_Returns: 34
-- OZON_FBS: 0 ❌
-- OZON_FBO: 0 ❌

-- После исправления и перепроведения
SELECT registrator_type, COUNT(*) FROM p904_sales_data WHERE date LIKE '2025-12%' GROUP BY registrator_type;
-- WB_Sales: 5436, YM_Order: 1133, YM_Returns: 34
-- OZON_FBS: ~900+ ✓
-- OZON_FBO: ~300+ ✓
-- OZON_Returns: ~141 ✓
```
