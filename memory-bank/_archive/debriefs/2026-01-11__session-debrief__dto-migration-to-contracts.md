---
type: session-debrief
date: 2026-01-11
topic: DTO Migration to Contracts
tags: [refactoring, dto, contracts, architecture, layer-separation]
status: completed
---

# Session Debrief: DTO Migration to Contracts

## Summary

Выполнена миграция List DTO из backend handlers в contracts crate для устранения нарушений слоёв архитектуры и дублирования кода между backend и frontend.

## Выполненные задачи

| Фаза | Агрегат | DTO | Результат |
|------|---------|-----|-----------|
| 1 | a016_ym_returns | YmReturnListItemDto | ✅ Перемещён в contracts |
| 2.1 | a007_marketplace_product | MarketplaceProductListItemDto | ✅ Перемещён в contracts |
| 2.2 | a012_wb_sales | WbSalesListItemDto | ⏭️ Пропущен (frontend использует JSON) |
| 2.3 | a013_ym_order | YmOrderListDto | ✅ Перемещён в contracts |
| 2.4 | a015_wb_orders | WbOrdersListItemDto | ⏭️ Пропущен (flatten pattern) |
| 3 | - | - | ✅ Компиляция успешна |

## Main Difficulties

### 1. Скрытые зависимости в state.rs
- **Проблема**: После переименования YmOrderDto → YmOrderListDto, файл `state.rs` продолжал импортировать старое имя через `use super::YmOrderDto`
- **Причина**: Поиск по коду не выявил этот файл, т.к. искал по имени структуры, а не по путям импорта
- **Решение**: Исправил импорт в state.rs после ошибки компиляции

### 2. Различные паттерны DTO на frontend
- **a012**: Frontend использует `serde_json::Value` вместо типизированного DTO
- **a015**: Backend использует `#[serde(flatten)]` на агрегате, frontend делает ручную трансформацию
- **Решение**: Пропустить эти случаи, т.к. они требуют отдельного рефакторинга

### 3. Разные имена одной структуры
- Backend: `YmOrderListDto`
- Frontend: `YmOrderDto`
- **Решение**: Стандартизировано на `YmOrderListDto` (имя из backend)

## Resolutions

1. **Паттерн размещения DTO**: List DTO размещаются в `contracts/domain/{aggregate}/aggregate.rs` в секции `// List DTO for frontend`
2. **serde(default)**: Для frontend-friendly DTO добавляются `#[serde(default)]` атрибуты на опциональные поля
3. **Проверка зависимостей**: При переименовании структур нужно проверять не только прямые использования, но и re-exports через `use super::`

## Created Notes

- [[LL__dto-contracts-migration__2026-01-11]] - Урок о миграции DTO
- [[RB__move-dto-to-contracts__v1]] - Runbook для миграции DTO
- [[KI__frontend-dto-patterns__2026-01-11]] - Известные паттерны DTO на frontend

## TODO / Open Questions

- [ ] Рефакторинг a015 - требует отдельного решения для flatten pattern
- [ ] Рефакторинг a012 - решить, нужен ли типизированный DTO на frontend
- [ ] Обновить KI__domain-depends-on-handlers - отметить как resolved для исправленных агрегатов
