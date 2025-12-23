# System Architecture & Patterns

## Архитектурный обзор

### Принципы

Проект построен на комбинации двух подходов:

1. **DDD (Domain-Driven Design)**: Разделение на bounded contexts и aggregates
2. **VSA (Vertical Slice Architecture)**: Группировка кода по фичам, а не по техническим слоям

### Структура Cargo Workspace

```
leptos_marketplace_1/
├── crates/
│   ├── contracts/    # Shared types (DTOs, aggregates)
│   ├── backend/      # Axum server
│   └── frontend/     # Leptos WASM app
├── marketplace.db    # SQLite database
└── memory-bank/      # Documentation for AI
```

**Contracts (shared)** - общие типы между frontend и backend:

- Гарантирует type safety на всем стеке
- Изменение в DTO → компилятор требует обновить frontend и backend
- Без runtime ошибок десериализации

## Индексированная система именования

Это ключевой паттерн проекта. Все фичи имеют индекс:

### Aggregates: a001-a499

Доменные сущности с бизнес-логикой

**Реализовано (a001-a016):**

- `a001_connection_1c` - Подключения к 1С
- `a002_organization` - Организации
- `a003_counterparty` - Контрагенты
- `a004_nomenclature` - Номенклатура
- `a005_marketplace` - Маркетплейсы
- `a006_connection_mp` - Подключения к маркетплейсам
- `a007-a016` - Продукты, продажи, заказы и возвраты маркетплейсов

**Структура:**

```
crates/backend/src/domain/a001_connection_1c/
├── mod.rs            # Re-exports
├── service.rs        # Business logic orchestration
├── repository.rs     # Database CRUD
└── aggregate.rs      # Optional: data structure (may be in contracts)

crates/contracts/src/domain/a001_connection_1c/
└── aggregate.rs      # Shared DTO/struct

crates/frontend/src/domain/a001_connection_1c/
└── ui/
    ├── list/         # List view
    └── details/      # Details/edit view
```

### UseCases: u501-u999

Операции, часто затрагивающие несколько aggregates

**Реализовано (u501-u506):**

- `u501_import_from_ut` - Импорт из 1С:УТ11
- `u502_import_from_ozon` - Импорт из Ozon
- `u503_import_from_yandex` - Импорт из Яндекс.Маркет
- `u504_import_from_wildberries` - Импорт из Wildberries
- `u505_match_nomenclature` - Сопоставление номенклатуры
- `u506_import_from_lemanapro` - Импорт из LemanaPro

**Структура:**

```
crates/backend/src/usecases/u501_import_from_ut/
├── mod.rs
├── executor.rs       # Main logic
└── ut_odata_client.rs # External integration

crates/contracts/src/usecases/u501_import_from_ut/
├── request.rs
├── response.rs
└── progress.rs       # For long-running operations

crates/frontend/src/usecases/u501_import_from_ut/
├── view.rs           # UI widget
└── monitor.rs        # Progress tracking
```

### Projections: p900-p999

Read models, аналитика, отчеты (CQRS-подобный подход)

**Реализовано (p900-p906):**

- `p900_mp_sales_register` - Регистр продаж маркетплейсов
- `p901_nomenclature_barcodes` - Штрих-коды номенклатуры
- `p902_ozon_finance_realization` - Финансовая реализация Ozon
- `p903_wb_finance_report` - Финансовый отчет Wildberries
- `p904_sales_data` - Аналитика продаж
- `p905_wb_commission_history` - История комиссий Wildberries
- `p906_nomenclature_prices` - Цены номенклатуры

## Domain Layer Patterns

### Ответственность слоев

```
┌─────────────────────────────────────┐
│         HTTP Handlers               │  ← Axum routes
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         Service Layer               │  ← Orchestration
│  - Calls validators                 │  ← Business rules
│  - Calls lifecycle hooks            │  ← Transaction boundaries
│  - Coordinates operations           │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│       Repository Layer              │  ← Pure CRUD
│  - Database operations              │  ← NO business logic
│  - Model ↔ Aggregate mapping        │
└─────────────────────────────────────┘
               │
               ▼
        [SQLite Database]
```

### Lifecycle Events (1C-style)

Паттерн, вдохновленный событиями 1С (ПередЗаписью, ПриЗаписи и т.д.):

| Событие           | Когда вызывается   | Где определено             |
| ----------------- | ------------------ | -------------------------- |
| `validate()`      | Перед сохранением  | aggregate.rs или events.rs |
| `before_write()`  | Перед записью в БД | aggregate.rs или events.rs |
| `before_delete()` | Перед удалением    | aggregate.rs или events.rs |

**Порядок вызова при создании/обновлении:**

```rust
1. Создать/загрузить aggregate
2. aggregate.validate()?        // Может заблокировать
3. aggregate.before_write()     // Может модифицировать
4. Бизнес-правила (в service)
5. repository::save()
6. После сохранения (опционально)
```

### UseCase vs Service

**Service** (domain layer):

- Операции над ОДНИМ aggregate
- Пример: create, update, delete для Connection1C

**UseCase**:

- Операции над НЕСКОЛЬКИМИ aggregates
- Пример: импорт из 1С затрагивает Organization + Product + Nomenclature

## Thaw UI Integration

### ConfigProvider Pattern

Приложение использует Thaw ConfigProvider для управления темами:

- **Theme switching**: light / dark / forest
- **CSS переменные** для кастомизации: `--colorNeutralBackground1`, `--colorBrandBackground` и др.
- **Программная модификация** стилей через DOM API (для особых случаев типа transparent background)

```rust
use thaw::{ConfigProvider, Theme};

#[component]
pub fn App() -> impl IntoView {
    let theme = create_rw_signal(Theme::dark());

    view! {
        <ConfigProvider theme>
            // App content
        </ConfigProvider>
    }
}
```

### Table Components

Проект использует **гибридный подход** к таблицам:

#### 1. Thaw Table

Используется когда нужны готовые компоненты с минимальной настройкой.

**Преимущества:**

- Встроенная стилизация через Thaw CSS переменные
- Готовые компоненты TableColumn, TableRow
- Автоматическая адаптация к теме

**Ограничения:**

- Resize columns требует workarounds
- Меньше контроля над DOM структурой
- Некоторые кастомизации требуют программной модификации CSS

**Примеры:** a006_connection_mp

#### 2. Native HTML `<table>`

Используется когда нужен полный контроль или сложная кастомизация.

**Преимущества:**

- Полный контроль над стилями
- Легкая кастомизация
- Прямой доступ к DOM

**Недостатки:**

- Требует ручной реализации сортировки
- Нужно вручную поддерживать стили

**Примеры:** a002_organization, a016_ym_returns

**См. также:**

- `memory-bank/runbooks/RB-thaw-table-sorting-v1.md` - добавление сортировки
- `memory-bank/known-issues/KI-thaw-table-style-limitations-2025-12-21.md` - известные ограничения

### Signal Reactivity Pattern

**Проблема:** Non-reactive props не обновляют компоненты при изменении родительского state.

**Решение:** Использовать Signal параметры для реактивных данных.

```rust
// ❌ Bad - не реактивно
#[component]
fn MyComponent(id: Option<String>) -> impl IntoView { ... }

// ✅ Good - реактивно
#[component]
fn MyComponent(#[prop(into)] id: Signal<Option<String>>) -> impl IntoView {
    Effect::new(move |_| {
        if let Some(current_id) = id.get() {
            // Этот код выполнится при каждом изменении id
        }
    });
}
```

**См. также:** `memory-bank/lessons/LL-leptos-signal-vs-value-2025-12-21.md`

## Frontend Patterns

### Component Architecture

```
domain/{feature}/ui/
├── list/
│   ├── mod.rs        # Table view with sorting, filtering
│   └── state.rs      # State management (optional)
└── details/
    └── mod.rs        # Form for create/edit
```

### Leptos Signals & State Management

**Reactive system:**

```rust
// Read signal
let (data, set_data) = create_signal(Vec::new());

// Derived signal
let filtered_data = create_memo(move |_| {
    data().into_iter().filter(|x| x.active).collect()
});

// Resource (async data)
let data_resource = create_resource(
    move || (),
    |_| async { fetch_data().await }
);
```

**Recent pattern:** Separate `state.rs` для управления состоянием компонента.

### Common UI Utilities

- `shared/list_utils.rs` - Sorting, filtering for tables
- `shared/date_utils.rs` - Date formatting
- `layout/center/tabs/tabs.rs` - Tab management
- CSS в `frontend/styles/3-components/`

## Database Patterns

### Table Naming

```sql
-- Aggregates
CREATE TABLE a001_connection_1c_database (...);

-- UseCases history
CREATE TABLE u501_import_history (...);

-- Projections
CREATE TABLE p904_sales_data (...);
```

### Common Fields

```sql
id TEXT PRIMARY KEY,           -- UUID или auto-increment
code TEXT,                     -- Бизнес-ключ
description TEXT,              -- Отображаемое имя
created_at TEXT,               -- ISO timestamp
updated_at TEXT,               -- ISO timestamp
is_deleted INTEGER DEFAULT 0   -- Мягкое удаление
```

### Migration Strategy

- SQL файлы: `migrate_*.sql`
- Ручное применение или через `migrate_db.py`
- Нет ORM миграций (SQLite простая)

## API Patterns

### REST Endpoints

**Aggregates:**

```
GET    /api/a001/connection_1c
GET    /api/a001/connection_1c/:id
POST   /api/a001/connection_1c
PUT    /api/a001/connection_1c/:id
DELETE /api/a001/connection_1c/:id
```

**UseCases:**

```
POST /api/u501/import/start
GET  /api/u501/import/:session_id/progress
GET  /api/u501/import/history
```

**Projections:**

```
GET /api/p904/sales_data?from=2024-01-01&to=2024-12-31
```

## Integration Patterns

### External Systems

1. **1C:УТ11 (OData v4)**

   - Basic authentication
   - Standard OData queries
   - Client: `ut_odata_client.rs`

2. **Wildberries API**

   - Token-based auth
   - Multiple endpoints (sales, orders, finance)
   - Rate limiting considerations

3. **Ozon API**
   - Client ID + API Key
   - Pagination for large datasets
   - Transaction-based model

## Ключевые правила

### ✅ DO

1. Группируй код по фичам (вертикальные срезы)
2. Shared contracts между frontend/backend
3. Service для одного aggregate, UseCase для нескольких
4. Repository - только CRUD, без бизнес-логики
5. Используй indexed naming (a001, u501, p904)

### ❌ DON'T

1. Не размещай бизнес-логику в repository
2. Не дублируй типы между frontend/backend - используй contracts
3. Не создавай UseCase для операций над одним aggregate
4. Не вызывай repository напрямую из handlers
5. Не группируй код по техническим слоям (все controllers в одной папке и т.д.)

## Дополнительная информация

Детальные описания архитектурных паттернов:

- `architecture/domain-layer-architecture.md` - Полное описание domain layer
- `architecture/naming-conventions.md` - Детали системы именования
- `architecture/project-structure.md` - Структура workspace
