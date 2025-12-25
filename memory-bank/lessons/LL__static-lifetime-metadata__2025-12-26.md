---
type: lesson-learned
date: 2025-12-26
topic: Static Lifetime for Metadata
tags: [rust, lifetime, optimization]
---

# Lesson Learned: Static Lifetime for Metadata

## Context

При проектировании Field Metadata System возник вопрос о типах строк:
- `String` — owned, heap allocation
- `&'static str` — compile-time literal

## Lesson

**Использовать `'static` lifetime для compile-time известных данных.**

### Преимущества:

1. **Zero runtime allocation**
   ```rust
   // Плохо: runtime allocation
   pub struct Meta { name: String }
   
   // Хорошо: compile-time
   pub struct Meta { name: &'static str }
   ```

2. **Copy trait**
   ```rust
   #[derive(Copy, Clone)]  // Возможно с &'static str
   pub struct FieldMetadata { ... }
   ```

3. **Простая передача**
   ```rust
   fn get_meta() -> &'static EntityMetadataInfo {
       &ENTITY_METADATA  // Нет lifetime headaches
   }
   ```

### Когда применять:

- Конфигурация известна в compile-time
- Данные не меняются в runtime
- Метаданные, схемы, константы

### Когда НЕ применять:

- Пользовательский ввод
- Данные из файлов/сети
- Динамически формируемые строки

## Generated Code Pattern

```rust
// metadata_gen.rs (автогенерация)
pub static ENTITY_METADATA: EntityMetadataInfo = EntityMetadataInfo {
    entity_name: "Connection1CDatabase",  // &'static str
    // ...
};
```

## References

- `memory-bank/decisions/ADR__0001__field-metadata-system.md`
- `memory-bank/architecture/metadata-system.md`

