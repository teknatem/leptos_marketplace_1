---
type: lesson
date: 2026-01-16
tags: [architecture, repository, service, separation-of-concerns]
---

# Lesson: Repository vs Service Separation

## Принцип

```
Handlers → Service → Repository → Database
```

## Repository (только CRUD)

- SeaORM Model/Entity
- `insert`, `update`, `upsert`, `delete`
- `get_by_id`, `get_by_*`, `list_*`
- Простые фильтры и пагинация
- **НЕ содержит**: группировку, агрегацию, бизнес-правила

## Service (оркестрация + бизнес-логика)

- Вызывает repository для CRUD
- Содержит бизнес-логику (группировка, агрегация)
- Единая точка входа для handlers и других модулей
- Логирование операций
- Вызов событий жизненного цикла (validate, before_write)

## Handlers

- **НЕ вызывают repository напрямую**
- Работают только через service

## Пример нарушения (до рефакторинга)

```rust
// ❌ НЕПРАВИЛЬНО - бизнес-логика в repository
pub async fn get_stats_by_date(...) -> Result<Vec<DailyStat>> {
    let items = query.all(conn()).await?;
    let mut stats_map: HashMap<...> = HashMap::new();
    // группировка в памяти
}
```

## Правильный подход (после рефакторинга)

```rust
// repository.rs - только SELECT
pub async fn list_by_date_range(...) -> Result<Vec<Model>> {
    query.all(conn()).await
}

// service.rs - бизнес-логика
pub async fn calculate_daily_stats(...) -> Result<Vec<DailyStat>> {
    let items = repository::list_by_date_range(...).await?;
    // группировка в памяти
}
```
