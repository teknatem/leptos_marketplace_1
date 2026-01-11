---
type: session-debrief
date: 2026-01-11
topic: API Structure Refactoring
tags: [refactoring, architecture, backend, api, handlers]
status: completed
---

# Session Debrief: API Structure Refactoring

## Summary

Провели масштабный рефакторинг структуры backend для создания симметричной архитектуры API:
- Бизнес API: `api/routes.rs` + `api/handlers/`
- Системный API: `system/api/routes.rs` + `system/api/handlers/`

## Контекст

Исходная структура имела несколько проблем:
1. Все handlers (бизнес + системные) в одном каталоге `handlers/`
2. Один огромный файл `routes.rs` (~640 строк)
3. Служебные handlers (`logs`, `form_settings`, `tasks`) смешаны с бизнес-handlers
4. Несимметричная структура между `api` и `system`

## Выполненные действия

### Фаза 1: Создание api/ модуля
- Создан `api/handlers/` с 25 бизнес-handlers
- Создан `api/routes.rs` с функцией `configure_business_routes()`
- Создан `api/mod.rs`

### Фаза 2: Создание system/api/ модуля
- Перемещены `auth.rs`, `users.rs` из `system/handlers/`
- Перемещены `logs.rs`, `form_settings.rs`, `tasks.rs` из `handlers/`
- Создан `system/api/routes.rs` с функцией `configure_system_routes()`

### Фаза 3: Обновление main.rs
- Заменён `routes::configure_routes()` на `Router::merge()` двух функций

### Фаза 4-5: Очистка
- Удалены старые `handlers/` и `routes.rs`

### Фаза 6: Проверка
- `cargo check --bin backend` — успешно

## Сложности

### 1. Скрытая зависимость в domain
**Проблема**: `domain/a016_ym_returns/repository.rs` импортировал DTO из handlers:
```rust
use crate::handlers::a016_ym_returns::YmReturnListItemDto;
```

**Решение**: Обновлён путь на `crate::api::handlers::a016_ym_returns::YmReturnListItemDto`

**Урок**: При перемещении модулей всегда искать все ссылки через `grep`

### 2. Архитектурный вопрос
Domain-слой зависит от DTO в handlers — это нарушение слоёв. Следует рассмотреть перенос DTO в contracts или domain.

## Итоговая структура

```
crates/backend/src/
├── api/
│   ├── mod.rs
│   ├── routes.rs                    # configure_business_routes()
│   └── handlers/                    # 25 файлов
│
├── system/
│   ├── api/
│   │   ├── mod.rs
│   │   ├── routes.rs                # configure_system_routes()
│   │   └── handlers/                # 5 файлов
│   ├── auth/
│   ├── tasks/
│   └── ...
│
└── main.rs
```

## Связанные заметки

- [[RB__api-handlers-refactoring__v1]] — Runbook для подобных рефакторингов
- [[KI__domain-depends-on-handlers__2026-01-11]] — Известная проблема зависимости слоёв
- [[ADR__0004__api-structure-separation]] — Решение о разделении API

## TODO / Открытые вопросы

- [ ] Рассмотреть перенос `YmReturnListItemDto` из handlers в contracts
- [ ] Проверить другие domain модули на аналогичные зависимости от handlers
- [ ] Добавить в CI проверку на импорты из handlers в domain
