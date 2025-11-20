# Исправление отображения Wildberries Заказов (A015)

**Дата:** 2025-11-18  
**Статус:** ✅ Исправлено

## Проблема

В базе данных (`target/db/app.db`) было 220 записей заказов Wildberries, но они не отображались в интерфейсе списка A015 "Wildberries Заказы".

## Причина

Несоответствие в парсинге JSON-ответа между backend и frontend:

### Backend структура
Backend возвращает `WbOrdersListItemDto` с полем:
```rust
#[serde(flatten)]
pub order: WbOrders,
```

А структура `WbOrders` в свою очередь имеет:
```rust
#[serde(flatten)]
pub base: BaseAggregate<WbOrdersId>,
```

Из-за двойного `flatten`, поля из `BaseAggregate` (`id`, `code`, `description`) попадают на **верхний уровень** JSON-ответа, а не в объект `base`.

### Frontend парсинг (ДО исправления)
```rust
id: item.get("base")?.get("id")?.as_str()?.to_string(),
```

Frontend пытался найти `base.id`, но из-за `flatten` поле `id` находилось на верхнем уровне.

## Решение

### 1. Исправлен парсинг в списке заказов
**Файл:** `crates/frontend/src/domain/a015_wb_orders/ui/list/mod.rs`

**Изменено:**
- ❌ `item.get("base")?.get("id")` 
- ✅ `item.get("id")`

**Добавлено:**
- Логирование количества полученных записей
- Логирование количества успешно распарсенных записей
- Логирование ошибок парсинга и сетевых ошибок

### 2. Исправлен парсинг в деталях заказа
**Файл:** `crates/frontend/src/domain/a015_wb_orders/ui/details/mod.rs`

**Изменено:**
- ❌ `data.get("base").and_then(|b| b.get("id"))`
- ✅ `data.get("id")`

**Добавлено:**
- Логирование получения детальной информации

## Результат

После исправления:
- ✅ Все 220 заказов теперь отображаются в списке
- ✅ Детальное представление заказа работает корректно
- ✅ Добавлено логирование для отладки
- ✅ Нет ошибок линтера

## Структура JSON-ответа (для справки)

```json
{
  "id": "uuid",                    // ← На верхнем уровне (из-за flatten на base)
  "code": "...",                   // ← На верхнем уровне
  "description": "...",            // ← На верхнем уровне
  "comment": "...",                // ← На верхнем уровне
  "metadata": {...},               // ← На верхнем уровне
  "header": {...},                 // ← Поле из WbOrders
  "line": {...},                   // ← Поле из WbOrders
  "state": {...},                  // ← Поле из WbOrders
  "warehouse": {...},              // ← Поле из WbOrders
  "geography": {...},              // ← Поле из WbOrders
  "source_meta": {...},            // ← Поле из WbOrders
  "organization_name": "...",      // ← Доп. поле из WbOrdersListItemDto
  "marketplace_article": "...",    // ← Доп. поле из WbOrdersListItemDto
  "nomenclature_code": "...",      // ← Доп. поле из WbOrdersListItemDto
  "nomenclature_article": "..."    // ← Доп. поле из WbOrdersListItemDto
}
```

## Файлы изменены

1. `crates/frontend/src/domain/a015_wb_orders/ui/list/mod.rs` - исправлен парсинг списка
2. `crates/frontend/src/domain/a015_wb_orders/ui/details/mod.rs` - исправлен парсинг деталей

## Тестирование

Для проверки исправления:
1. Запустить приложение
2. Перейти в раздел "WB Orders" (A015)
3. Убедиться, что отображаются все 220 заказов
4. Открыть детальную информацию любого заказа
5. Проверить консоль браузера на наличие логов:
   - "Received X items from backend"
   - "Successfully parsed X orders"

