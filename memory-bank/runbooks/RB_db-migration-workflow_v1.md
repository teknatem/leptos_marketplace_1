# Runbook: Работа с миграциями БД

_Версия: v1 | Дата: 2026-02-18_

## Контекст

Проект использует `sqlx::migrate::Migrator` для управления схемой SQLite.
Миграции хранятся в директории `migrations/` и применяются **автоматически** при каждом старте backend.

## Структура директории migrations/

```
migrations/
├── 0001_baseline_schema.sql   <- полная начальная схема (40+ таблиц)
├── 0002_...sql                <- будущие изменения
└── archive/                   <- старые migrate_*.sql (только история, не исполняются)
```

## Добавить новое изменение схемы

### 1. Определить номер

```powershell
# Посмотреть последний применённый номер
sqlite3 marketplace.db "SELECT MAX(version) FROM _sqlx_migrations"
# Следующий файл: MAX + 1 (четыре цифры с нулями)
```

### 2. Создать файл

Имя файла: `migrations/NNNN_description.sql`

Примеры:
- `migrations/0002_a020_new_aggregate.sql`
- `migrations/0003_add_column_to_a006.sql`
- `migrations/0004_p907_new_projection.sql`

### 3. Написать SQL

SQLite не поддерживает `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`. В рамках tracked-миграций это не нужно:
каждая миграция применяется **ровно один раз** и отслеживается по номеру.

```sql
-- migrations/0002_a020_new_aggregate.sql

CREATE TABLE IF NOT EXISTS a020_new_entity (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_a020_code ON a020_new_entity (code);
```

### 4. Применить

```powershell
# Миграция применится при следующем запуске backend
cargo run --bin backend
```

При старте в логах будет:

```
Step 3: Running database migrations...
INFO  backend::shared::data::migration_runner: Using migrations directory: migrations
INFO  backend::shared::data::migration_runner: Database migrations applied successfully
✓ Database migrations processed
```

---

## Проверить текущее состояние миграций

```powershell
# Через sqlite3 (если установлен)
sqlite3 marketplace.db "SELECT version, description, installed_on, success FROM _sqlx_migrations ORDER BY version"

# Через sqlx-cli (если установлен)
sqlx migrate info --database-url "sqlite:marketplace.db"
```

Пример вывода:

```
1 | baseline schema | 2026-02-18 09:18:39 | 1
2 | a020_new_entity | 2026-02-20 14:00:00 | 1
```

---

## Fresh install (новая машина / новый путь к БД)

1. Настроить `config.toml` рядом с `backend.exe`:

```toml
[database]
path = "C:/Users/udv/Desktop/MPI/data/app.db"

[scheduled_tasks]
enabled = true
```

2. Запустить backend:

```powershell
.\backend.exe
```

Backend автоматически:
- Создаст директорию для БД
- Создаст SQLite файл
- Применит `0001_baseline_schema.sql` (создаст все таблицы)
- Создаст пользователя `admin/admin` (показывает предупреждение в логе)

---

## Диагностика проблем с миграцией

### Ошибка: `error returned from database: (code: 1) near "...": syntax error`

SQL-синтаксис в файле миграции. Проверить:

```powershell
# Валидировать SQL без применения (Python)
$script = @"
import sqlite3
text = open('migrations/0002_example.sql', 'r', encoding='utf-8').read()
c = sqlite3.connect(':memory:')
errors = []
for i, part in enumerate(text.split(';'), 1):
    s = part.strip()
    if not s or s.startswith('--'): continue
    try: c.execute(s)
    except Exception as e: errors.append((i, str(e), s[:100]))
print('errors:', len(errors))
for i, e, s in errors: print(i, e, s)
"@
python -c $script
```

### Ошибка: `migrations directory not found`

Файл `migrations/` не найден. Стандартные пути поиска в `migration_runner.rs`:
1. Рядом с `.exe` → `<exe_dir>/migrations/`
2. CWD → `migrations/`
3. `../../migrations/` (из `target/debug/`)

При деплое: положить папку `migrations/` рядом с `backend.exe`.

### Ошибка: `migration checksum mismatch`

Изменён уже применённый файл миграции. **Никогда не редактировать применённые миграции**.
Исправление: создать новую миграцию с нужными изменениями.

---

## Принципы

| Правило | Почему |
|---|---|
| Миграции только в `migrations/` | db.rs — только коннект, не схема |
| Не редактировать применённые файлы | Checksum-защита sqlx |
| Нумерация 4 цифры (0001, 0002...) | Сортировка по имени = порядок применения |
| Имена файлов без пробелов | Кроссплатформенная совместимость |
| CREATE TABLE IF NOT EXISTS в baseline | Базовый 0001 — idempotent |
| Обычный CREATE TABLE в новых миграциях | Применяется только один раз |

---

## Связанные файлы

- `crates/backend/src/shared/data/migration_runner.rs` — логика запуска
- `crates/backend/src/shared/data/db.rs` — только коннект к БД
- `crates/backend/src/main.rs` — шаг 3 в последовательности старта
- `migrations/0001_baseline_schema.sql` — полная исходная схема
- `migrations/archive/` — старые файлы для истории
