# CLAUDE.md

Гид для Claude Code по проекту **leptos_marketplace_1** — десктопная система управления маркетплейсами (1С:УТ 11, Wildberries, Ozon, Yandex Market) на Rust/Leptos.

> **Источник истины — код.** При расхождениях приоритет: `код` > `.claude/memory/` > `memory-bank/` > `docs/`.
> Прежде чем опираться на доку, проверь, что файл/функция/флаг ещё существуют в коде.

> **Карта объектов — `ARCHITECTURE.md`** (генерируется из кода): полный каталог агрегатов
> a0XX (с описаниями из metadata.json), проекций, use-cases, плана счетов, видов оборотов
> и всех API-роутов. Читай его, чтобы найти нужный объект, **не grep'ая** исходники.
> Регенерация после изменений: `powershell -File tools/gen_architecture.ps1`.
> Авто-обновление: git pre-commit hook (`tools/hooks/`) регенерирует карту, когда в коммит
> попадают domain/projections/usecases/реестры/`routes.rs`. После свежего клона включи один раз:
> `git config core.hooksPath tools/hooks`.

---

## Сборка и запуск

Dev (два терминала, из корня):
```powershell
cargo run -p backend          # Axum API на http://localhost:3000
trunk serve --port 8080       # Leptos/WASM фронт на http://localhost:8080 (проксирует API на :3000)
```

> Уже запущенный backend.exe держит `target\debug\backend.exe` — повторный `cargo run`
> падает с «Access is denied». Перезапуск: `powershell -File tools/restart_backend.ps1`
> (останавливает процесс и запускает свежую сборку).

Проверка перед коммитом:
```powershell
cargo check -p backend
cargo check -p contracts
cargo check -p frontend --target wasm32-unknown-unknown   # frontend — только wasm-таргет
cargo test -p backend router_builds   # после правок роутов: конфликт путей axum виден только при сборке Router
```

Release:
```powershell
trunk build --release                      # → dist/ (фронт)
cargo build --release --bin backend        # → target/release/backend.exe
```

Заметки по профилям (`Cargo.toml`): dev-сборка без оптимизаций ради скорости; `adobe-cmap-parser` собирается без overflow-checks (иначе паника при извлечении PDF в dev).

---

## Крейты (workspace)

| Крейт | Роль |
|---|---|
| `crates/backend` | Axum-сервер, бизнес-логика, БД (SQLite + SeaORM), проекции, главная книга |
| `crates/frontend` | Leptos/WASM SPA (Trunk), thaw UI + кастомный BEM/CSS |
| `crates/contracts` | Общие DTO, определения агрегатов, metadata — разделяемы между фронтом и бэком |

Все три крейта зеркалят одну структуру слоёв: `domain/`, `projections/`, `general_ledger/`, `dashboards/`, `quality/`, `system/`, `shared/`, `usecases/`.

---

## Схема именования (ключ к навигации)

Код объекта = префикс + номер. Зная код, прыгай сразу в файл — не ищи.

| Префикс | Что это | Где (backend) | Диапазон |
|---|---|---|---|
| `a0XX` | **Агрегат** (домен-сущность/документ) | `domain/a0XX_*` | a001–a035 |
| `p9XX` | **Проекция** (производная read-модель) | `projections/p9XX_*` | p900–p915 |
| `u5XX` | **Use-case** (импорты, репост) | `usecases/u5XX_*` | u501–u508 |
| `dsXX` | **Базовая схема данных** (роль *base schema*, движок universal_dashboard; UI: «Схемы таблиц») | `data_schemes/dsXX_*` | ds01–ds03 |
| `dvXX` | **DataView** (роль *виртуальная таблица*: курируемые метрики, 2 периода, кэш; UI: «DataView») | `data_view/dvXXX_*` | dv001–dv007 |
| `d4XX` | **Dashboard** (готовый дашборд — *потребитель* слоя) | `dashboards/d4XX_*` | d400–d405 |
| `task0XX` | **Запланированная задача** (поллинг/импорт) | `system/tasks/managers/task0XX_*` | task001+ |

Примеры: a013 = YM order, a015 = WB orders, a034 = YM realization; p904 = sales_data, p907 = YM payment report; u503 = import from Yandex; ds01–ds03 = базовые схемы для «Конструктора запросов» (ds01→p903, ds02→p900, ds03→p904). d400–d405 = готовые дашборды (d400 сводка за месяц, d401 WB Finance, d402/d403 история заказов WB/YM, d404 отчёт по рекламе WB, d405 метаданные) — это **потребители** слоя, НЕ схемы (прежняя пометка «ds01/ds02 доступны как d401/d402» была неверной; коллизия двух d401 устранена — метаданные перенесены на d405).

## Слой данных: три роли источников (см. `memory-bank/decisions/ADR-0010-data-source-roles.md`)

Доступ к аналитическим данным — три независимых движка с разными ролями (выбирай по дереву):
- **DataView `dvXX`** (`data_view/`) — курируемые «виртуальные таблицы»: составные метрики, **2 периода**, кэш. Для благословлённых показателей и BI (a024/a025). Сложные метрики (revenue = customer_in+customer_out, GL turnover CASE) живут здесь.
- **Базовая схема `dsXX`** (`data_schemes/` + движок `shared/universal_dashboard/`; UI: «Схемы таблиц») — декларативное описание таблицы; гибкий ad-hoc (группировки/фильтры/агрегаты) через `QueryBuilder`. Governance по построению (поля — allowlist).
- **Сырой SQL** (`execute_query`) — нестандартные/разовые случаи; укреплённый escape-hatch.

Перекрытие источников по одной таблице допустимо только при разных ролях (напр. `p904`: ds03 — гибкий, dv001 — курируемый 2-периодный). UI-инструменты слоя собраны в sidebar-группе «Источники данных».

**Термины код ↔ UI** (код-идентификаторы не меняются, в интерфейсе свои подписи): `dsXX` → «Схемы таблиц» (каталог) + «Конструктор запросов» (построитель); `dvXX` → «DataView»; сайдбар-группа `semantic_layer` → «Источники данных».

---

## Внутренняя структура агрегата `a0XX`

**Backend** (`crates/backend/src/domain/a0XX_*/`):
- `mod.rs` — сборка модуля
- `repository.rs` — доступ к БД (SeaORM)
- `service.rs` — бизнес-логика, в т.ч. `insert_test_data`
- `posting.rs` — проведение в Главную книгу / проекции (есть не у всех)
- `representation.rs` — представление агрегата (title+date+doc_id) для drilldown
- `change_token.rs` — инвалидация кэша/реактивность

**Contracts** (`crates/contracts/src/domain/a0XX_*/`):
- `aggregate.rs` — структура агрегата (DTO)
- `metadata.json` + `metadata_gen.rs` — система метаданных полей (генерируемая)

**Frontend** (`crates/frontend/src/domain/a0XX_*/`): UI по MVVM, страницы details со вкладками — напр. `ui/details/tabs/general.rs`.

---

## Карта бэкенда (`crates/backend/src/`)

| Модуль | Назначение |
|---|---|
| `api/routes.rs` | Все HTTP-маршруты (единый файл) |
| `api/handlers/` | Обработчики по объектам |
| `domain/` | Агрегаты a0XX |
| `projections/` | Проекции p9XX + `projections/general_ledger/` |
| `general_ledger/` | Главная книга: `account_registry`, `turnover_registry`, `report_repository`, `drilldown_*`, `account_view/`, `service.rs` |
| `data_schemes/` | dsXX — схемы для universal dashboard |
| `dashboards/` | d4XX |
| `quality/checks/` | Quality-checks (популяция/нарушения/доля) |
| `system/` | `auth`, `access`, `roles`, `users`, `tasks` (планировщик), `audit`, `history`, `favorites`, `settings`, `middleware`, `initialization` |
| `shared/` | `analytics` (account/turnover registry, нормализация, wb_mapping), `indicators` (BI compute), `llm`, `marketplaces`, `representation`, `universal_dashboard`, `drilldown`, `format`, `config` |
| `bi_timeline/` | BI timeline |

---

## Учётные слои (Главная книга)

GL — скелет финансовой модели. Поверх неё концептуальные слои учёта (см. `.claude/memory/`):
- **fact / fina / ybuh** — параллельные слои оборотов для сверки выручки (fina заменяет fact для p903/p907; ybuh — официальные отчёты о реализации, напр. a034).
- `p914` — зеркало GL-проводок слоя fina; `p907` — YM payment report.
- Регистры: `account_registry` (план счетов), `turnover_registry` (обороты).
- После правок в маппинге/проводках нужен **репост документов** через `u508`.

Детали бизнес-правил YM/WB/GL — в `.claude/memory/MEMORY.md` (подгружается автоматически каждую сессию). Не дублируй их здесь.

---

## Конвенции

- Rust 2021, Leptos 0.8, SQLite (SeaORM), фронт — WASM.
- **Миграции БД** — SQL-файлы `migrations/NNNN_имя.sql`, применяются автоматически при старте бэкенда (`shared/data/migration_runner.rs`, трекинг по checksum). Новая миграция = следующий номер.
- Soft delete (`is_deleted`); сложные поля хранятся JSON-ом в БД.
- Фронт: `spawn_local` для async, `RwSignal` для состояния; per-page CSS в `static/pages/<page>.css` под корневым классом страницы (см. memory `per-page-css-convention`).
- Боевая БД и knowledge — вне репозитория: `E:/dev/rust/2/data/` (пути в `config.toml`).

---

## Где искать глубже

- `.claude/memory/MEMORY.md` — бизнес-факты YM/WB/GL, текущие планы (свежее всего).
- `memory-bank/` — ADR (`decisions/`), уроки (`lessons/`), runbook'и (`runbooks/`), known-issues, debrief'ы. Контент перекошен во фронтенд-миграции конца 2025 — сверяйся с кодом.
- `docs/` — актуальные гайды по фичам (завершённые/устаревшие вынесены в `docs/_archive/`).
- `general_ledger/llm.md` — заметки по GL для LLM.
