---
id: marketplace-funnel-analysis
title: Анализ воронки маркетплейса
description: Расчёт, проверка и сравнение этапов воронки OZON, Wildberries и Яндекс.Маркета.
intents:
  - marketplace_funnel_analysis
tools:
  - list_data_sources
  - find_data_sources
  - query_data_schema
  - preview_data
  - execute_query
resources:
  - references/funnel-model.md
  - references/ozon-mapping.md
  - references/wildberries-mapping.md
  - references/wildberries-diagnostics.md
  - references/source-data-and-rebuild.md
  - references/nomenclature-mapping.md
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
  - id: wb-funnel
    title: Воронка Wildberries (реальные поля p916)
    runtime: javascript
    entrypoint: scripts/wb-funnel.mjs
    export: run
    mode: stable
    input_schema: schemas/wb-funnel.json
    capabilities: [network:none]
---

Этот навык — **единый авторитет по воронке продаж маркетплейса** в системе: закрывает всё
от источников исходных данных, через преобразование, до отображения, диагностики отклонений и
расследования отдельных значений. Другие навыки (marketing-analytics и т.д.) по вопросам
воронки ссылаются сюда.

## Wildberries (основной сценарий)

1. Прочитай `references/wildberries-mapping.md` — реальный пайплайн: источники
   `a036`/`a026`/`a015`/`a012` → проекция `p916_mp_sales_funnel_turnovers` → дашборд `d406`.
   `a040` в воронку **не входит**; органические показы `N/A`.
   После чтения mapping используй готовые SQL-шаблоны напрямую. Не вызывай
   `get_architecture_overview` и не перечитывай схему p916, если документированный запрос не
   завершился ошибкой схемы. SQL для `execute_query` передавай без комментариев `--`.
2. Данные бери из `p916` через `execute_query` (готовые SQL-шаблоны в mapping). Выбирай ось:
   `cohort_date` (когорта = дата заказа) или `event_date` (поток = дата транзакции). Показы
   nullable → `COALESCE(...)`, `N/A ≠ 0`.
3. Для расчёта используй задачу `wb-funnel` (канальный сплит paid/free, отмены, возвраты,
   конверсии) или generic `calculate-funnel` для линейной воронки. Перед расчётом — `validate-funnel`.
   Никогда не дели платные показы `show_paid_count` на общие `open_count`/`cart_count`/
   `order_count`/`buyout_count`: это несовместимые каналы. Если органические показы недоступны,
   общие конверсии от показа — `N/A`. Платную воронку считай только по последовательности
   `show_paid_count → paid_open_count → paid_cart_count → paid_order_count → paid_buyout_count`;
   отсутствующее звено оставляй `N/A`, не заменяй общей метрикой.
4. Для динамики — `compare-periods`; для товаров/кампаний/категорий — `compare-segments`.
5. При отклонениях — `references/wildberries-diagnostics.md` (симптом → причина → SQL).
6. Если `p916` пуст или неполон, **до вывода об отсутствии данных** прочитай
   `references/source-data-and-rebuild.md`: проверь сохранённые документы
   `a036`/`a026`/`a015`/`a012`, затем отдели отсутствие первички от необходимости пересбора
   через `u508_repost_documents`.
7. Для разрезов по товарам, категориям и характеристикам используй основной каталог 1С
   `a004_nomenclature` и маппинг через `a007_marketplace_product` по правилам
   `references/nomenclature-mapping.md`.
8. Сверяй числа с дашбордом `d406` (`/api/dashboards/wb-sales-funnel`) на том же периоде/оси/канале.

## Общие правила

- Различай `funnel_order_count` (маркетинговый счётчик a036) и `order_count` (фактические
  заказы a015) — это разные числа. Для «сколько заказано» бери `order_count`, а не
  `funnel_order_count`.
- **Лаг выкупа в текущем/незавершённом месяце.** `order→buyout` по свежим когортам занижен:
  заказы последних 3–7 дней ещё в доставке и физически не выкуплены (когорты конца периода
  дают ~0% выкупа). Для текущего месяца всегда предупреждай об этом и оценивай по зрелым
  когортам либо сравнивай аналогичный отрезок прошлого месяца — не подавай техническую
  просадку `order→buyout` как падение спроса. Проверить лаг: разбей выкупы по `cohort_date`
  в конце периода (SQL-шаблон 2 с `GROUP BY cohort_date`).
- Отсутствующий этап (`NULL`/`N/A`) не интерпретируй как ноль; всегда сообщай о предупреждениях
  качества и указывай использованные ось и канал.
- Не называй проекцию пустой, пока не проверены документы-источники. Если первичка есть, а
  движений `p916` нет, это кандидат на перепроведение/пересбор, а не отсутствие бизнес-событий.
- Категория маркетплейса не заменяет классификацию 1С. Для аналитических разрезов приоритетны
  шесть измерений `a004`: категория, линия, модель, формат, мойка и размер.
- Для OZON/YM пока действуй по generic-модели (`funnel-model.md` + соответствующий mapping);
  заземление под конкретный пайплайн — по мере готовности.
