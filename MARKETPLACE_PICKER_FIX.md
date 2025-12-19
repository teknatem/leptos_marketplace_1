# Исправление модальных диалогов выбора (Picker)

**Дата:** 2025-12-19  
**Статус:** ✅ Выполнено

---

## Обзор

Обновлены модальные диалоги выбора (`MarketplacePicker` и `GenericAggregatePicker`) для соответствия единым UI стандартам проекта с использованием BEM методологии.

---

## Выполненные изменения

### 1. MarketplacePicker (Grid вариант)

**Файл:** `crates/frontend/src/domain/a005_marketplace/ui/picker/mod.rs`

#### Изменения:

- ✅ Обновлены CSS классы на BEM формат:

  - `.picker-container` → `.picker`
  - `.picker-header` → удален (заголовок в родительском Modal)
  - `.picker-content` → `.picker__content`
  - `.picker-list` → `.picker__grid`
  - `.picker-item` → `.picker__item`
  - `.picker-item.selected` → `.picker__item--selected`
  - `.picker-item-logo` → `.picker__item-logo`
  - `.picker-item-icon` → `.picker__item-icon`
  - `.picker-item-description` → `.picker__item-description`
  - `.picker-item-code` → `.picker__item-code`
  - `.picker-actions` → `.picker__actions`

- ✅ Обновлен стиль отображения ошибок на warning-box компонент
- ✅ Сохранены кнопки действий внизу (для совместимости с текущей архитектурой)

### 2. GenericAggregatePicker (Table вариант)

**Файл:** `crates/frontend/src/shared/picker_aggregate/component.rs`

#### Изменения:

- ✅ Обновлены CSS классы на BEM формат:

  - `.picker-container` → `.picker`
  - `.picker-header` → удален (не нужен, заголовок в родительском Modal)
  - `.picker-content` → `.picker__content`
  - `.picker-table` → `.picker__table` + `.table__data` + `.table--striped`
  - `.picker-row` → `.picker__row` + `.table__row`
  - `.picker-row.selected` → `.picker__row--selected` + `.table__row--selected`
  - `.picker-loading` → `.picker__loading`
  - `.picker-error` → `.warning-box`
  - `.picker-empty` → `.picker__empty`
  - `.picker-actions` → `.picker__actions`

- ✅ Использованы существующие BEM классы для таблиц (`.table__head`, `.table__header-cell`, `.table__cell`)
- ✅ Исправлена неиспользуемая переменная `title` → `_title`
- ✅ Исправлена ошибка компиляции с `item_id_for_selected` (добавлен дополнительный clone)

### 3. CSS стили

**Файл:** `crates/frontend/static/themes/core/components.css`

#### Добавлены новые BEM стили:

```css
/* Picker (базовые стили) */
.picker
.picker__content
.picker__actions

/* Grid вариант (для MarketplacePicker) */
.picker__grid
.picker__item
.picker__item--selected
.picker__item-logo
.picker__item-image
.picker__item-icon
.picker__item-description
.picker__item-code

/* Table вариант (для GenericAggregatePicker) */
.picker__table
.picker__row
.picker__row--selected

/* Состояния */
.picker__loading
.picker__empty

/* Dark theme adjustments */
body[data-theme="dark"] .picker__*;
```

#### Особенности стилей:

- ✅ Grid layout для карточек маркетплейсов (auto-fill, minmax(200px, 1fr))
- ✅ Hover эффекты с transform и box-shadow
- ✅ Визуальное выделение выбранного элемента
- ✅ Анимация загрузки (spinner)
- ✅ Адаптация под темную тему
- ✅ Использование CSS-переменных (var(--spacing-md), var(--color-primary), и т.д.)

---

## Структура BEM классов

### Picker (общие)

```
.picker                          # Блок
├── .picker__content             # Элемент: контейнер содержимого
├── .picker__actions             # Элемент: контейнер кнопок действий
├── .picker__loading             # Элемент: индикатор загрузки
└── .picker__empty               # Элемент: сообщение о пустом списке
```

### Grid вариант

```
.picker__grid                    # Элемент: grid контейнер
└── .picker__item                # Элемент: карточка элемента
    ├── --selected               # Модификатор: выбранный элемент
    ├── .picker__item-logo       # Элемент: логотип
    ├── .picker__item-image      # Элемент: изображение
    ├── .picker__item-icon       # Элемент: иконка (fallback)
    ├── .picker__item-description # Элемент: описание
    └── .picker__item-code       # Элемент: код
```

### Table вариант

```
.picker__table                   # Элемент: таблица
└── .picker__row                 # Элемент: строка таблицы
    └── --selected               # Модификатор: выбранная строка
```

---

## Использование

### MarketplacePicker (Grid вариант)

```rust
use crate::domain::a005_marketplace::ui::{MarketplacePicker, MarketplacePickerItem};

// В родительском компоненте внутри Modal
<MarketplacePicker
    initial_selected_id=initial_id
    on_selected=handle_selected
    on_cancel=handle_cancel
/>
```

### GenericAggregatePicker (Table вариант)

```rust
use crate::shared::picker_aggregate::{
    GenericAggregatePicker,
    AggregatePickerResult,
    TableDisplayable,
};

// В родительском компоненте внутри Modal
<GenericAggregatePicker<YourItemType>
    items=items_signal
    error=error_signal
    loading=loading_signal
    initial_selected_id=initial_id
    on_confirm=handle_confirm
    on_cancel=handle_cancel
    title="Выбор элемента".to_string()
/>
```

---

## Преимущества новой структуры

1. **Единообразие:**

   - Все picker-ы используют единую BEM методологию
   - Консистентные названия классов
   - Легко поддерживать и расширять

2. **Гибкость:**

   - Grid вариант для визуальных элементов (маркетплейсы с логотипами)
   - Table вариант для списков данных (организации, продукты)
   - Легко добавить новые варианты

3. **Качество кода:**

   - CSS-переменные вместо hardcode значений
   - Правильная вложенность и специфичность
   - Адаптация под темы (dark, light, forest)

4. **UX улучшения:**
   - Анимации hover и выделения
   - Spinner загрузки
   - Визуальная обратная связь при выборе
   - Двойной клик для быстрого подтверждения

---

## Миграция существующих picker-ов

Если есть другие picker-ы, их можно обновить следуя этому паттерну:

1. **Обновить CSS классы:**

   - Все классы должны начинаться с `.picker__`
   - Использовать модификаторы для состояний: `--selected`, `--disabled`
   - Удалить inline-стили

2. **Использовать существующие компоненты:**

   - Для табличных списков → `GenericAggregatePicker`
   - Для grid-списков → создать по аналогии с `MarketplacePicker`

3. **Проверить:**
   - BEM классы корректны
   - Нет inline-стилей
   - CSS-переменные используются
   - Темная тема работает

---

## Связанные файлы

- `crates/frontend/src/domain/a005_marketplace/ui/picker/mod.rs` - MarketplacePicker
- `crates/frontend/src/shared/picker_aggregate/component.rs` - GenericAggregatePicker
- `crates/frontend/static/themes/core/components.css` - CSS стили
- `memory-bank/architecture/modal-ui-standard.md` - Стандарт модальных окон
- `memory-bank/architecture/table-standards.md` - Стандарт таблиц

---

## Тестирование

Для проверки изменений:

1. Открыть страницу с подключениями маркетплейсов:

   ```
   http://localhost:8080/?active=a006_connection_mp
   ```

2. Нажать "Новое подключение" или редактировать существующее

3. В форме нажать кнопку "..." рядом с полем "Маркетплейс"

4. Проверить:

   - ✅ Отображение сетки маркетплейсов
   - ✅ Hover эффекты работают
   - ✅ Выбор элемента выделяет его
   - ✅ Двойной клик подтверждает выбор
   - ✅ Кнопки "Выбрать" и "Отмена" работают
   - ✅ Темная тема корректно отображается

5. Проверить организацию picker (table вариант):
   - Нажать кнопку "..." рядом с полем "Организация"
   - Проверить табличное отображение
   - Проверить выделение строки при выборе

---

## Версия

- **Дата:** 2025-12-19
- **Автор:** AI Assistant
- **Версия стандарта:** 1.0
- **Статус:** ✅ Готово к использованию
