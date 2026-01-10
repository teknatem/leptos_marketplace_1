---
title: "Session Debrief - Aggregate QC system + metadata/AI description quality scoring"
date: 2025-12-29
topic: "quality-control, metadata, ai-description, scripts"
---

## Summary

В сессии обсуждали систему **quality control для агрегатов**, чтобы понимать качество “юнита” без чтения кода и ручного вскрытия каждого модуля. За основу взяли:

- Field Metadata System (`metadata.json` → `metadata_gen.rs`) как “паспорт” агрегата.
- Скрипт `scripts/domain_analisys.py` как удобную **табличную визуализацию** по слоям (contracts/backend/frontend/handlers) — расширить его до QC-репорта.
- Идею периодически оценивать **качество `ai.description`** через LLM и сохранять score как индикатор.

## Main difficulties

- Документация UI-стандартов ссылалась на пути вида `styles/3-components/forms.css`, но фактически стили находятся в `crates/frontend/static/themes/core/components.css`, что создало неопределённость “где стандарт сейчас живёт”.
- Не было системного механизма для оценки “качества агрегата” в одном месте (нужны проверяемые сигналы: безопасность, консистентность DTO/metadata, дисциплина UI/CSS, анти-паттерны вроде inline styles и raw fetch).
- Вопрос о LLM-оценке `ai.description` требовал уточнения по месту исполнения (локально vs внешний API) и стратегии хранения результатов.

## Resolutions

- Определили набор QC-сигналов, которые можно автоматически собирать и показывать в UI:
  - безопасность (секреты + `Debug`),
  - консистентность DTO/metadata (required/Option),
  - дисциплина UI/CSS (inline styles, `<style>` в компонентах),
  - анти-паттерны “системной работы с данными” в UI (дубли `api_base`, raw fetch),
  - “сложность” (аномально большие модули по размерам).
- Выяснили фактическое расположение CSS системы: `static/themes/core/components.css` содержит `.form__*` и `.modal-*` стили.
- Зафиксировали текущую картину по метаданным: `metadata.json` сейчас есть только у `a001_connection_1c`.

## Links to created notes

- [[memory-bank/lessons/LL__aggregate-qc-json-reporting__2025-12-29.md|LL: Aggregate QC JSON reporting]]
- [[memory-bank/runbooks/RB__generate-aggregate-qc-json__v1.md|RB: Generate aggregate QC JSON]]
- [[memory-bank/known-issues/KI__ui-standards-css-paths-drift__2025-12-29.md|KI: UI standards doc paths drift]]

## TODO / open questions

- Определить место исполнения LLM-оценки `ai.description`:
  - локально (on-prem),
  - или через внешний API (и как хранить ключи/настройки).
- Определить формат хранения и обновления QC JSON:
  - статический артефакт (например в `dist/static/...`) vs backend endpoint.
- Уточнить целевой “эталон” для `a001`:
  - убрать риск утечки секрета через `Debug`,
  - убрать inline styles и привести к CSS/Modal стандарту,
  - унифицировать работу UI с API (вынести raw fetch и `api_base`).


