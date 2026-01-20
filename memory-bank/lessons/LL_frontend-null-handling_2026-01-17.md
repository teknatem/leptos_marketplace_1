---
title: Handling Null Values in Rust Frontend JSON
date: 2026-01-17
tags: [rust, frontend, json, serde, leptos, null-handling]
category: lesson-learned
difficulty: beginner
---

# Lesson: Handling Null Values in Rust Frontend JSON

## Context

When building JSON payloads in Rust WASM frontend code (e.g., Leptos), you may need to send `null` values to REST APIs. Rust doesn't have a `null` keyword, which can cause confusion.

## The Problem

Attempting to use `null` directly in Rust code fails:

```rust
// ❌ This doesn't work in Rust:
let dto = serde_json::json!({
    "id": null,
    "model_name": if model_name.trim().is_empty() { null } else { model_name }
});

// Error: cannot find value `null` in this scope
```

## Why This Happens

- **JavaScript/JSON**: Has a `null` primitive value
- **Rust**: Uses `Option<T>` type system (`Some(value)` or `None`)
- When serializing to JSON, Rust `None` becomes JSON `null`

## Solutions

### Solution 1: Use Option<T>

The Rusty way - let serde handle the serialization:

```rust
// ✅ Idiomatic Rust:
let model_value: Option<&str> = if model_name.trim().is_empty() {
    None
} else {
    Some(model_name)
};

let dto = serde_json::json!({
    "id": None::<String>,           // explicit None with type
    "model_name": model_value        // serde converts None -> null
});
```

### Solution 2: Use serde_json::Value::Null

For explicit null values:

```rust
// ✅ Using serde_json types:
let dto = serde_json::json!({
    "id": serde_json::Value::Null,
    "code": serde_json::Value::Null,
    "model_name": model_value
});
```

### Solution 3: Conditional JSON Building

Build JSON conditionally:

```rust
// ✅ Only include non-null values:
let mut dto = serde_json::json!({
    "description": description,
    "agent_id": agent_id,
});

if !model_name.trim().is_empty() {
    dto["model_name"] = serde_json::json!(model_name);
}
```

## Comparison Table

| Approach | Pros | Cons | Use When |
|----------|------|------|----------|
| `Option<T>` | Idiomatic Rust, type-safe | Requires type annotations | Default choice |
| `Value::Null` | Explicit, clear intent | More verbose | API requires specific null |
| Conditional | Only sends needed fields | More complex code | Optional fields |

## Best Practices

1. **Prefer Option<T>**: Most Rusty approach, works with serde out of the box
2. **Type annotations**: Rust can't always infer `None` type, use `None::<Type>`
3. **Match API contract**: Check if API needs null or can omit field entirely
4. **Test serialization**: Print JSON before sending to verify format

```rust
// Good pattern for API calls:
#[derive(Serialize)]
struct CreateChatDto {
    description: String,
    agent_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_name: Option<String>,  // Omit if None
}
```

## Example: Full Pattern

```rust
async fn create_chat(
    description: &str, 
    agent_id: &str, 
    model_name: &str
) -> Result<(), String> {
    // Convert empty string to None
    let model_value: Option<&str> = if model_name.trim().is_empty() {
        None
    } else {
        Some(model_name)
    };
    
    // Build JSON with proper null handling
    let dto = serde_json::json!({
        "id": None::<String>,
        "code": None::<String>,
        "description": description,
        "comment": None::<String>,
        "agent_id": agent_id,
        "model_name": model_value  // Becomes null or string
    });
    
    let body = wasm_bindgen::JsValue::from_str(&dto.to_string());
    // ... send request
}
```

## Common Mistakes

### Mistake 1: Using JavaScript null keyword
```rust
// ❌ Wrong:
"field": null

// ✅ Right:
"field": None::<String>
// or
"field": serde_json::Value::Null
```

### Mistake 2: Forgetting type annotation
```rust
// ❌ May not compile:
"id": None

// ✅ Clear:
"id": None::<String>
```

### Mistake 3: Not handling empty strings
```rust
// ❌ Sends empty string instead of null:
"model_name": model_name

// ✅ Converts empty to null:
"model_name": if model_name.is_empty() { None::<&str> } else { Some(model_name) }
```

## Related

- serde_json documentation: https://docs.rs/serde_json/
- Rust Option type: https://doc.rust-lang.org/std/option/
- [[RB_llm-chat-enhancement_v1]] - See API call examples

## Key Takeaway

**Remember**: Rust doesn't have `null` - use `Option<T>` and let serde handle JSON serialization. When in doubt, use `serde_json::Value::Null` for explicit nulls.
