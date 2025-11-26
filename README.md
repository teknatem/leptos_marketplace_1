# Leptos Marketplace

Полнофункциональная десктопная система управления маркетплейсами с интеграцией 1С:Управление торговлей 11, Wildberries и Ozon.

## 🚀 Quick Start

### Требования

- **Rust** (stable, edition 2021)
- **Trunk** (`cargo install trunk`)
- **SQLite** (для прямого доступа к БД)
- **Node.js + pnpm** (для некоторых dev tools)

### Запуск для разработки

Откройте два терминала:

**Терминал 1 - Backend:**
```powershell
cargo run --bin backend
```
Backend запустится на `http://localhost:3000`

**Терминал 2 - Frontend:**
```powershell
trunk serve --port 8080
```
Frontend будет доступен на `http://localhost:8080`

### Production Build

```powershell
# Build frontend
trunk build --release

# Build backend
cargo build --release --bin backend

# Результат: dist/ (frontend) + target/release/backend.exe
```

## 📚 Документация

### Для AI-ассистентов

- **`.cursorrules`** - Быстрый справочник по проекту
- **`memory-bank/`** - Полная база знаний для AI

### Архитектура

- **`memory-bank/projectbrief.md`** - Общее описание проекта
- **`memory-bank/systemPatterns.md`** - Архитектурные паттерны
- **`memory-bank/architecture/`** - Детальная документация архитектуры
  - `domain-layer-architecture.md` - Domain layer rules
  - `naming-conventions.md` - Система индексированного именования
  - `project-structure.md` - Структура workspace

### Разработка

- **`memory-bank/techContext.md`** - Технологический стек и setup
- **`memory-bank/code-standards/`** - Стандарты кодирования
  - `code-quality-rules.md` - Правила качества кода
  - `dev-commands.md` - Build команды

### Фичи

- **`memory-bank/features/`** - Документация по конкретным фичам
  - `usecase-u501-import-from-ut.md` - Импорт из 1С
  - `README_u501.md` - Quick start по u501
  - `aggregate_picker_implementation.md` - Picker компоненты

### Прогресс

- **`memory-bank/progress.md`** - Что реализовано, что в планах
- **`memory-bank/activeContext.md`** - Текущий фокус разработки

## 🏗️ Архитектура

### Структура workspace

```
leptos_marketplace_1/
├── crates/
│   ├── contracts/    # Shared DTOs & types
│   ├── backend/      # Axum server
│   └── frontend/     # Leptos WASM app
├── memory-bank/      # Documentation
├── marketplace.db    # SQLite database
└── dist/            # Frontend build output
```

### Принципы

- **DDD** (Domain-Driven Design)
- **VSA** (Vertical Slice Architecture)
- **Indexed naming**: a001-a499 (aggregates), u501-u999 (usecases), p901-p999 (projections)
- **Shared contracts**: Type safety между frontend и backend

## 🔑 Основные фичи

### Aggregates (Domain entities)
- **a001**: Подключения к 1С
- **a002**: Организации
- **a004**: Номенклатура
- **a005**: Подключения Wildberries
- **a006**: Подключения Ozon
- **a014**: Транзакции Ozon
- **a015**: Заказы Wildberries

### UseCases (Operations)
- **u501**: Импорт из 1С:УТ11
- **u504**: Интеграция Wildberries
- **u505**: Интеграция Ozon
- **u506**: Интеграция LemanaPro

### Projections (Analytics)
- **p902**: Регистр продаж
- **p904**: Аналитика продаж
- **p905**: История комиссий WB

## 🛠️ Development

### Команды

```powershell
# Проверка кода
cargo check

# Форматирование
cargo fmt

# Linting
cargo clippy

# Тесты
cargo test
```

### База данных

- **File**: `marketplace.db` (SQLite)
- **Migrations**: `migrate_*.sql` файлы
- **Tools**: sqlite3 CLI, DB Browser for SQLite

### Применение миграции

```powershell
sqlite3 marketplace.db < migrate_xxx.sql
```

## 📖 Дополнительная информация

### Полезные ссылки

- [Leptos Book](https://book.leptos.dev/)
- [Axum Documentation](https://docs.rs/axum/)
- [Rust Book](https://doc.rust-lang.org/)

### Внутренняя документация

- `docs/` - Дополнительные гайды
- `memory-bank/todo/` - Планируемые фичи
- `.cursorrules` - Project intelligence для AI

## 📝 License

Proprietary. All rights reserved.

---

**Для получения полной информации о проекте, архитектуре и паттернах, см. `memory-bank/` папку.**

