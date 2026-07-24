---
title: a040 — поисковая аналитика WB (показы, позиции, запросы)
tags: [marketplaces, wildberries, search, показы, impressions, a040, seo, funnel, воронка, джем]
related: [a036_wb_sales_funnel_daily, a037_wb_product_snapshot, p916_mp_sales_funnel_turnovers, task024_wb_search_analytics_daily]
updated: 2026-07-21
---

# a040_wb_search_analytics_daily — поисковая аналитика Wildberries

Ежедневные снимки поисковой аналитики WB в разрезе номенклатуры (источник — **search-report
API**, «Товары по контенту» / «Аналитика поисковых запросов», подписка **«Джем»**). Даёт
**органические показы** в поиске — то, чего нет в воронке a036 (там только переходы `openCard`).

Грань — `nm_id × дата`, один документ = один кабинет WB × одна дата. **Forward-only** (WB отдаёт
только недавнее окно). Импортирует задача `task024_wb_search_analytics_daily` (по кабинету, раз в
день); при отсутствии «Джем»/доступа (403) задача логирует и завершается без ошибки.

## Что хранит (на товар, `lines_json`)
- `metrics`: **impressions** (показы), **open_card** (переходы из поиска), `ctr`, `add_to_cart`,
  `orders`, **avg_position** (средняя позиция в выдаче), `visibility`.
- `top_queries[]`: топ поисковых запросов на товар — `text`, `frequency` (частотность),
  `impressions`, `clicks`, `orders`, `avg_position` (для SEO/семантики карточки).
- Итоги-колонки: `total_impressions`, `total_open_card`, `total_orders`.

## Связь с воронкой p916
a040 **НЕ питает** воронку p916. Живой WB-эндпоинт `/table/details` отдаёт только `visibility`
(% показов в поиске), а не счётчик показов — `impressions` в ответе всегда 0 (см.
`parse_search_report_row`). Писать процент в `show_free_count` нельзя (SUM смешал бы штуки и
проценты), поэтому связь a040→p916 удалена; органические показы в воронке сейчас `N/A` до
появления реального источника счётчика органических показов.

## Оговорка по полям
Точная форма ответа WB search-report офлайн не верифицирована — парсинг в
`wildberries_api_client.rs` (`fetch_search_report`/`fetch_search_texts`) сделан толерантно
(несколько кандидатов-ключей; сырой ответ логируется блоками `=== SEARCH REPORT RESPONSE ===`).
При первом живом прогоне сверить имена полей (особенно «показы» vs `visibility`) и при
необходимости поправить `parse_search_report_row`/`parse_search_query_row`.

## API
- `GET /api/a040/wb-search-analytics/list` — список снимков (пагинация, фильтры период/кабинет).
- `GET /api/a040/wb-search-analytics/:id` — документ с товарными строками и топ-запросами.
