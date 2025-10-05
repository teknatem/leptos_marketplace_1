# Aggregate Standard & Validation Rules

## 🎯 Цель

Строгий стандарт структуры агрегатов для:

1. ✅ Автоматического сканирования Project Explorer
2. ✅ Выявления нарушений границ и архитектурных правил
3. ✅ Обеспечения единообразия в проекте с 100+ агрегатами
4. ✅ Упрощения навигации и поддержки

---

## 📁 1. СТАНДАРТ СТРУКТУРЫ ФАЙЛОВ

### 1.1 Именование агрегатов

**Формат:** `a{NNN}_{snake_case_name}`

- `a` - префикс (aggregate)
- `{NNN}` - трёхзначный номер (001-999)
- `_` - разделитель
- `{name}` - имя в snake_case

**Примеры:**

```
✅ a001_connection_1c
✅ a002_user_profile
✅ a050_invoice_payment
✅ a100_product_catalog

❌ connection_1c          (нет префикса)
❌ a1_user                (не трёхзначный)
❌ a001-connection        (не snake_case)
❌ a001_ConnectionProfile (не snake_case)
```

### 1.2 Обязательная структура агрегата

Каждый агрегат **ДОЛЖЕН** иметь следующую структуру:

```
{aggregate_id}_{aggregate_name}/
├── _aggregate.toml              # Метаданные агрегата (обязательно!)
├── mod.rs                        # Корневой модуль с re-exports
│
├── api/src/domain/{aggregate_id}_{aggregate_name}/
│   ├── mod.rs                    # API layer root
│   ├── aggregate.rs              # Domain entities & value objects
│   ├── commands.rs               # (optional) Commands
│   ├── events.rs                 # (optional) Domain events
│   └── errors.rs                 # (optional) Domain errors
│
├── server/src/domain/{aggregate_id}_{aggregate_name}/
│   ├── mod.rs                    # Server layer root
│   ├── repository.rs             # Database access (sea-orm entities)
│   ├── handlers.rs               # (optional) Business logic handlers
│   └── migrations.rs             # (optional) DB migrations
│
└── app/src/domain/{aggregate_id}_{aggregate_name}/
    ├── mod.rs                    # App layer root
    ├── ui/
    │   ├── mod.rs
    │   ├── list.rs               # List view
    │   └── details/              # Details views
    │       ├── mod.rs
    │       ├── form.rs
    │       └── view.rs
    └── state.rs                  # (optional) Local state management
```

---

## 📋 2. МЕТАДАННЫЕ АГРЕГАТА (\_aggregate.toml)

Каждый агрегат **ОБЯЗАН** иметь файл `_aggregate.toml` с метаданными.

### 2.1 Расположение

```
api/src/domain/{aggregate_id}_{aggregate_name}/_aggregate.toml
```

### 2.2 Формат файла

```toml
# Aggregate Metadata
[aggregate]
id = "a001"                              # Уникальный ID (строго совпадает с префиксом)
name = "connection_1c"                    # Имя (строго совпадает с суффиксом)
display_name = "1C Database Connection"   # Человекочитаемое название
version = "1.0.0"                         # Версия агрегата
category = "integration"                  # Категория
status = "production"                     # Статус: draft | development | production | deprecated

[metadata]
description = """
Manages connections to 1C:Enterprise databases via OData protocol.
Supports multiple database configurations with primary/secondary selection.
"""
author = "Team Name"
created_at = "2025-01-15"
updated_at = "2025-02-01"

[layers]
api = true                                # Присутствует API layer
server = true                             # Присутствует Server layer
app = true                                # Присутствует App layer

[database]
tables = [                                # Список таблиц БД
    "a001_connection_1c_database",
    "a001_connection_1c_events"
]
prefix = "a001_connection_1c_"            # Префикс таблиц

[domain]
# Основные типы домена (для валидации)
aggregates = [
    "Connection1CDatabase",
]
value_objects = [
    "Connection1CDatabaseId",
]
forms = [
    "Connection1CDatabaseDto",
]

[dependencies]
# Разрешённые зависимости от других агрегатов (пусто = изолирован)
aggregates = []                           # Список ID других агрегатов
# Пример: aggregates = ["a002", "a005"]

[validation]
enforce_isolation = true                  # Запретить импорты из других агрегатов
require_all_layers = true                 # Требовать наличия всех слоёв
check_table_prefix = true                 # Проверять префиксы таблиц
check_naming_convention = true            # Проверять naming conventions

[ui]
has_list_view = true                      # Есть list.rs
has_details_view = true                   # Есть details/
has_form = true                           # Есть form.rs
```

### 2.3 Категории агрегатов

Стандартные категории (можно расширять):

```toml
category = "core"           # Базовые сущности (User, Settings)
category = "integration"    # Интеграции (1C, External APIs)
category = "payment"        # Платежи и финансы
category = "catalog"        # Каталоги и справочники
category = "order"          # Заказы и продажи
category = "report"         # Отчёты и аналитика
category = "notification"   # Уведомления
category = "security"       # Безопасность и права
```

---

## 🔒 3. ПРАВИЛА ИЗОЛЯЦИИ АГРЕГАТОВ

### 3.1 Запрещённые зависимости

**❌ ЗАПРЕЩЕНО** импортировать код из других агрегатов:

```rust
// ❌ ПЛОХО - Прямой импорт из другого агрегата
use crate::domain::a002_user_profile::aggregate::UserProfile;

// ❌ ПЛОХО - Импорт через server layer
use server::domain::a003_invoice::repository::InvoiceRepository;
```

**✅ РАЗРЕШЕНО** только:

```rust
// ✅ ХОРОШО - Импорт из shared/common модулей
use crate::shared::data::db::get_connection;

// ✅ ХОРОШО - Импорт базовых типов
use crate::domain::common::BaseAggregate;

// ✅ ХОРОШО - Внешние crate
use sea_orm::EntityTrait;
```

### 3.2 Разрешённые зависимости

Если агрегат **ДОЛЖЕН** зависеть от другого, это **ОБЯЗАТЕЛЬНО** декларируется:

```toml
[dependencies]
aggregates = ["a002"]  # Разрешена зависимость от a002_user_profile
reason = "Invoice requires user ownership validation"
```

Тогда в коде:

```rust
// ✅ ХОРОШО - Задекларированная зависимость
use crate::domain::a002_user_profile::aggregate::UserId;
```

### 3.3 Общий код (Shared)

Для общей функциональности используем:

```
api/src/domain/
├── _common/                # Базовые типы и traits для всех агрегатов
│   ├── mod.rs
│   ├── aggregate_root.rs   # Trait AggregateRoot
│   ├── base_types.rs       # BaseAggregate, EntityMetadata
│   ├── events.rs           # EventStore
│   └── errors.rs           # Общие ошибки
│
└── {aggregates}/           # Конкретные агрегаты
```

---

## 🗄️ 4. СТАНДАРТ БАЗЫ ДАННЫХ

### 4.1 Именование таблиц

**Формат:** `{aggregate_id}_{aggregate_name}_{entity}`

```sql
✅ a001_connection_1c_database
✅ a001_connection_1c_events
✅ a002_user_profile_users
✅ a002_user_profile_sessions

❌ connection_1c_database          (нет префикса)
❌ a001_database                   (нет полного имени агрегата)
❌ users                           (нет префикса вообще)
```

### 4.2 Обязательные поля

Каждая таблица агрегата **ДОЛЖНА** содержать:

```sql
CREATE TABLE a001_connection_1c_database (
    -- Primary Key
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Business fields
    -- ... (специфичные для агрегата)

    -- Audit fields (ОБЯЗАТЕЛЬНО!)
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,              -- ISO 8601 format
    updated_at TEXT NOT NULL,              -- ISO 8601 format
    version INTEGER NOT NULL DEFAULT 1     -- Optimistic locking
);
```

### 4.3 Индексы

Обязательный индекс для soft delete:

```sql
CREATE INDEX IF NOT EXISTS idx_a001_connection_1c_database_deleted
ON a001_connection_1c_database(is_deleted);
```

---

## 📝 5. СТАНДАРТ КОДА

### 5.1 mod.rs структура

**Каждый** `mod.rs` должен следовать шаблону:

```rust
// api/src/domain/a001_connection_1c/mod.rs

//! # a001_connection_1c - 1C Database Connection
//!
//! **Category:** Integration
//! **Status:** Production
//! **Version:** 1.0.0
//!
//! Manages connections to 1C:Enterprise databases via OData protocol.

// Re-exports
pub mod aggregate;

// Optional modules
#[cfg(feature = "commands")]
pub mod commands;

#[cfg(feature = "events")]
pub mod events;

/// Aggregate metadata
pub mod meta {
    pub const ID: &str = "a001";
    pub const NAME: &str = "connection_1c";
    pub const FULL_NAME: &str = "a001_connection_1c";
    pub const CATEGORY: &str = "integration";
    pub const VERSION: &str = "1.0.0";
}

// Re-export main types
pub use aggregate::{
    Connection1CDatabase,
    Connection1CDatabaseId,
    Connection1CDatabaseDto,
};
```

### 5.2 aggregate.rs структура

```rust
// api/src/domain/a001_connection_1c/aggregate.rs

use serde::{Deserialize, Serialize};
use crate::domain::_common::{AggregateRoot, BaseAggregate, EntityMetadata};

// ============================================================================
// ID Types
// ============================================================================

/// Unique identifier for Connection1CDatabase aggregate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Connection1CDatabaseId(pub i32);

impl Connection1CDatabaseId {
    pub fn new(value: i32) -> Self {
        Self(value)
    }

    pub fn value(&self) -> i32 {
        self.0
    }
}

// ============================================================================
// Aggregate Root
// ============================================================================

/// Connection to 1C:Enterprise Database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection1CDatabase {
    #[serde(flatten)]
    pub base: BaseAggregate<Connection1CDatabaseId>,

    // Business fields
    pub description: String,
    pub url: String,
    pub comment: Option<String>,
    pub login: String,
    pub password: String,

    #[serde(rename = "isPrimary", default)]
    pub is_primary: bool,
}

impl AggregateRoot for Connection1CDatabase {
    type Id = Connection1CDatabaseId;

    fn id(&self) -> Self::Id {
        self.base.id
    }

    fn metadata(&self) -> &EntityMetadata {
        &self.base.metadata
    }

    fn aggregate_type() -> &'static str {
        "Connection1CDatabase"
    }

    fn aggregate_id() -> &'static str {
        super::meta::ID
    }
}

// ============================================================================
// Forms / DTOs
// ============================================================================

/// Form for creating/updating Connection1CDatabase
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Connection1CDatabaseDto {
    pub id: Option<String>,
    pub description: String,
    pub url: String,
    pub comment: Option<String>,
    pub login: String,
    pub password: String,
    #[serde(rename = "isPrimary", default)]
    pub is_primary: bool,
}
```

### 5.3 repository.rs структура (server)

```rust
// server/src/domain/a001_connection_1c/repository.rs

use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use crate::shared::data::db::get_connection;

use api::domain::a001_connection_1c::aggregate::{
    Connection1CDatabase,
    Connection1CDatabaseId,
    Connection1CDatabaseDto,
};

// ============================================================================
// SeaORM Entity
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "a001_connection_1c_database")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub description: String,
    pub url: String,
    pub comment: Option<String>,
    pub login: String,
    pub password: String,
    pub is_primary: bool,

    // Audit fields
    pub is_deleted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ============================================================================
// Mapper: Model -> Aggregate
// ============================================================================

impl From<Model> for Connection1CDatabase {
    fn from(m: Model) -> Self {
        // Implementation...
    }
}

// ============================================================================
// Repository Functions
// ============================================================================

pub async fn list_all() -> anyhow::Result<Vec<Connection1CDatabase>> {
    // Implementation...
}

pub async fn get_by_id(id: i32) -> anyhow::Result<Option<Connection1CDatabase>> {
    // Implementation...
}

pub async fn upsert(dto: Connection1CDatabaseDto) -> anyhow::Result<i32> {
    // Implementation...
}

pub async fn soft_delete(id: i32) -> anyhow::Result<bool> {
    // Implementation...
}
```

---

## ✅ 6. ПРАВИЛА ВАЛИДАЦИИ

### 6.1 Структурная валидация

Validator должен проверять:

**V-001: Именование агрегата**

```
✅ Формат: a{NNN}_{snake_case}
✅ Уникальность ID в проекте
✅ Совпадение префикса во всех трёх слоях
```

**V-002: Наличие обязательных файлов**

```
✅ _aggregate.toml существует
✅ mod.rs в каждом слое
✅ aggregate.rs в api слое
✅ repository.rs в server слое (если server = true)
```

**V-003: Метаданные (\_aggregate.toml)**

```
✅ Все обязательные поля заполнены
✅ id совпадает с префиксом папки
✅ name совпадает с суффиксом папки
✅ version валидный semver
✅ category из допустимого списка
```

**V-004: Префиксы таблиц БД**

```
✅ Все таблицы начинаются с {id}_{name}_
✅ Таблицы совпадают со списком в _aggregate.toml
✅ Таблицы содержат обязательные audit поля
```

### 6.2 Изоляция агрегатов

**V-005: Запрет межагрегатных импортов**

Парсим все `.rs` файлы агрегата и проверяем:

```rust
// ❌ Нарушение изоляции
use crate::domain::a002_user_profile::...;
use crate::domain::a999_*::...;

// ✅ Разрешено
use crate::domain::_common::...;
use crate::shared::...;
```

**Исключения:**

- Если в `_aggregate.toml` → `dependencies.aggregates` содержит ID

**V-006: Запрет SQL-связей между агрегатами**

```sql
-- ❌ Нарушение: FOREIGN KEY на другой агрегат
CREATE TABLE a001_connection_1c_database (
    user_id INTEGER REFERENCES a002_user_profile_users(id)  -- ЗАПРЕЩЕНО!
);

-- ✅ Разрешено: хранить ID как значение
CREATE TABLE a001_connection_1c_database (
    owner_user_id INTEGER  -- OK, просто значение без FK
);
```

### 6.3 Naming Conventions

**V-007: Именование типов**

```rust
// Aggregate ID: {AggregateName}Id
✅ Connection1CDatabaseId
❌ Connection1CId, DatabaseId

// Aggregate: {AggregateName}
✅ Connection1CDatabase
❌ Connection1CDb, C1CDatabase

// Form: {AggregateName}Form
✅ Connection1CDatabaseDto
❌ Connection1CDbForm, CreateConnection1C
```

**V-008: Именование функций repository**

Стандартный набор:

```rust
✅ list_all() -> Vec<Aggregate>
✅ get_by_id(id) -> Option<Aggregate>
✅ upsert(form) -> Result<id>
✅ soft_delete(id) -> Result<bool>
```

### 6.4 Архитектурные правила

**V-009: Направление зависимостей**

```
app  ──depends on──>  api
server ──depends on──>  api
app  ──NO dependency──>  server
```

Проверка через `Cargo.toml` каждого crate.

**V-010: Sliced vertical boundaries**

```
api layer:     Только domain logic, NO database, NO UI
server layer:  Только persistence, NO business logic
app layer:     Только UI, NO business logic, NO database
```

---

## 🛠️ 7. ИНСТРУМЕНТЫ ВАЛИДАЦИИ

### 7.1 Cargo-based validator

Создать `tools/aggregate-validator`:

```rust
// Pseudo-code
fn validate_project() {
    let aggregates = scan_aggregates(".");

    for agg in aggregates {
        // Structural validation
        check_naming(&agg)?;
        check_files_exist(&agg)?;
        check_metadata(&agg)?;

        // Isolation validation
        check_no_cross_aggregate_imports(&agg)?;
        check_no_foreign_keys(&agg)?;

        // Naming conventions
        check_type_naming(&agg)?;
        check_table_naming(&agg)?;

        // Architecture
        check_layer_dependencies(&agg)?;
    }
}
```

### 7.2 CI/CD Integration

```yaml
# .github/workflows/validate.yml
name: Validate Aggregates

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run Aggregate Validator
        run: cargo run --bin aggregate-validator
```

### 7.3 Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

cargo run --bin aggregate-validator --quiet
if [ $? -ne 0 ]; then
    echo "❌ Aggregate validation failed!"
    exit 1
fi
```

---

## 📊 8. ОТЧЁТ ВАЛИДАЦИИ

Validator должен выводить структурированный отчёт:

```
🔍 VSA Aggregate Validator v1.0.0
📁 Project: leptos_marketplace_1
⏱️  Scan time: 1.23s

📊 SUMMARY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total Aggregates:   42
✅ Valid:            40
⚠️  Warnings:        1
❌ Errors:           1

📦 AGGREGATE STATUS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ a001_connection_1c         [3/3 layers, 2 tables, 0 issues]
✅ a002_user_profile          [2/3 layers, 1 table,  0 issues]
⚠️  a003_invoice               [3/3 layers, 3 tables, 1 warning]
❌ a004_payment               [2/3 layers, 2 tables, 2 errors]
✅ a005_product_catalog       [3/3 layers, 4 tables, 0 issues]
... (37 more)

⚠️  WARNINGS (1)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
[W-001] a003_invoice
  ├─ Missing UI: app/src/domain/a003_invoice/ui/list.rs
  └─ Recommendation: Add list view or set has_list_view = false

❌ ERRORS (2)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
[E-001] a004_payment
  ├─ Isolation Violation: api/src/domain/a004_payment/aggregate.rs:12
  │  use crate::domain::a003_invoice::aggregate::InvoiceId;
  └─ Fix: Declare dependency in _aggregate.toml or remove import

[E-002] a004_payment
  ├─ Table Prefix Violation: payment_transactions
  └─ Fix: Rename to a004_payment_transactions

🎯 NEXT STEPS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1. Fix 2 errors in a004_payment
2. Review 1 warning in a003_invoice
3. Run: cargo run --bin aggregate-validator --fix (auto-fix some issues)

Exit code: 1 (errors found)
```

---

## 🚀 9. ГЕНЕРАТОР АГРЕГАТОВ

Создать CLI tool для генерации нового агрегата по стандарту:

```bash
cargo run --bin aggregate-gen -- \
    --id a042 \
    --name product_review \
    --category catalog \
    --description "Product reviews and ratings"
```

Генерирует:

```
✅ Created: api/src/domain/a042_product_review/
✅ Created: api/src/domain/a042_product_review/_aggregate.toml
✅ Created: api/src/domain/a042_product_review/mod.rs
✅ Created: api/src/domain/a042_product_review/aggregate.rs
✅ Created: server/src/domain/a042_product_review/mod.rs
✅ Created: server/src/domain/a042_product_review/repository.rs
✅ Created: app/src/domain/a042_product_review/mod.rs
✅ Created: app/src/domain/a042_product_review/ui/mod.rs
✅ Created: app/src/domain/a042_product_review/ui/list.rs
✅ Created: app/src/domain/a042_product_review/ui/details/mod.rs
✅ Migration: server/migrations/042_create_a042_product_review_tables.sql

🎉 Aggregate a042_product_review created successfully!

Next steps:
1. Update api/src/domain/mod.rs with: pub mod a042_product_review;
2. Update server/src/domain/mod.rs with: pub mod a042_product_review;
3. Update app/src/domain/mod.rs with: pub mod a042_product_review;
4. Run: cargo run --bin aggregate-validator
```

---

## 📝 10. MIGRATION PLAN

### Как перевести существующий проект на стандарт:

1. **Переименовать агрегаты**

   ```bash
   mv api/src/domain/connection_1c api/src/domain/a001_connection_1c
   mv server/src/domain/connection_1c server/src/domain/a001_connection_1c
   mv app/src/domain/connection_1c app/src/domain/a001_connection_1c
   ```

2. **Создать \_aggregate.toml для каждого**

   ```bash
   cargo run --bin aggregate-gen -- --migrate a001_connection_1c
   ```

3. **Переименовать таблицы БД**

   ```sql
   ALTER TABLE connection_1c_database
   RENAME TO a001_connection_1c_database;
   ```

4. **Обновить импорты**

   ```bash
   # Find-replace во всех файлах
   connection_1c → a001_connection_1c
   ```

5. **Валидировать**
   ```bash
   cargo run --bin aggregate-validator
   ```

---

## 🎯 ИТОГО: ЧТО ПОЛУЧАЕМ

### Для Project Explorer:

✅ **Детектирование:** Паттерн `a\d{3}_\w+` → 100% точность  
✅ **Метаданные:** Всё в `_aggregate.toml` → парсинг за O(1)  
✅ **Структура:** Гарантированные пути → нет проверок на существование  
✅ **БД:** Префиксы таблиц → автоматическое связывание  
✅ **Граф зависимостей:** `dependencies.aggregates` → визуализация

### Для разработчиков:

✅ **Единообразие:** Все агрегаты выглядят одинаково  
✅ **Быстрый старт:** Генератор → готовый scaffold за 5 секунд  
✅ **Ранний feedback:** CI/CD → ошибки выявляются до merge  
✅ **Изоляция:** Невозможно случайно нарушить границы  
✅ **Навигация:** Любой агрегат находится за `Ctrl+P` → `a042`

### Для архитектуры:

✅ **Контроль:** Валидатор следит за соблюдением правил  
✅ **Масштабируемость:** От 1 до 999 агрегатов без изменений  
✅ **Документированность:** `_aggregate.toml` = живая документация  
✅ **Refactoring safety:** Переименование через validator  
✅ **Onboarding:** Новый разработчик понимает структуру за 5 минут
