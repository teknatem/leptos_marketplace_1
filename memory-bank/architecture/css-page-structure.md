---
date: 2026-01-30
type: architecture
status: accepted
tags: [frontend, css, bem, structure]
---

# CSS Page Structure Standard

## Overview

Унифицированная BEM-структура для всех страниц, открываемых в табах. Все классы следуют методологии BEM (Block Element Modifier) для обеспечения консистентности и предсказуемости.

## DOM Hierarchy

```
.app-layout                      (корневой layout)
├── .top-header                  (верхний header приложения)
└── .app-body                    (основное тело)
    ├── .panel-left              (левая панель навигации)
    ├── .app-main                (центральная область, min-width: 400px)
    │   └── .tabs                (контейнер табов, бывший .center)
    │       └── .tabs__page      (индивидуальная страница таба)
    │           └── .page        (корень страницы)
    │               ├── .page__header
    │               │   ├── .page__header-left
    │               │   │   ├── .page__icon
    │               │   │   ├── .page__title
    │               │   │   ├── .page__subtitle
    │               │   │   └── .page__badge
    │               │   └── .page__header-right
    │               │       └── .page__actions
    │               └── .page__content (опционально)
    └── .right-panel             (правая панель, resizable, max-width: 50vw)
        ├── .right-panel__resizer      (resize handle)
        ├── .right-panel__content      (контент панели)
        └── .right-panel__resize-overlay (оверлей при resize)
```

## BEM Blocks

### 1. Tabs Block

**Block:** `.tabs` (бывший `.center`)

Контейнер для системы табов. Располагается внутри `.app-main`. Напрямую содержит страницы табов.

**Elements:**
- `.tabs__page` — индивидуальная страница таба

**Modifiers:**
- `.tabs--dimmed` — применяется когда табы активны
- `.tabs__page--hidden` — скрывает неактивную страницу таба

**Example:**
```html
<div class="tabs tabs--dimmed">
  <div class="tabs__page" data-tab-key="a002_organization">
    <!-- page content -->
  </div>
  <div class="tabs__page tabs__page--hidden" data-tab-key="a001_connection_1c">
    <!-- hidden page content -->
  </div>
</div>
```

### 2. Page Block

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
- `.page__content` — основной контент страницы (опционально)

**Modifiers:**
- `.page--list` — модификатор для списковых страниц
- `.page--detail` — модификатор для детальных страниц
- `.page--dashboard` — модификатор для дашбордов
- `.page--wide` — модификатор для широких страниц

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

### Block
Независимый компонент интерфейса:
- `page`
- `tabs`
- `panel-left`

### Element (Block__element)
Часть блока, которая не имеет смысла без него:
- `page__header`
- `page__title`
- `tabs__content`
- `tabs__page`

### Modifier (Block--modifier или Block__element--modifier)
Определяет внешний вид, состояние или поведение:
- `tabs--dimmed`
- `tabs__page--hidden`
- `page--list`
- `page--wide`

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
    <table>...</table>
  </div>
</div>
```

### Detail Page
Страница с детальной информацией об элементе (форма)

```html
<div class="page page--detail">
  <div class="page__header">
    <div class="page__header-left">
      <h1 class="page__title">Редактирование: Организация #123</h1>
    </div>
    <div class="page__header-right">
      <button class="button button--primary">Сохранить</button>
      <button class="button button--secondary">Отмена</button>
    </div>
  </div>
  <form>...</form>
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

## Migration Notes

### Changed Classes

| Old Class               | New Class            | Notes                          |
|-------------------------|----------------------|--------------------------------|
| `.center`               | `.tabs`              | Семантически более точное имя |
| `.tab-content`          | (removed)            | Избыточная обертка             |
| `.tab-page`             | `.tabs__page`        | BEM element                    |
| `.tab-page.hidden`      | `.tabs__page--hidden`| BEM modifier                   |
| `.header` (in page)     | `.page__header`      | Унификация с page-header       |
| `.header__*`            | `.page__*`           | Элементы в page блоке          |
| `.page-header`          | `.page__header`      | BEM element                    |
| `.page-header__content` | `.page__header-left` | Более точное имя               |
| `.page-header__text`    | (removed)            | Избыточная обёртка             |
| `.page-header__title`   | `.page__title`       | Прямой element блока page      |
| `.page-header__icon`    | `.page__icon`        | Прямой element блока page      |
| `.page-header__actions` | `.page__header-right`| Более точное имя               |

### System Header (Unchanged)

Системный header приложения (`.header` в `layout/header/header.rs`) не изменяется - это отдельный компонент верхнего уровня, не связанный со страницами.

## Related Files

### CSS
- `crates/frontend/static/themes/core/layout.css` — основные стили

### Components
- `crates/frontend/src/layout/center/center.rs` — компонент `.tabs`
- `crates/frontend/src/layout/tabs/page.rs` — компонент `.tabs__page`
- `crates/frontend/src/shared/components/page_header.rs` — компонент `.page__header`

### Pages
- `crates/frontend/src/domain/*/ui/list/mod.rs` — списковые страницы
- `crates/frontend/src/domain/*/ui/details/*.rs` — детальные страницы
- `crates/frontend/src/dashboards/*` — дашборды

## Principles

1. **Consistency** — все страницы используют одинаковую структуру
2. **BEM Methodology** — строгое следование Block__Element--Modifier
3. **Semantic Naming** — классы отражают назначение, а не внешний вид
4. **Predictability** — любой разработчик может понять структуру по именам классов
5. **Maintainability** — изменения в одном месте не ломают другие компоненты

## Future Improvements

- [ ] Добавить `.page__footer` для футера страницы
- [ ] Рассмотреть `.page__sidebar` для страниц с боковой панелью
- [ ] Создать модификаторы для разных состояний (loading, error, empty)
- [ ] Документировать CSS переменные для кастомизации
