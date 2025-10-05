# UseCase u501: Импорт из УТ 11

## Описание

Загрузка справочников из 1С:Управление торговлей 11 через OData API.

## Архитектура

### DDD Подход

UseCase реализован как **Application Service** (не агрегат), который оркестрирует работу с доменными агрегатами:
- Читает конфигурацию из `a001_connection_1c`
- Загружает данные через OData
- Создает/обновляет агрегаты (например, `a002_organization`)

### Event Sourcing

События импорта записываются в БД для аудита:
- `ImportStarted` - импорт запущен
- `ImportProgress` - прогресс по агрегату
- `ImportError` - ошибка импорта
- `ImportCompleted` - импорт завершен

### CQRS-lite

- **Command**: `POST /api/u501/import/start` запускает импорт
- **Query**: `GET /api/u501/import/:session_id/progress` получает текущий прогресс

## Компоненты

### 1. Contracts (contracts/src/usecases/u501_import_from_ut/)

#### request.rs
```rust
pub struct ImportRequest {
    pub connection_id: String,           // ID из a001_connection_1c
    pub target_aggregates: Vec<String>,  // ["a002_organization"]
    pub mode: ImportMode,                // Interactive / Background
}
```

#### response.rs
```rust
pub struct ImportResponse {
    pub session_id: String,  // UUID для отслеживания прогресса
    pub status: ImportStartStatus,
    pub message: String,
}
```

#### progress.rs
```rust
pub struct ImportProgress {
    pub session_id: String,
    pub status: ImportStatus,           // Running / Completed / Failed
    pub aggregates: Vec<AggregateProgress>,
    pub total_processed: i32,
    pub total_inserted: i32,
    pub total_updated: i32,
    pub total_errors: i32,
    pub errors: Vec<ImportError>,
}
```

### 2. Backend (backend/src/usecases/u501_import_from_ut/)

#### executor.rs
Главная логика UseCase:
- `start_import()` - запускает async task, возвращает session_id
- `run_import()` - выполняет импорт в фоне
- `import_organizations()` - загружает Organizations через OData
- `process_organization()` - upsert одной организации

#### ut_odata_client.rs
HTTP-клиент для 1C OData:
- `fetch_collection()` - получить коллекцию с пагинацией
- `get_collection_count()` - получить количество элементов ($count)

#### progress_tracker.rs
In-memory хранилище прогресса:
- `HashMap<session_id, ImportProgress>`
- Обновляется в реальном времени
- Очистка старых сессий через `cleanup_old_sessions()`

### 3. Domain (backend/src/domain/a002_organization/)

#### from_ut_odata.rs
Маппинг OData JSON → Organization aggregate:
```rust
pub struct UtOrganizationOData {
    pub ref_key: String,
    pub code: String,
    pub description: String,
    pub inn: Option<String>,
    pub kpp: Option<String>,
}

impl UtOrganizationOData {
    pub fn to_aggregate(&self) -> Result<Organization, String>
    pub fn should_update(&self, existing: &Organization) -> bool
}
```

## API Endpoints

### POST /api/u501/import/start

**Request:**
```json
{
  "connection_id": "uuid-here",
  "target_aggregates": ["a002_organization"],
  "mode": "interactive"
}
```

**Response:**
```json
{
  "session_id": "session-uuid",
  "status": "started",
  "message": "Импорт запущен"
}
```

### GET /api/u501/import/:session_id/progress

**Response:**
```json
{
  "session_id": "session-uuid",
  "status": "running",
  "started_at": "2025-01-15T10:00:00Z",
  "aggregates": [
    {
      "aggregate_index": "a002_organization",
      "aggregate_name": "Организации",
      "status": "running",
      "processed": 50,
      "total": 100,
      "inserted": 10,
      "updated": 40,
      "errors": 0
    }
  ],
  "total_processed": 50,
  "total_inserted": 10,
  "total_updated": 40,
  "total_errors": 0,
  "errors": []
}
```

## OData Маппинг

### УТ 11 → Organization

| OData поле УТ              | Поле Organization  |
|----------------------------|--------------------|
| `Ref_Key`                  | (используется как код временно) |
| `Code`                     | `code`             |
| `Description`              | `description`      |
| `ПолноеНаименование`       | `full_name`        |
| `ИНН`                      | `inn`              |
| `КПП`                      | `kpp`              |
| `DeletionMark`             | `is_deleted` (в будущем) |

### Коллекции OData УТ 11

- `Catalog_Организации` → `a002_organization`
- `Catalog_Номенклатура` → `a003_product` (в будущем)
- `Catalog_Контрагенты` → `a004_counterparty` (в будущем)

## Алгоритм Upsert

1. Получить порцию данных из OData (100 записей)
2. Для каждой записи:
   - Найти существующую по `code`
   - Если найдена:
     - Сравнить поля
     - Обновить если есть изменения
   - Если не найдена:
     - Создать новую запись
3. Обновить прогресс
4. Перейти к следующей порции

## Обработка ошибок

- **Ошибка подключения**: Статус `Failed`, в errors добавляется запись
- **Ошибка парсинга**: Пропускается запись, счетчик ошибок увеличивается
- **Ошибка валидации**: Пропускается запись, добавляется в `errors[]`
- **Ошибка БД**: Откат транзакции, статус `Failed`

## Расширение на другие агрегаты

Для добавления нового агрегата (например, `a003_product`):

1. Создать `from_ut_odata.rs` в `backend/src/domain/a003_product/`
2. Определить `UtProductOData` структуру
3. Реализовать `to_aggregate()` и `should_update()`
4. Добавить метод `import_products()` в executor
5. Добавить case в `run_import()`:
   ```rust
   match aggregate_index.as_str() {
       "a002_organization" => self.import_organizations(...).await?,
       "a003_product" => self.import_products(...).await?,
       ...
   }
   ```

## Тестирование

### Локальный запуск

```bash
# 1. Запустить backend
cargo run --bin backend

# 2. Создать тестовое подключение
curl -X POST http://localhost:3000/api/connection_1c/testdata

# 3. Получить ID подключения
curl http://localhost:3000/api/connection_1c

# 4. Запустить импорт
curl -X POST http://localhost:3000/api/u501/import/start \
  -H "Content-Type: application/json" \
  -d '{
    "connection_id": "connection-uuid-here",
    "target_aggregates": ["a002_organization"]
  }'

# 5. Проверить прогресс
curl http://localhost:3000/api/u501/import/SESSION_ID/progress
```

### Примечания

- Убедитесь что URL в connection_1c указывает на реальный сервер УТ 11
- OData endpoint должен быть вида: `http://server/base/odata/standard.odata`
- Пользователь должен иметь права на чтение справочников

## Производительность

- **Batch size**: 100 записей (можно настроить)
- **Timeout**: 30 секунд на запрос
- **Параллелизм**: Последовательная обработка агрегатов (для упрощения)
- **Memory**: In-memory прогресс занимает ~1KB на сессию

## Планы развития

- [ ] WebSocket для real-time прогресса вместо polling
- [ ] Поддержка отмены импорта (`POST /api/u501/import/:id/cancel`)
- [ ] Фоновый режим с планировщиком (cron)
- [ ] Инкрементальный импорт (только измененные записи)
- [ ] Сохранение истории в таблицу `u501_import_from_ut_history`
- [ ] Frontend UI (виджет + монитор прогресса)
