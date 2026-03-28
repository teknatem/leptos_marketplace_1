---
title: "General Ledger — Главная книга"
tags: [general_ledger, gl, accounting, turnover, accounts, journal, sys_general_ledger]
related: [a012_wb_sales, a026_wb_advert_daily, p903_wb_finance_report, p909_mp_order_line_turnovers, p910_mp_unlinked_turnovers, a024_bi_indicator]
updated: 2026-03-28
---

# General Ledger (Главная книга)

## Что это такое

General Ledger (GL) — центральный учётный регистр всех хозяйственных операций.
Хранится в таблице `sys_general_ledger`. Каждая строка — одна проводка (запись журнала операций):
дебет одного счёта / кредит другого на сумму операции.

Это не аналитическая проекция — это **факт учёта**, порождённый агрегатами при их проведении (posting).

## Таблица sys_general_ledger

| Колонка | Тип | Описание |
|---|---|---|
| `id` | TEXT PK | UUID записи журнала |
| `entry_date` | TEXT | Дата проводки (YYYY-MM-DD или ISO datetime) |
| `layer` | TEXT | Слой: `oper` / `fact` / `plan` |
| `cabinet_mp` | TEXT NULL | Ссылка на кабинет МП (a006_connection_mp.id) |
| `registrator_type` | TEXT | Тип регистратора: `a012_wb_sales`, `p903_wb_finance_report`, `a026_wb_advert_daily`, ... |
| `registrator_ref` | TEXT | Ссылка на документ-источник: `{registrator_type}:{id}` |
| `debit_account` | TEXT | Дебетуемый счёт по плану счетов |
| `credit_account` | TEXT | Кредитуемый счёт по плану счетов |
| `amount` | REAL | Сумма проводки (всегда положительная или отрицательная по знаку оборота) |
| `qty` | REAL NULL | Количество (если применимо) |
| `turnover_code` | TEXT | Код вида оборота из реестра (см. ниже) |
| `resource_table` | TEXT | Источник значения (таблица) |
| `resource_field` | TEXT | Поле в источнике |
| `resource_sign` | INT | Знак ресурса: +1 или -1 |
| `created_at` | TEXT | Время создания записи |

## API эндпоинты

```
GET  /api/general-ledger                       → список проводок (с фильтрами)
GET  /api/general-ledger/:id                   → конкретная проводка
GET  /api/general-ledger/turnovers             → реестр видов оборотов с количеством записей
```

Параметры GET /api/general-ledger:
- `date_from`, `date_to` — диапазон дат entry_date
- `registrator_ref`, `registrator_type` — фильтр по источнику
- `layer` — `oper` / `fact` / `plan`
- `turnover_code` — конкретный вид оборота
- `debit_account`, `credit_account` — фильтр по счёту
- `cabinet_mp` — фильтр по кабинету МП
- `limit`, `offset` — пагинация (default limit=100)

## Слои данных (layer)

| Слой | Значение |
|---|---|
| `oper` | Оперативный: данные на основе заказов, до финансового подтверждения |
| `fact` | Фактический: данные из финансовых отчётов маркетплейса |
| `plan` | Плановый (резерв, пока не используется) |

## План счетов (ACCOUNT_REGISTRY)

| Счёт | Название | Тип |
|---|---|---|
| 41 | Товары | Активный (баланс) |
| 44 | Расходы на продажу | Активный (P&L) |
| 4401 | Расходы на продажу — маркетплейс | Активный (P&L) |
| 4402 | Комиссия МП | Активный (P&L) |
| 4403 | Эквайринг МП | Активный (P&L) |
| 4404 | Логистика/хранение МП | Активный (P&L) |
| 62 | Расчёты с покупателями | Активно-пассивный (баланс) |
| 76 | Расчёты с прочими | Активно-пассивный (баланс) |
| 7609 | Расчёты с маркетплейсом | Активно-пассивный (баланс) |
| 90 | Продажи | Активно-пассивный (P&L) |
| 9001 | Выручка от продаж | Пассивный (P&L) |
| 9002 | Себестоимость продаж | Активный (P&L) |
| 91 | Прочие доходы и расходы | Активно-пассивный (P&L) |
| 9102 | Штрафы от МП | — |

## Виды оборотов (turnover_code) — ключевые

Каждый оборот привязан к report_group. Инструмент `list_gl_turnovers` даёт полный список.

| Код | Название | Дт | Кт | Report group |
|---|---|---|---|---|
| `customer_revenue` | Выручка от покупателя | 62 | 9001 | revenue |
| `customer_return` | Возврат покупателю | 9001 | 62 | returns |
| `mp_commission` | Комиссия МП | 4402 | 7609 | commission |
| `mp_commission_adjustment` | Корректировка комиссии | 4402 | 7609 | commission |
| `mp_acquiring` | Эквайринг МП | 4403 | 7609 | acquiring |
| `mp_logistics` | Логистика МП | 4404 | 7609 | logistics |
| `mp_storage` | Хранение МП | 4404 | 7609 | storage |
| `mp_penalty` | Штраф МП | 9102 | 7609 | penalty |
| `advertising_allocated` | Реклама (WB Advert) | 4401 | 7609 | advertising |
| `voluntary_return_compensation` | Добровольная компенсация | 7609 | 91 | other |

## Источники проводок (кто создаёт записи GL)

| Источник | registrator_type | Метод |
|---|---|---|
| WB финансовый отчёт | `p903_wb_finance_report` | posting при импорте p903 |
| WB продажи | `a012_wb_sales` | posting при sync |
| WB реклама | `a026_wb_advert_daily` | posting при sync |

Проводки создаются функцией `general_ledger::service::save_entries()`.
При обновлении документа старые проводки удаляются через `remove_by_registrator_ref()`.

## Связь с BI и DataView

BI индикаторы (a024) могут агрегировать данные GL через DataView.
DataView `dv001_revenue`, `dv002`, `dv003` используют данные из проекций,
которые в свою очередь строятся на основе GL-записей через `p909`, `p910`.

Для анализа GL через SQL:
```sql
-- Обороты по счетам за период
SELECT debit_account, credit_account, turnover_code, SUM(amount) as total
FROM sys_general_ledger
WHERE entry_date BETWEEN '2026-01-01' AND '2026-01-31'
  AND layer = 'fact'
GROUP BY debit_account, credit_account, turnover_code
ORDER BY ABS(SUM(amount)) DESC;

-- Проводки конкретного регистратора
SELECT * FROM sys_general_ledger
WHERE registrator_ref = 'p903_wb_finance_report:{id}'
ORDER BY entry_date;
```

## Ключевые особенности

1. **registrator_ref** — всегда формат `{type}:{id}` (например `a012_wb_sales:uuid`)
2. **Идемпотентность**: перед записью новых проводок старые по registrator_ref удаляются
3. **layer=fact** — основной слой для финансового анализа
4. **turnover_code** — ключ для понимания экономического смысла проводки
5. **generates_journal_entry=false** в реестре оборотов означает, что этот оборот НЕ порождает запись в GL (только в проекции p909/p910)
