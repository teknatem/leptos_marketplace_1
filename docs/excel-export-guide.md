# Руководство по добавлению экспорта в Excel

## Обзор

В проекте реализован универсальный механизм экспорта данных из списков агрегатов в Excel/CSV формат с поддержкой кириллицы (UTF-8 BOM).

## Компоненты системы

### 1. Модуль экспорта (`frontend/src/shared/export.rs`)

Содержит:
- **Trait `ExcelExportable`** - интерфейс для типов данных, которые можно экспортировать
- **Функция `export_to_excel()`** - универсальная функция экспорта с созданием и скачиванием CSV файла

### 2. Иконка Excel (`frontend/src/shared/icons.rs`)

Добавлена векторная иконка "excel" для использования на кнопках экспорта.

## Как добавить экспорт в существующий список

### Шаг 1: Импортировать необходимые модули

В файле `mod.rs` вашего списка добавьте импорты:

```rust
use crate::shared::export::{export_to_excel, ExcelExportable};
use crate::shared::icons::icon;
```

### Шаг 2: Реализовать трейт `ExcelExportable` для структуры Row

Пример для `MarketplaceProductRow`:

```rust
impl ExcelExportable for MarketplaceProductRow {
    fn headers() -> Vec<&'static str> {
        vec![
            "Код",
            "Маркетплейс",
            "Наименование",
            "Артикул",
            "SKU",
            "Штрихкод",
            "Цена",
            "Остаток",
            "Связь 1С",
            "Номенклатура 1С",
        ]
    }

    fn to_csv_row(&self) -> Vec<String> {
        vec![
            self.code.clone(),
            self.marketplace_name.clone(),
            self.product_name.clone(),
            self.art.clone(),
            self.marketplace_sku.clone(),
            self.barcode.clone().unwrap_or_else(|| "-".to_string()),
            self.price.clone(),
            self.stock.clone(),
            if self.nomenclature_id.is_some() { "Да" } else { "Нет" }.to_string(),
            self.nomenclature_name.clone().unwrap_or_else(|| "-".to_string()),
        ]
    }
}
```

**Важно:**
- `headers()` - возвращает заголовки колонок (статический массив строк)
- `to_csv_row()` - преобразует одну строку данных в массив строк для CSV
- Порядок полей в `to_csv_row()` должен совпадать с порядком в `headers()`

### Шаг 3: Добавить обработчик экспорта в компонент

В функции компонента списка добавьте обработчик:

```rust
let handle_export = move || {
    // Получаем текущие отфильтрованные элементы
    let filtered_items: Vec<YourRowType> = items
        .get()
        .into_iter()
        // Примените ваши фильтры здесь, если есть
        .filter(|row| {
            // Ваша логика фильтрации
            true
        })
        .collect();

    if filtered_items.is_empty() {
        if let Some(win) = web_sys::window() {
            let _ = win.alert_with_message("Нет данных для экспорта");
        }
        return;
    }

    // Формируем имя файла
    let filename = "ваши_данные.csv".to_string();

    // Экспортируем данные
    if let Err(e) = export_to_excel(&filtered_items, &filename) {
        if let Some(win) = web_sys::window() {
            let _ = win.alert_with_message(&format!("Ошибка экспорта: {}", e));
        }
    }
};
```

### Шаг 4: Добавить кнопку в UI

В разметке компонента добавьте кнопку:

```rust
<button class="btn btn-success" on:click=move |_| handle_export()>
    {icon("excel")}
    {"Excel"}
</button>
```

## Пример: Список товаров маркетплейса

Полный рабочий пример можно посмотреть в:
- `crates/frontend/src/domain/a007_marketplace_product/ui/list/mod.rs`

## Особенности реализации

### Формат CSV
- Разделитель: точка с запятой (`;`)
- Кодировка: UTF-8 с BOM (для корректного отображения кириллицы в Excel)
- Экранирование: автоматическое экранирование ячеек, содержащих разделители и кавычки

### Экспорт с учетом фильтров
Обработчик `handle_export` должен учитывать текущее состояние фильтров в списке, экспортируя только видимые пользователю данные.

### Динамическое имя файла
Рекомендуется формировать имя файла на основе:
- Текущих фильтров (например, `товары_Ozon.csv` для фильтра по маркетплейсу)
- Даты экспорта
- Типа данных

## Стилизация кнопки

Используйте класс `btn btn-success` для зеленой кнопки или создайте специальный класс для Excel кнопок.

## Расширение функциональности

Для добавления других форматов экспорта (XLSX, JSON) можно:
1. Расширить трейт `ExcelExportable` дополнительными методами
2. Добавить новые функции экспорта в модуль `export.rs`
3. Добавить соответствующие зависимости в `Cargo.toml`
