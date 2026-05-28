# a033_wb_day_close — Документ «Закрытие дня WB»

## Назначение

Ежедневный снимок финансового результата по одному кабинету WB.
Объединяет данные из `p903_wb_finance_report` (финансовый отчёт WB),
`p913_wb_advert_order_attr` (реклама), `a012_wb_sales` (реализации/возвраты),
`a015_wb_orders` (заказы), `p912_nomenclature_costs` (дилерские цены).

---

## Ключевые концепции

### Агрегат

| Поле          | Тип                      | Описание                                          |
|---------------|--------------------------|---------------------------------------------------|
| `id`          | `WbDayCloseId`           | UUID документа                                    |
| `connection_id` | `String`               | UUID кабинета WB (связь с `a006_connection_mp`)   |
| `business_date` | `String` (YYYY-MM-DD)  | Дата закрытия дня                                 |
| `lines`       | `Vec<WbDayCloseLine>`    | Строки документа (по p903-группам)                |
| `problems`    | `Vec<WbDayCloseProblem>` | Список обнаруженных проблем                       |
| `totals`      | `WbDayCloseTotals`       | Агрегированные итоги по документу                 |
| `advert_clicks_no_order_lines` | `Vec<WbDayCloseAdvertNoOrderLine>` | Снапшот p911 (advert_clicks_no_order), заполняется при recalculate |
| `advert_clicks_order_accrual_lines` | `Vec<WbDayCloseAdvertOrderAccrualLine>` | Снапшот p913 (advert_clicks_order_accrual), заполняется при recalculate |
| `is_archived` | `bool`                   | Флаг архивной версии                              |
| `snapshot_hash` | `String`               | SHA256-хэш строк + рекламных снапшотов для обнаружения изменений |

### Строка документа (`WbDayCloseLine`)

Агрегат по группе `(srid, nm_id, nomenclature_ref, sa_name, supplier_oper_name)` из p903.
Строки **без srid** (хранение, штрафы, возмещение ПВЗ, приёмка) также включаются.

#### 10 финансовых колонок (знаковое соглашение: доход +, расход −)

| №  | Поле           | Формула                                                                       |
|----|----------------|-------------------------------------------------------------------------------|
| 1  | `revenue`      | `SUM(retail_amount) − SUM(return_amount)`                                     |
| 2  | `advertising`  | `−advert_clicks_order_expense` matched a012 (`p913` по `registrator_ref`) |
| 3  | `logistics`    | `−(delivery_rub + rebill_logistic_cost + storage_fee)`                        |
| 4  | `acquiring`    | `−SUM(acquiring_fee)`                                                         |
| 5  | `commission`   | `−(ppvz_vw + ppvz_vw_nds + ppvz_sales_commission)`                           |
| 6  | `penalty`      | `−SUM(penalty)`                                                               |
| 7  | `other`        | `SUM(additional_payment) + SUM(cashback_amount)`                              |
| 8  | `result`       | `Σ(1..7)` — вычисляемое; проверяется инвариантом                             |
| 9  | `dealer_price` | `−(dealer_price_ut × qty)` из a012; fallback p912                            |
| 10 | `margin_diff`  | `result + dealer_price`                                                       |

#### Поля классификатора (новые, с `#[serde(default)]`)

| Поле                | Тип                   | Описание                                           |
|---------------------|-----------------------|----------------------------------------------------|
| `kind`              | `LineKind`            | Тип операции строки                                |
| `detail`            | `LineDetail`          | Уровень детализации (наличие srid и nm_id)         |
| `order_id`          | `Option<String>`      | UUID документа a015 (заказ WB)                     |
| `order_date`        | `Option<String>`      | Дата заказа (YYYY-MM-DD) из a015                   |
| `order_is_cancelled`| `bool`                | Заказ помечен отменённым                           |
| `sales_doc_id`      | `Option<String>`      | UUID документа a012 (реализация/возврат)           |
| `sales_doc_no`      | `Option<String>`      | document_no из a012 (= srid WB)                    |
| `sales_event_type`  | `Option<String>`      | «sale» или «return» из a012                        |
| `sales_extra_ids`   | `Vec<String>`         | Лишние a012 с тем же srid и типом (дубли)          |

---

## Классификатор строк (`LineKind`)

Источник правды — `supplier_oper_name` из p903, суммы и наличие `srid`/`nm_id`.
Синхронизирован с GL-логикой `p903_wb_finance_report/general_ledger_builder.rs`.

| `LineKind`                        | Условие классификации                                        | Требует a015 | Требует a012 | GL-код                             |
|-----------------------------------|--------------------------------------------------------------|--------------|--------------|------------------------------------|
| `Sale`                            | `oper='Продажа'` или `retail_amount > 0` при наличии srid    | ✓ Warn        | ✓ (sale) Warn | `customer_revenue`, `mp_commission`|
| `Return`                          | `oper='Возврат'` или `return_amount > 0` при наличии srid    | ✓ Warn        | ✓ (return) Warn | `customer_return`                |
| `CommissionAdjustment`            | srid есть, не продажа/возврат, есть ppvz-суммы              | —            | —            | `mp_commission_adjustment`         |
| `Logistics`                       | `oper='Логистика'`                                           | —            | —            | `mp_rebill_logistic_cost`          |
| `Storage`                         | `oper='Хранение'`                                            | —            | —            | `mp_storage`                       |
| `Penalty`                         | `oper='Штраф'`                                               | —            | —            | `mp_penalty`                       |
| `PpvzReward`                      | `oper='Возмещение за выдачу и возврат товаров на ПВЗ'`       | —            | —            | `mp_ppvz_reward`                   |
| `VoluntaryReturnCompensation`     | `oper='Добровольная компенсация при возврате'`               | —            | —            | `voluntary_return_compensation`    |
| `TransportStorageReimbursement`   | `oper='Возмещение издержек по перевозке/по складским...'`    | —            | —            | `mp_rebill_logistic_cost`          |
| `Acceptance`                      | нет srid, `delivery_amount != 0`                            | —            | —            | `acceptance`                       |
| `Other`                           | не удалось классифицировать                                  | —            | —            | warn                               |

### `LineDetail` — уровень детализации

| `LineDetail`            | Условие                                  |
|-------------------------|------------------------------------------|
| `OrderAndNomenclature`  | есть srid и nm_id/nomenclature_ref        |
| `OrderOnly`             | только srid                              |
| `NomenclatureOnly`      | только nm_id (нет srid)                  |
| `General`               | нет ни srid, ни nm_id                    |

---

## Связи a015 и a012

### Правило «ровно 1 a012 на srid»

Для строк `Sale`/`Return` допускается ровно один документ a012 нужного типа (`event_type=sale` или `event_type=return`). Любое отклонение — проблема:
- 0 a012 нужного типа → `a012_sale_missing` / `a012_return_missing` (Warn)
- >1 a012 нужного типа → `multiple_a012_for_srid` (Block), лишние UUID в `sales_extra_ids`

### Как строится связь a015

`a015_wb_orders.document_no` = `srid` из p903. Поля:
- `order_date` = `SUBSTR(a015.state.order_dt, 1, 10)` → только дата
- `order_is_cancelled` = `a015.state.is_cancel`

Если a015 не найден для `Sale`/`Return` → проблема `a015_order_missing` (Warn).

### Как строится связь a012

`a012_wb_sales.document_no` = `srid`. Выборка: `sale_date` **≤** `business_date + 1 день` (лаг WB: в p903 `rr_dt` может быть на сутки раньше `sale_date` в a012).
Сопоставление с p903-строкой:
- **Продажа** (`LineKind::Sale`) — a012 с `COALESCE(amount_line, finished_price, total_price) > 0`
- **Возврат** (`LineKind::Return`) — a012 с той же суммой **< 0**

При `recalculate` дозаполняются a012 из этой выборки; перепроведение — только если поля изменились.

---

## Детекторы проблем

| Код                                | Серьёзность | Описание                                                                |
|------------------------------------|-------------|-------------------------------------------------------------------------|
| `advert_clicks_order_accrual_without_expense`   | Block       | p913 reserve есть, expense нет — реклама не списана                     |
| `a012_unposted_for_p903_row`       | Warn        | a012 для srid не проведён                                               |
| `advert_attributed_to_cancelled_order` | Warn   | advert_clicks_order_accrual есть, но a015 помечен отменённым                         |
| `dealer_price_missing`             | Warn        | dealer_price не найден (только для Sale/Return)                         |
| `column_invariant_mismatch`        | Block       | Σ(1..7) ≠ result — внутренняя ошибка формул                             |
| `multiple_a012_for_srid`           | Block       | >1 a012 одного типа для srid                                            |
| `a015_order_missing`               | Warn        | Sale/Return без a015                                                    |
| `a012_sale_missing`                | Warn        | Sale без a012(event_type=sale)                                          |
| `a012_return_missing`              | Warn        | Return без a012(event_type=return)                                      |
| `mixed_sale_and_return_for_srid`   | Block       | qty_sold > 0 И qty_returned > 0 в одной строке Sale/Return             |
| `unknown_line_type`                | Warn        | Не удалось классифицировать строку                                      |

---

## Итоги документа (`WbDayCloseTotals`)

| Поле            | Описание                                                  |
|-----------------|-----------------------------------------------------------|
| `lines_count`   | Число строк                                               |
| `problem_lines` | Число строк, у которых есть хотя бы одна проблема         |
| `problems_block`| Число проблем с серьёзностью Block                        |
| `problems_warn` | Число проблем с серьёзностью Warn                         |
| `problems_info` | Число проблем с серьёзностью Info                         |
| `revenue` ..    | Суммы 10 колонок по всем строкам                          |

---

## Жизненный цикл

1. `POST /api/a033/wb-day-close/find-or-create` → создаёт документ и запускает пересчёт.
2. `POST /api/a033/wb-day-close/{id}/recalculate` → дозаполняет a012 по srid из p903, пересчитывает строки, проблемы, totals.
3. `POST /api/a033/wb-day-close/{id}/repost-all` → перепроводит a012 для проблемных строк.
4. `POST /api/a033/wb-day-close/{id}/archive-and-recreate` → архивирует и создаёт новую версию.
5. `GET /api/a033/wb-day-close/{id}/compare/{archived_id}` → сравнение версий.

---

## Файловая структура

```
crates/backend/src/domain/a033_wb_day_close/
├── mod.rs               — объявление модулей
├── advert_builder.rs    — рекламные снапшоты: fetch p911/p913, обогащение a004/a015
├── lines_builder.rs     — построение строк: классификатор LineKind, fetch p903/a012/a015/p913
├── problem_detectors.rs — реестр PROBLEM_DETECTORS (коды, серьёзность, пояснение)
├── repository.rs        — ORM-модель, ser/deserialization из JSON-колонок
├── service.rs           — бизнес-логика (find_or_create, recalculate, repost-all)
└── llm.md               — этот файл

crates/contracts/src/domain/a033_wb_day_close/
├── mod.rs
└── aggregate.rs         — LineKind, LineDetail, WbDayCloseLine, WbDayCloseTotals, DTOs,
                           WbDayCloseAdvertNoOrderLine, WbDayCloseAdvertOrderAccrualLine

crates/frontend/src/domain/a033_wb_day_close/
└── ui/
    ├── list/mod.rs      — список документов
    └── details/mod.rs   — детали: TabList (Результат / Строки / Проблемы / Реклама)
```

---

## Совместимость с БД

Структура таблицы не менялась: `lines_json`, `problems_json`, `totals_json` хранят сериализованный JSON. Все новые поля добавлены с `#[serde(default)]` — старые документы читаются корректно; при следующем `recalculate` JSON перезапишется с новыми полями. Никаких SQL-миграций не требовалось.
