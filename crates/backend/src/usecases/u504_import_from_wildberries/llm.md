---
title: U504 Import From Wildberries
tags: [wb, import, api, indicators, usecase, u504]
related: [a012_wb_sales, p904_sales_data]
updated: 2026-03-17
---

# Summary

`u504_import_from_wildberries` отвечает за получение данных из Wildberries API, преобразование ответа в локальные структуры и сохранение документов WB в систему.

# API Role

- Этот модуль является владельцем знаний о внешнем WB API для продаж.
- Основная DTO для строк продаж: `WbSaleRow`.
- Поля DTO отражают API-имена WB, например `nmId`, `supplierArticle`, `forPay`, `finishedPrice`, `saleID`.

# Used WB Endpoints

- `POST /content/v2/get/cards/list`
  Content API для загрузки карточек товаров WB. В `u504` используется как источник товарного справочника и связки `nmId` <-> карточка/бренд/предмет/баркод. По документации WB это cursor-based endpoint: в теле запроса передаются `settings.cursor` и `limit`. В коде используется пагинация по `cursor.updatedAt` и `cursor.nmID`, без фильтра `findByNmID`.
- `GET /api/v1/supplier/sales`
  Statistics API для строк продаж и возвратов. Это основной upstream для `a012_wb_sales`. По документации WB ключевые параметры: `dateFrom`, `dateTo`, `flag`. В `u504` этот endpoint читается как интервал продаж, а тип события выводится из знака `quantity`: отрицательное значение трактуется как возврат.
- `GET /api/v1/supplier/orders`
  Statistics API для заказов. В usecase он нужен как соседний источник WB-операций, но не является главным downstream-источником для индикаторов выручки. По документации основной параметр отбора — `dateFrom`; в коде используется backfill по `lastChangeDate` с `flag=0`, а `dateTo` служит soft-stop уже на стороне приложения.
- `GET /api/v5/supplier/reportDetailByPeriod`
  Statistics API для детального финансового отчета за период. Используется для финансового слоя WB, который затем участвует в BI-расчетах. По документации и по коду важны `dateFrom`, `dateTo`, `rrdid`, `limit`. Эндпоинт имеет жесткое ограничение частоты запросов; в коде это учтено через ожидание после `429 Too Many Requests` и пагинацию по `rrdid`.
- `GET /api/v1/tariffs/commission?locale=ru`
  Common API для комиссий WB по предметам/категориям. В `u504` endpoint используется как справочник комиссий для обогащения локальных расчетов. Параметр `locale=ru` влияет на язык категорий в ответе.
- `GET /api/v2/list/goods/filter?limit={limit}&offset={offset}`
  Prices and Discounts API для цен и скидок по товарам. В usecase применяется как источник текущих ценовых параметров WB. По документации у endpoint offset-based pagination; в коде используется пара `limit/offset`.
- `GET /api/v1/calendar/promotions`
  Promotion Calendar API для списка активных и будущих акций. В коде вызывается с `startDateTime`, `endDateTime`, `allPromo`; объединяются `promotions` и `upcomingPromos` из ответа.
- `GET /api/v1/calendar/promotions/details`
  Promotion Calendar API для детальной информации по акциям. В коде используется batched-запрос по `promotionIDs`, поскольку детализация нужна уже после получения списка акций.
- `GET /api/v1/calendar/promotions/nomenclatures`
  Promotion Calendar API для получения `nmId`, относящихся к акции. По документации обязательны `promotionID` и `inAction`; в коде дополнительно используется пагинация `limit/offset` и запрашиваются оба состояния `inAction=true` и `inAction=false`. Для акций типа `auto` endpoint пропускается, потому что WB его для них не поддерживает.

# Endpoint Notes

- Все используемые методы требуют seller API key в заголовке `Authorization`.
- Для product cards, prices и promotions usecase в основном строит справочники и enrichments; главный источник фактов по продажам для цепочки индикаторов — `GET /api/v1/supplier/sales`.
- Для финансовых индикаторов нельзя опираться только на Sales API: часть сумм и удержаний берется из `reportDetailByPeriod`.
- `a012_wb_sales` — это нормализованный локальный документ после маппинга WB API, а не прямой снимок ответа одного endpoint.
- Для BI и dashboard downstream-источником является не raw WB DTO, а уже локальная проекция `p904_sales_data`.

# WB Sales Field Semantics

- `srid`
  Стабильный идентификатор строки заказа/продажи в отчетах WB. В документации WB именно `srid` рекомендуется использовать для идентификации строки. В проекте он участвует как внешний id строки и используется для `header.document_no` и `line.line_id`.
- `saleID`
  Идентификатор строки операции WB, а не номер заказа. Нужен для дедупликации и трассировки операции. В проекте маппится в `header.sale_id`.
- `odid`
  Идентификатор заказа WB. Это ближе к сущности заказа, чем `saleID`. Если нужно связывать продажу с order flow, смотреть нужно сначала на `odid` и `srid`, а не только на `saleID`.
- `quantity`
  Количество по строке WB. В проекте знак количества важен: отрицательное значение интерпретируется как возврат, положительное как продажа.
- `totalPrice`
  Полная цена товара до применения скидок. Это базовая цена строки в отчете WB. В проекте хранится отдельно, потому что она не равна ни сумме к выплате продавцу, ни фактической цене для покупателя.
- `discount`
  Скидка продавца в строке WB Sales API. Это не итоговая сумма по строке, а компонент скидочной логики. В проекте маппится в `line.discount_total`.
- `priceWithDisc`
  Цена со скидкой продавца в Sales API. По документации WB это поле может рассчитываться по упрощенной логике и временно быть `0`, а также может отличаться от более точных значений в детализированном финансовом отчете. В проекте используется как ближайший операционный аналог цены строки и маппится в `line.price_list` и `line.price_effective`.
- `finishedPrice`
  Фактическая цена строки с учетом скидок на стороне WB. Это ближе к цене для покупателя, чем `forPay`. По документации WB поле может заполняться асинхронно и временно быть `0`. В проекте хранится отдельно в `line.finished_price`.
- `forPay`
  Сумма к перечислению продавцу по строке операции в Sales API. Это ближайшая интерпретация "сколько продавец получает по строке" на операционном уровне. В проекте именно это поле маппится в `line.amount_line`. По документации WB значение может быть рассчитано по упрощенной логике и отличаться от более точного `ppvz_for_pay` из `reportDetailByPeriod`.
- `paymentSaleAmount`
  Сумма платежа по продаже, если WB вернуло это поле в ответе. Поле не является главным источником для `line.amount_line`; для этого в проекте используется `forPay`.
- `spp`
  Поле WB, связанное со скидочным механизмом SPP. Его нужно трактовать как отдельный компонент скидки/корректировки WB, а не как готовую сумму выручки. В проекте хранится отдельно в `line.spp`.
- `date`
  Дата операции продажи/возврата в отчете WB. Маппится в `state.sale_dt`.
- `lastChangeDate`
  Момент последнего изменения строки на стороне WB. Полезен для инкрементального чтения, повторной синхронизации и cursor/backfill логики. Маппится в `state.last_change_dt`.

# Interpretation Priority

- Если нужно понять "что получил покупатель", сначала смотреть на `finishedPrice`.
- Если нужно понять "что причитается продавцу по строке", сначала смотреть на `forPay`.
- Если нужно понять "какая была базовая цена до скидок", смотреть на `totalPrice`.
- Если нужна точная финансовая аналитика по удержаниям и выплатам, `Sales API` недостаточно; нужно дополнительно использовать `reportDetailByPeriod`.

# Mapping To A012

- `srid` -> `header.document_no` и `line.line_id`
- `saleID` -> `header.sale_id`
- `supplierArticle` -> `line.supplier_article`
- `nmId` -> `line.nm_id`
- `barcode` -> `line.barcode`
- `priceWithDisc` -> `line.price_list` и `line.price_effective`
- `discount` -> `line.discount_total`
- `forPay` -> `line.amount_line`
- `finishedPrice` -> `line.finished_price`
- `totalPrice` -> `line.total_price`
- `spp` -> `line.spp`
- `quantity` -> `line.qty` и через знак определяет `state.event_type`
- `date` -> `state.sale_dt`
- `lastChangeDate` -> `state.last_change_dt`
- `isSupply` -> `state.is_supply`
- `isRealization` -> `state.is_realization`

# Rules

- Перед сохранением выполняется дедупликация по `sale_id`.
- Если `sale_id` отсутствует, генерируется surrogate key.
- Даты парсятся в несколько форматов; при неуспехе используется текущее UTC время.
- `event_type = return`, если `quantity < 0`, иначе `sale`.

# Downstream

- Результат импорта сохраняется как `a012_wb_sales`.
- После posting данные попадают в `p904_sales_data`, где уже формируются BI-суммы и показатели для индикаторов.

# Pitfalls

- В коде `saleID` — это идентификатор строки операции, а не номер заказа.
- Для построения индикаторов по выручке и прибыли правильным downstream-источником является `p904_sales_data`, не raw DTO и не `a012_wb_sales`.
- `dateTo` поддерживается не всеми WB Statistics endpoints одинаково; для `orders` в `u504` он используется как ограничение на стороне приложения, а не как полноценный server-side filter.
- Нельзя смешивать product, sales, finance и promotion endpoints как один источник истины: они описывают разные аспекты данных WB и сходятся только после локального маппинга и projection logic.

# Official Docs

- Reports and statistics: `https://dev.wildberries.ru/openapi/reports`
- Content / product cards: `https://dev.wildberries.ru/openapi/work-with-products`
- Prices and discounts: `https://dev.wildberries.ru/openapi/prices-and-discounts`
- Tariffs: `https://dev.wildberries.ru/openapi/tariffs`
- Promotions: `https://dev.wildberries.ru/openapi/promotion`
