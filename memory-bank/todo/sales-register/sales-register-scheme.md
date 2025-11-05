Ниже — практичный план реализации именно функционала, с явным перечнем **эндпоинтов маркетплейсов** и без привязки к регионам/валютам. Фокус: документы-агрегаты → унифицированный Sales Register → минимальная задержка «продажи за сегодня».

# 1) СCOPE и принцип

- **Документы (Aggregates)**: 1:1 с исходными API-ответами (OZON FBS/FBO Postings; WB Sales; YM Orders).
- **Sales Register**: проекция из Документов; 1 строка = 1 проданная позиция; без полей, зависящих от конкретного маркетплейса.
- **Событие «продажа»**: берём момент «Доставлен/Выкуплен» у площадки (либо «sale row» у WB).
- **Время и деньги**: сохраняем «как есть» из источников; нормализации и конвертации в этом этапе **нет**.

---

# 2) Источники и эндпоинты (конкретно, что вызывать)

## OZON — заказы/отправления

- **FBS (продажи по FBS):**

  - `POST /v3/posting/fbs/list` — список отправлений, фильтрация по статусам/периоду; использовать для инкрементальной выборки. ([Ozon Docs][1])
  - `POST /v3/posting/fbs/get` — детали конкретного отправления (дозаполнение финансовых/строк). ([Ozon Docs][1])

- **FBO (продажи по FBO):**

  - `POST /v2/posting/fbo/list` — список отправлений по FBO (с фильтрами/пагинацией). ([dev.ozon.ru][2])
  - `POST /v2/posting/fbo/get` — детали отправления FBO. ([dev.ozon.ru][2])

## Wildberries — продажи

- **Статистика продаж (построчно):**

  - `GET https://statistics-api.wildberries.ru/api/v1/supplier/sales` — 1 строка = 1 продажа/возврат, обновление ~каждые 30 мин; идентификатор строки — `srid`. Использовать как основной источник продаж. ([WB API][3])

## Yandex Market — заказы

- **Список заказов:**

  - `GET v2/campaigns/{campaignId}/orders` — фильтры по статусам/дате обновления; для delivered-строк формируем продажи. (Учтите ограничение: заказы, доставленные/отменённые более 30 дней назад, списком не возвращаются — для бэкапа есть отчёты). ([yandex.ru][4])
  - `GET v2/campaigns/{campaignId}/orders/{orderId}` — детали заказа/строк (цены/скидки/идентификаторы). ([yandex.ru][5])

- **Уведомления (рекомендуется):**

  - API-уведомления о событиях: создание/изменение заказа, смена статуса, создание «невыкупа/возврата» — использовать как триггер догрузки/гидратации заказа. ([yandex.ru][6])

---

# 3) Четыре Документа (агрегаты) и правила наполнения

> Каждый Документ хранит: Header, Lines, State (сырые статусы/временные поля), Monetary (как в API), SourceMeta (raw JSON, fetched_at, версия).

1. **OZON_FBS_PostingDocument**

   - Источник: `POST /v3/posting/fbs/list` + `.../get`.
   - Идентификаторы: `posting_number` (документ), `line_id` = детерминированный ключ строки (из products/financial_data).
   - Отбор: статусы, соответствующие «доставлен/выкуплен». ([Ozon Docs][1])

2. **OZON_FBO_PostingDocument**

   - Источник: `POST /v2/posting/fbo/list` + `.../get`.
   - Идентификаторы: `posting_number`, `line_id` по строкам FBO. ([dev.ozon.ru][2])

3. **WB_SalesDocument**

   - Источник: `GET /api/v1/supplier/sales` (каждая строка — продажа/возврат).
   - Идентификатор: `srid`.
   - Отбор: для Sales Register брать «sale»-строки; возвраты пока не проецируем. ([WB API][3])

4. **YM_OrderDocument (Delivered)**

   - Источник: `GET v2/campaigns/{campaignId}/orders` (+ detail `.../orders/{orderId}` для гидратации).
   - Идентификаторы: `orderId` (документ), `itemId` (строка).
   - Отбор: строки с состоянием «DELIVERED/RECEIVED». Уведомления — как триггер. ([yandex.ru][4])

---

# 4) Sales Register — структура и проекция

**Гранулярность:** 1 строка = 1 проданная позиция.
**Ключ (NK):** `(marketplace, document_no, line_id)` — строго из источника (без трансформаций).
**Минимальные поля (без регионов/валют):**

- Идентификация: `marketplace`, `document_no`, `line_id`, `document_type`, `document_version`, `source_ref` (путь к raw JSON).
- Даты/время (как в источнике): `event_time_source` (момент продажи/выкупа/доставки), `source_updated_at`.
- Товар: `seller_sku` (ваш), `mp_item_id` (offerId/shopSku/nmId/product_id), `barcode` (если есть), `title` (опционально).
- Количество/деньги: `qty`, `price_list`, `discount_total`, `price_effective`, `amount_line` (как в источнике, 1:1).
- Статусы: `status_source`, `status_norm` (например, DELIVERED).

**Правила проекции (по источникам):**

- **OZON FBS/FBO:** брать строки с конечным статусом «доставлен/выкуплен» из Postings; по каждой строке формировать запись Register. ([Ozon Docs][1])
- **WB:** каждая «sale»-строка из `/sales` сразу становится строкой Register. ([WB API][3])
- **YM:** из Orders брать строки в состоянии DELIVERED/RECEIVED; при получении уведомления — подтянуть detail и сформировать строку Register. ([yandex.ru][4])

---

# 5) Алгоритм выборки и обновления (микро-пакеты)

1. **Инкрементальные окна**

   - OZON FBS/FBO: `list` с фильтром по периоду + статусам (rolling window), пагинация/offset; повтор с overlap для безопасности. ([Ozon Docs][1])
   - WB: `/sales` с `dateFrom/lastChangeDate` (поддерживает обновление каждые ~30 мин). ([WB API][3])
   - YM: `orders` по `updatedAt` + статусы; подписка на уведомления как триггер. ([yandex.ru][4])

2. **Пагинация и лимиты**

   - Обрабатывать `has_next/offset` (OZON), стандартные page/limit; у WB — постраничная выдача; у YM — ограничения периода и объёма ответа. ([Postman][7])

3. **Идемпотентность**

   - Upsert по NK `(marketplace, document_no, line_id)` как в источнике; хранить `document_version` и «последняя версия победила».

4. **Сырые данные и трассировка**

   - Каждый ответ сохранять «как есть» (raw JSON) с детерминированным путём; в Register держать `source_ref` для обратной проверки.

---

# 6) Пошаговый план внедрения (реализация)

**Шаг 1 — Документы (4 спецификации + хранилище):**

- Описать поля Header/Lines/State/Monetary/SourceMeta для каждого Документа.
- Создать хранилище для сырых JSON и «плоских» таблиц Документов (типизация колонок + индексы: `document_no`, `line_id`, `source_updated_at`).
- Acceptance: по каждому Документу есть образец входного JSON и пример развернутых строк.

**Шаг 2 — Коннекторы (fetchers):**

- Реализовать вызовы эндпоинтов из разд.2 с параметрами инкремента и пагинации.
- Вести чекпоинты «последнего успешно обработанного обновления» на уровне потока/эндпоинта.
- YM: поднять приём уведомлений (эндпоинт + верификация) и сценарий «hydrate by id». ([yandex.ru][8])
- Acceptance: повторный прогон окна не создаёт дублей; DLQ/повторы покрыты.

**Шаг 3 — Проекция в Sales Register:**

- Маппинг полей (ID/qty/цены/скидки/время/статусы) из каждого Документа.
- Набор unit-примеров «вход → выход» по каждому источнику.
- Acceptance: дневные итоги по количеству/сумме воспроизводимы из Документов.

**Шаг 4 — Бэкапы/историчность:**

- Бэкофис-процедура исторической загрузки (денежно не нормализуем; просто «как есть»).
- YM: учесть, что список не возвращает старые (>30 дней) доставленные/отменённые — при необходимости использовать заказ по ID/отчёты. ([yandex.ru][4])

**Шаг 5 — Контроль качества:**

- Сверка «кол-во строк в Документах vs. Register» за сутки.
- Сенсоры задержки для WB (~30 минут SLA обновления sales) и проверка отсутствия «дырок» в окнах. ([WB API][3])

---

# 7) Мини-беклог задач (готово к занесению в трекер)

- [ ] **Спецификации Документов**: OZON_FBS_Posting, OZON_FBO_Posting, WB_Sales, YM_Order (Delivered).
- [ ] **Схема Sales Register**: ключи/поля (без регионов/валют), правила проекции.
- [ ] **Коннектор OZON FBS**: `/v3/posting/fbs/list` + `/v3/posting/fbs/get`; инкремент + пагинация. ([Ozon Docs][1])
- [ ] **Коннектор OZON FBO**: `/v2/posting/fbo/list` + `/v2/posting/fbo/get`. ([dev.ozon.ru][2])
- [ ] **Коннектор WB**: `/api/v1/supplier/sales` (инкремент по lastChangeDate). ([WB API][3])
- [ ] **Коннектор YM**: `GET v2/campaigns/{id}/orders` + `.../{orderId}`; вебхуки/уведомления. ([yandex.ru][4])
- [ ] **Raw-storage**: сохранение каждого ответа + ссылка `source_ref` в Документах/Регистрe.
- [ ] **Идемпотентные upsert-проекции**: NK-ключ, политика «последняя версия победила».
- [ ] **Набор эталонных примеров**: по 3 кейса на источник (создание, позднее обновление, отмена/исключение).
- [ ] **Мониторинг и алерты**: лаг инкремента, 429/5xx, «тихие сутки» у потока, расхождение количества между Документами и Register.
- [ ] **DoD**: прогоны «скользящего окна», отсутствие дублей, воспроизводимость дневных итогов из raw.

---

# 8) Что важно помнить

- На этом этапе **никаких** нормализаций по времени/валюте/региону — только хранение и проекция «как в источнике».
- Вся специфика маркетплейсов (поля статусов, имена идентификаторов) остаётся в Документах; в Sales Register — унифицированные имена и минимальный набор полей.
- Для минимальной задержки — инкрементальные окна + YM-уведомления как триггеры загрузки. ([yandex.ru][8])

Если нужно, подготовлю короткие «field-mapping» таблицы по каждому из четырёх Документов → в Sales Register (без регионов и валют), чтобы команда сразу разбила работу на подпроекции.

[1]: https://docs.ozon.ru/global/api/intro/?utm_source=chatgpt.com "Работа с API"
[2]: https://dev.ozon.ru/start/300-Mapping-rolei-i-metodov-Seller-API/?utm_source=chatgpt.com "Маппинг ролей и методов Seller API"
[3]: https://dev.wildberries.ru/openapi/reports?utm_source=chatgpt.com "Reports - Documentation — WB API - Wildberries"
[4]: https://yandex.ru/dev/market/partner-api/doc/ru/reference/orders/getOrders?utm_source=chatgpt.com "О списке заказов - Информация о заказах | API ..."
[5]: https://yandex.ru/dev/market/partner-api/doc/en/reference/orders/getOrder?utm_source=chatgpt.com "Information about orders | Yandex.Market API for sellers"
[6]: https://yandex.ru/dev/market/partner-api/doc/ru/push-notifications/reference/sendNotification?utm_source=chatgpt.com "Получение уведомлений - API-уведомления"
[7]: https://www.postman.com/googlesheets/ozon-seller-api/request/pvq9mfu/3?utm_source=chatgpt.com "Список отправлений (версия 3) | Ozon Seller API"
[8]: https://yandex.ru/dev/market/partner-api/doc/en/push-notifications/reference/sendNotification?utm_source=chatgpt.com "Receiving notifications - Yandex.Market API for sellers"
