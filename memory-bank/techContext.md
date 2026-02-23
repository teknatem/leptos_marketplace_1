# Technical Context

## Технологический стек

### Core Technologies

**Frontend:**

- **Leptos 0.8** - Реактивный Rust фреймворк для веба
- **Thaw UI 0.5.0-beta** - Компонентная библиотека для Leptos
- **WASM** - Компиляция Rust в WebAssembly
- **Trunk** - Build tool и dev server для Leptos
- **Signals** - Реактивная система управления состоянием

**Backend:**

- **Axum 0.7** - Async web framework на Tokio
- **SQLite** - Встроенная база данных
- **Sea-ORM 0.12** - ORM для работы с БД
- **Tokio 1.x** - Async runtime

**Shared:**

- **Serde 1.x** - Сериализация/десериализация (JSON)
- **Chrono 0.4** - Работа с датами и временем
- **UUID 1.11** - Генерация уникальных идентификаторов
- **Once_cell 1.x** - Глобальное состояние

### Thaw UI Integration

**Компонентная библиотека:** Thaw UI 0.5.0-beta предоставляет готовые React-подобные компоненты для Leptos.

**Основные компоненты:**

- **ConfigProvider** - корневой компонент для управления темами
- **Table** - таблицы с встроенной поддержкой сортировки
- **Button** - кнопки с различными вариантами стилей
- **Input, Select** - формы ввода данных
- **DatePicker** - выбор дат

**Темизация:**

- Theme switching: light / dark / forest
- CSS переменные: `--colorNeutralBackground1`, `--colorBrandBackground`, и др.
- Программная модификация через DOM API (для кастомизации)

**Гибридный подход:**

- Некоторые таблицы используют Thaw `<Table>`
- Другие используют нативный HTML `<table>` для полного контроля
- Оба подхода поддерживаются и применяются по необходимости

**Документация:**

- См. `memory-bank/runbooks/RB-thaw-table-sorting-v1.md` - добавление сортировки
- См. `memory-bank/runbooks/RB-thaw-ui-migration-v1.md` - миграция на Thaw
- См. `memory-bank/known-issues/KI-thaw-table-style-limitations-2025-12-21.md` - ограничения

### Workspace Structure

```toml
[workspace]
members = [
    "crates/frontend",
    "crates/backend",
    "crates/contracts",
]
```

**contracts**: Shared DTOs и типы

- Dependency для frontend и backend
- Обеспечивает type safety на всем стеке

**backend**: Axum server

- HTTP API endpoints
- Database access (SQLite)
- Business logic
- External integrations (1C, Wildberries, Ozon)

**frontend**: Leptos WASM app

- UI components
- API client
- State management
- Routing

## Development Environment

### OS & Shell

- **OS**: Windows 11
- **Shell**: PowerShell
- **Important**: НЕ использовать `&&` для цепочки команд - использовать `;` или отдельные команды

### Node.js Ecosystem

- **Package manager**: `pnpm` (НЕ npm или yarn)
- Используется для некоторых dev tools

### Rust Toolchain

- **Edition**: 2021
- **Toolchain**: Stable
- **Target**: wasm32-unknown-unknown (для frontend)

## Build & Development

### Backend Development

```powershell
# Запуск backend сервера
# ВАЖНО: при старте автоматически применяются pending-миграции из migrations/
cargo run --bin backend

# Backend слушает на http://localhost:3000
```

### Frontend Development

```powershell
# Development server с hot reload
trunk serve --port 8080

# Frontend доступен на http://localhost:8080
# Проксирует API запросы на backend:3000
```

### Production Build

```powershell
# Build backend
cargo build --release --bin backend

# Build frontend
trunk build --release

# Результат в dist/
```

### Common Commands

**Cargo:**

```powershell
# Check код без компиляции
cargo check

# Format code
cargo fmt

# Run clippy
cargo clippy

# Run tests
cargo test

# Update dependencies
cargo update
```

**Trunk:**

```powershell
# Clean build artifacts
trunk clean

# Build without serving
trunk build

# Serve with specific port
trunk serve --port 8080 --open
```

## Database

### SQLite Configuration

- **File**: `marketplace.db` в корне проекта
- **Version**: SQLite 3
- **Encoding**: UTF-8
- **Journal mode**: WAL (Write-Ahead Logging) рекомендуется

### Migrations

**Стратегия:** Формальная система через `sqlx::migrate::Migrator` (с 2026-02-18).

**Автоматическое применение при старте:**

Все pending-миграции применяются автоматически при каждом запуске `cargo run --bin backend`.

**Добавить новую миграцию:**

```powershell
# Создать файл с номером после последнего
# Файл: migrations/0002_description.sql

# При следующем запуске backend применится автоматически
cargo run --bin backend
```

**Проверить состояние:**

```powershell
# Через sqlite3 CLI
sqlite3 marketplace.db "SELECT version, description, installed_on FROM _sqlx_migrations"

# Или через sqlx-cli (если установлен):
sqlx migrate info --database-url "sqlite:marketplace.db"
```

**Структура:**

```
migrations/
├── 0001_baseline_schema.sql   <- полная схема при первом запуске
├── 0002_...sql                <- новые изменения (добавлять сюда)
└── archive/                   <- старые migrate_*.sql (только история)
```

**Ключевые файлы:**

- `crates/backend/src/shared/data/migration_runner.rs` — запускает `sqlx::migrate::Migrator`
- `crates/backend/src/shared/data/db.rs` — только коннект (`get_connection()` + `migrate_wb_sales_denormalize()`)

### Database Tools

- **CLI**: `sqlite3` - консольный клиент
- **GUI**: DB Browser for SQLite, DBeaver

## External Integrations

### 1C:Управление торговлей 11

**Protocol**: OData v4
**Auth**: Basic authentication
**Endpoints**:

- Organizations: `/odata/standard.odata/Catalog_Организации`
- Nomenclature: `/odata/standard.odata/Catalog_Номенклатура`

**Client implementation**: `backend/src/usecases/u501_import_from_ut/ut_odata_client.rs`

### Wildberries API

**Auth**: Token-based (x-api-key header)
**Base URL**: `https://statistics-api.wildberries.ru`
**Rate limits**: Требуется обработка 429 ответов

**Endpoints:**

- Sales: `/api/v1/supplier/sales`
- Orders: `/api/v1/supplier/orders`
- Stocks: `/api/v1/supplier/stocks`
- Finance reports: Various endpoints

### Ozon API

**Auth**: Client-Id + Api-Key headers
**Base URL**: `https://api-seller.ozon.ru`

**Endpoints:**

- Transactions: `/v3/finance/transaction/list`
- Products: `/v2/product/list`
- Orders: `/v3/posting/fbs/list`

### LemanaPro

**Auth**: API Key
**Integration**: Analytics and planning data

## Technical Constraints

### Database

- **SQLite only** - не PostgreSQL, MySQL и т.д.
- **Single file** - вся БД в одном файле
- **No concurrent writes** - SQLite ограничение, но для desktop app достаточно
- **Max DB size** - практически не ограничено для наших целей

### Frontend

- **WASM bundle size** - следить за размером артефактов
- **No multithreading** - WASM пока без threads
- **Browser compatibility** - современные браузеры (ES2018+)

### Backend

- **Async runtime** - Tokio, все I/O operations асинхронные
- **Single instance** - приложение для одного пользователя

## Development Workflow

### Typical Development Session

1. **Start backend** (миграции применятся автоматически):

   ```powershell
   cargo run --bin backend
   # В логах: "✓ Database migrations processed"
   ```

2. **Start frontend** (in another terminal):

   ```powershell
   trunk serve --port 8080
   ```

3. **Open browser**: http://localhost:8080

4. **Make changes**:
   - Frontend hot reload автоматически
   - Backend требует restart (Ctrl+C, cargo run снова)

5. **Добавить изменение схемы БД**:
   - Создать `migrations/NNNN_description.sql`
   - Restart backend — миграция применится автоматически

### Debugging

**Backend logs:**

```rust
// В коде
println!("Debug: {:?}", value);
log::info!("Information");
log::error!("Error occurred");
```

**Frontend logs:**

```rust
// В коде
leptos::logging::log!("Debug message");
web_sys::console::log_1(&"Value".into());
```

**Browser DevTools:**

- Console для WASM logs
- Network для API calls
- Application для локальное хранилище

## Configuration Files

### Cargo.toml (root)

- Workspace definition
- Shared dependencies

### Cargo.toml (each crate)

- Crate-specific dependencies
- Build configuration

### Trunk.toml

- Frontend build configuration
- Asset pipeline
- Dev server settings

### .cursorrules

- AI assistant project context
- Critical patterns и rules

### config.toml (рядом с backend.exe)

```toml
[database]
path = "C:/path/to/data/app.db"   # абсолютный путь рекомендуется

[scheduled_tasks]
enabled = true                     # false для dev без фонового воркера
```

- При отсутствии файла используется дефолт: `target/db/app.db`
- `build.rs` backend автоматически копирует `config.toml` из корня проекта в `target/debug/` при сборке

## Known Issues & Workarounds

### Windows-Specific

- **Command chaining**: Использовать `;` вместо `&&`
- **Paths**: Использовать `\` или `/` (оба работают в PowerShell)
- **Line endings**: Git должен использовать CRLF для .bat файлов

### SQLite

- **Locked database**: Закрыть все connections перед инспекцией (не перед миграцией — миграции теперь авто)
- **Performance**: Индексы критичны для больших таблиц
- **Migration checksum**: Никогда не редактировать уже применённые файлы в `migrations/` — sqlx проверяет checksum

### WASM

- **Debug build size**: Очень большие, использовать --release для тестирования
- **Async**: Требует wasm-bindgen-futures

## Dependencies Updates

### Strategy

- **Regular updates**: Проверять обновления ежемесячно
- **Breaking changes**: Читать CHANGELOG перед обновлением major версий
- **Security**: Использовать `cargo audit` для проверки уязвимостей

### Key Dependencies to Watch

- **Leptos 0.8**: Активно развивается, следить за минорными обновлениями
- **Thaw UI 0.5.0-beta**: Beta версия, возможны изменения API
- **Axum 0.7**: Стабильный, редкие breaking changes
- **Sea-ORM 0.12**: ORM используется для доменных запросов (repositories)
- **sqlx 0.7**: Используется для миграций (`sqlx::migrate::Migrator`)
- **Serde**: Очень стабильный

## Resources & Documentation

### Official Docs

- Leptos: https://book.leptos.dev/
- Axum: https://docs.rs/axum/
- Rust: https://doc.rust-lang.org/

### Project-Specific

- See `memory-bank/architecture/` для архитектурных деталей
- See `memory-bank/runbooks/RB_db-migration-workflow_v1.md` — как добавлять миграции
- See `memory-bank/features/` для документации по фичам
- See `.cursorrules` для quick reference

## Environment Variables

### Backend

Конфигурация через `config.toml` (рядом с исполняемым файлом), не через env vars.

```toml
[database]
path = "E:/data/app.db"     # путь к SQLite (абсолютный или относительный от .exe)

[scheduled_tasks]
enabled = true
```

Env var для уровня логирования (опционально):
```
RUST_LOG=info               # или debug, warn, error
```

### Frontend

Обычно не требуются, конфигурация в Trunk.toml

## CI/CD

Currently: **Manual deployment**

- Build locally
- Test manually
- Deploy as desktop application

Future: Потенциально GitHub Actions для automated builds
