---
type: runbook
version: 1
topic: Adding Metadata to an Aggregate
tags: [metadata, aggregate, howto]
date: 2025-12-26
---

# Runbook: Adding Metadata to an Aggregate

## Prerequisites

- Aggregate уже создан в `crates/contracts/src/domain/a00X_*/`
- `build.rs` и shared metadata types существуют

## Steps

### 1. Создать metadata.json

```powershell
# Скопировать шаблон из a001
Copy-Item crates/contracts/src/domain/a001_connection_1c/metadata.json `
          crates/contracts/src/domain/a00X_new_entity/metadata.json
```

### 2. Отредактировать metadata.json

Обязательные секции:

```json
{
  "$schema": "../../schemas/metadata.schema.json",
  "schema_version": "1.0",
  "entity": {
    "type": "aggregate",
    "name": "NewEntityName",
    "index": "a00X",
    "collection_name": "new_entities",
    "table_name": "a00X_new_entity_database",
    "ui": {
      "element_name": "Новая сущность",
      "list_name": "Новые сущности"
    },
    "ai": {
      "description": "Описание для LLM",
      "questions": ["Вопрос 1?"],
      "related": ["a001_connection_1c"]
    }
  },
  "fields": [
    // ... поля
  ]
}
```

### 3. Обновить mod.rs

```rust
// crates/contracts/src/domain/a00X_new_entity/mod.rs
pub mod aggregate;
mod metadata_gen;  // Добавить

pub use metadata_gen::{ENTITY_METADATA, FIELDS};  // Добавить
```

### 4. Реализовать методы AggregateRoot

```rust
// В aggregate.rs
use crate::shared::metadata::{EntityMetadataInfo, FieldMetadata};

impl AggregateRoot for NewEntity {
    // ... существующие методы ...
    
    fn entity_metadata_info() -> &'static EntityMetadataInfo {
        &super::ENTITY_METADATA
    }
    
    fn field_metadata() -> &'static [FieldMetadata] {
        super::FIELDS
    }
}
```

### 5. Запустить сборку

```powershell
cargo build -p contracts
```

`metadata_gen.rs` будет сгенерирован автоматически.

### 6. Проверить

```powershell
cargo check -p contracts
cargo check -p backend
```

## Troubleshooting

### build.rs не находит JSON

- Проверить путь: `src/domain/a00X_*/metadata.json`
- Имя папки должно начинаться с `a`

### Ошибки компиляции metadata_gen.rs

- Проверить JSON синтаксис
- Проверить соответствие `rust_type` и `field_type`

## Verification

```rust
// В тесте или main
use contracts::domain::a00X_new_entity::aggregate::NewEntity;
use contracts::domain::common::AggregateRoot;

let meta = NewEntity::entity_metadata_info();
assert_eq!(meta.entity_index, "a00X");
```

