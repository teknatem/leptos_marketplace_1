# Document General Ledger Tab Standard

Стандарт для вкладки документа, показывающей записи General Ledger, созданные этим документом.

## Назначение

Вкладка используется в detail-страницах документов, которые формируют проводки в General Ledger.
Новые документы должны подключать общий компонент, а не создавать собственную GL-таблицу.

## Backend/API

- Основной endpoint для UI: `GET /api/general-ledger`.
- Обязательные фильтры документной вкладки:
  - `registrator_type=<document_key>`
  - `registrator_ref=<document_id>`
  - `sort_by=created_at`
  - `sort_desc=false`
  - `limit=500`
- Frontend helper: `fetch_document_general_ledger_entries(registrator_type, registrator_ref)`.
- DTO: `GeneralLedgerEntryDto` должен содержать `turnover_name`; fallback в UI — `turnover_code`.

## Frontend Component

Использовать:

```rust
use crate::general_ledger::ui::{
    document_general_ledger_entries_nav_id, DocumentGeneralLedgerEntries,
};
```

```rust
<DocumentGeneralLedgerEntries
    entries=entries
    loading=loading
    error=error
    nav_id=document_general_ledger_entries_nav_id("a026_wb_advert_daily")
    title="Журнал операций"
    empty_message="Записи General Ledger не найдены. Проведите документ для формирования проводок."
/>
```

Компонент обязан поддерживать:

- loading/error/empty states;
- summary badges: количество проводок, short id первой GL-записи, сумма;
- копирование в Excel через TSV;
- в Excel сумма копируется с десятичной запятой;
- сортировку по колонкам с треугольным индикатором как в `a005_marketplace`;
- переход в карточку GL-записи из колонки `ID`.

## Table Layout

Колонки в стандартном порядке:

1. Дата
2. Слой
3. Наименование оборота
4. Код оборота
5. Дт
6. Кт
7. Сумма
8. ID

Стандарт визуального поведения:

- фиксированные ширины для коротких колонок;
- колонка `Наименование оборота` занимает свободное место;
- таблица ограничена по максимальной ширине и центрируется;
- заголовки выровнены влево;
- только ячейки суммы выровнены вправо;
- `Дт` использует `badge badge--success`;
- `Кт` использует `badge badge--primary`.

## Navigation ID

Для `CardAnimated.nav_id` использовать helper:

```rust
document_general_ledger_entries_nav_id("<document_key>")
```

Он формирует:

```text
<document_key>_details_general_ledger_entries_table
```

Суффикс `_journal_table` не использовать для новых подключений: он слишком общий и не отличает GL-проводки от других журналов документа. Стандартный суффикс:

```text
general_ledger_entries_table
```

Примеры:

- `a012_wb_sales_details_general_ledger_entries_table`
- `a026_wb_advert_daily_details_general_ledger_entries_table`

## Refresh Rules

После `post` / `unpost` документа GL-сигналы должны сбрасываться или перезагружаться через общий loader, чтобы счетчик и таблица не показывали устаревшие проводки.
