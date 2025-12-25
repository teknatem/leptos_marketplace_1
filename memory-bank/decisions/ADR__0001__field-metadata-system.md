---
type: adr
number: "0001"
title: Field Metadata System
status: accepted
date: 2025-12-26
tags: [metadata, architecture, codegen]
---

# ADR-0001: Field Metadata System

## Status

**Accepted** — реализовано как POC на a001_connection_1c

## Context

Необходимо декларативно описывать структуру агрегатов для:
- Автогенерации UI (формы, таблицы)
- Предоставления контекста для LLM чата
- Валидации данных
- Документации и интернационализации

## Decision

Использовать систему на основе JSON метаданных с генерацией Rust кода:

```
metadata.json → build.rs → metadata_gen.rs
```

### Ключевые решения:

1. **JSON как Single Source of Truth**
   - Файлы `metadata.json` редактируются вручную
   - JSON Schema для валидации и автодополнения IDE

2. **`'static` lifetimes для строк**
   - Все строки — compile-time literals
   - Позволяет использовать `Copy` trait
   - Нет runtime allocations для метаданных

3. **Расположение в contracts crate**
   - Метаданные доступны и frontend, и backend
   - Генерация через `build.rs` в contracts

4. **AggregateRoot trait extension**
   - Методы `entity_metadata_info()` и `field_metadata()`
   - Возвращают `&'static` references

## Alternatives Considered

### 1. Derive макросы

```rust
#[derive(Metadata)]
#[metadata(label = "Подключение 1С")]
struct Connection1C { ... }
```

**Отклонено**: 
- Сложнее парсить внешними инструментами
- AI контекст неудобно описывать в атрибутах

### 2. Runtime JSON парсинг

```rust
fn metadata() -> serde_json::Value {
    serde_json::from_str(include_str!("metadata.json"))
}
```

**Отклонено**:
- Runtime overhead
- Нет type safety

### 3. YAML вместо JSON

**Отклонено**:
- Меньше поддержки в IDE (JSON Schema)
- JSON достаточно читаем

## Consequences

**Positive:**
- Единственный источник истины (JSON)
- Полная type safety (Rust генерация)
- Zero-cost abstractions ('static)
- Доступно внешним инструментам и LLM

**Negative:**
- Дополнительный файл на каждый aggregate
- Необходимость синхронизации JSON и Rust структуры
- Build time слегка увеличивается

## References

- `memory-bank/architecture/metadata-system.md` — полная документация
- `crates/contracts/build.rs` — генератор
- `crates/contracts/schemas/metadata.schema.json` — JSON Schema

