# Исправление бесконечного цикла в Wildberries Sales Details (Links Tab)

## Проблема

При переходе на вкладку **Links** в деталях Wildberries Sales происходил бесконечный цикл запросов к эндпоинту `/api/p903/finance-report/search-by-srid`.

## Причина

В компоненте `WbSalesDetail` был Effect, который отслеживал состояние `linked_finance_reports`:

```rust
Effect::new(move || {
    let tab = active_tab.get();
    if tab == "links" || tab == "line" {
        if let Some(sale_data) = sale.get() {
            let srid_val = sale_data.header.document_no.clone();
            if !srid_val.is_empty() && linked_finance_reports.get().is_empty() {
                // Запрос к API
            }
        }
    }
});
```

**Сценарий бесконечного цикла:**
1. Effect срабатывает, проверяет `linked_finance_reports.get().is_empty()` → `true`
2. Отправляется запрос к API
3. Запрос возвращает пустой массив (нет связанных отчетов) или ошибку
4. `set_links_loading.set(false)` обновляет состояние
5. `linked_finance_reports` остается **пустым**
6. Effect срабатывает снова (из-за изменения `links_loading`)
7. Условие `linked_finance_reports.get().is_empty()` снова `true`
8. **Цикл повторяется бесконечно** ♾️

## Решение

Добавлен флаг `links_fetch_attempted`, который предотвращает повторные запросы после первой попытки:

### Изменения в `crates/frontend/src/domain/a012_wb_sales/ui/details/mod.rs`

1. **Добавлен новый сигнал:**
```rust
let (links_fetch_attempted, set_links_fetch_attempted) = signal(false);
```

2. **Изменено условие в Effect:**
```rust
// Было:
if !srid_val.is_empty() && linked_finance_reports.get().is_empty() {

// Стало:
if !srid_val.is_empty() && !links_fetch_attempted.get() {
```

3. **Флаг устанавливается перед запросом:**
```rust
set_links_fetch_attempted.set(true);
```

## Результат

- ✅ Запрос к API выполняется **только один раз** при первом открытии вкладки Links
- ✅ Если отчеты не найдены (пустой массив), повторный запрос **не выполняется**
- ✅ Если возникла ошибка, повторный запрос **не выполняется**
- ✅ Бесконечный цикл полностью **устранен**

## Тестирование

Для проверки исправления:
1. Запустить приложение
2. Открыть детали любого документа Wildberries Sales
3. Перейти на вкладку **Links**
4. Проверить в DevTools (вкладка Network), что запрос к `/api/p903/finance-report/search-by-srid` выполняется **только один раз**

## Файлы изменены

- `crates/frontend/src/domain/a012_wb_sales/ui/details/mod.rs`

