---
date: 2026-01-11
type: lesson-learned
tags: [rust, modules, architecture]
---

# Lesson: File vs Directory for Rust Modules

## Context

При рефакторинге frontend обсуждали: создавать `app_root/mod.rs` или `app_shell.rs`?

## Lesson

**Используй файл, если модуль один. Используй каталог, если модулей несколько.**

### Файл лучше когда:

- Один логический модуль
- Нет sub-modules
- Код < 500 строк

```text
✅ app_shell.rs           # Один модуль с auth gate + MainLayout
```

### Каталог лучше когда:

- Несколько связанных модулей
- Есть sub-modules
- Код > 500 строк или логически разделим

```text
✅ layout/tabs/
     ├── mod.rs
     ├── page.rs
     └── registry.rs
```

## Anti-pattern

```text
❌ app_root/
     └── mod.rs          # Только один файл — зачем каталог?
```

## Application

В этом проекте:

- `app_shell.rs` — файл (один модуль)
- `layout/tabs/` — каталог (page + registry)
