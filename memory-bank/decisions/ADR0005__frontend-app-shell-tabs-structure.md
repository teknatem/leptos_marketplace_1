---
date: 2026-01-11
type: decision-record
status: accepted
tags: [frontend, architecture, module-structure]
---

# ADR-0005: Frontend App Shell and Tabs Module Structure

## Context

Frontend имел запутанную структуру модулей:

- `routes/routes.rs` содержал auth gate, MainLayout и TabPage (не роуты!)
- Дублирование TabPage в `layout/center/tabs/tabs.rs` (неиспользуемый код)
- Непонятно где искать корневые компоненты приложения

## Decision

### Структура модулей

```text
crates/frontend/src/
├── app.rs              # Root: providers + AppShell + ModalHost
├── app_shell.rs        # AppShell (auth gate) + MainLayout
├── layout/
│   └── tabs/
│       ├── page.rs     # TabPage wrapper
│       └── registry.rs # match key → View (единственный источник)
```

### Принципы

1. **`app_shell.rs` как файл, не каталог** — один модуль = один файл
2. **Tabs в layout/** — табы это часть UI-каркаса, не отдельная "навигация"
3. **Единый registry** — все tab keys в одном месте

## Alternatives Considered

### 1. Три каталога (app_root/, navigation/, layout/)

- ❌ Избыточно
- ❌ navigation/tabs/ логически принадлежит layout

### 2. Всё в layout/

- ❌ AppShell (auth gate) — не часть layout, это уровень выше

### 3. shell/ вместо app_shell

- ❌ Конфликт с существующим `layout::Shell`

## Consequences

### Positive

- Чёткое разделение: `app.rs` → `app_shell.rs` → `layout/`
- Единый источник правды для tab registry
- Удалено ~680 строк дублирующегося кода

### Negative

- `layout/center/tabs/tab.rs` остался на старом месте (minor inconsistency)

## Related

- Session debrief: [[2026-01-11__session-debrief__frontend-navigation-tabs-refactor]]
