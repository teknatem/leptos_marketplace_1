# Aggregate Picker System

Универсальная система выбора агрегатов с табличным отображением. Открытие в модалке делается через `ModalStackService`.

## Структура

```
picker_aggregate/
├── mod.rs          # Модуль с экспортами и документацией
├── traits.rs       # AggregatePickerResult + TableDisplayable
├── component.rs    # GenericAggregatePicker компонент
└── README.md       # Эта документация
```

## Быстрый старт

### 1. Модальные окна

В проекте модалки централизованы через `crate::shared::modal_stack::ModalStackService` (предоставляется в `App`).

### 2. Создание типа для пикера

```rust
use crate::shared::picker_aggregate::{AggregatePickerResult, TableDisplayable};

#[derive(Clone)]
pub struct MyPickerItem {
    pub id: String,
    pub code: String,
    pub description: String,
}

impl AggregatePickerResult for MyPickerItem {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn display_name(&self) -> String {
        self.description.clone()
    }
}

impl TableDisplayable for MyPickerItem {
    fn code(&self) -> String {
        self.code.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}
```

### 3. Использование пикера

```rust
use crate::shared::picker_aggregate::{GenericAggregatePicker, Modal, ModalService};

#[component]
pub fn MyPicker<F, G>(
    initial_selected_id: Option<String>,
    on_confirm: F,
    on_cancel: G,
) -> impl IntoView
where
    F: Fn(Option<MyPickerItem>) + 'static + Clone + Send,
    G: Fn(()) + 'static + Clone + Send,
{
    let (items, set_items) = signal::<Vec<MyPickerItem>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);

    // Загрузка данных
    wasm_bindgen_futures::spawn_local(async move {
        match fetch_items().await {
            Ok(data) => set_items.set(data),
            Err(e) => set_error.set(Some(e)),
        }
        set_loading.set(false);
    });

    view! {
        <GenericAggregatePicker
            items=items
            error=error
            loading=loading
            initial_selected_id=initial_selected_id
            on_confirm=on_confirm
            on_cancel=on_cancel
            title="Выбор элемента".to_string()
        />
    }
}
```

### 4. Использование с модальным окном (ModalStackService)

```rust
#[component]
pub fn MyComponent() -> impl IntoView {
    let modal_stack = use_context::<ModalStackService>().expect("ModalStackService not found");

    let handle_confirm = move |selected: Option<MyPickerItem>| {
        modal.hide();
        set_show_picker.set(false);
        if let Some(item) = selected {
            // Обработка выбранного элемента
        }
    };

    let handle_cancel = move |_| {
        modal.hide();
        set_show_picker.set(false);
    };

    view! {
        <button on:click=move |_| {
            modal_stack.push_with_frame(
                Some("max-width: min(1100px, 95vw); width: min(1100px, 95vw);".to_string()),
                Some("picker-modal".to_string()),
                move |handle| {
                    view! {
                        <MyPicker
                            initial_selected_id=None
                            on_confirm=move |selected| { handle_confirm(selected); handle.close(); }
                            on_cancel=move |_| { handle_cancel(()); handle.close(); }
                        />
                    }
                    .into_any()
                },
            );
        }>
            "Открыть пикер"
        </button>
    }
}
```

## Особенности

### Автоскролл
Пикер автоматически прокручивается к предвыбранному элементу при открытии.

### Выбор элемента
- **Одинарный клик**: выделить элемент
- **Двойной клик**: выбрать и подтвердить сразу
- **Кнопка "Выбрать"**: подтвердить выбранный элемент

### Состояния загрузки
- `loading=true`: показывает "Загрузка..."
- `error=Some(msg)`: показывает сообщение об ошибке
- `items` пустой: показывает "Нет доступных элементов"

## Поиск в проекте

### Glob паттерны
```bash
# Найти все файлы пикеров
**/picker_*/**

# Найти aggregate пикеры
**/picker_aggregate/**

# Найти domain пикеры
domain/**/picker/**
```

### Grep паттерны
```bash
# Найти использование пикеров
use.*picker_

# Найти модалки (новый механизм)
ModalStackService|push_with_frame|ModalHost|ModalFrame

# Найти трейты
AggregatePickerResult|TableDisplayable
```

## Примеры в кодовой базе

- **Organization Picker**: `domain/a002_organization/ui/picker/mod.rs`
- **Marketplace Picker** (кастомный): `domain/a005_marketplace/ui/picker/mod.rs`
- **Использование**: `domain/a006_connection_mp/ui/details/view.rs`

## Расширение

Для создания других типов пикеров (например, `picker_enum`, `picker_date`):

1. Создайте каталог `shared/picker_{name}/`
2. Определите необходимые трейты и компоненты
3. Используйте префикс `picker_` для единообразия
4. Документируйте в README.md
