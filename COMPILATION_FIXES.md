# Исправления ошибок компиляции JWT Auth System

## ✅ Статус: ВСЕ ОШИБКИ ИСПРАВЛЕНЫ

**Backend:** 0 ошибок ✅  
**Frontend:** 0 ошибок ✅  

## 🎯 Системная проблема (Корневая причина)

### Проблема #1: Неправильная зависимость в contracts

**`jsonwebtoken` был ошибочно добавлен в `crates/contracts/Cargo.toml`:**

```
Frontend → contracts → jsonwebtoken → ring → требует clang для WASM
```

**Ошибка:**
```
error: failed to find tool "clang": program not found
error occurred in cc-rs: failed to find tool "clang"
```

**Решение:** Удалить `jsonwebtoken = "9"` из `contracts/Cargo.toml`

**Обоснование:** Contracts crate должен содержать ТОЛЬКО DTOs и shared types. Backend уже имеет `jsonwebtoken` в своих зависимостях, где и должна выполняться вся криптография.

### Проблема #2: Дублирующаяся функция get_connection()

В `crates/backend/src/shared/data/db.rs` были ДВЕ функции `get_connection()`:
- Строки 6-10: возвращала `Result<&'static DatabaseConnection>`  
- Строка 2347: возвращала `&'static DatabaseConnection`

JWT код использовал первую версию с `?` оператором, что вызывало ошибки компиляции.

**Решение:** Удалить дублирующую функцию (строки 6-10)

### Проблема #3: Leptos 0.6 → 0.8 несовместимость

JWT auth код был написан для Leptos 0.6, но проект использует Leptos 0.8 с новыми API.

---

## 🔧 Детальные исправления

### 1. Backend исправления

#### db.rs - Удаление дубликата
**Файл:** `crates/backend/src/shared/data/db.rs`
- Удалены строки 6-10 (дублирующаяся функция)
- Оставлена единственная версия на строке 2347

#### Обновление вызовов get_connection()
Заменено `get_connection()?` на `get_connection()` в:
- `src/system/auth/jwt.rs` - 2 места (строки 83, 108)
- `src/system/users/repository.rs` - 10 мест (все `let conn = get_connection()?;`)
- `src/system/handlers/auth.rs` - 3 места (строки 112, 138, 165)
- `src/system/initialization.rs` - 1 место (строка 12)

### 2. Contracts исправления

#### Cargo.toml - Удаление jsonwebtoken
**Файл:** `crates/contracts/Cargo.toml`
```diff
 [dependencies]
 serde = { workspace = true }
 serde_json = { workspace = true }
 chrono = { workspace = true }
 uuid = { workspace = true }
 anyhow = "1"
-jsonwebtoken = "9"
```

### 3. Frontend исправления (Leptos 0.8)

#### Обновление импортов
**Файлы с изменениями:**
- `src/system/auth/context.rs`
- `src/system/auth/guard.rs`
- `src/system/pages/login.rs`
- `src/system/users/ui/list/mod.rs`
- `src/system/users/ui/details/mod.rs`

**Было:**
```rust
use leptos::*;
use leptos_router::*;
```

**Стало:**
```rust
use leptos::prelude::*;
use leptos::task::spawn_local;  // где нужно
use leptos_router::hooks::use_navigate;  // где нужно
```

#### Синтаксис For компонента
**Файл:** `src/system/users/ui/list/mod.rs`

**Было (Leptos 0.6):**
```rust
<For
    each=move || users.get()
    key=|user| user.id.clone()
    children=move |user: User| {
        view! { ... }
    }
/>
```

**Стало (Leptos 0.8):**
```rust
<For
    each=move || users.get()
    key=|user| user.id.clone()
    let:user
>
    {
        view! { ... }
    }
</For>
```

#### Тип Children
**Файл:** `src/system/auth/guard.rs`

**Было:**
```rust
children: Children
```

**Стало:**
```rust
children: ChildrenFn
```

#### События
**Файлы:** `src/system/pages/login.rs`, `src/system/users/ui/details/mod.rs`

**Было:**
```rust
move |ev: ev::SubmitEvent| {
```

**Стало:**
```rust
move |ev: leptos::ev::SubmitEvent| {
```

#### Клонирование для замыканий
**Файл:** `src/system/pages/login.rs`

Добавлено `let navigate = navigate.clone();` перед `spawn_local`, чтобы избежать проблем с FnOnce/FnMut.

#### Отображение строк
**Файл:** `src/system/users/ui/list/mod.rs`

**Было:**
```rust
<td>{&user.username}</td>
```

**Стало:**
```rust
<td>{user.username.clone()}</td>
```

---

## 📊 Результаты

### До исправлений
- **Backend:** 66 ошибок компиляции
- **Frontend:** 79 ошибок компиляции + критическая ошибка "clang not found"

### После исправлений
- **Backend:** ✅ 0 ошибок (2 warning - unused variables)
- **Frontend:** ✅ 0 ошибок (33 warnings - dead code, не критично)

**Оба проекта успешно компилируются!** 🎉

---

## 🏗️ Архитектурные принципы

### Contracts Crate
- ✅ Содержит ТОЛЬКО DTOs и shared types
- ❌ НЕ должен содержать криптографические библиотеки
- ❌ НЕ должен содержать бизнес-логику
- ❌ НЕ должен содержать database операции

### Backend
- Backend имеет `jsonwebtoken` в своих зависимостях
- Вся криптография JWT выполняется в backend
- `get_connection()` возвращает прямую ссылку, без Result

### Frontend  
- Использует Leptos 0.8 API
- Не зависит от криптографических библиотек
- Компилируется в WASM без необходимости в clang

---

---

## 🔥 Runtime исправления (tokio panic)

### Проблема #4: "Cannot start a runtime from within a runtime"

После исправления компиляции backend падал при запуске:

```
thread 'tokio-runtime-worker' panicked at crates\backend\src\system\auth\jwt.rs:85:26:
Cannot start a runtime from within a runtime.
```

**Причина:** JWT функции использовали `tokio::runtime::Handle::current().block_on()` внутри уже работающего async runtime.

**Решение:** Переделать все JWT функции в async и использовать `.await`:
- `get_jwt_secret()` → `async fn` + `.await`
- `get_jwt_secret_from_db()` → `async fn`  
- `save_jwt_secret_to_db()` → `async fn`
- `generate_access_token()` → `async fn` + `.await` в вызовах
- `validate_token()` → `async fn` + `.await` в middleware

### Проблема #5: Неправильный путь к SQL миграции

Backend искал `migrate_auth_system.sql` в текущей директории (`crates/backend`), но файл лежит в корне проекта.

**Решение:** Добавлен fallback:
```rust
let migration_sql = std::fs::read_to_string("migrate_auth_system.sql")
    .or_else(|_| std::fs::read_to_string("../../migrate_auth_system.sql"))
    .context("Failed to read migrate_auth_system.sql")?;
```

### Проблема #6: SQL миграция пропускала CREATE TABLE

SQL разделялся по `;`, но statements начинающиеся с `--` комментариев пропускались целиком, включая SQL код после комментария.

**Решение:** Фильтровать комментарии построчно:
```rust
let cleaned: String = statement
    .lines()
    .filter(|line| {
        let trimmed_line = line.trim();
        !trimmed_line.is_empty() && !trimmed_line.starts_with("--")
    })
    .collect::<Vec<_>>()
    .join("\n");
```

**Подробнее:** См. `ASYNC_JWT_FIXES.md`

---

## ✨ Готово к использованию!

JWT authentication система полностью реализована и готова к работе.

Все существующие бизнес-роуты пока работают без авторизации для обратной совместимости. Когда будете готовы, просто добавьте middleware к нужным роутам.

См. **`AUTH_SYSTEM_README.md`** для полной документации и инструкций по использованию.
