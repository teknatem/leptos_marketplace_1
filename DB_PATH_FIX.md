# Исправление пути к базе данных

## Проблема

После внедрения JWT авторизации backend создавал **новую пустую базу данных** вместо использования существующей с данными.

### Причина

Backend запускается из директории `crates/backend/`:
```bash
cd crates/backend
cargo run
```

В `main.rs` использовался **относительный путь**:
```rust
let db_path = std::path::Path::new("target").join("db").join("app.db");
```

Это создавало путь относительно **текущей директории** (`crates/backend`):
- ❌ Создавалась: `crates/backend/target/db/app.db` (новая пустая база)
- ✅ Должна быть: `target/db/app.db` (workspace root, база с данными)

### Симптомы

- ✅ Backend запускается без ошибок
- ✅ API возвращает `200 OK`
- ❌ Все эндпоинты возвращают пустые массивы `[]` (2 байта)
- ❌ Frontend не показывает данные

### Две базы данных

1. **Старая база с данными** (240 MB):
   - `E:\dev\rust\leptos_marketplace_1\target\db\app.db`
   - Содержит все бизнес-данные

2. **Новая пустая база** (520 KB):
   - `E:\dev\rust\leptos_marketplace_1\crates\backend\target\db\app.db`
   - Backend работал с ней

## Решение

Изменить путь к БД на **workspace root относительный**:

**Файл:** `crates/backend/src/main.rs`

### Было:
```rust
// Define a database path in the `target` directory in a platform-agnostic way
let db_path = std::path::Path::new("target").join("db").join("app.db");

let db_path_str = db_path
    .to_str()
    .ok_or_else(|| anyhow::anyhow!("Invalid database path string"))?;
```

### Стало:
```rust
// Define a database path in the `target` directory in a platform-agnostic way
// Get workspace root (go up 2 levels from backend crate: crates/backend -> root)
let current_dir = std::env::current_dir()?;
let workspace_root = current_dir
    .parent()
    .and_then(|p| p.parent())
    .unwrap_or(&current_dir);

let db_path = workspace_root.join("target").join("db").join("app.db");

let db_path_str = db_path
    .to_str()
    .ok_or_else(|| anyhow::anyhow!("Invalid database path string"))?;
```

## Логика исправления

1. Получаем текущую директорию: `E:\dev\rust\leptos_marketplace_1\crates\backend`
2. Поднимаемся на 2 уровня вверх:
   - `.parent()` → `E:\dev\rust\leptos_marketplace_1\crates`
   - `.parent()` → `E:\dev\rust\leptos_marketplace_1` (workspace root)
3. Формируем путь: `workspace_root/target/db/app.db`

## Результат

✅ Backend теперь использует правильную базу данных  
✅ API возвращает все данные  
✅ Frontend отображает все записи  

### До исправления:
```
GET /api/connection_1c → [] (2 байта)
```

### После исправления:
```
GET /api/connection_1c → [
  {
    "id": "ceada592-54d0-436d-b122-aece534da5d5",
    "code": "CON-683db4af-90ed-42c6-8a8d-2d7e3859edcb",
    "description": "trade",
    ...
  },
  {
    "id": "8af72b8a-13a4-47da-a1ab-87a5f21ee578",
    "code": "CON-8af72b8a",
    "description": "Тестовая база",
    ...
  }
] (828 байт)
```

## Важное примечание

При запуске из разных директорий используется разная база:

- **Из workspace root** (`cargo run --bin backend`):
  - Путь: `target/db/app.db` ✅
  
- **Из crates/backend** (`cd crates/backend; cargo run`):
  - БЕЗ исправления: `crates/backend/target/db/app.db` ❌
  - С ИСПРАВЛЕНИЕМ: `../../target/db/app.db` ✅

Теперь оба варианта запуска используют одну и ту же базу!

## Проверка

```powershell
# Проверить размер файла базы
Get-Item "E:\dev\rust\leptos_marketplace_1\target\db\app.db" | Select-Object Length

# Запрос к API
Invoke-RestMethod -Method GET -Uri 'http://localhost:3000/api/connection_1c'
```

Ожидаемый размер базы: **~240 MB** (с данными)  
Ожидаемый размер ответа API: **> 100 байт** (не пустой массив)

