# Руководство по добавлению поиска и сортировки в списки

## Обзор

В проекте реализованы универсальные механизмы для добавления поиска и сортировки в табличные списки агрегатов. Все утилиты находятся в модуле `frontend/src/shared/list_utils.rs`.

## Компоненты системы

### 1. Traits

**`Searchable`** - для типов данных с возможностью поиска:
- `matches_filter(&self, filter: &str) -> bool` - проверка соответствия фильтру
- `get_field_value(&self, field: &str) -> Option<String>` - получение значения поля для подсветки (опционально)

**`Sortable`** - для типов данных с возможностью сортировки:
- `compare_by_field(&self, other: &Self, field: &str) -> Ordering` - сравнение по полю

### 2. Компоненты

**`SearchInput`** - переиспользуемый компонент поиска с:
- Debounce (300ms)
- Визуальной индикацией активного фильтра (желтый фон)
- Кнопкой очистки (иконка X)
- Минимум 3 символа для активации поиска

### 3. Утилиты

- `highlight_matches(text, filter)` - подсветка совпадений в тексте
- `get_sort_indicator(current_field, field, ascending)` - индикаторы сортировки (▲▼⇅)
- `sort_list()` - сортировка списка по полю
- `filter_list()` - фильтрация списка по запросу

## Как добавить поиск и сортировку в существующий список

### Шаг 1: Импорты

```rust
use crate::shared::list_utils::{
    highlight_matches,
    Searchable,
    Sortable,
    SearchInput,
    get_sort_indicator
};
use std::cmp::Ordering;
```

### Шаг 2: Реализовать Searchable для Row типа

```rust
impl Searchable for YourRow {
    fn matches_filter(&self, filter: &str) -> bool {
        let filter_lower = filter.to_lowercase();

        self.field1.to_lowercase().contains(&filter_lower)
            || self.field2.to_lowercase().contains(&filter_lower)
            // Добавьте все поля для поиска
    }

    fn get_field_value(&self, field: &str) -> Option<String> {
        match field {
            "field1" => Some(self.field1.clone()),
            "field2" => Some(self.field2.clone()),
            _ => None,
        }
    }
}
```

**Важно:**
- Поиск case-insensitive
- Для Option полей используйте `.as_ref().map_or(false, |v| ...)`

### Шаг 3: Реализовать Sortable для Row типа

```rust
impl Sortable for YourRow {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "field1" => self.field1.to_lowercase().cmp(&other.field1.to_lowercase()),
            "field2" => self.field2.to_lowercase().cmp(&other.field2.to_lowercase()),
            // Для чисел:
            "price" => {
                let a = self.price.parse::<f64>().unwrap_or(0.0);
                let b = other.price.parse::<f64>().unwrap_or(0.0);
                a.partial_cmp(&b).unwrap_or(Ordering::Equal)
            },
            _ => Ordering::Equal,
        }
    }
}
```

**Важно:**
- Для строк используйте `.to_lowercase()` для case-insensitive сортировки
- Для чисел парсите из строки, если они хранятся как String
- Для Option полей используйте `.unwrap_or_default()`

### Шаг 4: Добавить signals в компонент

```rust
#[component]
pub fn YourList() -> impl IntoView {
    let (items, set_items) = signal::<Vec<YourRow>>(Vec::new());

    // Добавьте signals для поиска и сортировки
    let (filter_text, set_filter_text) = signal(String::new());
    let (sort_field, set_sort_field) = signal::<String>("default_field".to_string());
    let (sort_ascending, set_sort_ascending) = signal(true);

    // ... остальной код
}
```

### Шаг 5: Создать функцию фильтрации и сортировки

```rust
let get_filtered_sorted_items = move || -> Vec<YourRow> {
    let mut result: Vec<YourRow> = items
        .get()
        .into_iter()
        // Добавьте дополнительные фильтры если нужно
        .filter(|row| {
            // Ваши дополнительные фильтры
            true
        })
        // Поиск
        .filter(|row| {
            let filter = filter_text.get();
            if filter.trim().is_empty() || filter.trim().len() < 3 {
                true
            } else {
                row.matches_filter(&filter)
            }
        })
        .collect();

    // Сортировка
    let field = sort_field.get();
    let ascending = sort_ascending.get();
    result.sort_by(|a, b| {
        let cmp = a.compare_by_field(b, &field);
        if ascending { cmp } else { cmp.reverse() }
    });

    result
};
```

### Шаг 6: Создать обработчик сортировки

```rust
let toggle_sort = move |field: &'static str| {
    move |_| {
        if sort_field.get() == field {
            set_sort_ascending.update(|v| *v = !*v);
        } else {
            set_sort_field.set(field.to_string());
            set_sort_ascending.set(true);
        }
    }
};
```

### Шаг 7: Добавить SearchInput в UI

```rust
view! {
    <div class="header">
        <h2>{"Заголовок"}</h2>
        <div class="header-actions">
            <SearchInput
                value=filter_text
                on_change=Callback::new(move |val: String| set_filter_text.set(val))
                placeholder="Поиск...".to_string()
            />
            // ... остальные кнопки
        </div>
    </div>
}
```

### Шаг 8: Сделать заголовки таблицы кликабельными

```rust
<thead>
    <tr>
        <th
            class="cursor-pointer user-select-none"
            on:click=toggle_sort("field_name")
            title="Сортировать"
        >
            {move || format!("Заголовок{}", get_sort_indicator(&sort_field.get(), "field_name", sort_ascending.get()))}
        </th>
        // ... остальные заголовки
    </tr>
</thead>
```

### Шаг 9: Обновить tbody для использования отфильтрованных данных

```rust
<tbody>
    {move || {
        let filtered = get_filtered_sorted_items();
        let current_filter = filter_text.get();

        filtered.into_iter().map(|row| {
            // Подсветка совпадений (опционально)
            let field_view = if current_filter.len() >= 3 {
                highlight_matches(&row.field, &current_filter)
            } else {
                view! { <span>{row.field.clone()}</span> }.into_any()
            };

            view! {
                <tr>
                    <td>{field_view}</td>
                    // ... остальные ячейки
                </tr>
            }
        }).collect_view()
    }}
</tbody>
```

## Пример: MarketplaceProductList

Полный рабочий пример смотрите в:
- [frontend/src/domain/a007_marketplace_product/ui/list/mod.rs](../crates/frontend/src/domain/a007_marketplace_product/ui/list/mod.rs)

## CSS классы

Убедитесь, что в вашем CSS есть:

```css
.cursor-pointer {
    cursor: pointer;
}

.user-select-none {
    user-select: none;
}
```

## Особенности реализации

### Минимальная длина поиска
Поиск активируется только при вводе 3+ символов для повышения производительности.

### Debounce
SearchInput использует debounce 300ms для уменьшения количества обновлений при вводе.

### Подсветка совпадений
Функция `highlight_matches()` подсвечивает все вхождения поискового запроса оранжевым цветом.

### Индикаторы сортировки
- `▲` - сортировка по возрастанию
- `▼` - сортировка по убыванию
- `⇅` - колонка не активна

### Комбинирование фильтров
Поиск можно комбинировать с другими фильтрами (например, dropdown). Все фильтры применяются последовательно в `get_filtered_sorted_items()`.

## Производительность

- Поиск и сортировка выполняются на клиенте (WASM)
- Для больших списков (1000+ элементов) рассмотрите серверную пагинацию
- Debounce снижает нагрузку при вводе

## Расширение функциональности

### Добавление новых полей для поиска
Просто добавьте проверку в `matches_filter()`:

```rust
|| self.new_field.to_lowercase().contains(&filter_lower)
```

### Добавление новых полей для сортировки
Добавьте case в `compare_by_field()`:

```rust
"new_field" => self.new_field.to_lowercase().cmp(&other.new_field.to_lowercase()),
```

### Сохранение состояния сортировки
Для сохранения выбранного поля и направления сортировки между сессиями используйте localStorage через web-sys.

## Troubleshooting

### Поиск не работает
- Проверьте, что введено минимум 3 символа
- Убедитесь, что `matches_filter()` проверяет нужные поля
- Проверьте, что фильтр применяется в `get_filtered_sorted_items()`

### Сортировка работает неправильно
- Для строк используйте `.to_lowercase()`
- Для чисел парсите значение перед сравнением
- Проверьте, что вернули правильный `Ordering`

### Подсветка не появляется
- Убедитесь, что используете `highlight_matches()`
- Проверьте, что передаете текущий фильтр
- Убедитесь, что фильтр содержит 3+ символа
