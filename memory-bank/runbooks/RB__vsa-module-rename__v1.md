---
date: 2026-01-11
type: runbook
version: v1
topic: VSA Module Rename
tags:
  - vsa
  - refactoring
  - rust
  - procedure
---

# Runbook: Переименование модуля в VSA архитектуре

## Когда использовать

При необходимости переименовать модуль (каталог) в одном или нескольких слоях (contracts, backend, frontend) с сохранением консистентности VSA.

## Предварительные условия

- Проект компилируется (`cargo check --workspace` проходит)
- Нет незакоммиченных изменений (или готовы их потерять)

## Процедура

### Шаг 1: Аудит затрагиваемых файлов

```powershell
# Найти все файлы с упоминанием старого имени
rg "old_module_name" --files-with-matches crates/
```

Записать список файлов для последующей проверки.

### Шаг 2: Переименование каталогов

```powershell
# Contracts
Rename-Item -Path "crates\contracts\src\{category}\old_name" -NewName "new_name"

# Backend
Rename-Item -Path "crates\backend\src\{category}\old_name" -NewName "new_name"

# Frontend (если отличается)
Rename-Item -Path "crates\frontend\src\{category}\old_name" -NewName "new_name"
```

### Шаг 3: Обновление mod.rs

Обновить `mod.rs` в каждом затронутом родительском каталоге:

- `contracts/src/{category}/mod.rs`
- `backend/src/{category}/mod.rs`
- `frontend/src/{category}/mod.rs`

```rust
// Было
pub mod old_name;

// Стало
pub mod new_name;
```

### Шаг 4: Проверка компиляции и исправление импортов

```powershell
cargo check --workspace 2>&1 | Select-String "error\[E0433\]"
```

Для каждой ошибки обновить импорты:

```rust
// Было
use contracts::{category}::old_name::SomeType;
use crate::{category}::old_name::something;

// Стало
use contracts::{category}::new_name::SomeType;
use crate::{category}::new_name::something;
```

### Шаг 5: Обновление handlers (если применимо)

Если модуль имеет handler в `backend/src/handlers/`:

1. Переименовать файл handler'а
2. Обновить `handlers/mod.rs`
3. Обновить `routes.rs`

### Шаг 6: Обновление таблицы БД (если применимо)

1. Обновить `#[sea_orm(table_name = "...")]` в repository.rs
2. Обновить код создания таблицы в `shared/data/db.rs`
3. Создать SQL миграцию:

```sql
ALTER TABLE old_table_name RENAME TO new_table_name;
```

### Шаг 7: Финальная проверка

```powershell
cargo check --workspace
cargo test --workspace  # если есть тесты
```

## Типичные ошибки

### Забытые внутренние ссылки

**Симптом:** `cargo check` показывает ошибки в файлах внутри переименованного каталога.

**Решение:** Проверить файлы внутри каталога на наличие `crate::{category}::old_name` или `super::old_name`.

### UI идентификаторы

**Симптом:** Вкладки или меню перестают работать.

**Решение:** UI идентификаторы (tab keys, sidebar keys) независимы от путей к модулям. Их можно оставить без изменений или обновить отдельно во всех местах:

- `layout/tabs/registry.rs`
- `layout/left/sidebar.rs`
- Компоненты, открывающие вкладки

## Checklist

- [ ] Каталоги переименованы во всех слоях
- [ ] mod.rs обновлены
- [ ] Импорты обновлены (contracts:: и crate::)
- [ ] Handlers переименованы
- [ ] routes.rs обновлён
- [ ] SeaORM table_name обновлён (если есть)
- [ ] SQL миграция создана (если есть БД)
- [ ] `cargo check --workspace` проходит
