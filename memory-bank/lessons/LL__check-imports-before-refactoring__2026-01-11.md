---
type: lesson-learned
date: 2026-01-11
tags: [refactoring, imports, rust]
severity: medium
---

# Lesson Learned: Проверять все импорты перед перемещением модулей

## Ситуация

При рефакторинге handlers в api/ была обнаружена ошибка компиляции — файл в domain/ импортировал тип из handlers.

## Что пошло не так

1. Переместили handlers/ в api/handlers/
2. Обновили очевидные места (main.rs, mod.rs)
3. `cargo check` выявил скрытую зависимость в `domain/a016_ym_returns/repository.rs`

## Правильный подход

**ПЕРЕД** перемещением модулей всегда искать все ссылки:

```bash
# Поиск всех импортов из перемещаемого модуля
rg "crate::handlers::" crates/backend/src/
rg "use crate::handlers" crates/backend/src/
rg "super::handlers::" crates/backend/src/
```

Составить список файлов и обновить их ВМЕСТЕ с перемещением.

## Чеклист для рефакторинга модулей

1. [ ] Найти все `use crate::<module>::` в проекте
2. [ ] Найти все `use super::<module>::`
3. [ ] Записать список файлов для обновления
4. [ ] Переместить модуль
5. [ ] Обновить все найденные файлы
6. [ ] `cargo check`
7. [ ] Если есть ошибки — значит что-то пропустили

## Инструменты

```bash
# ripgrep для поиска
rg "pattern" path/

# VSCode: Ctrl+Shift+F для глобального поиска

# Rust Analyzer: "Find All References" на модуле
```

## Связано с

- [[RB__api-handlers-refactoring__v1]]
- [[KI__domain-depends-on-handlers__2026-01-11]]
