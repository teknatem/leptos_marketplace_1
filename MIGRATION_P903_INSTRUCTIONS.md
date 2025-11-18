# Инструкция по миграции P903 WB Finance Report

## Описание
Миграция создает таблицу `p903_wb_finance_report` для хранения финансовых отчетов Wildberries из API endpoint `reportDetailByPeriod`.

## Автоматическое применение миграции

### Вариант 1: При запуске сервера (рекомендуется)
Таблица создается **автоматически** при первом запуске бэкенд-сервера:

```bash
# Просто запустите сервер
cargo run --package backend
```

Миграция выполнится в функции `initialize_database()` в файле `crates/backend/src/shared/data/db.rs`.

## Ручное применение миграции

### Вариант 2: Через SQLite CLI

#### Windows
```powershell
# Перейти в папку с базой данных
cd target\db

# Применить миграцию
sqlite3 app.db < ..\..\migrate_p903_wb_finance_report.sql

# Проверить что таблица создана
sqlite3 app.db "SELECT name FROM sqlite_master WHERE type='table' AND name='p903_wb_finance_report';"
```

#### Linux/Mac
```bash
# Перейти в папку с базой данных
cd target/db

# Применить миграцию
sqlite3 app.db < ../../migrate_p903_wb_finance_report.sql

# Проверить что таблица создана
sqlite3 app.db "SELECT name FROM sqlite_master WHERE type='table' AND name='p903_wb_finance_report';"
```

### Вариант 3: Через Python скрипт

Используйте существующий скрипт `migrate_db.py`:

```bash
python migrate_db.py migrate_p903_wb_finance_report.sql
```

## Проверка миграции

После применения миграции проверьте:

```sql
-- Проверить структуру таблицы
PRAGMA table_info(p903_wb_finance_report);

-- Проверить индексы
SELECT * FROM sqlite_master WHERE type='index' AND tbl_name='p903_wb_finance_report';

-- Проверить что таблица пустая (перед импортом)
SELECT COUNT(*) FROM p903_wb_finance_report;
```

## Структура таблицы

### Основные поля
- **rr_dt** - дата строки отчета (часть primary key)
- **rrd_id** - ID строки отчета (часть primary key)
- **connection_mp_ref** - ссылка на подключение (важно для предотвращения дублей!)
- **organization_ref** - ссылка на организацию

### Финансовые данные (22 поля)
- acquiring_fee, acquiring_percent
- additional_payment, bonus_type_name
- commission_percent
- delivery_amount, delivery_rub
- nm_id (артикул WB)
- penalty
- ppvz_vw, ppvz_vw_nds
- quantity
- rebill_logistic_cost
- retail_amount, retail_price, retail_price_withdisc_rub
- return_amount
- sa_name (артикул продавца)
- storage_fee
- subject_name
- supplier_oper_name

### Технические поля
- **loaded_at_utc** - время загрузки
- **payload_version** - версия структуры
- **extra** - полный JSON из API

### Индексы
- idx_p903_rr_dt (по дате)
- idx_p903_nm_id (по артикулу WB)
- idx_p903_connection_mp_ref (по подключению)

## Использование в приложении

После применения миграции:

1. **Импорт данных**: 
   - Откройте UI приложения
   - Перейдите в UseCases → Wildberries Import
   - Выберите подключение
   - Отметьте чекбокс "p903_wb_finance_report - Финансовый отчет WB"
   - Укажите период (дата от/до)
   - Нажмите "Начать импорт"

2. **Просмотр данных**:
   - В левом меню выберите "WB Finance Report (P903)"
   - Используйте фильтры для поиска
   - Кликните на строку для открытия детальной информации

3. **Экспорт**:
   - Нажмите кнопку "Export Excel" для экспорта в CSV

## Особенности импорта

- Импорт происходит **по дням**
- Перед загрузкой новых данных за день **удаляются старые данные за этот день**
- Загружаются только **ежедневные отчеты** (report_type = 1)
- Для каждой строки сохраняется **полный JSON** в поле `extra`

## Откат миграции

Если нужно удалить таблицу:

```sql
DROP TABLE IF EXISTS p903_wb_finance_report;
DROP INDEX IF EXISTS idx_p903_rr_dt;
DROP INDEX IF EXISTS idx_p903_nm_id;
DROP INDEX IF EXISTS idx_p903_connection_mp_ref;
```

## Примечания

- Таблица использует **композитный первичный ключ** (rr_dt, rrd_id)
- Поле `connection_mp_ref` обязательно для предотвращения дублирования при работе с несколькими подключениями
- Все числовые поля могут быть NULL (кроме технических полей)

