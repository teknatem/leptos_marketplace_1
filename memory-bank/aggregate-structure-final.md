# Финальная структура агрегатов

## Общая информация

Проект использует паттерн DDD (Domain-Driven Design) с агрегатами для организации ~100 доменных сущностей.

## Терминология

### Уровень класса агрегата (статические метаданные)

- **aggregate_index** (например, "a001", "a002") - уникальный номер агрегата в системе
- **collection_name** (например, "connection_1c") - имя для БД коллекции/таблиц
- **element_name** (например, "Подключение 1С") - имя элемента для UI (единственное число)
- **list_name** (например, "Подключения 1С") - имя списка для UI (множественное число)
- **origin** - источник данных (C1, Bitrix, Ozon, Self_)

### Уровень экземпляра агрегата (данные записи)

- **id** (UUID/String/i32) - уникальный идентификатор записи
- **code** - бизнес-код записи (например, "CON-001", "ORD-2025-123")
- **description** - описание/название записи
- **comment** - комментарий к записи
- **is_deleted** - мягкое удаление (soft delete)
- **is_posted** - проведен (для документов)
- **created_at**, **updated_at** - временные метки аудита
- **version** - версия для optimistic locking

## Структура файлов

### Именование агрегатов

**Формат:** `a{NNN}_{snake_case_name}`

Примеры:
- ✅ `a001_connection_1c`
- ✅ `a002_user_profile`
- ✅ `a050_invoice_payment`

### Структура директорий

```
crates/
├── contracts/src/domain/
│   ├── common/                      # Базовые типы для всех агрегатов
│   │   ├── origin.rs                # Enum Origin
│   │   ├── entity_metadata.rs       # Метаданные экземпляра
│   │   ├── event_store.rs           # Хранилище событий
│   │   ├── base_aggregate.rs        # BaseAggregate<Id>
│   │   ├── aggregate_root.rs        # Trait AggregateRoot
│   │   └── aggregate_id.rs          # Trait AggregateId
│   │
│   └── a001_connection_1c/          # Конкретный агрегат
│       ├── mod.rs                   # Re-exports
│       └── aggregate.rs             # Domain types
│
├── backend/src/domain/
│   └── a001_connection_1c/
│       ├── mod.rs
│       ├── service.rs               # Бизнес-логика
│       └── repository.rs            # Sea-ORM + БД
│
└── frontend/src/domain/
    └── a001_connection_1c/
        └── ui/
            ├── list/                # Список
            └── details/             # Детали/форма
```

## Базовые типы (contracts/src/domain/common/)

### Origin (origin.rs)

```rust
pub enum Origin {
    C1,      // 1C:Enterprise (GUID-based IDs)
    Bitrix,  // Bitrix24
    Ozon,    // Ozon Marketplace
    Self_,   // Собственная система
}
```

### EntityMetadata (entity_metadata.rs)

```rust
pub struct EntityMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub version: i32,
}
```

### BaseAggregate (base_aggregate.rs)

```rust
pub struct BaseAggregate<Id> {
    pub id: Id,
    pub code: String,           // Бизнес-код записи
    pub description: String,    // Название/описание
    pub comment: Option<String>,
    pub metadata: EntityMetadata,
    pub events: EventStore,
}
```

### AggregateRoot (aggregate_root.rs)

```rust
pub trait AggregateRoot {
    type Id;

    // Методы экземпляра
    fn id(&self) -> Self::Id;
    fn code(&self) -> &str;
    fn description(&self) -> &str;
    fn metadata(&self) -> &EntityMetadata;
    fn metadata_mut(&mut self) -> &mut EntityMetadata;
    fn events(&self) -> &EventStore;
    fn events_mut(&mut self) -> &mut EventStore;

    // Метаданные класса агрегата (статические)
    fn aggregate_index() -> &'static str;      // "a001"
    fn collection_name() -> &'static str;      // "connection_1c"
    fn element_name() -> &'static str;         // "Подключение 1С"
    fn list_name() -> &'static str;            // "Подключения 1С"
    fn origin() -> Origin;                     // Origin::C1

    // Вспомогательные методы
    fn full_name() -> String {
        format!("{}_{}", Self::aggregate_index(), Self::collection_name())
    }
    fn table_prefix() -> String {
        format!("{}_", Self::full_name())
    }
}
```

### AggregateId (aggregate_id.rs)

```rust
pub trait AggregateId:
    Clone + Copy + PartialEq + Eq + Hash + Serialize + DeserializeOwned + Debug
{
    fn as_string(&self) -> String;
    fn from_string(s: &str) -> Result<Self, String>;
}

// Реализовано для: i32, i64, uuid::Uuid
```

## Пример агрегата (a001_connection_1c)

### ID Type

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Connection1CDatabaseId(pub Uuid);

impl AggregateId for Connection1CDatabaseId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }
    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(Connection1CDatabaseId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}
```

### Aggregate Root

```rust
pub struct Connection1CDatabase {
    #[serde(flatten)]
    pub base: BaseAggregate<Connection1CDatabaseId>,

    // Специфичные поля агрегата
    pub url: String,
    pub login: String,
    pub password: String,
    pub is_primary: bool,
}

impl AggregateRoot for Connection1CDatabase {
    type Id = Connection1CDatabaseId;

    fn id(&self) -> Self::Id { self.base.id }
    fn code(&self) -> &str { &self.base.code }
    fn description(&self) -> &str { &self.base.description }
    fn metadata(&self) -> &EntityMetadata { &self.base.metadata }
    fn metadata_mut(&mut self) -> &mut EntityMetadata { &mut self.base.metadata }
    fn events(&self) -> &EventStore { &self.base.events }
    fn events_mut(&mut self) -> &mut EventStore { &mut self.base.events }

    fn aggregate_index() -> &'static str { "a001" }
    fn collection_name() -> &'static str { "connection_1c" }
    fn element_name() -> &'static str { "Подключение 1С" }
    fn list_name() -> &'static str { "Подключения 1С" }
    fn origin() -> Origin { Origin::Self_ }
}
```

## Структура БД

### Именование таблиц

**Формат:** `{aggregate_index}_{collection_name}_{entity}`

Примеры:
- ✅ `a001_connection_1c_database`
- ✅ `a001_connection_1c_events`

### Обязательные поля таблицы

```sql
CREATE TABLE a001_connection_1c_database (
    -- Primary Key
    id TEXT PRIMARY KEY NOT NULL,

    -- Обязательные поля из BaseAggregate
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL,
    comment TEXT,

    -- Специфичные бизнес-поля
    url TEXT NOT NULL,
    login TEXT NOT NULL,
    password TEXT NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 0,

    -- Audit fields (обязательно!)
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 0
);
```

## Миграция существующих агрегатов

Автоматическая миграция в `backend/src/shared/data/db.rs`:

1. Проверка существования старой таблицы `connection_1c_database`
2. Создание новой таблицы `a001_connection_1c_database` с новой схемой
3. Миграция данных с генерацией `code` из ID
4. Удаление старой таблицы

## Преимущества структуры

1. **Масштабируемость** - поддержка до 999 агрегатов (a001-a999)
2. **Единообразие** - все агрегаты следуют одинаковой структуре
3. **Изоляция** - агрегаты связаны только через ID, без FK в БД
4. **Типобезопасность** - строгая типизация ID через newtype pattern
5. **Метаданные** - встроенная поддержка аудита и версионирования
6. **Гибкость** - поддержка разных типов ID (UUID, i32, i64)
7. **Расширяемость** - легко добавлять новые агрегаты по шаблону

## Следующие шаги

1. Создать генератор агрегатов (`tools/aggregate-gen`) для автоматической генерации scaffold
2. Создать валидатор агрегатов (`tools/aggregate-validator`) для проверки соблюдения стандартов
3. Добавить документацию для каждого агрегата в `_aggregate.toml`
