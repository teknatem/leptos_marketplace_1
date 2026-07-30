---
date: 2026-01-11
type: adr
number: "0003"
title: VSA System Module Naming Convention
status: accepted
tags:
  - architecture
  - vsa
  - naming
---

# ADR-0003: Соглашение об именовании system модулей в VSA

## Status

Accepted

## Context

В проекте используется VSA (Vertical Slice Architecture) где код одного механизма должен находиться в каталогах с одинаковым названием во всех слоях (contracts, backend, frontend).

Обнаружено несоответствие в именовании system модулей:

- `contracts/src/system/sys_scheduled_task/`
- `backend/src/system/sys_scheduled_task/`
- `frontend/src/system/tasks/` (отличается!)

Также `sys_` префикс избыточен внутри каталога `system/`.

## Decision

1. **Убрать `sys_` префикс** из имён модулей внутри `system/`
2. **Использовать короткие имена** для system модулей: `auth`, `users`, `tasks`
3. **Таблицы БД** могут сохранять `sys_` префикс для явного разделения от domain таблиц, НО было решено переименовать в `sys_tasks` для консистентности
4. **UI идентификаторы** остаются независимыми от путей к модулям

### Итоговая структура system

```
contracts/src/system/     backend/src/system/       frontend/src/system/
├── auth/                 ├── auth/                 ├── auth/
├── users/                ├── users/                ├── users/
├── tasks/                ├── tasks/                ├── tasks/
└── mod.rs                └── ...                   └── ...
```

## Alternatives Considered

### A: Оставить sys_scheduled_task

- Pros: Нет изменений, нет риска
- Cons: Несоответствие между слоями, избыточный префикс

### B: Переименовать frontend tasks → sys_scheduled_task

- Pros: Минимум изменений
- Cons: Сохраняется избыточный префикс

### C: Переименовать всё в tasks (выбрано)

- Pros: Консистентность, чистые имена
- Cons: Много файлов для изменения

## Consequences

### Positive

- Консистентность именования между слоями
- Более чистые и короткие пути импортов
- Соответствие принципам VSA

### Negative

- Требуется SQL миграция для существующих БД
- Временная несовместимость с предыдущими версиями

### Neutral

- UI идентификаторы остаются как `sys_scheduled_tasks` для соответствия API путям
