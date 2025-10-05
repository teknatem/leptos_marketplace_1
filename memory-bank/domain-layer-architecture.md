# Domain Layer Architecture

## Принципы организации domain слоя

### Основная идея

Каждый агрегат содержит внутри себя всю логику, относящуюся **только** к этому агрегату.
Логика, затрагивающая **несколько агрегатов**, выносится в `usecase` слой.

---

## Структура файлов агрегата (backend)

```
crates/backend/src/domain/{aggregate_name}/
├── mod.rs            # Re-exports модулей
├── aggregate.rs      # Структура агрегата + методы (validate, before_write)
├── service.rs        # Оркестрация операций над агрегатом
└── repository.rs     # Чистый CRUD (работа с БД)
```

### Альтернатива (если есть сложная логика событий):

```
crates/backend/src/domain/{aggregate_name}/
├── mod.rs            # Re-exports модулей
├── aggregate.rs      # Структура агрегата + простые методы
├── events.rs         # Статические функции-обработчики (validate, before_write, before_delete)
├── service.rs        # Оркестрация операций
└── repository.rs     # Чистый CRUD
```

**events.rs создается только при наличии сложной логики валидации/обработки событий.**
Если логика простая - методы помещаются в `impl` блок агрегата.

---

## Ответственность каждого файла

### 1. aggregate.rs

**Только данные + простые методы экземпляра:**

```rust
pub struct Connection1CDatabase {
    pub base: BaseAggregate<Connection1CDatabaseId>,
    pub description: String,
    pub url: String,
    pub comment: Option<String>,
    pub login: String,
    pub password: String,
    pub is_primary: bool,
}

impl Connection1CDatabase {
    // Конструкторы
    pub fn new_for_insert(...) -> Self { ... }

    // Простые методы экземпляра
    pub fn touch_updated(&mut self) {
        self.base.metadata.updated_at = chrono::Utc::now();
    }

    pub fn update(&mut self, dto: &Connection1CDatabaseDto) {
        self.description = dto.description.clone();
        // ... остальные поля
    }

    // Методы валидации (если логика простая)
    pub fn validate(&self) -> Result<(), String> {
        if self.url.is_empty() {
            return Err("URL не может быть пустым".into());
        }
        Ok(())
    }

    // Before write (если логика простая)
    pub fn before_write(&mut self) {
        self.touch_updated();
    }
}
```

### 2. events.rs (опционально)

**Статические функции-обработчики (аналог методов класса в ООП):**

Создается **только** если:

- Логика валидации сложная
- Есть кастомные обработчики событий
- Нужно разделить код по смысловым блокам

```rust
// Валидация - статическая функция
pub fn validate(aggregate: &Connection1CDatabase) -> Result<(), String> {
    if aggregate.url.is_empty() {
        return Err("URL не может быть пустым".into());
    }
    if !aggregate.url.starts_with("http://") && !aggregate.url.starts_with("https://") {
        return Err("URL должен начинаться с http:// или https://".into());
    }
    if aggregate.description.trim().is_empty() {
        return Err("Описание не может быть пустым".into());
    }
    Ok(())
}

// Before write - статическая функция
pub fn before_write(aggregate: &mut Connection1CDatabase) {
    aggregate.touch_updated();
}

// Before delete - статическая функция
pub fn before_delete(aggregate: &Connection1CDatabase) -> Result<(), String> {
    if aggregate.is_primary {
        return Err("Нельзя удалить основное подключение".into());
    }
    Ok(())
}
```

**Важно:** Обработчики событий - это **статические функции**, а не поля структуры с замыканиями.
Простота важнее гибкости.

### 3. service.rs

**Оркестрация операций над агрегатом:**

```rust
use super::{aggregate::*, repository};
// Опционально: use super::events; (если есть events.rs)

// Создание нового агрегата
pub async fn create(dto: Connection1CDatabaseDto) -> anyhow::Result<i32> {
    let mut aggregate = Connection1CDatabase::new_for_insert(
        dto.description, dto.url, dto.comment,
        dto.login, dto.password, dto.is_primary
    );

    // Вызов обработчиков
    aggregate.validate()?;  // или events::validate(&aggregate)?
    aggregate.before_write(); // или events::before_write(&mut aggregate)

    // Бизнес-логика специфичная для агрегата
    if aggregate.is_primary {
        repository::clear_other_primary_flags(None).await?;
    }

    // Сохранение через repository
    repository::insert(&aggregate).await
}

// Обновление агрегата
pub async fn update(dto: Connection1CDatabaseDto) -> anyhow::Result<()> {
    let id = dto.id.as_ref()
        .and_then(|s| s.parse::<i32>().ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid ID"))?;

    let mut aggregate = repository::get_by_id(id).await?
        .ok_or_else(|| anyhow::anyhow!("Not found"))?;

    aggregate.update(&dto);

    aggregate.validate()?;
    aggregate.before_write();

    if aggregate.is_primary {
        repository::clear_other_primary_flags(Some(id)).await?;
    }

    repository::update(&aggregate).await
}

// Удаление
pub async fn delete(id: i32) -> anyhow::Result<bool> {
    let aggregate = repository::get_by_id(id).await?
        .ok_or_else(|| anyhow::anyhow!("Not found"))?;

    aggregate.before_delete()?; // или events::before_delete(&aggregate)?

    repository::soft_delete(id).await
}

// Простые запросы (pass-through к repository)
pub async fn get_by_id(id: i32) -> anyhow::Result<Option<Connection1CDatabase>> {
    repository::get_by_id(id).await
}

pub async fn list_all() -> anyhow::Result<Vec<Connection1CDatabase>> {
    repository::list_all().await
}
```

**Ответственность service:**

- Координация операций (create, update, delete)
- Вызов обработчиков событий в правильном порядке
- Применение бизнес-правил специфичных для агрегата
- Управление транзакциями (если будут)
- API для внешних слоев (handlers, usecase)

### 4. repository.rs

**Только CRUD операции, БЕЗ бизнес-логики:**

```rust
use sea_orm::entity::prelude::*;
use contracts::domain::connection_1c::aggregate::*;

// SeaORM Model
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "connection_1c_database")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub description: String,
    // ... остальные поля
}

// Mapper: Model -> Aggregate
impl From<Model> for Connection1CDatabase {
    fn from(m: Model) -> Self { ... }
}

// Repository functions - ТОЛЬКО работа с БД
pub async fn insert(aggregate: &Connection1CDatabase) -> anyhow::Result<i32> {
    // Чистая работа с БД, БЕЗ вызовов обработчиков событий
}

pub async fn update(aggregate: &Connection1CDatabase) -> anyhow::Result<()> {
    // Чистая работа с БД
}

pub async fn get_by_id(id: i32) -> anyhow::Result<Option<Connection1CDatabase>> {
    // Чистая работа с БД
}

pub async fn list_all() -> anyhow::Result<Vec<Connection1CDatabase>> {
    // Чистая работа с БД
}

pub async fn soft_delete(id: i32) -> anyhow::Result<bool> {
    // Чистая работа с БД
}

// Вспомогательные функции для специфичной бизнес-логики агрегата
pub async fn clear_other_primary_flags(except_id: Option<i32>) -> anyhow::Result<()> {
    // Специфичная для Connection1C логика
}
```

**Ответственность repository:**

- Маппинг Model ↔ Aggregate
- CRUD операции (insert, update, delete, get, list)
- Специфичные запросы для бизнес-логики агрегата (например, clear_other_primary_flags)
- БЕЗ валидации, БЕЗ обработчиков событий, БЕЗ бизнес-правил

---

## UseCase слой

**Располагается в:** `crates/backend/src/usecase/`

**Создается только для операций между несколькими агрегатами:**

```rust
// crates/backend/src/usecase/sync_1c_with_products.rs

use crate::domain::connection_1c::service as connection_service;
use crate::domain::product::service as product_service;

pub async fn sync_products_from_1c(connection_id: i32) -> anyhow::Result<()> {
    // 1. Получить подключение
    let connection = connection_service::get_by_id(connection_id).await?
        .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

    // 2. Загрузить продукты из 1C
    let products_1c = fetch_products_from_1c(&connection).await?;

    // 3. Обновить продукты в нашей системе
    for product_data in products_1c {
        product_service::upsert_from_1c(product_data).await?;
    }

    Ok(())
}
```

**UseCase НЕ создается для:**

- Операций над одним агрегатом (это делает service внутри агрегата)
- Простых CRUD операций (это делает service)

---

## Правила и принципы

### ✅ DO (Делай так)

1. **Вся логика одного агрегата - внутри агрегата** (aggregate.rs + service.rs + repository.rs)
2. **Методы валидации/обработки - статические функции**, НЕ поля структуры
3. **service.rs вызывает обработчики события** в правильном порядке
4. **repository.rs - только CRUD**, без бизнес-логики
5. **events.rs - только если логика сложная**, иначе методы в impl агрегата
6. **usecase - только для межагрегатных операций**

### ❌ DON'T (Не делай так)

1. ❌ **Не размещай бизнес-логику в repository**
2. ❌ **Не создавай поля в структуре агрегата для обработчиков событий** (типа `on_validate: Option<Box<dyn Fn...>>`)
3. ❌ **Не создавай usecase для операций над одним агрегатом**
4. ❌ **Не создавай events.rs если логика простая** (2-3 строки валидации)
5. ❌ **Не вызывай repository напрямую из handlers** - только через service
6. ❌ **Не дублируй логику между aggregate и events** - выбери одно место

---

## Примеры событий жизненного цикла

По аналогии с 1С модулями (Справочник/Документ):

| Событие 1С         | Rust аналог                       | Где размещается            |
| ------------------ | --------------------------------- | -------------------------- |
| `ПередЗаписью`     | `before_write()`                  | aggregate.rs или events.rs |
| `ПриЗаписи`        | после `repository::save()`        | service.rs                 |
| `ПередУдалением`   | `before_delete()`                 | aggregate.rs или events.rs |
| `ПриУдалении`      | после `repository::delete()`      | service.rs                 |
| `Заполнение`       | конструктор `new_with_defaults()` | aggregate.rs               |
| `Проведение`       | `post()` / `on_posting()`         | aggregate.rs или events.rs |
| `ОтменаПроведения` | `unpost()`                        | aggregate.rs или events.rs |

**Методы могут отсутствовать** - они опциональны и вызываются только если существуют.

---

## Порядок выполнения при операциях

### Create/Update:

```
1. Создание/загрузка агрегата
2. validate()           ← может заблокировать операцию
3. before_write()       ← может модифицировать данные
4. Бизнес-логика (service)
5. repository::save()
6. После сохранения (опционально)
```

### Delete:

```
1. Загрузка агрегата
2. before_delete()      ← может заблокировать операцию
3. repository::soft_delete()
4. После удаления (опционально)
```

---

## Принцип минимализма

**Чем меньше кода - тем лучше.**

- Если валидация 2-3 строки → методы в `impl` агрегата, БЕЗ events.rs
- Если логика сложная (10+ строк) → выносим в events.rs
- Если операция касается одного агрегата → service внутри агрегата
- Если операция касается нескольких агрегатов → usecase
- Если метод события не нужен → не создаем его

**Всегда начинай с простого варианта, усложняй только при необходимости.**
