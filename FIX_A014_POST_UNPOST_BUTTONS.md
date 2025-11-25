# Fix: OZON Transaction Post/Unpost Buttons Not Working

## Problem
На форме "Детали транзакции OZON" не работали кнопки "Провести" (Post) и "Отменить проведение" (Unpost).

## Root Cause
Неправильные URL-адреса API endpoints во frontend-коде.

**Использовались неправильные URLs:**
- `/api/ozon_transactions/{id}/post`
- `/api/ozon_transactions/{id}/unpost`

**Правильные URLs (зарегистрированные на backend):**
- `/api/a014/ozon-transactions/{id}/post`
- `/api/a014/ozon-transactions/{id}/unpost`

## Solution

### File: `crates/frontend/src/domain/a014_ozon_transactions/ui/details/mod.rs`

Исправлены URL-адреса в обоих обработчиках кнопок:

**Unpost Button (line ~187):**
```rust
// БЫЛО:
let url = format!("http://localhost:3000/api/ozon_transactions/{}/unpost", doc_id);

// СТАЛО:
let url = format!("http://localhost:3000/api/a014/ozon-transactions/{}/unpost", doc_id);
```

**Post Button (line ~228):**
```rust
// БЫЛО:
let url = format!("http://localhost:3000/api/ozon_transactions/{}/post", doc_id);

// СТАЛО:
let url = format!("http://localhost:3000/api/a014/ozon-transactions/{}/post", doc_id);
```

## Backend Routes (Reference)

From `crates/backend/src/main.rs`:
```rust
.route(
    "/api/a014/ozon-transactions/:id/post",
    post(handlers::a014_ozon_transactions::post_document),
)
.route(
    "/api/a014/ozon-transactions/:id/unpost",
    post(handlers::a014_ozon_transactions::unpost_document),
)
```

## Backend Handlers (Reference)

From `crates/backend/src/handlers/a014_ozon_transactions.rs`:

### Post Document Handler
```rust
pub async fn post_document(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a014_ozon_transactions::posting::post_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to post document: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true})))
}
```

### Unpost Document Handler
```rust
pub async fn unpost_document(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a014_ozon_transactions::posting::unpost_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to unpost document: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true})))
}
```

## Posting Logic (Reference)

From `crates/backend/src/domain/a014_ozon_transactions/posting.rs`:

### Post Document
- Sets `is_posted = true`
- Sets `posting_ref` and `posting_ref_type` based on delivery schema
- Creates projections in P904 (Sales Data)
- Saves document to database

### Unpost Document
- Sets `is_posted = false`
- Clears `posting_ref` and `posting_ref_type`
- Deletes projections from P904
- Saves document to database

## Testing
✅ Code compiles without errors
✅ No linter errors
✅ URLs match backend routes
✅ Buttons now correctly call backend APIs
✅ Transaction status updates after post/unpost operations

## Impact
- ✅ "Провести" button now works correctly
- ✅ "Отменить проведение" button now works correctly
- ✅ Status badge updates after operations
- ✅ Transaction data reloads automatically
- ✅ P904 projections created/deleted as expected

