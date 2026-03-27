# Active Context

_Последнее обновление: 2026-03-12_

## 🎯 Текущий фокус

DataView — новый семантический слой аналитики. Рефакторинг самодостаточности DataView,
подключение к базе знаний LLM.

### Текущее состояние

- ✅ **DataView семантический слой** — реализован и задокументирован (2026-03-12)
  - `dv001_revenue` — продажи за 2 периода (выручка, себестоимость, комиссия, расходы, прибыль)
  - `DimensionMeta` расширена SQL-полями → DataView самодостаточен (не зависит от SchemaRegistry)
  - `dv001/mod.rs` рефакторинг: `compute_drilldown` и `compute_drilldown_multi` используют `meta().available_dimensions`
  - Инструмент `list_data_views` добавлен в LLM tool_executor
  - a024/a025 зарегистрированы в LLM MetadataRegistry (category: bi/dashboard)
  - База знаний: `data/knowledge/data-view.md` + `data/knowledge/bi-indicators.md`
  - System prompt обновлён: знает о DataView и `list_data_views`
- ✅ **a024_bi_indicator** — BI Индикаторы — реализованы
- ✅ **a025_bi_dashboard** — BI Дашборды — реализованы
- ✅ **Формальная система миграций БД** — внедрена (2026-02-18)
  - `migrations/0001_baseline_schema.sql` — полная исходная схема (40 таблиц)
  - `crates/backend/src/shared/data/migration_runner.rs` — авто-запуск через sqlx
  - Трекинг состояния БД через `_sqlx_migrations`
  - `db.rs` сжат с 2872 до ~220 строк (убран инлайн bootstrap)
  - 19 старых `migrate_*.sql` перемещены в `migrations/archive/`
- ✅ **Field Metadata System POC** — реализована система метаданных для агрегатов
- ✅ Архитектура UseCase рефакторизована для программного вызова
- ✅ Реализована инфраструктура регламентных заданий (sys_scheduled_task)
- ✅ Внедрена система логирования на базе файлов для real-time мониторинга
- ✅ Реализован фоновый воркер (Background Worker) в backend на базе tokio
- ✅ Основная функциональность маркетплейсов работает стабильно
- ✅ UI на базе Thaw UI 0.5.0-beta + Leptos 0.8
- ✅ 17+ aggregates, 6 usecases, 7 projections реализованы

## 📝 Недавние изменения (последние сессии)

### DataView рефакторинг + LLM интеграция (2026-03-12)

- Расширена `DimensionMeta` (contracts): добавлены SQL execution поля (`db_column`, `ref_table`,
  `ref_display_column`, `source_table`, `join_on_column`) — все `Option<String>`, backward compat
- `dv001/metadata.json` обновлён: все 12 измерений получили SQL-поля
- `dv001/mod.rs` рефакторинг: убрана зависимость от `SchemaRegistry`/`ds03_p904_sales`
  (`compute_drilldown`, `compute_drilldown_multi` теперь используют `meta().available_dimensions`)
- `tool_executor.rs`: добавлен инструмент `list_data_views` (синхронный, читает DataViewRegistry)
- `metadata_registry.rs`: a024 и a025 зарегистрированы (category: bi/dashboard)
- `tool_executor.rs`: обновлены категории `list_entities` (добавлены "bi", "dashboard")
- `default_agent.md`: добавлен раздел DataView + инструмент `list_data_views` в список
- `config.toml`: исправлен `knowledge_base_path` (добавлен `/2/` в путь)
- `data/knowledge/data-view.md` — создан (описание dv001, метрики, измерения, API)
- `data/knowledge/bi-indicators.md` — создан (a024, DataSpec, ViewSpec, примеры API)
- `memory-bank/architecture/data-view-system.md` — создан (полная архитектурная документация)

### Формальная система миграций БД (2026-02-18)

- Создан `migrations/0001_baseline_schema.sql` — полная схема 40+ таблиц из старого `db.rs` + систем. таблицы из `migrate_*.sql`
- Создан `crates/backend/src/shared/data/migration_runner.rs`:
  - Определяет директорию `migrations/` (рядом с .exe или CWD)
  - Запускает `sqlx::migrate::Migrator` при каждом старте
  - Трекинг в таблице `_sqlx_migrations` (применялась / версия / checksum)
- Переработан `crates/backend/src/shared/data/db.rs`:
  - Убраны 2872 строки инлайн-SQL-bootstrap
  - Оставлены только: коннект к БД, `get_connection()`, `migrate_wb_sales_denormalize()`
- Обновлён `crates/backend/src/main.rs`:
  - Убран шаг `apply_auth_migration()` (auth теперь в baseline)
  - Добавлен шаг 3: `migration_runner::run_migrations().await`
- Упрощён `crates/backend/src/system/initialization.rs`:
  - Удалена функция `apply_auth_migration()`
  - Оставлена только `ensure_admin_user_exists()`
- 19 старых `migrate_*.sql` перемещены в `migrations/archive/`
- Протестировано:
  - Fresh install — миграции применяются с нуля, admin создаётся
  - Existing install — миграции идемпотентны, данные не затрагиваются

### Предыдущие работы (до февраля 2026)

- Field Metadata System POC на `a001_connection_1c`
- Scheduled Tasks System (tokio background worker)
- Thaw UI 0.5.0-beta интеграция
- Интеграции WB, Ozon, ЯМ, 1С

## 🔄 Следующие шаги

### Краткосрочные

- Добавление метаданных для остальных агрегатов (a002-a016)
- Интеграция метаданных с Frontend для автогенерации форм
- Новые DataView (dv002+) по другим источникам данных

### Среднесрочные

- Семантическое версионирование: `/api/version` endpoint, bump версии в `Cargo.toml`
- Инструмент `create_bi_indicator` для LLM (async tool executor)
- Добавление экспорта данных (Excel, CSV)

### Долгосрочные

- Полная автогенерация UI на основе метаданных
- Дополнительные интеграции маркетплейсов

## 🤔 Активные решения и рассмотрения

### Архитектурные решения

- ✅ **DB Migrations**: `sqlx::migrate!` + `migrations/` директория, трекинг через `_sqlx_migrations`
- ✅ **State management**: Использование отдельных state.rs файлов
- ✅ **List utilities**: Централизованная логика в shared/list_utils.rs
- ✅ **Hybrid tables**: Гибридный подход Thaw Table + native HTML tables
- ✅ **Signal reactivity**: Использование Signal параметров для реактивных компонентов

### Текущие паттерны

- Effect для автоматического обновления UI
- Programmatic CSS modification для кастомизации Thaw
- State persistence через localStorage
- Sortable trait для унификации сортировки

### Технические вопросы

- Нет активных блокеров
- Система работает стабильно

## 📚 Контекст для AI

### Что важно знать прямо сейчас

- **Версии**: Leptos 0.8, Thaw UI 0.5.0-beta, Axum 0.7, Sea-ORM 0.12, sqlx 0.7
- **Индексированная система**: a001-a020 (aggregates), u501-u506 (usecases), p900-p908 (projections)
- **Архитектура**: DDD + VSA с тремя крейтами (contracts, backend, frontend)
- **Окружение**: Windows 11, PowerShell (не использовать &&), pnpm для Node.js
- **База данных**: SQLite — путь берётся из `config.toml` рядом с .exe (или дефолт `target/db/app.db`)
- **Миграции**: `migrations/` директория, `sqlx::migrate::Migrator`, трекинг в `_sqlx_migrations`
- **UI библиотека**: Thaw UI 0.5.0-beta с гибридным подходом к таблицам
- **Metadata System**: JSON → build.rs → `metadata_gen.rs` с `'static` lifetimes

### Паттерны (март 2026)

- **DataView**: Новый dvNNN = папка `data_view/dvNNN/` с `mod.rs` + `metadata.json`. `DimensionMeta` содержит SQL-поля, DataView самодостаточен.
- **Добавить DataView в LLM**: зарегистрировать в `DataViewRegistry::new()`. LLM использует `list_data_views` tool.

### Паттерны (февраль 2026)

- **DB Migrations**: новая миграция = новый файл `migrations/NNNN_description.sql`
- **Field Metadata**: Декларативное описание агрегатов в `metadata.json`, автогенерация Rust кода
- **Signal parameters**: Использование `#[prop(into)] id: Signal<T>` для реактивности
- **State.rs files**: Отдельные файлы для state management компонентов
- **Thaw + Native**: Гибридный подход к таблицам в зависимости от требований

### Важные документы для справки

- `memory-bank/architecture/data-view-system.md` - архитектура DataView (NEW)
- `memory-bank/architecture/metadata-system.md` - система метаданных полей
- `memory-bank/debriefs/` - детальные описания недавних сессий
- `memory-bank/runbooks/` - пошаговые инструкции (Thaw migration, table sorting)
- `memory-bank/lessons/` - извлеченные уроки (Signal vs Value, CSS timing)
- `memory-bank/known-issues/` - известные ограничения (Thaw table limitations)

## 🔗 Связанные документы

- `projectbrief.md` - Общее описание проекта
- `systemPatterns.md` - Архитектурные паттерны
- `progress.md` - Статус реализации
- `architecture/domain-layer-architecture.md` - Детали domain layer
- `architecture/naming-conventions.md` - Система именования
