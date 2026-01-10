---
title: "Known Issue - UI standards doc paths drift vs actual CSS location"
date: 2025-12-29
severity: "minor"
area: "frontend-ui, documentation"
---

## Problem

Документация UI стандартов (Modal & Forms) указывала на пути вида `crates/frontend/styles/3-components/forms.css` и `modals.css`, но в текущем репозитории такие файлы отсутствуют.

Фактическая CSS система находится в `crates/frontend/static/themes/core/components.css` и содержит:

- `.form__*` (form groups/labels/inputs)
- `.modal-*` (modal overlay/header/body/actions)

## Detection

- Попытка открыть `crates/frontend/styles/3-components/forms.css` / `modals.css` приводит к “file not found”.
- Поиск по CSS показывает определения `.modal-overlay`, `.modal-header`, `.form__group` в `static/themes/core/components.css`.

## Impact

- Новый код может начать писать inline styles или “изобретать” новые классы, потому что “непонятно где стандарт”.
- QC/линтеры/ревью сложнее применять без единого источника истины по CSS.

## Fix

- Обновить документацию UI стандартов, чтобы она ссылалась на актуальные пути в `static/themes/core/`.
- Ввести QC-сигнал “INLINE_CSS” и “USES_CORE_FORM_CLASSES”, чтобы быстро видеть соблюдение стандарта.
