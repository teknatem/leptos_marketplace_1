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
  - list_quality_checks
  - run_quality_check
  - get_latest_quality_check
  - prepare_funnel_repair
  - execute_funnel_repair
  - get_funnel_repair_status
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
  - references/repair-workflow.md
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
2. До чтения p916 и бизнес-интерпретации WB запусти
   `run_quality_check(check_id="wb_funnel_projection_coverage")` для заказов/продаж и
   `run_quality_check(check_id="wb_marketing_projection_coverage")` для marketing-stage. Эти централизованные
   проверка — авторитет по полноте `a036`/`a026`/`a015`/`a012 → p916`; не дублируй её SQL.
   Если есть `projection_missing`, `projection_extra` или `source_missing`, не считай
   конверсии затронутых периодов достоверными. Для способа исправления прочитай
   `references/source-data-and-rebuild.md` и рекомендуй `u508_repost_documents` только когда
   первичка есть, а проекция неполна.
3. Данные бери из `p916` через `execute_query` (готовые SQL-шаблоны в mapping). Выбирай ось:
   `cohort_date` (когорта = дата заказа) или `event_date` (поток = дата транзакции). Показы
   nullable → `COALESCE(...)`, `N/A ≠ 0`.
4. Для расчёта используй задачу `wb-funnel` (канальный сплит paid/free, отмены, возвраты,
   конверсии) или generic `calculate-funnel` для линейной воронки. Перед расчётом — `validate-funnel`.
   Никогда не дели платные показы `show_paid_count` на общие `open_count`/`cart_count`/
   `order_count`/`buyout_count`: это несовместимые каналы. Если органические показы недоступны,
   общие конверсии от показа — `N/A`. Платную воронку считай только по последовательности
   `show_paid_count → paid_open_count → paid_cart_count → paid_order_count → paid_buyout_count`;
   отсутствующее звено оставляй `N/A`, не заменяй общей метрикой.
5. Для динамики — `compare-periods`; для товаров/кампаний/категорий — `compare-segments`.
6. При отклонениях — `references/wildberries-diagnostics.md` (симптом → причина → SQL).
7. Для разрезов по товарам, категориям и характеристикам используй основной каталог 1С
   `a004_nomenclature` и маппинг через `a007_marketplace_product` по правилам
   `references/nomenclature-mapping.md`.
8. Сверяй числа с дашбордом `d406` (`/api/dashboards/wb-sales-funnel`) на том же периоде/оси/канале.

## Проверка и исправление p916

Если пользователь просит добиться корректных данных за период, прочитай
`references/repair-workflow.md`. Сначала вызови `prepare_funnel_repair` и покажи пользователю
полученные `preview_text`, действия и ограничения. Не запускай исправление в том же ходе.
Только после явного согласия в следующем сообщении передай без изменений `repair_spec` и
`payload_hash` в `execute_funnel_repair(confirm=true)`, затем отслеживай итог через
`get_funnel_repair_status`. Не объявляй успех до post-check.

## Отказы: две метрики, не путать

`p916` хранит отмены в двух видах, и они не взаимозаменяемы:

- **`funnel_cancel_count`** — дневной счётчик отказов самого маркетплейса (WB — `a036`,
  YM — `a041` «Отмены и невыкупы за период»), стадия `marketing`. **Это ответ по умолчанию
  на вопрос «сколько отказов/отмен за период»**: счётчик полнее order-level.
- **`cancel_count`** — отмены по документам заказов (WB — `a015`, YM — `a013`), стадия
  `fulfillment`, на дату отмены конкретного заказа. Нужен для когортного анализа и
  drilldown до заказа.

Складывать их нельзя. Долю отказов считай от «своего» знаменателя:
`funnel_cancel_count / funnel_order_count` либо `cancel_count / order_count` — не крест-накрест.
Если в ответе фигурируют обе, назови явно, какая откуда, и не выдавай их за расхождение
данных: это два измерения одного явления.

Отдельно «отказ при получении» (невыкуп) не выделяется — в отчётах маркетплейсов он входит
в общий счётчик отказов. Не обещай такой разрез.

## Общие правила

- Различай `funnel_order_count` (маркетинговый счётчик a036/a041) и `order_count`
  (фактические заказы a015/a013) — это разные числа. Для «сколько заказано» бери
  `order_count`, а не `funnel_order_count`.
- Все метрики `p916` — **положительные величины**, включая возвраты и отмены. Чистые выкупы
  = `buyout_count - return_count` (вычитать явно). Отрицательные значения в срезе — признак
  данных до нормализации, кандидат на пересбор `u508`.
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
- Для OZON пока действуй по generic-модели (`funnel-model.md` + `ozon-mapping.md`);
  заземление под конкретный пайплайн — по мере готовности.

## Яндекс.Маркет

YM заземлён на реальный пайплайн — см. `references/yandex-market-mapping.md`. Кратко:

1. Источники: `a041_ym_shows_sales_daily` (отчёт «Аналитика продаж» — единственный источник
   показов и кликов YM), `a013_ym_order` (заказы/отмены/выкупы), `a016_ym_returns`
   (возвраты). Всё сходится в тот же `p916`, что и WB.
2. Показы YM лежат в **`total_impressions`** — это готовый общий счётчик, а не сумма
   `show_free_count + show_paid_count` (те у YM пусты). Не складывай эти колонки.
3. Перед выводами запусти `run_quality_check(check_id="ym_funnel_projection_coverage")`.
4. Глубина истории `a041` ограничена тарифом YM: 90 дней без подписки/на «Лайт», 400 на
   «Медиум». За пределами окна показов просто нет — это `N/A`, а не ноль, и восстановить
   их нельзя.
5. Канального сплита paid/free у YM нет (механика `p913` — WB-специфичная): на вопросы про
   платный/органический трафик YM отвечай `N/A`.

## Терминология верха воронки (не подменяй)

- `open_count` (a036, он же метрика dv008) — **переходы в карточку**, а НЕ показы. Называть их
  «просмотрами» нельзя: пользователь читает это как impressions и считает, что показы загружены.
- **Показы**: платные — `show_paid_count` (из a026), органические — `show_free_count`, сейчас всегда
  `NULL`/`N/A`. `a040` даёт **видимость** (% присутствия в выдаче), счётчика показов WB не отдаёт
  (`impressions` там всегда 0). Никогда не обещай «загрузим `a040` — появятся показы».
- Одна и та же цифра не может в одном ответе называться «переходы», а в следующем «просмотры».
  Если пришлось уточнить термин — скажи прямо, что это то же поле, и назови его.

## Проверка загрузки данных («что загрузилось / проверь данные»)

Это отдельный сценарий, а не анализ воронки. Правила — `references/data-quality-rules.md`; минимум:

1. Считай по каждому кабинету отдельно: строк, `COUNT(DISTINCT дата)`, `MIN`/`MAX` даты, дубликаты
   (`GROUP BY кабинет, дата HAVING COUNT(*) > 1`), время последней загрузки.
2. Ожидаемое число дней в периоде сверяй с фактическим — «строк = дней × кабинетов» проверяй на
   уровне кабинета, а не общей суммой: общая сумма скрывает перекос между кабинетами.
3. `truncated: true` в ответе инструмента — выборка неполная, выводы о полноте по ней запрещены;
   переписывай запрос на агрегат.
4. Прежде чем сказать «за период данных нет», сделай контрольный запрос **без фильтров**
   (`MIN/MAX/COUNT` по кабинету) и отдели «не загружено» от «не подошёл фильтр».
5. Отвечай ровно про тот слой, о котором спросили. Если проверяли воронку (`a036`), не подменяй
   ответ выводами про `a012`/`a015` — упомяни их отдельной строкой, если это важно.
