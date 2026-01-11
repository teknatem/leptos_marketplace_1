---
type: known-issue
date: 2026-01-11
topic: Frontend DTO Patterns
tags: [frontend, dto, patterns, architecture]
severity: info
status: documented
---

# Known Issue: Frontend DTO Patterns

## Description

В проекте существует несколько паттернов работы с DTO на frontend, что влияет на возможность унификации через contracts.

## Patterns Found

### Pattern 1: Типизированный DTO (стандарт)

```rust
// Frontend определяет свою структуру
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceProductListItemDto { ... }

// Использование:
let items: Vec<MarketplaceProductListItemDto> = response.json().await?;
```

**Агрегаты**: a007, a013, a016

**Решение**: Переместить DTO в contracts ✅

### Pattern 2: Dynamic JSON

```rust
// Frontend использует serde_json::Value
pub struct PaginatedResponse {
    pub items: Vec<serde_json::Value>,
    ...
}

// Доступ к полям через .get()
let id = item.get("id").and_then(|v| v.as_str());
```

**Агрегаты**: a012

**Решение**: Оставить как есть или типизировать при необходимости

### Pattern 3: Flatten with Manual Mapping

```rust
// Backend использует flatten
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbOrdersListItemDto {
    #[serde(flatten)]
    pub order: WbOrders,  // Весь агрегат
    pub organization_name: Option<String>,
}

// Frontend делает ручной маппинг из JSON
let dto = WbOrdersDto {
    id: json.get("base").and_then(|b| b.get("id"))...,
    ...
};
```

**Агрегаты**: a015

**Решение**: Требует рефакторинга backend для возврата flat DTO

## Detection

При аудите нового агрегата проверить:

```bash
# Проверить, есть ли типизированный DTO на frontend
rg "struct.*ListItem|struct.*ListDto" crates/frontend/src/domain/{aggregate}/

# Проверить, используется ли serde_json::Value
rg "serde_json::Value" crates/frontend/src/domain/{aggregate}/

# Проверить flatten в backend
rg "#\[serde\(flatten\)\]" crates/backend/src/api/handlers/{aggregate}.rs
```

## Resolution

| Pattern            | Action                          |
| ------------------ | ------------------------------- |
| Типизированный DTO | Переместить в contracts         |
| Dynamic JSON       | Оценить необходимость типизации |
| Flatten            | Рефакторинг backend API         |

## Related

- [[RB__move-dto-to-contracts__v1]]
- [[2026-01-11__session-debrief__dto-migration-to-contracts]]
