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

**Примеры:**
- `a001_connection_1c` - Подключения к 1С
- `a002_organization` - Организации
- `a003_product` - Продукты
- `a014_ozon_transactions` - Транзакции Ozon
- `a015_wb_orders` - Заказы Wildberries

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

**Примеры:**
- `u501_import_from_ut` - Импорт из 1С:УТ11
- `u504_import_from_wildberries` - Импорт из WB
- `u505_import_from_ozon` - Импорт из Ozon

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

### Projections: p901-p999
Read models, аналитика, отчеты (CQRS-подобный подход)

**Примеры:**
- `p902_sales_register` - Регистр продаж
- `p904_sales_data` - Аналитика продаж
- `p905_wb_commission_history` - История комиссий WB

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

| Событие | Когда вызывается | Где определено |
|---------|-----------------|----------------|
| `validate()` | Перед сохранением | aggregate.rs или events.rs |
| `before_write()` | Перед записью в БД | aggregate.rs или events.rs |
| `before_delete()` | Перед удалением | aggregate.rs или events.rs |

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

## Frontend Patterns

### Component Architecture

```
domain/{feature}/ui/
├── list/
│   └── mod.rs        # Table view with sorting, filtering
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

