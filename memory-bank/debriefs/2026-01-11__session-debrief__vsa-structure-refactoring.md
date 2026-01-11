---
date: 2026-01-11
type: session-debrief
topic: VSA Structure Refactoring
tags:
  - vsa
  - refactoring
  - naming-conventions
  - architecture
---

# Session Debrief: VSA Structure Refactoring

## Summary

Проведён анализ и рефакторинг структуры проекта для соответствия принципам VSA (Vertical Slice Architecture). Основная цель - одинаковые имена каталогов для одного механизма во всех трёх слоях (contracts, backend, frontend).

## Выполненные задачи

### 1. Анализ текущей структуры
- Оценка соответствия VSA: **8/10**
- Domain (a001-a016): ✅ консистентно
- UseCases (u501-u506): ✅ консистентно  
- Projections (p900-p906): ✅ консистентно
- System tasks: ❌ `sys_scheduled_task` vs `tasks`
- System auth/users: ⚠️ разный уровень вложенности в contracts

### 2. Выполненный рефакторинг

| Задача | Описание | Затронуто файлов |
|--------|----------|------------------|
| Task 3 | Создание каталогов auth/ и users/ в contracts | 4 файла |
| Task 2 | Переименование handlers p900, p901 | 3 файла |
| Task 1 | Переименование sys_scheduled_task → tasks | ~20 файлов |
| Task 4 | Переименование таблицы БД sys_scheduled_tasks → sys_tasks | 2 файла + миграция |

## Main Difficulties

### 1. Неполный grep после переименования каталогов
**Проблема:** После переименования каталогов `sys_scheduled_task → tasks`, grep не находил ссылки внутри переименованных папок, но `cargo check` показывал ошибки компиляции.

**Причина:** Файлы внутри переименованных каталогов всё ещё содержали старые импорты вида `contracts::system::sys_scheduled_task::` и `crate::system::sys_scheduled_task::`.

### 2. Множество мест для обновления импортов
**Проблема:** 29 файлов содержали ссылки на `sys_scheduled_task`.

**Решение:** Итеративный подход - `cargo check` → исправление ошибок → повторная проверка.

## Resolutions

1. **Использование `cargo check`** как основного инструмента валидации после массовых переименований
2. **Итеративное исправление** файлов по ошибкам компиляции
3. **Отдельная обработка** UI идентификаторов (не переименовывались, т.к. соответствуют API endpoints)
4. **SQL миграция** для существующих баз данных

## Files Changed

### Contracts
- `src/system/mod.rs` - обновлён mod tasks
- `src/system/auth/mod.rs` - создан из auth.rs
- `src/system/users/mod.rs` - создан из users.rs
- `src/system/tasks/response.rs` - обновлены импорты

### Backend
- `src/system/mod.rs` - обновлён mod tasks
- `src/system/tasks/*.rs` - обновлены импорты (7 файлов)
- `src/system/tasks/managers/*.rs` - обновлены импорты (4 файла)
- `src/handlers/mod.rs` - переименован mod tasks + p900/p901
- `src/handlers/tasks.rs` - обновлены импорты
- `src/handlers/p900_mp_sales_register.rs` - переименован
- `src/handlers/p901_nomenclature_barcodes.rs` - переименован
- `src/routes.rs` - обновлены ссылки на handlers
- `src/main.rs` - обновлён путь к initialization
- `src/shared/data/db.rs` - обновлено имя таблицы

### Frontend
- `src/system/tasks/api.rs` - обновлены импорты
- `src/system/tasks/ui/list/*.rs` - обновлены импорты
- `src/system/tasks/ui/details/mod.rs` - обновлены импорты

### Migrations
- `migrate_sys_tasks.sql` - миграция для переименования таблицы БД

## Decisions Made

1. **UI идентификаторы оставлены без изменений** - `sys_scheduled_tasks`, `sys_scheduled_task_detail_*` соответствуют API path `/api/sys/scheduled_tasks`
2. **Таблица БД переименована** по запросу пользователя: `sys_scheduled_tasks → sys_tasks`
3. **Порядок выполнения** - от простых независимых задач к сложным зависимым

## Related Notes

- [[RB__vsa-module-rename__v1]]
- [[LL__cargo-check-after-refactoring__2026-01-11]]

## TODO / Open Questions

- [ ] Рассмотреть унификацию UI идентификаторов с именами модулей
- [ ] Документировать соглашения об именовании system компонентов
