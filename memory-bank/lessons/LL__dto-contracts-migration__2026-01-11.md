---
type: lesson-learned
date: 2026-01-11
topic: DTO Migration to Contracts
tags: [dto, contracts, refactoring, imports]
---

# Lesson: DTO Migration to Contracts

## Context

При рефакторинге List DTO из handlers в contracts возникают скрытые зависимости, которые не выявляются простым поиском по имени структуры.

## Problem

После переименования `YmOrderDto` → `YmOrderListDto` компиляция упала с ошибкой:
```
error[E0432]: unresolved import `super::YmOrderDto`
 --> crates\frontend\src\domain\a013_ym_order\ui\list\state.rs:1:5
```

## Root Cause

Файл `state.rs` импортировал структуру через `use super::YmOrderDto`, но поиск `YmOrderDto` в файлах не находил этот импорт, т.к. поиск был ограничен определённой директорией или паттерном.

## Solution

При переименовании структур использовать `replace_all: true` и проверять:
1. Прямые использования структуры
2. Импорты через `use super::`
3. Импорты через `use crate::`
4. Re-exports в `mod.rs`

## Prevention

```bash
# Полный поиск по всему crate при переименовании
rg "OldStructName" crates/frontend/
rg "use.*OldStructName" crates/frontend/
```

## Related

- [[2026-01-11__session-debrief__dto-migration-to-contracts]]
- [[RB__move-dto-to-contracts__v1]]
