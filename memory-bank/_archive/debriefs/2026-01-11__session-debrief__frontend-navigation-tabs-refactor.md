---
date: 2026-01-11
type: session-debrief
tags: [refactoring, frontend, leptos, module-structure]
related:
  - "[[ADR0005__frontend-app-shell-tabs-structure]]"
  - "[[RB__frontend-module-refactor__v1]]"
---

# Session Debrief: Frontend Navigation & Tabs Refactor

## Summary

Провели рефакторинг структуры модулей frontend:
- Убрали дублирование кода TabPage (~680 строк)
- Переименовали `routes/` → `app_shell.rs` + `layout/tabs/`
- Удалили устаревший `logs_api.rs`

## Исходная проблема

1. **Неправильное именование**: `routes/routes.rs` содержал НЕ роуты, а auth gate + MainLayout + TabPage
2. **Дублирование кода**: Почти идентичный `TabPage` и `match key → View` в двух местах:
   - `routes/routes.rs` (~336 строк)
   - `layout/center/tabs/tabs.rs` (~340 строк)
3. **Мёртвый код**: `layout/center/tabs/tabs.rs` вообще не использовался

## Сложности / неопределённости

| Проблема | Что вызвало неопределённость |
|----------|------------------------------|
| Структура каталогов | План предлагал 3 каталога (app_root, navigation, layout), что избыточно |
| Именование app_root | Название `app_root` не очевидно, обсудили альтернативы |
| Разные tab keys | Два файла содержали РАЗНЫЕ tab keys, нужно было объединить |

## Решения

1. **2 каталога вместо 3**: `layout/tabs/` вместо отдельного `navigation/`
2. **Файл вместо каталога**: `app_shell.rs` вместо `app_root/mod.rs + main_layout.rs`
3. **Объединённый registry**: `layout/tabs/registry.rs` содержит ВСЕ tab keys из обоих файлов

## Выполненные изменения

| Действие | Файл |
|----------|------|
| ✅ Создан | `app_shell.rs` - AppShell (auth gate) + MainLayout |
| ✅ Создан | `layout/tabs/mod.rs` |
| ✅ Создан | `layout/tabs/page.rs` - TabPage wrapper |
| ✅ Создан | `layout/tabs/registry.rs` - единый реестр (~350 строк) |
| ✅ Обновлён | `app.rs` - импорт AppShell |
| ✅ Обновлён | `lib.rs` - убран routes, добавлен app_shell |
| ❌ Удалён | `routes/routes.rs` |
| ❌ Удалён | `routes/mod.rs` |
| ❌ Удалён | `layout/center/tabs/tabs.rs` |
| ❌ Удалён | `layout/right/panel/logs_api.rs` |
| ✅ Упрощён | `layout/right/panel/right_panel.rs` → placeholder |

## Итоговая структура

```text
crates/frontend/src/
├── app.rs              # Providers + AppShell + ModalHost
├── app_shell.rs        # AppShell (auth gate) + MainLayout
├── lib.rs              # Module declarations
│
├── layout/
│   ├── tabs/           # NEW: единственный источник правды
│   │   ├── mod.rs
│   │   ├── page.rs     # TabPage wrapper
│   │   └── registry.rs # match key → View
│   ├── center/
│   │   └── tabs/
│   │       └── tab.rs  # Tab header component (сохранён)
│   └── ...
```

## TODO / Open Questions

- [ ] Smoke test: логин, открытие табов, ?active= восстановление
- [ ] Возможно перенести `layout/center/tabs/tab.rs` в `layout/tabs/tab_header.rs` для консистентности

## Ключевые выводы

1. **Проверять использование кода** перед рефакторингом (grep для импортов)
2. **Файл > каталог** если модуль один
3. **Меньше вложенности** = проще навигация
