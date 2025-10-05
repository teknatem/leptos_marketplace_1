# Соглашения об именовании

## Индексы сущностей

### Агрегаты (Aggregates): a001-a499

**Формат:** `a{NNN}_{snake_case_name}`

**Примеры:**
- `a001_connection_1c` - Подключения к 1С
- `a002_organization` - Организации
- `a003_product` - Товары

**Структура файлов:**
```
crates/
├── contracts/src/domain/a001_connection_1c/
│   ├── mod.rs
│   └── aggregate.rs
├── backend/src/domain/a001_connection_1c/
│   ├── mod.rs
│   ├── service.rs
│   └── repository.rs
└── frontend/src/domain/a001_connection_1c/
    └── ui/
        ├── list/
        └── details/
```

**БД таблицы:**
```sql
-- Основная таблица
CREATE TABLE a001_connection_1c_database (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL,
    ...
);
```

### Операции (UseCases): u501-u999

**Формат:** `u{NNN}_{snake_case_name}`

**Примеры:**
- `u501_import_from_ut` - Импорт из 1С:УТ 11
- `u502_export_to_ozon` - Экспорт на Ozon
- `u503_sync_prices` - Синхронизация цен

**Структура файлов:**
```
crates/
├── contracts/src/usecases/u501_import_from_ut/
│   ├── mod.rs
│   ├── request.rs
│   ├── response.rs
│   ├── events.rs
│   └── progress.rs
├── backend/src/usecases/u501_import_from_ut/
│   ├── mod.rs
│   ├── executor.rs
│   ├── ut_odata_client.rs
│   └── progress_tracker.rs
└── frontend/src/usecases/u501_import_from_ut/
    ├── mod.rs
    ├── widget.rs
    └── monitor.rs
```

**БД таблицы:**
```sql
-- Общая таблица событий для всех UseCases
CREATE TABLE usecase_events (
    id TEXT PRIMARY KEY,
    usecase_index TEXT NOT NULL,      -- "u501"
    usecase_name TEXT NOT NULL,       -- "import_from_ut"
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,
    session_id TEXT,
    created_at TEXT NOT NULL,
    INDEX idx_session (session_id),
    INDEX idx_usecase (usecase_index, created_at)
);

-- Опциональная таблица истории для конкретного UseCase
CREATE TABLE u501_import_from_ut_history (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL,
    stats_json TEXT
);
```

## API Endpoints

### Агрегаты
```
GET    /api/a001/connection_1c
GET    /api/a001/connection_1c/:id
POST   /api/a001/connection_1c
PUT    /api/a001/connection_1c/:id
DELETE /api/a001/connection_1c/:id
```

### UseCases
```
POST /api/u501/import/start
GET  /api/u501/import/:session_id/progress
GET  /api/u501/import/history
```

## Преимущества системы индексов

1. **Явное разделение** - сразу видно агрегат (a*) или операция (u*)
2. **Нет коллизий** - 499 слотов для агрегатов, 499 для операций
3. **Масштабируемость** - достаточно для большинства проектов
4. **Навигация** - легко найти код по индексу (u501 → grep "u501")
5. **URL-friendly** - `/api/u501/import/start` понятнее чем `/api/import/start`
6. **БД изоляция** - таблицы `u501_*` не пересекаются с `a001_*`
7. **Документация** - можно автоматически генерировать справку по индексам

## Семантическая группировка UseCases

Рекомендуемая разметка по диапазонам:

- **u501-u549**: Импорт данных (import_from_*)
- **u550-u599**: Экспорт данных (export_to_*)
- **u600-u649**: Синхронизация (sync_*)
- **u650-u699**: Отчеты (report_*)
- **u700-u749**: Бизнес-процессы (process_*)
- **u750-u799**: Утилиты (utility_*)
- **u800-u849**: Администрирование (admin_*)
- **u850-u899**: Интеграции (integration_*)
- **u900-u999**: Резерв

## Trait UseCaseMetadata

Каждый UseCase должен реализовывать `UseCaseMetadata`:

```rust
impl UseCaseMetadata for ImportFromUt {
    fn usecase_index() -> &'static str { "u501" }
    fn usecase_name() -> &'static str { "import_from_ut" }
    fn display_name() -> &'static str { "Импорт из УТ 11" }
    fn description() -> &'static str {
        "Загрузка справочников из 1С:Управление торговлей 11 через OData"
    }
}
```

## Контрольный список при создании нового UseCase

- [ ] Создать структуру в `contracts/usecases/uNNN_name/`
- [ ] Определить Request/Response DTOs
- [ ] Определить события (если нужны)
- [ ] Создать executor в `backend/usecases/uNNN_name/`
- [ ] Реализовать `UseCaseMetadata`
- [ ] Добавить API endpoints в `backend/main.rs`
- [ ] Создать UI (если нужен) в `frontend/usecases/uNNN_name/`
- [ ] Добавить таблицы БД (если нужны)
- [ ] Обновить документацию в `memory-bank/`
