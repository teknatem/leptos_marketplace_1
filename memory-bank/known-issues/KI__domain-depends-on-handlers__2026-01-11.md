---
type: known-issue
date: 2026-01-11
severity: medium
status: resolved
resolved_date: 2026-01-11
tags: [architecture, layering, domain, handlers]
---

# Known Issue: Domain Layer Depends on Handlers DTOs

## Описание

Domain-слой (`crates/backend/src/domain/`) импортировал типы из handlers-слоя, что нарушало принцип слоёной архитектуры.

## Обнаружение

При рефакторинге handlers в api/ обнаружена ошибка компиляции:

```
error[E0433]: failed to resolve: unresolved import
  --> crates\backend\src\domain\a016_ym_returns\repository.rs:14:12
   |
14 | use crate::handlers::a016_ym_returns::YmReturnListItemDto;
```

## Проблема

```
┌─────────────────┐
│    handlers     │  ← API слой (DTOs)
└────────┬────────┘
         │ ❌ domain импортирует из handlers
         ▼
┌─────────────────┐
│     domain      │  ← Domain слой (должен быть независимым)
└─────────────────┘
```

## ✅ Решение (выполнено 2026-01-11)

DTO перемещены в contracts crate:

| Агрегат | DTO                           | Новый путь                                               |
| ------- | ----------------------------- | -------------------------------------------------------- |
| a016    | YmReturnListItemDto           | `contracts::domain::a016_ym_returns::aggregate`          |
| a007    | MarketplaceProductListItemDto | `contracts::domain::a007_marketplace_product::aggregate` |
| a013    | YmOrderListDto                | `contracts::domain::a013_ym_order::aggregate`            |

## Проверка

```bash
# Должен вернуть пустой результат
rg "use crate::(api::)?handlers::" crates/backend/src/domain/
```

**Результат**: ✅ Нет импортов из handlers в domain

## Архитектура после исправления

```
┌─────────────────┐
│   contracts     │  ← Shared DTOs
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
┌───────┐ ┌───────┐
│handler│ │domain │  ← Оба импортируют из contracts
└───────┘ └───────┘
```

## Related

- [[2026-01-11__session-debrief__dto-migration-to-contracts]]
- [[RB__move-dto-to-contracts__v1]]
- [[LL__dto-contracts-migration__2026-01-11]]
