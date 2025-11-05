Ниже — **схема данных БД** для регистра **`sales_register`** (единый, без специфики валют/регионов) и **таблица соответствий** полям четырёх документов (агрегатов): `OZON_FBS_PostingDocument`, `OZON_FBO_PostingDocument`, `WB_SalesDocument`, `YM_DeliveredOrderDocument`. Формулировки сделаны через поля _Документов_ (Header / Lines / State / Monetary / SourceMeta), чтобы не зависеть от точных имён в внешних API и держать 1:1 связь.

---

# 1) Регистр Sales — схема таблицы

> Гранулярность: **1 строка = 1 проданная позиция** (финальное событие продажи).
> Идемпотентность: **NK = (marketplace, document_no, line_id)**.
> Никаких вычислений по валютам/региону — **значения “как в источнике”**.

```sql
-- Референсный DDL (PostgreSQL). Можно адаптировать под любой SQL-движок.
CREATE TABLE sales_register (
  -- Идентификация и родословная
  marketplace           TEXT NOT NULL CHECK (marketplace IN ('OZON','WB','YM')),
  scheme                TEXT NULL,                              -- FBS/FBO/DBS/Express (если есть в Документе)
  document_type         TEXT NOT NULL,                          -- 'OZON_FBS_Posting' | 'OZON_FBO_Posting' | 'WB_Sales' | 'YM_Order'
  document_no           TEXT NOT NULL,                          -- posting_number | srid | orderId (из Header)
  line_id               TEXT NOT NULL,                          -- ключ строки (из Lines.*)
  document_version      INTEGER NOT NULL DEFAULT 1,             -- версия Документа (SourceMeta)
  source_ref            TEXT NOT NULL,                          -- ссылка на raw JSON (SourceMeta.raw_payload_ref)

  -- Время / статусы
  event_time_source     TIMESTAMPTZ NOT NULL,                   -- момент продажи/выкупа/доставки из State.*
  source_updated_at     TIMESTAMPTZ NULL,                       -- updated_at из источника
  status_source         TEXT NOT NULL,                          -- сырой статус из State.*
  status_norm           TEXT NOT NULL,                          -- нормализованный (например, 'DELIVERED'/'RECEIVED')

  -- Товарная идентификация
  seller_sku            TEXT NULL,                              -- ваш SKU (если маппится)
  mp_item_id            TEXT NOT NULL,                          -- offerId/shopSku/nmId/product_id (из Lines/Product)
  barcode               TEXT NULL,
  title                 TEXT NULL,                              -- опционально, снэпшот названия

  -- Кол-во и деньги (как в источнике, без конвертаций)
  qty                   NUMERIC(12,4) NOT NULL,                 -- обычно 1
  price_list            NUMERIC(18,4) NULL,                     -- цена до скидок
  discount_total        NUMERIC(18,4) NULL,                     -- суммарная скидка на строку/ед.
  price_effective       NUMERIC(18,4) NULL,                     -- цена после скидок на ед.
  amount_line           NUMERIC(18,4) NULL,                     -- сумма за строку (если приходит отдельно)
  currency_code         TEXT NULL,                              -- как в источнике (опционально)

  -- Тех. поля
  loaded_at_utc         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  payload_version       INTEGER NOT NULL DEFAULT 1,             -- версия распаковки/проекции
  extra                 JSONB NULL,                             -- любые доп. поля “как есть” при необходимости

  -- Ключи/индексы
  CONSTRAINT sales_register_pk PRIMARY KEY (marketplace, document_no, line_id),
  CONSTRAINT sales_register_docver_chk CHECK (document_version >= 1)
);

-- Рекомендуемые индексы
CREATE INDEX idx_sales_register_event_time_source ON sales_register (event_time_source);
CREATE INDEX idx_sales_register_source_updated_at ON sales_register (source_updated_at);
CREATE INDEX idx_sales_register_seller_sku ON sales_register (seller_sku);
CREATE INDEX idx_sales_register_mp_item_id ON sales_register (mp_item_id);
CREATE INDEX idx_sales_register_status_norm ON sales_register (status_norm);
```

**Примечания по модели**

- `amount_line`: если источник отдаёт уже рассчитанную сумму строки — сохраняем; иначе можно не заполнять (агрегаты/BI посчитают `price_effective * qty` вне этого этапа).
- `extra`: безопасный «клапан» для редких полей, которые не хочется выкидывать (без влияния на унификацию).
- Обновления «поздних изменений»: перезаписываем строку по NK, инкрементируя `payload_version` (или `document_version`, если меняется Документ).

---

# 2) Соответствия полям Документов (mapping)

Ниже — как каждая колонка регистра заполняется из **унифицированной структуры Документов**.
Для краткости, нотация:

- **Header.** — заголовочные поля Документа
- **Lines.** — поля строки Документа
- **State.** — статусы/временные поля
- **Monetary.** — цены/скидки/валюта
- **SourceMeta.** — служебные поля (raw, обновления, версия)

## 2.1 Общая матрица (для всех Документов)

| Колонка `sales_register` | Источник в Документе                                            |
| ------------------------ | --------------------------------------------------------------- |
| `marketplace`            | константа на поток: `'OZON'` / `'WB'` / `'YM'`                  |
| `scheme`                 | `Header.scheme` (если есть; напр., FBS/FBO)                     |
| `document_type`          | константа на тип Документа: `'OZON_FBS_Posting'`, …             |
| `document_no`            | `Header.document_no` (e.g. posting_number / srid / orderId)     |
| `line_id`                | `Lines.line_id` (детерминированный ID строки)                   |
| `document_version`       | `SourceMeta.document_version`                                   |
| `source_ref`             | `SourceMeta.raw_payload_ref`                                    |
| `event_time_source`      | `State.event_ts_sold` (см. ниже по каждому Документу)           |
| `source_updated_at`      | `SourceMeta.updated_at_source`                                  |
| `status_source`          | `State.status_raw`                                              |
| `status_norm`            | `State.status_norm` (маппинг raw→нормализованное)               |
| `seller_sku`             | `Lines.seller_sku` (если известен при распаковке)               |
| `mp_item_id`             | `Lines.marketplace_item_id` (offerId/shopSku/nmId/product_id …) |
| `barcode`                | `Lines.barcode`                                                 |
| `title`                  | `Lines.title` (опционально)                                     |
| `qty`                    | `Lines.qty`                                                     |
| `price_list`             | `Monetary.price_list`                                           |
| `discount_total`         | `Monetary.discount_total`                                       |
| `price_effective`        | `Monetary.price_effective`                                      |
| `amount_line`            | `Monetary.amount_line` (если присутствует в Документе)          |
| `currency_code`          | `Monetary.currency_code`                                        |
| `extra`                  | любые дополнительные поля строки/заголовка (опционально)        |

## 2.2 Специфика по Документам (что брать как `event_ts_sold`, `line_id`, `marketplace_item_id` и т.п.)

### A) `OZON_FBS_PostingDocument`

- **document_no:** `Header.posting_number`
- **line_id:** `Lines.line_id` (например, детерминированный из `product_id|offer_id + index` или исходный line key)
- **event_time_source (State.event_ts_sold):** `State.delivered_at` (момент доставки/выкупа по строке/постингу)
- **status_source:** `State.status_raw` (из posting.status/substatus)
- **status_norm:** маппинг `DELIVERED`
- **mp_item_id:** `Lines.offer_id` **или** `Lines.product_id` (в зависимости от принятых идентификаторов)
- **qty:** `Lines.qty` (обычно `1`)
- **Monetary:** брать «как есть» из строки постинга:
  `price_list` → исходная цена; `discount_total` → сумм. скидок; `price_effective` → цена после скидок; `amount_line` → если предоставлено отдельно; `currency_code` → из строки/заголовка.

### B) `OZON_FBO_PostingDocument`

- Аналогично FBS, отличия: `Header.scheme = 'FBO'`; поля соответствуют структуре FBO-постинга.
  Ключи/денежные поля — «как есть» из строк FBO.

### C) `WB_SalesDocument`

- **document_no:** `Header.srid` (уникальный идентификатор строки события в WB)
- **line_id:** совпадает с `Header.srid` **или** `Lines.srid` (WB выдает уник. на строку — допустимо использовать как и `document_no`, и `line_id`; для строгой модели можно задать `line_id = srid`, `document_no = srid`)
- **event_time_source:** `State.sale_dt` (момент продажи из события «sale»)
- **status_source:** тип события/статус из WB (в sales-ленте)
- **status_norm:** `DELIVERED` (для событий «sale»)
- **mp_item_id:** `Lines.nmId` **или** `Lines.vendorCode/supplierArticle` (что используете как marketplace_id)
- **qty:** `Lines.qty` (обычно `1`)
- **Monetary:** `price_list`/`discount_total`/`price_effective`/`amount_line`/`currency_code` — «как в sales-строке» (без пересчётов)

> Возвраты на этом этапе не проецируем; если позже решите добавлять — задайте `qty` отрицательным и единый `status_norm='RETURNED'` (в другой регистр или этот же).

### D) `YM_DeliveredOrderDocument`

- **document_no:** `Header.orderId`
- **line_id:** `Lines.itemId` (или детерминированный ключ строки заказа)
- **event_time_source:** `State.status_changed_at (→ DELIVERED/RECEIVED)` для конкретной строки
- **status_source:** исходный статус/субстатус строки
- **status_norm:** `DELIVERED` (или `RECEIVED`, если отличаете)
- **mp_item_id:** `Lines.shopSku` **или** `Lines.offerId` (по принятой у вас идентификации)
- **qty:** `Lines.count`
- **Monetary:** `price_list` (цена до скидок), `discount_total`, `price_effective` (итоговая цена на ед.), `amount_line` (если есть), `currency_code` — «как в заказе/строке»

---

# 3) Правила обновлений и качества

- **Upsert по NK** `(marketplace, document_no, line_id)`. При изменениях источника — перезапись строки с инкрементом `document_version` и/или `payload_version`.
- **Поздние изменения** (цена/скидка после доставки): разрешены; сохраняется «последняя версия победила».
- **Контроль целостности**:

  - Счётчик строк: кол-во строк в Документах за сутки == кол-ву строк, проецированных в регистр.
  - Лаг свежести: мониторить по `source_updated_at` и времени загрузки.
  - Трассировка: любая строка по `source_ref` ведёт к сырому JSON.

---

# 4) Мини-чеклист по внедрению

- [ ] Создать таблицу `sales_register` по DDL выше (или эквивалент).
- [ ] Зафиксировать **однозначные** поля для `document_no`, `line_id`, `mp_item_id` по каждому Документу (см. 2.2).
- [ ] Заполнить таблицы маппинга статусов (raw → `status_norm`).
- [ ] Реализовать проекцию Документ → Регистр (идемпотентный upsert по NK).
- [ ] Настроить мониторинг лагов и паритет количества (Документы ↔ Регистр).
