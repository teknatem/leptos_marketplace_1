---
date: 2026-02-08
type: architecture
status: accepted
tags: [frontend, css, bem, structure, app-shell]
---

# CSS Structure Standard

## Overview

Унифицированная CSS-архитектура с разделением на:

1. **App Shell** (`app-shell.css`) — структура приложения верхнего уровня
2. **Page Layout** (`layout.css`) — структура страниц и контента

Все классы следуют методологии BEM (Block Element Modifier) для обеспечения консистентности и предсказуемости.

## CSS Architecture

### File Structure

**app-shell.css** — Application structure (universal, content-independent)

- `.app-layout`, `.app-body`, `.app-main` — top-level layout
- `.app-header` — top header with navigation and user actions
- `.app-sidebar` — left navigation sidebar
- `.app-tabs` — tab system container
- `.app-panel` — resizable right panel
- `.windows-dropdown`, `.windows-list` — window management

**layout.css** — Page structure (standard page layouts)

- `.page` — page root (list, detail, dashboard)
- `.page__header`, `.page__content` — page sections
- `.filter-panel`, `.list-container` — page-specific components
- Pagination and filters

**components.css** — UI components

- Buttons, forms, tables, badges, modals, etc.

## DOM Hierarchy

```
.app-layout                        (корневой layout) [app-shell.css]
├── .app-header                    (верхний header приложения) [app-shell.css]
│   ├── .app-header__brand
│   ├── .app-header__actions
│   └── .windows-dropdown
└── .app-body                      (основное тело) [app-shell.css]
    ├── .app-sidebar               (левая навигация) [app-shell.css]
    │   ├── .app-sidebar__content
    │   ├── .app-sidebar__item
    │   └── .app-sidebar__children
    ├── .app-main                  (центральная область) [app-shell.css]
    │   └── .app-tabs              (контейнер табов) [app-shell.css]
    │       └── .app-tabs__item    (страница таба) [app-shell.css]
    │           └── .page          (корень страницы) [layout.css]
    │               ├── .page__header [layout.css]
    │               │   ├── .page__header-left
    │               │   │   ├── .page__icon
    │               │   │   ├── .page__title
    │               │   │   ├── .page__subtitle
    │               │   │   └── .page__badge
    │               │   └── .page__header-right
    │               ├── .page__tabs [layout.css]  (опционально, только для страниц с вкладками)
    │               │   └── button.page__tab / button.page__tab--active / button.page__tab:disabled
    │               └── .page__content [layout.css]
    └── .app-panel                 (правая панель) [app-shell.css]
        ├── .app-panel__resizer
        └── .app-panel__content
```

## BEM Blocks

### App Shell Blocks (app-shell.css)

#### 1. App Tabs Block

**Block:** `.app-tabs`

Контейнер для системы табов. Располагается внутри `.app-main`. Напрямую содержит страницы табов.

**Elements:**

- `.app-tabs__item` — индивидуальная страница таба

**Modifiers:**

- `.app-tabs--dimmed` — применяется когда табы активны
- `.app-tabs__item--hidden` — скрывает неактивную страницу таба

**Example:**

```html
<div class="app-tabs app-tabs--dimmed">
  <div class="app-tabs__item" data-tab-key="a002_organization">
    <!-- page content -->
  </div>
  <div
    class="app-tabs__item app-tabs__item--hidden"
    data-tab-key="a001_connection_1c"
  >
    <!-- hidden page content -->
  </div>
</div>
```

#### 2. App Header Block

**Block:** `.app-header`

Верхний header приложения с навигацией и действиями пользователя.

**Elements:**

- `.app-header__brand` — логотип/название
- `.app-header__title` — название приложения
- `.app-header__actions` — кнопки действий
- `.app-header__icon-button` — кнопка с иконкой
- `.app-header__user` — информация о пользователе

#### 3. App Sidebar Block

**Block:** `.app-sidebar`

Левая навигационная панель.

**Elements:**

- `.app-sidebar__content` — контент с меню
- `.app-sidebar__item` — элемент меню
- `.app-sidebar__item-content` — содержимое элемента
- `.app-sidebar__chevron` — иконка раскрытия
- `.app-sidebar__children` — вложенные элементы

**Modifiers:**

- `.app-sidebar__item--active` — активный элемент
- `.app-sidebar__chevron--expanded` — раскрытое состояние

#### 4. App Panel Block

**Block:** `.app-panel`

Правая панель с возможностью изменения размера.

**Elements:**

- `.app-panel__resizer` — handle для изменения размера
- `.app-panel__content` — содержимое панели

**Modifiers:**

- `.app-panel--hidden` — скрытое состояние
- `.app-panel--resizing` — состояние изменения размера

### Page Blocks (layout.css)

#### Page Block

**Block:** `.page`

Корневой блок страницы внутри таба.

**Elements:**

- `.page__header` — заголовочная область страницы
- `.page__header-left` — левая часть заголовка (title, icon, subtitle)
- `.page__header-right` — правая часть заголовка (actions)
- `.page__icon` — иконка страницы
- `.page__title` — заголовок страницы (h1 или h2)
- `.page__subtitle` — подзаголовок страницы
- `.page__badge` — badge/счетчик
- `.page__actions` — альтернативное имя для `.page__header-right`
- `.page__tabs` — полоса вкладок (32px, фон = header), между header и content
- `.page__tab` — отдельная кнопка-вкладка внутри `.page__tabs` (нативный `<button>`)
- `.page__content` — основной контент страницы

**Modifiers:**

- `.page--list` — модификатор для списковых страниц
- `.page--detail` — модификатор для детальных страниц
- `.page--dashboard` — модификатор для дашбордов
- `.page--wide` — модификатор для широких страниц
- `.page__tab--active` — активная (выбранная) вкладка
- `.page__tab:disabled` — недоступная вкладка (opacity 0.35, pointer-events none)

**Example:**

```html
<div class="page page--list">
  <div class="page__header">
    <div class="page__header-left">
      <span class="page__icon">📦</span>
      <h1 class="page__title">Организации</h1>
      <span class="page__badge">42</span>
    </div>
    <div class="page__header-right">
      <button class="button button--primary">Создать</button>
      <button class="button button--secondary">Обновить</button>
    </div>
  </div>
  <div class="page__content">
    <!-- table, filters, etc. -->
  </div>
</div>
```

## BEM Naming Convention

### Prefixes

- **app-\*** — application shell (universal structure)
- **page** — page content (specific layouts)

### Block

Независимый компонент интерфейса:

- **App Shell:** `app-header`, `app-sidebar`, `app-tabs`, `app-panel`
- **Page:** `page`, `filter-panel`, `list-container`

### Element (Block\_\_element)

Часть блока, которая не имеет смысла без него:

- **App Shell:** `app-header__brand`, `app-sidebar__item`, `app-tabs__item`, `app-panel__resizer`
- **Page:** `page__header`, `page__title`, `page__content`

### Modifier (Block--modifier или Block\_\_element--modifier)

Определяет внешний вид, состояние или поведение:

- **App Shell:** `app-tabs--dimmed`, `app-tabs__item--hidden`, `app-panel--hidden`, `app-sidebar__item--active`
- **Page:** `page--list`, `page--detail`, `page--wide`

## Component Usage (Rust/Leptos)

### PageHeader Component

Переиспользуемый компонент заголовка страницы:

```rust
use crate::shared::components::PageHeader;

view! {
    <div class="page">
        <PageHeader
            title="Организации"
            subtitle="Список всех организаций"
        >
            // actions
            <button class="button button--primary">
                {"Создать"}
            </button>
        </PageHeader>

        // content
    </div>
}
```

### Manual Header Structure

Для кастомных случаев:

```rust
view! {
    <div class="page page--list">
        <div class="page__header">
            <div class="page__header-left">
                <h1 class="page__title">{"Организации"}</h1>
            </div>
            <div class="page__header-right">
                <button class="button button--primary">
                    {"Создать"}
                </button>
            </div>
        </div>
    </div>
}
```

## Page Types

### List Page

Страница со списком элементов (таблица, список карточек)

```html
<div class="page page--list">
  <div class="page__header">...</div>
  <div class="filter-panel">...</div>
  <div class="list-container">
    <table>
      ...
    </table>
  </div>
</div>
```

### Detail Page

Страница с детальной информацией об элементе. Структура: header → tabs (опционально) → content.

```html
<div class="page page--detail">
  <div class="page__header">
    <div class="page__header-left">
      <h2>Редактирование номенклатуры</h2>
    </div>
    <div class="page__header-right">
      <button class="button button--primary">Сохранить</button>
      <button class="button button--secondary">Закрыть</button>
    </div>
  </div>
  <div class="page__tabs">
    <button class="page__tab page__tab--active">Основная</button>
    <button class="page__tab">Штрихкоды</button>
    <button class="page__tab" disabled>Дилерские цены</button>
  </div>
  <div class="page__content">
    <!-- detail-grid, CardAnimated, form fields -->
  </div>
</div>
```

### Dashboard

Страница с аналитикой и визуализацией данных

```html
<div class="page page--dashboard">
  <div class="page__header">
    <div class="page__header-left">
      <h1 class="page__title">Дашборд продаж</h1>
      <span class="page__subtitle">за январь 2026</span>
    </div>
    <div class="page__header-right">
      <button class="button button--secondary">Экспорт</button>
    </div>
  </div>
  <div class="dashboard-widgets">...</div>
</div>
```

## Migration Notes (2026-02-08)

### App Shell Refactoring

**Purpose:** Separate application structure from page content with app-\* prefix

| Old Class         | New Class             | Category  | Notes                          |
| ----------------- | --------------------- | --------- | ------------------------------ |
| `.top-header`     | `.app-header`         | App Shell | Унифицированный префикс        |
| `.top-header__*`  | `.app-header__*`      | App Shell | Все BEM elements переименованы |
| `.panel-left`     | `.app-sidebar`        | App Shell | Более семантичное имя          |
| `.panel-left__*`  | `.app-sidebar__*`     | App Shell | Все BEM elements переименованы |
| `.tabs`           | `.app-tabs`           | App Shell | Добавлен префикс app-          |
| `.tabs__item`     | `.app-tabs__item`     | App Shell | Element переименован           |
| `.tabs--dimmed`   | `.app-tabs--dimmed`   | App Shell | Modifier переименован          |
| `.right-panel`    | `.app-panel`          | App Shell | Унифицированное имя            |
| `.right-panel__*` | `.app-panel__*`       | App Shell | Все BEM elements переименованы |
| `.panel-right`    | `.app-panel-activity` | App Shell | Отдельный блок для активности  |

### Page Structure Unification

**Purpose:** Unify detail and list pages under `.page` block

| Old Class                   | New Class             | Category | Notes                     |
| --------------------------- | --------------------- | -------- | ------------------------- |
| `.detail-form`              | `.page.page--detail`  | Page     | Унификация с list pages   |
| `.detail-form-header`       | `.page__header`       | Page     | Shared header element     |
| `.detail-form-header-left`  | `.page__header-left`  | Page     | Shared element            |
| `.detail-form-header-right` | `.page__header-right` | Page     | Shared element            |
| `.detail-form-content`      | `.page__content`      | Page     | Shared content element    |
| `.page-header`              | `.page__header`       | Page     | BEM element (old style)   |
| `.page-header__content`     | `.page__header-left`  | Page     | Более точное имя          |
| `.page-header__text`        | (removed)             | Page     | Избыточная обёртка        |
| `.page-header__title`       | `.page__title`        | Page     | Прямой element блока page |
| `.page-header__icon`        | `.page__icon`         | Page     | Прямой element блока page |
| `.page-header__actions`     | `.page__header-right` | Page     | Более точное имя          |
| `.page-content`             | `.page__content`      | Page     | BEM element (old style)   |

## Related Files

### CSS

- `crates/frontend/static/themes/core/app-shell.css` — app structure (NEW)
- `crates/frontend/static/themes/core/layout.css` — page layout
- `crates/frontend/static/themes/core/components.css` — UI components
- `crates/frontend/static/themes/core/index.css` — entry point

### Layout Components

- `crates/frontend/src/layout/top_header/mod.rs` — `.app-header`
- `crates/frontend/src/layout/left/left.rs` — `.app-sidebar` wrapper
- `crates/frontend/src/layout/left/sidebar.rs` — `.app-sidebar` menu
- `crates/frontend/src/layout/center/center.rs` — `.app-tabs`
- `crates/frontend/src/layout/tabs/page.rs` — `.app-tabs__item`
- `crates/frontend/src/layout/right/right.rs` — `.app-panel`

### Page Components

- `crates/frontend/src/domain/*/ui/list/mod.rs` — list pages (`.page.page--list`)
- `crates/frontend/src/domain/*/ui/details/*.rs` — detail pages (`.page.page--detail`)
- `crates/frontend/src/dashboards/*` — dashboards (`.page.page--dashboard`)

## Principles

1. **Separation of Concerns** — app structure отделена от page content
2. **Consistency** — все страницы используют одинаковую структуру
3. **BEM Methodology** — строгое следование Block\_\_Element--Modifier
4. **Semantic Naming** — классы отражают назначение, а не внешний вид
5. **Predictability** — любой разработчик может понять структуру по именам классов
6. **Maintainability** — изменения в одном месте не ломают другие компоненты
7. **Prefix Convention** — `app-*` для структуры приложения, без префикса для контента

## CSS Variable Naming (Future)

Currently CSS variables retain old naming (e.g., `--top-header-bg`, `--sidebar-*`) for backwards compatibility. Future improvement: rename to match class names (e.g., `--app-header-bg`, `--app-sidebar-*`).

## Future Improvements

- [ ] Rename CSS variables to match new class names (`--app-header-bg`, etc.)
- [ ] Добавить `.page__footer` для футера страницы
- [ ] Рассмотреть `.page__sidebar` для страниц с боковой панелью
- [ ] Создать модификаторы для разных состояний (loading, error, empty)
- [ ] Создать отдельный файл для windows-\* компонентов (если нужно)
