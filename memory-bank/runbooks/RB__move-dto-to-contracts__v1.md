---
type: runbook
version: 1
date: 2026-01-11
topic: Move List DTO to Contracts
tags: [dto, contracts, refactoring, procedure]
---

# Runbook: Move List DTO to Contracts

## Overview

Процедура перемещения List DTO из backend handlers в contracts crate для обеспечения единого источника правды и устранения дублирования.

## Prerequisites

- [ ] Определить целевой агрегат (например, a007, a013, a016)
- [ ] Проверить, используется ли DTO на frontend
- [ ] Проверить, используется ли DTO в domain layer

## Step 1: Добавить DTO в contracts

Файл: `crates/contracts/src/domain/{aggregate}/aggregate.rs`

```rust
// =============================================================================
// List DTO for frontend (flat structure for list views)
// =============================================================================

/// DTO для списка (минимальные поля для list view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {Aggregate}ListItemDto {
    pub id: String,
    // ... поля из существующего DTO
    // Добавить #[serde(default)] для опциональных полей на frontend
}
```

## Step 2: Обновить backend handlers

Файл: `crates/backend/src/api/handlers/{aggregate}.rs`

```rust
// Было:
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {Aggregate}ListItemDto { ... }

// Стало:
use contracts::domain::{aggregate}::aggregate::{Aggregate}ListItemDto;
// Удалить локальное определение структуры
```

## Step 3: Обновить backend domain (если есть зависимость)

Файл: `crates/backend/src/domain/{aggregate}/repository.rs`

```rust
// Было:
use crate::api::handlers::{aggregate}::{Aggregate}ListItemDto;

// Стало:
use contracts::domain::{aggregate}::aggregate::{Aggregate}ListItemDto;
```

## Step 4: Обновить frontend (если есть дублирование)

Файл: `crates/frontend/src/domain/{aggregate}/ui/list/mod.rs`

```rust
// Добавить импорт:
use contracts::domain::{aggregate}::aggregate::{Aggregate}ListItemDto;

// Удалить локальное определение структуры
// Обновить все использования на новое имя (если отличается)
```

**ВАЖНО**: Проверить также:
- `state.rs` - может импортировать через `use super::`
- `mod_new.rs` - альтернативные версии компонентов

## Step 5: Проверка компиляции

```powershell
cargo check -p contracts
cargo check --bin backend
cargo check -p frontend
```

## Verification Checklist

- [ ] Нет импортов `crate::api::handlers::` в domain
- [ ] Нет дублирования DTO между backend и frontend
- [ ] DTO находится в `contracts/domain/*/aggregate.rs`
- [ ] Компиляция всех crates успешна

## Known Exceptions

| Агрегат | Причина пропуска |
|---------|------------------|
| a012 | Frontend использует `serde_json::Value` |
| a015 | Backend использует `#[serde(flatten)]` |

## Related

- [[LL__dto-contracts-migration__2026-01-11]]
- [[KI__domain-depends-on-handlers__2026-01-11]]
