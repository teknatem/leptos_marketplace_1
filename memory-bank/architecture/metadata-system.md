# Field Metadata System

_Версия: 1.0 | Дата: 2025-12-26_

## Обзор

Система метаданных полей (Field Metadata System) предоставляет единый источник истины для описания структуры агрегатов, их полей и связей. Метаданные определяются декларативно в JSON-файлах и автоматически преобразуются в статические Rust структуры во время компиляции.

## Цели системы

1. **Single Source of Truth** — JSON-файлы как единственный источник информации о структуре
2. **Type Safety** — Генерация статических Rust типов с `'static` lifetime
3. **AI Context** — Предоставление контекста для встроенного LLM чата
4. **UI Generation** — Информация для автогенерации форм и таблиц
5. **Internationalization** — Поддержка русского и английского языков

## Архитектура

```
┌─────────────────────────────────────────────────────────────────┐
│                    metadata.json (источник)                     │
│  - entity info (name, type, table)                              │
│  - UI metadata (labels, visibility)                             │
│  - AI context (description, questions)                          │
│  - field definitions (type, validation)                         │
└──────────────────────────┬──────────────────────────────────────┘
                           │ build.rs
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│              metadata_gen.rs (автогенерируется)                 │
│  - ENTITY_METADATA: &'static EntityMetadataInfo                 │
│  - FIELDS: &'static [FieldMetadata]                             │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                  AggregateRoot trait                            │
│  - entity_metadata_info() -> &'static EntityMetadataInfo        │
│  - field_metadata() -> &'static [FieldMetadata]                 │
└─────────────────────────────────────────────────────────────────┘
```

## Расположение файлов

```
crates/contracts/
├── build.rs                           # Генератор metadata_gen.rs
├── schemas/
│   └── metadata.schema.json           # JSON Schema для валидации
└── src/
    ├── shared/
    │   └── metadata/
    │       ├── mod.rs                 # Экспорты
    │       ├── types.rs               # EntityMetadataInfo, FieldMetadata
    │       ├── field_type.rs          # FieldType enum
    │       └── validation.rs          # ValidationRules
    └── domain/
        └── a001_connection_1c/
            ├── mod.rs                 # включает metadata_gen
            ├── aggregate.rs           # реализует AggregateRoot
            ├── metadata.json          # ИСХОДНЫЕ ДАННЫЕ
            └── metadata_gen.rs        # АВТОГЕНЕРИРУЕТСЯ
```

## Структура metadata.json

```json
{
  "$schema": "../../schemas/metadata.schema.json",
  "schema_version": "1.0",
  "entity": {
    "type": "aggregate",
    "name": "Connection1CDatabase",
    "index": "a001",
    "collection_name": "connections_1c",
    "table_name": "a001_connection_1c_database",
    "ui": {
      "element_name": "Подключение 1С",
      "element_name_en": "1C Connection",
      "list_name": "Подключения 1С",
      "list_name_en": "1C Connections",
      "icon": "database"
    },
    "ai": {
      "description": "Описание для LLM",
      "questions": ["Какие подключения настроены?"],
      "related": ["a002_organization"]
    }
  },
  "fields": [
    {
      "name": "server_url",
      "rust_type": "String",
      "field_type": "primitive",
      "ui": {
        "label": "URL сервера",
        "label_en": "Server URL",
        "visible_in_list": true,
        "visible_in_form": true
      },
      "validation": {
        "required": true,
        "pattern": "^https?://"
      },
      "ai_hint": "OData endpoint URL"
    }
  ]
}
```

## Rust Types

### EntityMetadataInfo

```rust
#[derive(Debug, Clone, Copy)]
pub struct EntityMetadataInfo {
    pub schema_version: &'static str,
    pub entity_type: EntityType,         // Aggregate, UseCase, Projection
    pub entity_name: &'static str,       // "Connection1CDatabase"
    pub entity_index: &'static str,      // "a001"
    pub collection_name: &'static str,   // "connections_1c"
    pub table_name: Option<&'static str>,
    pub ui: EntityUiMetadata,
    pub ai: EntityAiMetadata,
}
```

### EntityUiMetadata

```rust
#[derive(Debug, Clone, Copy)]
pub struct EntityUiMetadata {
    pub element_name: &'static str,      // "Подключение 1С"
    pub element_name_en: Option<&'static str>,
    pub list_name: &'static str,         // "Подключения 1С"  
    pub list_name_en: Option<&'static str>,
    pub icon: Option<&'static str>,
}
```

### EntityAiMetadata

```rust
#[derive(Debug, Clone, Copy)]
pub struct EntityAiMetadata {
    pub description: &'static str,           // Описание для LLM
    pub questions: &'static [&'static str],  // Типичные вопросы
    pub related: &'static [&'static str],    // Связанные сущности
}
```

### FieldMetadata

```rust
#[derive(Debug, Clone, Copy)]
pub struct FieldMetadata {
    pub name: &'static str,
    pub rust_type: &'static str,
    pub field_type: FieldType,
    pub source: FieldSource,                   // Specific, Base, Metadata
    pub ui: FieldUiMetadata,
    pub validation: ValidationRules,
    pub ai_hint: Option<&'static str>,
    
    // Для вложенных типов
    pub nested_fields: Option<&'static [FieldMetadata]>,
    pub ref_aggregate: Option<&'static str>,
    pub enum_values: Option<&'static [&'static str]>,
}
```

### FieldType

```rust
pub enum FieldType {
    Primitive,      // String, i32, bool, DateTime, etc.
    Enum,           // Rust enum (указать enum_values)
    AggregateRef,   // Ссылка на другой aggregate (указать ref_aggregate)
    NestedStruct,   // Вложенная структура (указать nested_fields)
    NestedTable,    // Табличная часть (массив вложенных структур)
}
```

### ValidationRules

```rust
#[derive(Default)]
pub struct ValidationRules {
    pub required: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<&'static str>,
    pub custom_error: Option<&'static str>,
}
```

## Использование в коде

### AggregateRoot trait

```rust
pub trait AggregateRoot {
    // ... существующие методы ...

    /// Получить метаданные сущности
    fn entity_metadata_info() -> &'static EntityMetadataInfo;

    /// Получить метаданные полей
    fn field_metadata() -> &'static [FieldMetadata];
}
```

### Реализация в aggregate

```rust
impl AggregateRoot for Connection1CDatabase {
    fn entity_metadata_info() -> &'static EntityMetadataInfo {
        &super::ENTITY_METADATA
    }

    fn field_metadata() -> &'static [FieldMetadata] {
        super::FIELDS
    }
}
```

### Пример доступа к метаданным

```rust
use contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase;
use contracts::domain::common::AggregateRoot;

// Метаданные сущности
let meta = Connection1CDatabase::entity_metadata_info();
println!("Entity: {}", meta.ui.element_name);
println!("AI: {}", meta.ai.description);

// Итерация по полям
for field in Connection1CDatabase::field_metadata() {
    if field.ui.visible_in_list {
        println!("{}: {}", field.name, field.ui.label);
    }
}
```

## Build Process

### build.rs

Скрипт `crates/contracts/build.rs`:

1. Сканирует `src/domain/*/metadata.json`
2. Парсит JSON в промежуточные структуры
3. Генерирует `metadata_gen.rs` рядом с `metadata.json`
4. Использует `'static` lifetimes для всех строк (compile-time literals)

```rust
// Пример сгенерированного кода
pub static ENTITY_METADATA: EntityMetadataInfo = EntityMetadataInfo {
    schema_version: "1.0",
    entity_type: EntityType::Aggregate,
    entity_name: "Connection1CDatabase",
    // ...
};

pub static FIELDS: &[FieldMetadata] = &[
    FieldMetadata {
        name: "server_url",
        rust_type: "String",
        // ...
    },
];
```

## JSON Schema

Файл `schemas/metadata.schema.json` обеспечивает:

- **Валидацию** — проверка структуры при сохранении
- **Автодополнение** — подсказки в IDE (VS Code, IntelliJ)
- **Документацию** — описания полей при наведении

Подключается через `"$schema"` в начале JSON файла.

## AI/LLM Integration

Система спроектирована для поддержки встроенного LLM чата:

### Entity-level context

```json
"ai": {
  "description": "Хранит настройки подключения к базе 1С:Управление торговлей 11",
  "questions": [
    "Какие подключения к 1С настроены?",
    "Как добавить новое подключение?"
  ],
  "related": ["a002_organization", "u501_import_from_ut"]
}
```

### Field-level hints

```json
{
  "name": "server_url",
  "ai_hint": "OData endpoint URL вида http://server/base/odata/standard.odata"
}
```

LLM может использовать эти данные для:

- Понимания структуры приложения
- Ответов на вопросы пользователя о данных
- Навигации между связанными сущностями
- Формирования контекстных подсказок

## Статус реализации

| Компонент | Статус |
|-----------|--------|
| Rust types (`shared/metadata/`) | ✅ Реализовано |
| JSON Schema | ✅ Реализовано |
| build.rs генератор | ✅ Реализовано |
| AggregateRoot trait extension | ✅ Реализовано |
| a001_connection_1c (POC) | ✅ Реализовано |
| Остальные aggregates | 📋 Планируется |
| Frontend integration | 📋 Планируется |
| LLM chat integration | 📋 Планируется |

## Добавление метаданных для нового aggregate

1. Создать `metadata.json` в папке aggregate:
   ```
   crates/contracts/src/domain/a00X_new_entity/metadata.json
   ```

2. Добавить `$schema` reference в начало файла

3. Заполнить entity и fields по образцу

4. Добавить в `mod.rs`:
   ```rust
   mod metadata_gen;
   pub use metadata_gen::{ENTITY_METADATA, FIELDS};
   ```

5. Реализовать методы в `aggregate.rs`:
   ```rust
   impl AggregateRoot for NewEntity {
       fn entity_metadata_info() -> &'static EntityMetadataInfo {
           &super::ENTITY_METADATA
       }
       fn field_metadata() -> &'static [FieldMetadata] {
           super::FIELDS
       }
   }
   ```

6. Запустить `cargo build` — `metadata_gen.rs` сгенерируется автоматически

## Связанные документы

- `memory-bank/todo/field-metadata-system.md` — Исходный план (помечен как реализованный)
- `memory-bank/architecture/aggregate-standard.md` — Стандарт структуры aggregate
- `memory-bank/architecture/domain-layer-architecture.md` — Архитектура domain layer

