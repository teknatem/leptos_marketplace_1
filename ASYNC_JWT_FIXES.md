# Исправление ошибок JWT авторизации (tokio runtime panic)

## Проблема

После внедрения JWT авторизации backend падал с ошибкой:
```
thread 'tokio-runtime-worker' panicked at crates\backend\src\system\auth\jwt.rs:85:26:
Cannot start a runtime from within a runtime. This happens because a function (like `block_on`) 
attempted to block the current thread while the thread is being used to drive asynchronous tasks.
```

## Причины

1. **Синхронные функции с `block_on()` внутри async runtime**
   - Функции `get_jwt_secret()`, `generate_access_token()`, `validate_token()` были синхронными
   - Внутри использовали `tokio::runtime::Handle::current().block_on()` для работы с БД
   - Вызывались из уже асинхронного контекста (Axum handlers)
   - Tokio запрещает вложенные `block_on` вызовы

2. **Неправильный путь к SQL миграции**
   - Backend запускается из `crates/backend/`, но файл искался в текущей директории
   - Исправлено: добавлен fallback на `../../migrate_auth_system.sql`

3. **SQL миграция пропускала CREATE TABLE из-за комментариев**
   - SQL statements разделялись по `;`
   - Statements, начинающиеся с `--`, пропускались целиком
   - Проблема: первый statement был `-- System settings table\nCREATE TABLE...`
   - Решение: фильтровать комментарии построчно, а не весь statement

## Исправления

### 1. Сделали JWT функции асинхронными

**`crates/backend/src/system/auth/jwt.rs`**:
```rust
// Было:
pub fn get_jwt_secret() -> Result<String> {
    match get_jwt_secret_from_db() { ... }
}

fn get_jwt_secret_from_db() -> Result<Option<String>> {
    let runtime = tokio::runtime::Handle::current();
    let result = runtime.block_on(async { ... })?;
    ...
}

// Стало:
pub async fn get_jwt_secret() -> Result<String> {
    match get_jwt_secret_from_db().await { ... }
}

async fn get_jwt_secret_from_db() -> Result<Option<String>> {
    let conn = get_connection();
    let result = conn.query_one(...).await?;
    ...
}
```

Аналогично для:
- `save_jwt_secret_to_db()` → `async fn`
- `generate_access_token()` → `async fn`
- `validate_token()` → `async fn`

### 2. Обновили вызовы с `.await`

**`crates/backend/src/system/handlers/auth.rs`**:
```rust
// Было:
let access_token = jwt::generate_access_token(&user.id, &user.username, user.is_admin)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

// Стало:
let access_token = jwt::generate_access_token(&user.id, &user.username, user.is_admin)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
```

**`crates/backend/src/system/auth/middleware.rs`**:
```rust
// Было:
let claims = super::jwt::validate_token(token).map_err(|_| StatusCode::UNAUTHORIZED)?;

// Стало:
let claims = super::jwt::validate_token(token).await.map_err(|_| StatusCode::UNAUTHORIZED)?;
```

### 3. Исправили путь к SQL миграции

**`crates/backend/src/system/initialization.rs`**:
```rust
// Было:
let migration_sql = std::fs::read_to_string("migrate_auth_system.sql")
    .context("Failed to read migrate_auth_system.sql")?;

// Стало:
let migration_sql = std::fs::read_to_string("migrate_auth_system.sql")
    .or_else(|_| std::fs::read_to_string("../../migrate_auth_system.sql"))
    .context("Failed to read migrate_auth_system.sql")?;
```

### 4. Исправили парсинг SQL миграции

**`crates/backend/src/system/initialization.rs`**:
```rust
// Было:
for statement in migration_sql.split(';') {
    let trimmed = statement.trim();
    if !trimmed.is_empty() && !trimmed.starts_with("--") {
        // Пропускал statements, начинающиеся с комментария
    }
}

// Стало:
for (idx, statement) in migration_sql.split(';').enumerate() {
    // Убираем строки-комментарии, но сохраняем SQL код
    let cleaned: String = statement
        .lines()
        .filter(|line| {
            let trimmed_line = line.trim();
            !trimmed_line.is_empty() && !trimmed_line.starts_with("--")
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    let trimmed = cleaned.trim();
    if !trimmed.is_empty() {
        tracing::info!("Executing migration statement #{}...", idx);
        conn.execute(Statement::from_string(...)).await?;
    }
}
```

## Результат

✅ Backend успешно запускается  
✅ SQL миграция применяется корректно  
✅ Создается admin user (admin/admin)  
✅ Сервер слушает на порту 3000  
✅ Frontend компилируется и работает на порту 8080  

## Проверка

1. Откройте http://localhost:8080/
2. Должна отобразиться страница логина
3. Войдите с учетными данными:
   - Username: `admin`
   - Password: `admin`

## Важно

⚠️ **НЕМЕДЛЕННО смените пароль admin после первого входа!**

## Технические детали

**Файлы изменены:**
- `crates/backend/src/system/auth/jwt.rs` - функции JWT → async
- `crates/backend/src/system/auth/middleware.rs` - добавлены `.await`
- `crates/backend/src/system/handlers/auth.rs` - добавлены `.await`
- `crates/backend/src/system/initialization.rs` - исправлены путь к SQL и парсинг миграции

**Паттерн решения:**
- Убрали все `tokio::runtime::Handle::current().block_on()` из async-контекстов
- Сделали функции, работающие с БД, асинхронными (`async fn`)
- Добавили `.await` во всех местах вызова этих функций
- Middleware в Axum уже были async, поэтому изменения минимальны

**Урок:**
В Tokio нельзя использовать `block_on()` изнутри async runtime. Если функция вызывается 
из async контекста, она сама должна быть async и использовать `.await` вместо `block_on()`.

