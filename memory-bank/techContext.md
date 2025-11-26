# Technical Context

## Технологический стек

### Core Technologies

**Frontend:**
- **Leptos 0.6.x** - Реактивный Rust фреймворк для веба
- **WASM** - Компиляция Rust в WebAssembly
- **Trunk** - Build tool и dev server для Leptos
- **Signals** - Реактивная система управления состоянием

**Backend:**
- **Axum** - Async web framework на Tokio
- **SQLite** - Встроенная база данных
- **Rusqlite** - SQLite driver для Rust
- **Tokio** - Async runtime

**Shared:**
- **Serde** - Сериализация/десериализация (JSON)
- **Chrono** - Работа с датами и временем
- **UUID** - Генерация уникальных идентификаторов
- **Once_cell** - Глобальное состояние

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

**Strategy:**
- SQL файлы с префиксом `migrate_*.sql`
- Ручное применение или через `migrate_db.py`

**Examples:**
```powershell
# Применить миграцию вручную
sqlite3 marketplace.db < migrate_a014_posting_ref.sql

# Или через Python скрипт
python migrate_db.py
```

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

1. **Start backend**:
   ```powershell
   cargo run --bin backend
   ```

2. **Start frontend** (in another terminal):
   ```powershell
   trunk serve --port 8080
   ```

3. **Open browser**: http://localhost:8080

4. **Make changes**:
   - Frontend hot reload автоматически
   - Backend требует restart (Ctrl+C, cargo run снова)

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

## Known Issues & Workarounds

### Windows-Specific
- **Command chaining**: Использовать `;` вместо `&&`
- **Paths**: Использовать `\` или `/` (оба работают в PowerShell)
- **Line endings**: Git должен использовать CRLF для .bat файлов

### SQLite
- **Locked database**: Закрыть все connections перед миграцией
- **Performance**: Индексы критичны для больших таблиц

### WASM
- **Debug build size**: Очень большие, использовать --release для тестирования
- **Async**: Требует wasm-bindgen-futures

## Dependencies Updates

### Strategy
- **Regular updates**: Проверять обновления ежемесячно
- **Breaking changes**: Читать CHANGELOG перед обновлением major версий
- **Security**: Использовать `cargo audit` для проверки уязвимостей

### Key Dependencies to Watch
- **Leptos**: Активно развивается, breaking changes возможны
- **Axum**: Стабильный, редкие breaking changes
- **Serde**: Очень стабильный

## Resources & Documentation

### Official Docs
- Leptos: https://book.leptos.dev/
- Axum: https://docs.rs/axum/
- Rust: https://doc.rust-lang.org/

### Project-Specific
- See `memory-bank/architecture/` для архитектурных деталей
- See `memory-bank/features/` для документации по фичам
- See `.cursorrules` для quick reference

## Environment Variables

### Backend
```
DATABASE_URL=marketplace.db  # SQLite database path (optional, defaults to marketplace.db)
RUST_LOG=info                # Logging level
```

### Frontend
Обычно не требуются, конфигурация в Trunk.toml

## CI/CD

Currently: **Manual deployment**
- Build locally
- Test manually
- Deploy as desktop application

Future: Потенциально GitHub Actions для automated builds

