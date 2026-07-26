---
id: marketplace-funnel-analysis
title: Анализ воронки маркетплейса
description: Расчёт, проверка и сравнение этапов воронки OZON, Wildberries и Яндекс.Маркета.
intents:
  - marketplace_funnel_analysis
tools:
  - list_data_sources
  - query_data_schema
  - preview_data
resources:
  - references/funnel-model.md
  - references/ozon-mapping.md
  - references/wildberries-mapping.md
  - references/yandex-market-mapping.md
  - references/diagnostic-rules.md
  - references/data-quality-rules.md
tasks:
  - id: calculate-funnel
    title: Рассчитать воронку
    runtime: javascript
    entrypoint: scripts/calculate-funnel.mjs
    export: run
    mode: stable
    input_schema: schemas/calculate-funnel.json
    capabilities: [network:none]
  - id: compare-periods
    title: Сравнить периоды
    runtime: javascript
    entrypoint: scripts/compare-periods.mjs
    export: run
    mode: stable
    input_schema: schemas/compare-periods.json
    capabilities: [network:none]
  - id: compare-segments
    title: Сравнить сегменты
    runtime: javascript
    entrypoint: scripts/compare-segments.mjs
    export: run
    mode: stable
    input_schema: schemas/compare-segments.json
    capabilities: [network:none]
  - id: validate-funnel
    title: Проверить качество данных
    runtime: javascript
    entrypoint: scripts/validate-funnel.mjs
    export: run
    mode: stable
    input_schema: schemas/validate-funnel.json
    capabilities: [network:none]
---

Используй этот навык для сквозного анализа воронки маркетплейса.

1. Определи маркетплейс и прочитай соответствующий mapping в `references/`.
2. Приведи исходные показатели к каноническим полям из `funnel-model.md`.
3. Перед расчётом выполни `validate-funnel`.
4. Для одной выборки используй `calculate-funnel`, для динамики — `compare-periods`,
   для товаров, кампаний или категорий — `compare-segments`.
5. Всегда сообщай о предупреждениях качества и не интерпретируй отсутствующий этап как ноль.
