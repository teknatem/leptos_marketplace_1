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

Проверка перед коммитом:
```powershell
cargo check -p backend
cargo check -p contracts
cargo check -p frontend --target wasm32-unknown-unknown   # frontend — только wasm-таргет
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
| `dsXX` | **Data scheme** (схемы universal dashboard) | `data_schemes/dsXX_*` | ds01–ds03 |
| `d4XX` | **Dashboard** (готовый дашборд) | `dashboards/d4XX_*` | d400 |
| `task0XX` | **Запланированная задача** (поллинг/импорт) | `system/tasks/managers/task0XX_*` | task001+ |

Примеры: a013 = YM order, a015 = WB orders, a034 = YM realization; p904 = sales_data, p907 = YM payment report; u503 = import from Yandex; ds01/ds02 = схемы дашбордов (в API доступны и как d401/d402).

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
- Soft delete (`is_deleted`); сложные поля хранятся JSON-ом в БД.
- Фронт: `spawn_local` для async, `RwSignal` для состояния; per-page CSS в `static/pages/<page>.css` под корневым классом страницы (см. memory `per-page-css-convention`).
- Боевая БД и knowledge — вне репозитория: `E:/dev/rust/2/data/` (пути в `config.toml`).

---

## Где искать глубже

- `.claude/memory/MEMORY.md` — бизнес-факты YM/WB/GL, текущие планы (свежее всего).
- `memory-bank/` — ADR (`decisions/`), уроки (`lessons/`), runbook'и (`runbooks/`), known-issues, debrief'ы. Контент перекошен во фронтенд-миграции конца 2025 — сверяйся с кодом.
- `docs/` — актуальные гайды по фичам (завершённые/устаревшие вынесены в `docs/_archive/`).
- `general_ledger/llm.md` — заметки по GL для LLM.
