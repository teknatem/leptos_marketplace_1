# Реализация универсального механизма выбора агрегатов

## Обзор

Система универсального выбора агрегатов позволяет выбирать любые сущности (организации, маркетплейсы, контрагенты и т.д.) через единообразный интерфейс с модальными окнами.

## Архитектура

### 1. Компоненты системы

```
┌─────────────────────────────────────────────────────────────┐
│                      ModalService                            │
│  (Централизованное управление модальными окнами)             │
│  - RwSignal<bool> для видимости                             │
│  - show() / hide()                                           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Modal Component                           │
│  (Контейнер для модального окна)                            │
│  - Overlay с затемнением                                     │
│  - Children для произвольного контента                       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              GenericAggregatePicker<T>                       │
│  (Универсальный компонент выбора)                           │
│  - Табличное отображение                                     │
│  - Предвыбор элемента (initial_selected_id)                 │
│  - Клик для выбора, двойной клик для подтверждения          │
│  - Автоскролл к выбранному                                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│          Специализированные пикеры                           │
│  - OrganizationPicker                                        │
│  - MarketplacePicker (старая реализация)                    │
│  - CounterpartyPicker (будущее)                             │
│  - NomenclaturePicker (будущее)                             │
└─────────────────────────────────────────────────────────────┘
```

### 2. Трейты для агрегатов

#### AggregatePickerResult
Базовый трейт для элементов, которые можно выбрать:

```rust
pub trait AggregatePickerResult {
    fn id(&self) -> String;
    fn display_name(&self) -> String;
}
```

#### TableDisplayable
Расширенный трейт для табличного отображения:

```rust
pub trait TableDisplayable: AggregatePickerResult {
    fn code(&self) -> String;
    fn description(&self) -> String;
}
```

## Реализация компонентов

### 1. ModalService

**Файл**: `crates/frontend/src/layout/modal_service.rs`

```rust
#[derive(Clone, Copy)]
pub struct ModalService {
    is_visible: RwSignal<bool>,
}

impl ModalService {
    pub fn new() -> Self {
        Self {
            is_visible: RwSignal::new(false),
        }
    }

    pub fn show(&self) {
        self.is_visible.set(true);
    }

    pub fn hide(&self) {
        self.is_visible.set(false);
    }

    pub fn is_open(&self) -> bool {
        self.is_visible.get()
    }
}
```

**Ключевые особенности**:
- Использует `RwSignal<bool>` вместо хранения view
- Copy-семантика для удобного использования в замыканиях
- Предоставляется через контекст Leptos

### 2. Modal Component

**Файл**: `crates/frontend/src/layout/modal_service.rs`

```rust
#[component]
pub fn Modal(children: ChildrenFn) -> impl IntoView {
    let modal = use_context::<ModalService>()
        .expect("ModalService not found");

    view! {
        <Show when=move || modal.is_visible.get()>
            <div class="modal-overlay" on:click=move |_| modal.hide()>
                <div class="modal-content" on:click=|e| e.stop_propagation()>
                    {children()}
                </div>
            </div>
        </Show>
    }
}
```

**Ключевые особенности**:
- Использует `ChildrenFn` для многократного вызова
- `Show` компонент для условного рендеринга
- Клик вне контента закрывает модальное окно
- `stop_propagation()` предотвращает закрытие при клике внутри

### 3. GenericAggregatePicker

**Файл**: `crates/frontend/src/shared/aggregate_picker.rs`

#### Сигнатура компонента

```rust
#[component]
pub fn GenericAggregatePicker<T>(
    items: ReadSignal<Vec<T>>,
    #[prop(optional)]
    error: Option<ReadSignal<Option<String>>>,
    #[prop(optional)]
    loading: Option<ReadSignal<bool>>,
    initial_selected_id: Option<String>,
    on_confirm: impl Fn(Option<T>) + 'static + Clone + Send,
    on_cancel: impl Fn(()) + 'static + Clone + Send,
    #[prop(optional)]
    title: Option<String>,
) -> impl IntoView
where
    T: TableDisplayable + Clone + Send + Sync + 'static,
```

#### Ключевые механизмы

**1. Управление выбором**:
```rust
let (selected_id, set_selected_id) =
    signal::<Option<String>>(initial_selected_id.clone());

let handle_row_click = move |item_id: String| {
    set_selected_id.set(Some(item_id));
};
```

**2. Реактивное выделение строк**:
```rust
view! {
    <tr
        class="picker-row"
        class:selected=move || selected_id.get().as_ref() == Some(&item_id)
        on:click=move |_| handle_row_click(item_id.clone())
        on:dblclick=move |_| on_confirm(Some(item.clone()))
    >
        <td>{item.description()}</td>
        <td>{item.code()}</td>
    </tr>
}
```

**Важно**: `class:selected` использует замыкание `move ||` для реактивности!

**3. Автоскролл к выбранному элементу**:
```rust
let selected_row_ref = NodeRef::<Tr>::new();

Effect::new(move |_| {
    if selected_id.get().is_some() && !loading_signal.get() {
        if let Some(element) = selected_row_ref.get() {
            let _ = element.scroll_into_view_with_bool(true);
        }
    }
});

// В рендеринге:
<tr node_ref=if is_initially_selected { selected_row_ref } else { NodeRef::new() }>
```

**4. Обработка подтверждения**:
```rust
let handle_confirm = {
    let on_confirm = on_confirm.clone();
    move |_| {
        let selected = selected_id.get();
        if let Some(id) = selected {
            items.with(|items_vec| {
                if let Some(item) = items_vec.iter().find(|i| i.id() == id) {
                    on_confirm(Some(item.clone()));
                } else {
                    on_confirm(None);
                }
            });
        } else {
            on_confirm(None);
        }
    }
};
```

## Создание специализированного пикера

### Пример: OrganizationPicker

**Файл**: `crates/frontend/src/domain/a002_organization/ui/picker.rs`

#### Шаг 1: Создать item-тип с трейтами

```rust
#[derive(Clone, Debug)]
pub struct OrganizationPickerItem {
    pub id: String,
    pub code: String,
    pub description: String,
    pub full_name: String,
    pub inn: String,
}

impl From<Organization> for OrganizationPickerItem {
    fn from(org: Organization) -> Self {
        Self {
            id: org.base.id.as_string(),
            code: org.base.code,
            description: org.base.description,
            full_name: org.full_name,
            inn: org.inn,
        }
    }
}

impl AggregatePickerResult for OrganizationPickerItem {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn display_name(&self) -> String {
        self.description.clone()
    }
}

impl TableDisplayable for OrganizationPickerItem {
    fn code(&self) -> String {
        self.code.clone()
    }

    fn description(&self) -> String {
        format!("{} (ИНН: {})", self.description, self.inn)
    }
}
```

#### Шаг 2: Создать компонент-обёртку

```rust
#[component]
pub fn OrganizationPicker<F, G>(
    initial_selected_id: Option<String>,
    on_confirm: F,
    on_cancel: G,
) -> impl IntoView
where
    F: Fn(Option<OrganizationPickerItem>) + 'static + Clone + Send,
    G: Fn(()) + 'static + Clone + Send,
{
    let (items, set_items) = signal::<Vec<OrganizationPickerItem>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);

    // Загрузка данных
    wasm_bindgen_futures::spawn_local(async move {
        match fetch_organizations().await {
            Ok(orgs) => {
                let picker_items: Vec<OrganizationPickerItem> =
                    orgs.into_iter().map(Into::into).collect();
                set_items.set(picker_items);
                set_error.set(None);
            }
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
            title="Выбор организации".to_string()
        />
    }
}
```

**Важно**:
- `initial_selected_id` БЕЗ `#[prop(optional)]` (это `Option<String>`)
- `error` и `loading` С `#[prop(optional)]` (это `Option<ReadSignal<...>>`)

#### Шаг 3: Функция загрузки данных

```rust
async fn fetch_organizations() -> Result<Vec<Organization>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let api_base = || {
        let window = web_sys::window().expect("no window");
        let location = window.location();
        let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
        let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
        format!("{}//{}:3000", protocol, hostname)
    };

    let url = format!("{}/api/organization", api_base());
    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("{e:?}"))?;

    request.headers().set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(
        window.fetch_with_request(&request)
    ).await.map_err(|e| format!("{e:?}"))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(
        resp.text().map_err(|e| format!("{e:?}"))?
    ).await.map_err(|e| format!("{e:?}"))?;

    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Vec<Organization> = serde_json::from_str(&text)
        .map_err(|e| format!("{e}"))?;
    Ok(data)
}
```

## Использование в форме

**Файл**: `crates/frontend/src/domain/a006_connection_mp/ui/details/view.rs`

### Шаг 1: Получить ModalService из контекста

```rust
let modal = use_context::<ModalService>().expect("ModalService not found");
```

### Шаг 2: Создать сигналы для хранения выбранного значения

```rust
let (organization_name, set_organization_name) = signal(String::new());
let (organization_id, set_organization_id) = signal::<Option<String>>(None);
let (show_organization_picker, set_show_organization_picker) = signal(false);
```

### Шаг 3: Создать обработчики

```rust
let handle_organization_selected = move |selected: Option<OrganizationPickerItem>| {
    modal.hide();
    set_show_organization_picker.set(false);
    if let Some(item) = selected {
        set_organization_id.set(Some(item.id.clone()));
        set_organization_name.set(item.description.clone());
        set_form.update(|f| f.organization = item.description.clone());
    }
};

let handle_organization_cancel = move |_| {
    modal.hide();
    set_show_organization_picker.set(false);
};
```

### Шаг 4: Добавить UI элементы

```rust
// Поле для отображения выбранного значения
<div class="form-group">
    <label for="organization">{"Организация"}</label>
    <div style="display: flex; gap: 8px; align-items: center;">
        <input
            type="text"
            id="organization"
            prop:value={move || organization_name.get()}
            readonly
            placeholder="Выберите организацию"
            style="flex: 1;"
        />
        <button
            type="button"
            class="btn btn-secondary"
            on:click=move |_| {
                set_show_organization_picker.set(true);
                modal.show();
            }
        >
            {icon("search")}
            {"Выбрать"}
        </button>
    </div>
</div>

// Модальное окно с пикером
<Modal>
    {move || {
        if show_organization_picker.get() {
            let selected_id = organization_id.get();
            view! {
                <OrganizationPicker
                    initial_selected_id=selected_id
                    on_confirm=handle_organization_selected
                    on_cancel=handle_organization_cancel
                />
            }.into_any()
        } else {
            view! { <></> }.into_any()
        }
    }}
</Modal>
```

## CSS стили

**Файл**: `crates/frontend/styles/3-components/modals.css`

### Основные стили пикера

```css
.picker-container {
    background: var(--color-bg-body);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
    width: 600px;
    max-width: 90vw;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.picker-header {
    padding: var(--space-xl);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-surface);
}

.picker-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-lg);
    min-height: 0;
}

.picker-actions {
    padding: var(--space-xl);
    border-top: 1px solid var(--color-border);
    background: var(--color-bg-surface);
    display: flex;
    gap: var(--space-lg);
    justify-content: flex-end;
}
```

### Стили для табличного варианта

```css
.picker-table {
    width: 100%;
    border-collapse: collapse;
}

.picker-table thead th {
    background: var(--color-bg-surface);
    padding: var(--space-md);
    text-align: left;
    border-bottom: 2px solid var(--color-border);
    font-weight: 600;
    position: sticky;
    top: 0;
    z-index: 1;
}

.picker-table tbody tr.picker-row {
    cursor: pointer;
    transition: background-color 0.15s ease;
}

.picker-table tbody tr.picker-row:hover {
    background: var(--color-bg-hover);
}

.picker-table tbody tr.picker-row.selected {
    background: rgba(59, 130, 246, 0.15);
    font-weight: 500;
}

.picker-table tbody tr.picker-row.selected:hover {
    background: rgba(59, 130, 246, 0.2);
}

.picker-table tbody td {
    padding: var(--space-md);
    border-bottom: 1px solid var(--color-border);
}
```

## Особенности реализации

### 1. Реактивность

**Проблема**: При первой реализации класс `.selected` не обновлялся при клике.

**Причина**: Условие вычислялось один раз при создании элемента:
```rust
// ❌ Неправильно
let is_selected = selected_id.get().as_ref() == Some(&item_id);
class:selected=is_selected
```

**Решение**: Использовать замыкание для реактивности:
```rust
// ✅ Правильно
class:selected=move || selected_id.get().as_ref() == Some(&item_id)
```

### 2. Владение и клонирование

**Проблема**: `item_id` перемещается в первое замыкание и недоступен для других.

**Решение**: Клонировать для каждого использования:
```rust
let item_id = item.id();
let item_id_for_selected = item_id.clone();
let item_id_for_click = item_id.clone();

class:selected=move || selected_id.get().as_ref() == Some(&item_id_for_selected)
on:click=move |_| handle_row_click(item_id_for_click.clone())
```

### 3. Параметры компонента с #[prop(optional)]

**Правило**: Если параметр имеет тип `Option<T>` и НЕ помечен `#[prop(optional)]`:
- Передавать нужно `Option<T>` напрямую
- Можно передать `None` или `Some(value)`

**Правило**: Если параметр имеет тип `Option<T>` и помечен `#[prop(optional)]`:
- Leptos трансформирует параметр
- Можно не передавать атрибут вообще (будет `default()`)
- Если передаёте, передавайте значение внутреннего типа

```rust
// Без #[prop(optional)]
initial_selected_id: Option<String>,

// Использование:
<Picker initial_selected_id=Some("id123".to_string()) />
<Picker initial_selected_id=None />

// С #[prop(optional)]
#[prop(optional)]
error: Option<ReadSignal<Option<String>>>,

// Использование:
<Picker error=some_signal />  // передаём ReadSignal, Leptos обернёт в Option
<Picker />  // не передаём, будет None
```

### 4. ChildrenFn vs Children

**ChildrenFn** (`Fn() -> View`):
- Можно вызывать многократно
- Подходит для динамического контента
- Используется в `Modal`

**Children** (`View`):
- Вызывается один раз
- Нельзя использовать в реактивных контекстах

## Поток данных

```
1. Пользователь кликает "Выбрать"
   └─> set_show_organization_picker(true)
   └─> modal.show()

2. Modal рендерится с OrganizationPicker
   └─> fetch_organizations() загружает данные
   └─> Создаётся GenericAggregatePicker с items

3. GenericAggregatePicker отображает таблицу
   └─> initial_selected_id передаётся в selected_id
   └─> Элемент с совпадающим id получает класс .selected
   └─> Автоскролл к элементу через Effect

4. Пользователь кликает на строку
   └─> handle_row_click(item_id)
   └─> set_selected_id(Some(item_id))
   └─> Реактивно обновляется class:selected на всех строках
   └─> Кнопка "Выбрать" становится активной

5. Пользователь кликает "Выбрать" или делает двойной клик
   └─> handle_confirm() или on_dblclick
   └─> Находит элемент по selected_id
   └─> Вызывает on_confirm(Some(item))

6. handle_organization_selected получает выбранный item
   └─> Сохраняет id и description в сигналы
   └─> Обновляет форму
   └─> modal.hide()
   └─> set_show_organization_picker(false)
```

## Рекомендации по расширению

### Для добавления нового пикера:

1. Создать `{Entity}PickerItem` с трейтами `AggregatePickerResult` и `TableDisplayable`
2. Создать `{Entity}Picker` компонент-обёртку
3. Добавить функцию `fetch_{entities}()` для загрузки данных
4. В форме использовать через `Modal` + сигналы для хранения выбранного значения

### Для изменения отображения:

- Добавить поля в `TableDisplayable::description()`
- Настроить CSS стили `.picker-table`
- Изменить разметку в `GenericAggregatePicker`

### Для добавления фильтрации:

- Добавить `RwSignal<String>` для поискового запроса
- Использовать `Memo` для фильтрации items
- Добавить `<input>` в `picker-header`

## Тестирование

### Проверить:
1. ✅ Предвыбор элемента при открытии (initial_selected_id)
2. ✅ Автоскролл к выбранному элементу
3. ✅ Выделение при клике (синий фон)
4. ✅ Курсор pointer при наведении
5. ✅ Hover эффект (подсветка)
6. ✅ Двойной клик для быстрого выбора
7. ✅ Закрытие по клику вне контента
8. ✅ Закрытие по кнопке "Отмена"
9. ✅ Кнопка "Выбрать" активна только при выборе
10. ✅ Отображение ошибок и индикатора загрузки
