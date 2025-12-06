# JWT Authentication System - Implementation Complete

## ✅ Что было реализовано

### Backend

1. **JWT инфраструктура** (`crates/backend/src/system/auth/`)

   - `jwt.rs` - генерация и валидация JWT токенов (24 часа lifetime)
   - `password.rs` - хеширование паролей через Argon2
   - `middleware.rs` - middleware для защиты endpoints (`require_auth`, `require_admin`)
   - `extractor.rs` - `CurrentUser` extractor для handlers

2. **Users управление** (`crates/backend/src/system/users/`)

   - `repository.rs` - CRUD операции с пользователями
   - `service.rs` - бизнес-логика (создание, обновление, смена пароля, проверка credentials)

3. **API Endpoints** (`crates/backend/src/system/handlers/`)

   - `POST /api/system/auth/login` - вход
   - `POST /api/system/auth/refresh` - обновление access token
   - `POST /api/system/auth/logout` - выход
   - `GET /api/system/auth/me` - получить текущего пользователя
   - `GET /api/system/users` - список пользователей (admin only)
   - `POST /api/system/users` - создать пользователя (admin only)
   - `PUT /api/system/users/:id` - обновить пользователя (admin only)
   - `DELETE /api/system/users/:id` - удалить пользователя (admin only)
   - `POST /api/system/users/:id/change-password` - сменить пароль

4. **Инициализация** (`crates/backend/src/system/initialization.rs`)
   - Автоматическое применение SQL миграции при первом запуске
   - Создание admin/admin пользователя если БД пустая
   - Автогенерация JWT_SECRET и сохранение в `sys_settings`

### Frontend

1. **Auth контекст** (`crates/frontend/src/system/auth/`)

   - `context.rs` - глобальное состояние авторизации (AuthProvider, use_auth hook)
   - `storage.rs` - работа с localStorage для токенов
   - `api.rs` - API клиент для авторизации
   - `guard.rs` - компоненты RequireAuth и RequireAdmin для защиты роутов

2. **UI компоненты** (`crates/frontend/src/system/`)

   - `pages/login.rs` - страница входа
   - `users/ui/list/mod.rs` - список пользователей с возможностью удаления
   - `users/ui/details/mod.rs` - форма создания пользователя
   - `users/api.rs` - API клиент для управления пользователями

3. **Интеграция**
   - `app.rs` - обертка в AuthProvider
   - `routes.rs` - показ LoginPage если не авторизован, иначе MainLayout
   - `styles/3-components/login.css` - стили для login page и модальных окон

### Database

- **SQL миграция** (`migrate_auth_system.sql`)
  - `sys_settings` - системные настройки (JWT_SECRET)
  - `sys_users` - пользователи
  - `sys_refresh_tokens` - refresh токены
  - `sys_audit_log` - лог аудита (готов к использованию)

### Contracts

- **Shared types** (`crates/contracts/src/system/`)
  - `auth.rs` - LoginRequest, LoginResponse, TokenClaims, RefreshRequest, etc.
  - `users.rs` - User, CreateUserDto, UpdateUserDto, ChangePasswordDto

## 🚀 Как запустить

### 1. Первый запуск backend

```bash
cd crates/backend
cargo run
```

При первом запуске вы увидите:

```
═══════════════════════════════════════════════
  Default admin user created!
  Username: admin
  Password: admin
  User ID: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
  ⚠️  PLEASE CHANGE THE PASSWORD IMMEDIATELY!
═══════════════════════════════════════════════
```

### 2. Запуск frontend

```bash
cd crates/frontend
trunk serve --port 8080
```

### 3. Вход в систему

Откройте браузер: `http://localhost:8080`

Вы увидите страницу логина. Используйте:

- **Username**: `admin`
- **Password**: `admin`

После успешного входа вы попадете в главное приложение.

## 🔐 Управление пользователями

### Доступ к управлению пользователями

Управление пользователями доступно **только администраторам**.

Чтобы открыть страницу управления пользователями, добавьте в код вызов компонента:

```rust
use crate::system::users::ui::list::UsersListPage;

// В каком-то месте UI:
view! {
    <UsersListPage />
}
```

### Создание нового пользователя

1. Нажмите "Добавить пользователя"
2. Заполните форму:
   - Username (обязательно)
   - Password (обязательно, минимум 4 символа)
   - Email (опционально)
   - Full Name (опционально)
   - Administrator (чекбокс)
3. Нажмите "Create User"

### Удаление пользователя

Нажмите кнопку "Delete" рядом с пользователем и подтвердите действие.

## 🔧 Технические детали

### Время жизни токенов

- **Access Token**: 24 часа
- **Refresh Token**: 90 дней (хранится в localStorage)

### Безопасность

- Пароли хешируются через **Argon2** (industry standard)
- JWT_SECRET генерируется автоматически при первом запуске (256 бит случайности)
- Refresh токены хешируются через SHA-256 перед сохранением в БД
- Middleware проверяет валидность JWT на каждом защищенном запросе

### Middleware

**Защита endpoints:**

```rust
// Требует JWT (любой авторизованный пользователь)
.layer(middleware::from_fn(system::auth::middleware::require_auth))

// Требует JWT + is_admin = true
.layer(middleware::from_fn(system::auth::middleware::require_admin))
```

**CurrentUser extractor:**

```rust
use crate::system::auth::extractor::CurrentUser;

async fn my_handler(CurrentUser(claims): CurrentUser) -> String {
    format!("Hello, {}! Admin: {}", claims.username, claims.is_admin)
}
```

### Frontend Guards

**RequireAuth** - требует авторизации:

```rust
use crate::system::auth::guard::RequireAuth;

view! {
    <RequireAuth>
        // Protected content
    </RequireAuth>
}
```

**RequireAdmin** - требует admin прав:

```rust
use crate::system::auth::guard::RequireAdmin;

view! {
    <RequireAdmin>
        // Admin-only content
    </RequireAdmin>
}
```

## 📝 Следующие шаги

### Миграция существующих endpoints на JWT

Когда будете готовы защитить существующие бизнес-endpoints:

```rust
// В main.rs, оберните существующие роуты:
let protected_routes = Router::new()
    .route("/api/connection_1c", get(list_connection_1c_handler))
    .route("/api/organization", get(list_organization_handler))
    // ... все остальные бизнес-роуты
    .layer(middleware::from_fn(system::auth::middleware::require_auth));
```

### Добавление audit logging

В `sys_audit_log` можно записывать все важные действия:

```rust
// Пример в service:
pub async fn create(dto: CreateUserDto, created_by: Option<String>) -> Result<String> {
    let user_id = // ... create user logic

    // Log audit
    audit::log_action(
        created_by.as_deref(),
        "user_created",
        Some("sys_users"),
        Some(&user_id),
        Some(&format!("Created user: {}", dto.username))
    ).await?;

    Ok(user_id)
}
```

### Смена пароля admin

**Через API:**

```bash
curl -X POST http://localhost:3000/api/system/users/{user_id}/change-password \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "YOUR_USER_ID",
    "old_password": "admin",
    "new_password": "new_secure_password"
  }'
```

**Через UI:** Создайте компонент для смены пароля или добавьте в детали пользователя.

## 🐛 Troubleshooting

### "Unauthorized" при запросах

- Проверьте что токен сохранен в localStorage
- Проверьте что токен не истек (24 часа)
- Попробуйте перелогиниться

### Backend не создает admin пользователя

- Проверьте что таблица `sys_users` пустая
- Проверьте логи backend на наличие ошибок миграции
- Удалите `target/db/app.db` и перезапустите backend

### Frontend не показывает login page

- Проверьте что `AuthProvider` обернут вокруг App в `app.rs`
- Проверьте консоль браузера на ошибки
- Очистите localStorage и перезагрузите страницу

## ✨ Готово!

JWT authentication система полностью реализована и готова к использованию.

Все существующие бизнес-роуты пока работают без авторизации для обратной совместимости. Когда будете готовы, просто добавьте middleware к нужным роутам.
