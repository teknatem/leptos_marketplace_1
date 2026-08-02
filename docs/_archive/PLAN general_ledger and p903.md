## P903 to General Ledger Only

### Summary
- `p903_wb_finance_report` остаётся единственным сырым источником фактических данных WB.
- В рамках этой задачи `p903` больше не формирует никаких записей в `p909_mp_order_line_turnovers` и совсем не использует `p910_mp_unlinked_turnovers`.
- Все производные записи из `p903` строятся только в `sys_general_ledger`.
- Drilldown из `general_ledger` делаем по 4 координатам:
  - `detail_kind`
  - `detail_id`
  - `resource_name`
  - `resource_sign`
- Перезагрузка `p903` переводится на дневной атомарный reconciliation: день либо не меняется, либо полностью пересобирается вместе со связанными GL-строками.

### Key Changes
- Удалить текущую связь `p903 -> p909` и `p903 -> p910` из import/rebuild logic.
  - убрать вызовы `project_wb_finance_entry` для `p909`
  - убрать вызовы `project_wb_finance_entry` для `p910`
  - убрать очистку `p909/p910` при обработке строк `p903`
- `p910_mp_unlinked_turnovers` считать выведенным из механизма `p903`.
  - в рамках этой задачи не расширять и не использовать
  - не строить из `p903` никаких новых записей в `p910`
- `p909_mp_order_line_turnovers` закрепить как проекцию только для `a012_wb_sales` и связанных с ним `general_ledger` движений.
  - `p903` больше не должен участвовать в `p909`
- В `p903_wb_finance_report` добавить стабильный идентификатор строки-источника:
  - `source_row_ref = p903:{rr_dt}:{rrd_id}`
  - индекс и уникальность на `source_row_ref`
- В `sys_general_ledger` добавить поля:
  - `resource_name TEXT NOT NULL`
  - `resource_sign INTEGER NOT NULL`
- Контракт GL для строк, построенных из `p903`:
  - `layer='fact'`
  - `registrator_type='p903_wb_finance_report'`
  - `registrator_ref=<source_row_ref>`
  - `detail_kind='p903_wb_finance_report'`
  - `detail_id=<source_row_ref>`
  - `resource_name=<канонический идентификатор ресурса/формулы>`
  - `resource_sign IN (1, -1)`
- `turnover_code` в `sys_general_ledger` сохраняется и продолжает определять бухгалтерский смысл проводки.
  `resource_name/resource_sign` отвечают только за надёжный drilldown к исходной сумме в строке `p903`.
- Ввести явный mapping `p903 -> general_ledger`.
  Для каждой GL-проводки зафиксировать:
  - условие создания
  - `turnover_code`
  - источник суммы
  - `resource_name`
  - `resource_sign`
  - правило знака итоговой суммы в GL
- Для составных сумм использовать не имя физической колонки, а канонический `resource_name` формулы.
  Примеры:
  - `commission_ppvz_vw_plus_ppvz_vw_nds`
  - `commission_full_with_sales_commission`
- Для простых ресурсов использовать прямые идентификаторы:
  - `retail_amount`
  - `return_amount`
  - `acquiring_fee`
  - `rebill_logistic_cost`
  - `storage_fee`
  - `penalty`
  - `ppvz_for_pay`
  - `delivery_amount`
  - `additional_payment`
  - `cashback_amount`
- Дневной reconciliation для `p903`:
  - загружать данные API по 1 суткам
  - строить нормализованный снимок дня из бизнес-полей `p903`
  - сравнивать его с текущим набором строк дня в БД
  - если совпадает, ничего не делать
  - если отличается, в одной транзакции:
    1. прочитать старые строки `p903` за день
    2. собрать их `source_row_ref`
    3. удалить по ним GL-строки из `sys_general_ledger`
    4. удалить старые строки `p903` за день
    5. вставить новые строки `p903`
    6. построить новый набор GL-строк только из новых строк `p903`
- Для сравнения дня использовать детерминированный нормализованный snapshot:
  - только бизнес-поля
  - без `loaded_at_utc`
  - сортировка по `rrd_id`
  - сравнение по канонической сериализации
- Пересборку вынести в один сервис и использовать и из импорта API, и из ручного rebuild.
- UI/API `p903` сохранить как raw-интерфейс.
  - `list` остаётся по исходным строкам WB
  - в `list` добавить `general_ledger_entries_count`
  - в `details` добавить таблицу связанных GL-строк
  - в таблице GL показывать `turnover_code`, `amount`, `resource_name`, `resource_sign`
- Drilldown со стороны `general_ledger`:
  - по `detail_kind='p903_wb_finance_report'` открыть строку `p903`
  - по `detail_id` найти точную строку
  - по `resource_name` определить ресурс/формулу
  - по `resource_sign` восстановить применённую инверсию знака

### Public Interfaces
- `p903_wb_finance_report`:
  - добавить `source_row_ref`
  - добавить в list/detail поле `general_ledger_entries_count` или эквивалентный DTO field
- `sys_general_ledger`:
  - добавить `resource_name`
  - добавить `resource_sign`
- `GeneralLedgerEntryDto`:
  - добавить `resource_name: String`
  - добавить `resource_sign: i32`
- API `p903 detail`:
  - вернуть связанные GL-строки отдельной коллекцией или отдельным endpoint, выбранный по умолчанию вариант: отдельная коллекция в detail response
- API `p903 list`:
  - вернуть счётчик связанных GL-строк без изменения базовой формы списка

### Test Plan
- `a012_wb_sales`:
  - sale/return создают только `oper`-движения
  - `p903` никак не участвует в `p909`
- `p903 -> general_ledger`:
  - одна строка `p903` может создавать несколько GL-строк
  - все они имеют одинаковые `detail_kind/detail_id/registrator_ref`
  - у каждой строки корректные `resource_name/resource_sign`
- `p903` не создаёт записи в `p909`
- `p903` не создаёт записи в `p910`
- Drilldown correctness:
  - по `detail_kind + detail_id + resource_name + resource_sign` всегда однозначно определяется источник суммы
- Composite-resource case:
  - для составной комиссии drilldown указывает на канонический ресурс формулы, а не на неоднозначную колонку
- No-op reload:
  - при неизменном дневном снимке ни `p903`, ни `sys_general_ledger` не меняются
- Replace-day reload:
  - при изменении дня старые GL-строки по этому дню полностью удаляются и заменяются новыми
  - orphan GL-строк не остаётся
- UI/API:
  - `p903 list` показывает правильный `general_ledger_entries_count`
  - `p903 detail` показывает тот же набор GL-строк, что лежит в `sys_general_ledger`

### Assumptions and Defaults
- `p903` не меняет количество строк и не превращается в таблицу оборотов.
- `p903` не создаёт никаких записей, кроме строк в `sys_general_ledger`.
- `p909` и `p910` не входят в новый механизм `p903`.
- Все GL-строки, построенные из `p903`, относятся только к `fact`.
- Для drilldown обязательны оба поля: `resource_name` и `resource_sign`.
- `resource_name` трактуется как идентификатор ресурса или формулы, а не строго как имя одной физической колонки.
