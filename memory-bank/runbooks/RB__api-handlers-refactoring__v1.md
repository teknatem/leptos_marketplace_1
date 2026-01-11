---
type: runbook
version: 1
date: 2026-01-11
topic: API Handlers Refactoring
tags: [runbook, refactoring, handlers, api]
---

# Runbook: Рефакторинг API Handlers

## Назначение

Пошаговая инструкция для перемещения handlers между модулями в backend.

## Предусловия

- [ ] Нет незакоммиченных изменений в git
- [ ] Backend компилируется (`cargo check --bin backend`)

## Шаги

### 1. Подготовка

```powershell
# Проверить текущее состояние
cargo check --bin backend
git status
```

### 2. Поиск всех зависимостей

**КРИТИЧЕСКИ ВАЖНО**: Перед перемещением найти все ссылки на перемещаемые модули:

```powershell
# В PowerShell или используя ripgrep
rg "crate::handlers::" crates/backend/src/
rg "use crate::handlers" crates/backend/src/
```

Записать все найденные файлы — их нужно будет обновить.

### 3. Создание целевой структуры

```powershell
mkdir crates/backend/src/api
mkdir crates/backend/src/api/handlers
# или для system:
mkdir crates/backend/src/system/api
mkdir crates/backend/src/system/api/handlers
```

### 4. Перемещение файлов

```powershell
Move-Item "src/handlers/a001_*.rs" "src/api/handlers/"
```

### 5. Создание mod.rs

Создать `api/handlers/mod.rs` со списком всех модулей:
```rust
pub mod a001_connection_1c;
pub mod a002_organization;
// ...
```

### 6. Создание routes.rs

Вынести соответствующие маршруты в `api/routes.rs`:
```rust
use axum::{routing::{get, post}, Router};
use super::handlers;

pub fn configure_business_routes() -> Router {
    Router::new()
        .route("/api/...", get(handlers::...))
}
```

### 7. Создание api/mod.rs

```rust
pub mod handlers;
mod routes;

pub use routes::configure_business_routes;
```

### 8. Обновление main.rs

```rust
// Было:
pub mod handlers;
pub mod routes;
let app = routes::configure_routes()

// Стало:
pub mod api;
let app = Router::new()
    .merge(system::api::configure_system_routes())
    .merge(api::configure_business_routes())
```

### 9. Обновление зависимостей

Обновить все файлы, найденные в шаге 2:
```rust
// Было:
use crate::handlers::a016_ym_returns::SomeDto;

// Стало:
use crate::api::handlers::a016_ym_returns::SomeDto;
```

### 10. Удаление старых файлов

```powershell
Remove-Item "src/handlers/mod.rs"
Remove-Item "src/handlers" -Force
Remove-Item "src/routes.rs"
```

### 11. Проверка

```powershell
cargo check --bin backend
cargo run --bin backend
```

## Частые ошибки

### E0433: failed to resolve: unresolved import

**Причина**: Не обновлён путь импорта в каком-то файле.

**Решение**: Найти файл по сообщению об ошибке и обновить `use crate::handlers::` на `use crate::api::handlers::`.

### Domain зависит от handlers

**Симптом**: `domain/*/repository.rs` импортирует DTO из handlers.

**Решение**: Либо обновить путь, либо (лучше) перенести DTO в contracts.

## Чеклист после рефакторинга

- [ ] `cargo check --bin backend` — без ошибок
- [ ] `cargo run --bin backend` — сервер запускается
- [ ] Проверить endpoints: `/health`, `/api/system/auth/login`, бизнес-endpoint
- [ ] Git commit с описательным сообщением
