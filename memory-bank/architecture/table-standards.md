# Стандарты таблиц (Table Standards)

## Обзор

Единый стандарт для всех списков в системе с двумя уровнями сложности.

**Дата создания:** 2025-12-19  
**Версия:** 1.0

---

## Два типа таблиц

### Сравнительная таблица

| Критерий           | Простая таблица                         | Сложная таблица                   |
| ------------------ | --------------------------------------- | --------------------------------- |
| **Примеры**        | Организации, Маркетплейсы, Номенклатура | Продажи, Заказы, Возвраты         |
| **Записей**        | До 100-200                              | 1000+                             |
| **Обновления**     | Редкие                                  | Частые                            |
| **Пагинация**      | ❌ Нет (все сразу)                      | ✅ Серверная (offset/limit)       |
| **Фильтры**        | Только поиск                            | Расширенная панель с collapse     |
| **Итоги**          | ❌ Нет                                  | ✅ Серверные итоги в header       |
| **Чекбоксы**       | ✅ Да                                   | ✅ Да                             |
| **Сортировка**     | ✅ Клиентская                           | ✅ Серверная                      |
| **Resize колонок** | ❌ Нет                                  | ✅ Да + сохранение в localStorage |
| **Экспорт Excel**  | Опционально                             | ✅ Да                             |
| **Post/Unpost**    | ❌ Нет                                  | ✅ Batch операции                 |

### Когда использовать каждый тип

**Простая таблица:**

- Справочники и классификаторы
- Данные редко меняются
- Количество записей стабильно и невелико (< 200)
- Не требуется сложная фильтрация

**Сложная таблица:**

- Транзакционные данные (документы, операции)
- Большие объёмы данных (1000+)
- Требуется фильтрация по датам и другим параметрам
- Нужны batch операции и аналитика

---

## BEM Методология (ОБЯЗАТЕЛЬНО)

### Правила именования

Все классы таблиц следуют BEM: `.table__element--modifier`

```css
/* Блок */
.table
.table__data
.table__head

/* Элементы */
.table__row
.table__cell
.table__header-cell
.table__checkbox

/* Модификаторы */
.table__cell--checkbox
.table__cell--right        /* ONLY for cell values, NOT for headers */
.table__row--selected
.table--striped;
```

**ВАЖНО по выравниванию:**
- Заголовки колонок всегда выравниваются **влево** (используйте `.table__header-cell` без модификатора)
- Числовые значения в ячейках выравниваются **вправо** (используйте `.table__cell--right`)
- ❌ НЕ используйте `.table__header-cell--right` - класс устарел

### ❌ Запрещено

```css
.checkbox-cell {
} /* kebab-case без BEM */
.tableCell {
} /* camelCase */
.cell {
} /* без префикса блока */
```

### ✅ Правильно

```rust
// В Rust компонентах
<td class="table__cell table__cell--checkbox">
<tr class="table__row table__row--selected">
```

---

## Простая таблица

### Обязательные элементы

1. **Header**

   - Заголовок списка
   - Кнопка "Создать"
   - Кнопка "Обновить"
   - Кнопка "Удалить выбранные" (disabled если ничего не выбрано)

2. **Таблица**

   - Колонка чекбоксов (первая, 40px)
   - Сортируемые колонки с индикаторами
   - Hover эффект на строках
   - Клик на строку → открыть detail

3. **Модальное окно**
   - Для создания/редактирования записи
   - Открывается по клику на строку или кнопку "Создать"

### Структура кода

```rust
pub mod state;

use leptos::prelude::*;
use crate::shared::modal::Modal;
use crate::domain::aXXX_feature::ui::details::FeatureDetails;

#[derive(Clone, Debug)]
pub struct FeatureRow {
    pub id: String,
    pub code: String,
    pub description: String,
    // ... другие поля
}

#[component]
pub fn FeatureList() -> impl IntoView {
    let (items, set_items) = signal::<Vec<FeatureRow>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (show_modal, set_show_modal) = signal(false);
    let (editing_id, set_editing_id) = signal::<Option<String>>(None);
    let (selected, set_selected) = signal::<HashSet<String>>(HashSet::new());

    let fetch = move || {
        // Загрузить ВСЕ записи (без пагинации)
    };

    let toggle_select = move |id: String, checked: bool| {
        set_selected.update(|s| {
            if checked {
                s.insert(id);
            } else {
                s.remove(&id);
            }
        });
    };

    view! {
        <div class="page">
            <div class="header">
                <div class="header__content">
                    <h1 class="header__title">"Справочник"</h1>
                </div>
                <div class="header__actions">
                    <button class="button button--primary" on:click=move |_| {
                        set_editing_id.set(None);
                        set_show_modal.set(true);
                    }>
                        {icon("plus")} "Создать"
                    </button>
                    <button class="button button--secondary" on:click=move |_| fetch()>
                        {icon("refresh")} "Обновить"
                    </button>
                    <button
                        class="button button--secondary"
                        on:click=move |_| delete_selected()
                        disabled={move || selected.get().is_empty()}
                    >
                        {icon("trash")} {move || format!("Удалить ({})", selected.get().len())}
                    </button>
                </div>
            </div>

            <div class="table">
                <table class="table__data table--striped">
                    <thead class="table__head">
                        <tr>
                            <th class="table__header-cell table__header-cell--checkbox">
                                <input type="checkbox" on:change=toggle_select_all />
                            </th>
                            <th class="table__header-cell table__header-cell--sortable" on:click=toggle_sort("code")>
                                "Код" {get_sort_indicator("code", &sort_field, sort_ascending)}
                            </th>
                            <th class="table__header-cell">"Наименование"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || items.get().into_iter().map(|row| {
                            let id = row.id.clone();
                            let is_selected = selected.get().contains(&id);
                            view! {
                                <tr
                                    class="table__row"
                                    class:table__row--selected=is_selected
                                    on:click=move |_| {
                                        set_editing_id.set(Some(id.clone()));
                                        set_show_modal.set(true);
                                    }
                                >
                                    <td class="table__cell table__cell--checkbox" on:click=|e| e.stop_propagation()>
                                        <input
                                            type="checkbox"
                                            class="table__checkbox"
                                            prop:checked=is_selected
                                            on:change=move |ev| {
                                                let checked = event_target_checked(&ev);
                                                toggle_select(id.clone(), checked);
                                            }
                                        />
                                    </td>
                                    <td class="table__cell">{row.code}</td>
                                    <td class="table__cell">{row.description}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>

            <Show when=move || show_modal.get()>
                <Modal
                    title=move || if editing_id.get().is_some() { "Редактирование" } else { "Создание" }
                    on_close=Callback::new(move |_| {
                        set_show_modal.set(false);
                        set_editing_id.set(None);
                    })
                >
                    <FeatureDetails
                        id=editing_id.get()
                        on_saved=Rc::new(move |_| {
                            set_show_modal.set(false);
                            set_editing_id.set(None);
                            fetch();
                        })
                        on_cancel=Rc::new(move |_| {
                            set_show_modal.set(false);
                            set_editing_id.set(None);
                        })
                    />
                </Modal>
            </Show>
        </div>
    }
}
```

### CSS классы (BEM)

```css
/* Контейнер страницы */
.page {
}

/* Header */
.header {
}
.header__content {
}
.header__title {
}
.header__actions {
}

/* Таблица */
.table {
}
.table__data {
}
.table__head {
}
.table__row {
}
.table__row--selected {
}
.table__cell {
}
.table__cell--checkbox {
}
.table__header-cell {
}
.table__header-cell--checkbox {
}
.table__header-cell--sortable {
}
.table__checkbox {
}

/* Модификатор полосатой таблицы */
.table--striped {
}
```

---

## Сложная таблица

### Обязательные элементы

1. **Page Header (градиентный фон)**

   - Заголовок списка с иконкой
   - Пагинация: `⏮ ◀ "1 / N (total)" ▶ ⏭` + select page_size
   - Кнопки Post/Unpost с счётчиком: `✓ Post (n)` / `✗ Unpost (n)`
   - Кнопка Excel: `📊 Excel`
   - Кнопка обновить

2. **Filter Panel (collapsible)**

   - Фильтр периода: DateInput + DateInput + MonthSelector
   - Дополнительные фильтры (организация, тип и т.п.)
   - Активные фильтры как removable tags
   - Кнопка "Очистить все"

3. **Таблица**
   - Колонка чекбоксов (первая, 40px)
   - Сортируемые колонки
   - **Строка итогов в thead** (sticky, серверные данные)
   - Resize колонок с сохранением в localStorage
   - Переход в detail по клику на строку

### Структура кода

```rust
pub mod state;

use self::state::create_state;
use leptos::prelude::*;
use crate::shared::components::{
    date_input::DateInput,
    month_selector::MonthSelector,
    pagination_controls::PaginationControls,
    table_totals_row::TableTotalsRow,
};
use crate::shared::table_utils::{init_column_resize, was_just_resizing};

const TABLE_ID: &str = "aXXX-feature-table";
const COLUMN_WIDTHS_KEY: &str = "aXXX_feature_column_widths";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<FeatureDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
    pub totals: Option<ServerTotals>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerTotals {
    pub total_records: usize,
    pub sum_amount: f64,
    // ... другие итоги
}

#[component]
pub fn FeatureList() -> impl IntoView {
    let state = create_state();
    let global_ctx = expect_context::<AppGlobalContext>();

    let (items, set_items) = signal::<Vec<FeatureDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    // Загрузка данных с сервера
    let load_data = move || {
        let current_state = state.get();
        set_loading.set(true);

        spawn_local(async move {
            let offset = current_state.page * current_state.page_size;
            let sort_desc = !current_state.sort_ascending;

            let url = format!(
                "http://localhost:3000/api/feature/list?limit={}&offset={}&sort_by={}&sort_desc={}&date_from={}&date_to={}",
                current_state.page_size,
                offset,
                current_state.sort_field,
                sort_desc,
                current_state.date_from,
                current_state.date_to
            );

            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        match response.json::<PaginatedResponse>().await {
                            Ok(data) => {
                                set_items.set(data.items);
                                state.update(|s| {
                                    s.total_count = data.total;
                                    s.total_pages = data.total_pages;
                                    s.server_totals = data.totals;
                                    s.is_loaded = true;
                                });
                                set_loading.set(false);
                            }
                            Err(e) => {
                                set_error.set(Some(format!("Parse error: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    }
                }
                Err(e) => {
                    set_error.set(Some(format!("Fetch error: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Init column resize
    Effect::new(move |_| {
        if state.get().is_loaded {
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(50).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    view! {
        <div class="page">
            // Page Header
            <div class="page-header">
                <div class="page-header__content">
                    <div class="page-header__icon">{icon("file-text")}</div>
                    <div class="page-header__text">
                        <h1 class="page-header__title">"Документы"</h1>
                        <div class="page-header__badge">
                            <Badge variant="primary".to_string()>
                                {move || state.get().total_count.to_string()}
                            </Badge>
                        </div>
                    </div>
                </div>
                <div class="page-header__actions">
                    <Button variant="primary".to_string() on_click=Callback::new(move |_| load_data())>
                        {icon("refresh")} "Обновить"
                    </Button>
                    <Button
                        variant="success".to_string()
                        on_click=Callback::new(batch_post)
                        disabled=state.get().selected_ids.is_empty()
                    >
                        {icon("check")} {move || format!("Post ({})", state.get().selected_ids.len())}
                    </Button>
                    <Button
                        variant="warning".to_string()
                        on_click=Callback::new(batch_unpost)
                        disabled=state.get().selected_ids.is_empty()
                    >
                        {icon("x")} {move || format!("Unpost ({})", state.get().selected_ids.len())}
                    </Button>
                    <Button variant="secondary".to_string() on_click=Callback::new(export_excel)>
                        {icon("download")} "Excel"
                    </Button>
                </div>
            </div>

            // Filter Panel
            <div class="filter-panel">
                <div class="filter-panel-header">
                    <div class="filter-panel-header__left" on:click=move |_| set_is_filter_expanded.update(|e| *e = !*e)>
                        <svg class=move || if is_filter_expanded.get() {
                            "filter-panel__chevron filter-panel__chevron--expanded"
                        } else {
                            "filter-panel__chevron"
                        }>
                            <polyline points="6 9 12 15 18 9"></polyline>
                        </svg>
                        {icon("filter")}
                        <span class="filter-panel__title">"Фильтры"</span>
                        {move || {
                            let count = active_filters_count.get();
                            if count > 0 {
                                view! { <Badge variant="primary".to_string()>{count}</Badge> }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }
                        }}
                    </div>
                    <div class="filter-panel-header__center">
                        <PaginationControls
                            current_page=Signal::derive(move || state.get().page)
                            total_pages=Signal::derive(move || state.get().total_pages)
                            total_count=Signal::derive(move || state.get().total_count)
                            page_size=Signal::derive(move || state.get().page_size)
                            on_page_change=Callback::new(go_to_page)
                            on_page_size_change=Callback::new(change_page_size)
                        />
                    </div>
                </div>

                <div class=move || if is_filter_expanded.get() {
                    "filter-panel__collapsible filter-panel__collapsible--expanded"
                } else {
                    "filter-panel__collapsible filter-panel__collapsible--collapsed"
                }>
                    <div class="filter-panel-content">
                        <div class="filter-grid">
                            <div class="form__group">
                                <label class="form__label">"Период:"</label>
                                <div style="display: flex; gap: var(--spacing-xs);">
                                    <DateInput
                                        value=Signal::derive(move || state.get().date_from)
                                        on_change=move |val| {
                                            state.update(|s| { s.date_from = val; s.page = 0; });
                                            load_data();
                                        }
                                    />
                                    <span>" — "</span>
                                    <DateInput
                                        value=Signal::derive(move || state.get().date_to)
                                        on_change=move |val| {
                                            state.update(|s| { s.date_to = val; s.page = 0; });
                                            load_data();
                                        }
                                    />
                                    <MonthSelector
                                        on_select=Callback::new(move |(from, to)| {
                                            state.update(|s| {
                                                s.date_from = from;
                                                s.date_to = to;
                                                s.page = 0;
                                            });
                                            load_data();
                                        })
                                    />
                                </div>
                            </div>
                        </div>

                        // Active filter tags
                        {move || {
                            let has_filters = active_filters_count.get() > 0;
                            if has_filters {
                                view! {
                                    <div class="filter-tags">
                                        // ... removable filter tags ...
                                        <Button variant="ghost".to_string() on_click=Callback::new(clear_all_filters)>
                                            "Очистить все"
                                        </Button>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }
                        }}
                    </div>
                </div>
            </div>

            // Table
            <div class="page-content">
                <div class="list-container">
                    <table id=TABLE_ID class="table__data table--striped">
                        <thead class="table__head">
                            <tr>
                                <th class="table__header-cell table__header-cell--checkbox">
                                    <input type="checkbox" on:change=toggle_select_all />
                                </th>
                                <th class="table__header-cell table__header-cell--sortable resizable" on:click=toggle_sort("date")>
                                    "Дата" {get_sort_indicator("date", &sort_field, sort_ascending)}
                                </th>
                                <th class="table__header-cell resizable">"Номер"</th>
                                <th class="table__header-cell resizable">"Сумма"</th>
                            </tr>

                            // Строка итогов (легко включить/выключить через if)
                            {move || {
                                if let Some(totals) = state.get().server_totals {
                                    view! {
                                        <TableTotalsRow>
                                            <td class="table__cell--checkbox"></td>
                                            <td>{format!("Записей: {}", totals.total_records)}</td>
                                            <td></td>
                                            <td class="table__cell--right">{format_number(totals.sum_amount)}</td>
                                        </TableTotalsRow>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}
                        </thead>
                        <tbody>
                            {move || items.get().into_iter().map(|item| {
                                let id = item.id.clone();
                                let is_selected = state.get().selected_ids.contains(&id);
                                view! {
                                    <tr
                                        class="table__row"
                                        class:table__row--selected=is_selected
                                        on:click=move |_| open_detail(id.clone())
                                    >
                                        <td class="table__cell table__cell--checkbox" on:click=|e| e.stop_propagation()>
                                            <input
                                                type="checkbox"
                                                class="table__checkbox"
                                                prop:checked=is_selected
                                                on:change=move |_| toggle_select(id.clone())
                                            />
                                        </td>
                                        <td class="table__cell">{format_date(&item.date)}</td>
                                        <td class="table__cell">{item.number}</td>
                                        <td class="table__cell table__cell--right">{format_number(item.amount)}</td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    }
}
```

### State структура

```rust
// state.rs
use chrono::{Datelike, Utc};
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct FeatureState {
    // Данные
    pub items: Vec<FeatureDto>,

    // Фильтры
    pub date_from: String,
    pub date_to: String,
    pub selected_organization_id: Option<String>,

    // Сортировка
    pub sort_field: String,
    pub sort_ascending: bool,

    // Множественный выбор
    pub selected_ids: Vec<String>,

    // Флаг загрузки
    pub is_loaded: bool,

    // Серверная пагинация (ОБЯЗАТЕЛЬНО)
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,

    // Серверные итоги
    pub server_totals: Option<ServerTotals>,
}

impl Default for FeatureState {
    fn default() -> Self {
        // Период по умолчанию: текущий месяц
        let now = Utc::now().date_naive();
        let year = now.year();
        let month = now.month();
        let month_start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .expect("Invalid month start");
        let month_end = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .map(|d| d - chrono::Duration::days(1))
                .expect("Invalid month end")
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
                .map(|d| d - chrono::Duration::days(1))
                .expect("Invalid month end")
        };

        Self {
            items: Vec::new(),
            date_from: month_start.format("%Y-%m-%d").to_string(),
            date_to: month_end.format("%Y-%m-%d").to_string(),
            selected_organization_id: None,
            sort_field: "date".to_string(),
            sort_ascending: false,  // новые сначала
            selected_ids: Vec::new(),
            is_loaded: false,
            // Пагинация
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
            // Итоги
            server_totals: None,
        }
    }
}

pub fn create_state() -> RwSignal<FeatureState> {
    RwSignal::new(FeatureState::default())
}
```

---

## Чеклист соответствия стандарту

### Простая таблица ✓

- [ ] Чекбоксы в первой колонке (40px фиксированная ширина)
- [ ] Загрузка всех записей без пагинации
- [ ] Клиентская сортировка с индикаторами
- [ ] Модальное окно для создания/редактирования
- [ ] Кнопка "Удалить выбранные" с счётчиком
- [ ] Переход в detail по клику на строку
- [ ] **BEM:** Все классы следуют `.table__element--modifier`
- [ ] Нет inline-стилей (кроме динамических)
- [ ] Используются CSS-переменные

### Сложная таблица ✓

- [ ] Чекбоксы в первой колонке (40px фиксированная ширина)
- [ ] Серверная пагинация (offset/limit)
- [ ] Фильтр-панель с collapse
- [ ] Строка итогов от сервера в thead (через TableTotalsRow)
- [ ] Кнопки Post/Unpost для выбранных с счётчиком
- [ ] Экспорт в Excel
- [ ] Resize колонок с сохранением в localStorage
- [ ] Переход в detail по клику на строку
- [ ] **BEM:** Все классы следуют `.table__element--modifier`
- [ ] Нет inline-стилей (кроме динамических width)
- [ ] Используются CSS-переменные
- [ ] Active filter tags с возможностью удаления

### BEM Code Review ✓

- [ ] Нет классов типа `.checkbox-cell` (используем `.table__cell--checkbox`)
- [ ] Нет inline-стилей (кроме динамических width для resize)
- [ ] Используются CSS-переменные вместо hardcode
- [ ] Модификаторы используются вместе с базовым классом
- [ ] Нет глубокой вложенности (max 2 уровня: `block__element`)
- [ ] Все новые классы с префиксом `.table__`

---

## Эталонные примеры

**Простая таблица:**

- `a002_organization` - Организации
- `a005_marketplace` - Маркетплейсы

**Сложная таблица:**

- `a016_ym_returns` - Возвраты Яндекс (ЭТАЛОН)
- `a012_wb_sales` - Продажи Wildberries

---

## См. также

- [List Standard](./list-standard.md) - Детальный стандарт списков
- [Detail Form Standard](./detail-form-standard.md) - Стандарт форм детальных записей
- [Modal UI Standard](./modal-ui-standard.md) - Стандарт модальных окон
- `E:\dev\bolt\bolt-mpi-ui-redesign\BEM_MIGRATION_MAP.md` - Референс BEM
