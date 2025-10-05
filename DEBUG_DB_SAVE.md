# Отладка: Данные не сохраняются в БД

## Проблема

Импорт прошел успешно (3 организации загружены), но данные не видны в списке Organizations.

---

## Шаг 1: Запустить backend с DEBUG логами

```bash
RUST_LOG=debug cargo run --bin backend
```

Windows PowerShell:
```powershell
$env:RUST_LOG="debug"
cargo run --bin backend
```

---

## Шаг 2: Запустить импорт

Через UI или curl

---

## Шаг 3: Проверить логи сохранения

### Что искать в логах:

**Создание новой организации:**
```
INFO Creating new organization: code=..., description=...
DEBUG Organization aggregate created: id=..., code=..., inn=..., kpp=...
INFO Successfully inserted organization: code=..., uuid=...
```

**Обновление существующей:**
```
DEBUG Found existing organization with code=...
INFO Updating organization: code=...
INFO Successfully updated organization: code=...
```

**Ошибка сохранения:**
```
ERROR Failed to insert organization: code=..., error=...
```

---

## Шаг 4: Проверить данные в БД напрямую

### Найти файл БД:

```
target/db/app.db
```

### Открыть через sqlite3:

```bash
sqlite3 target/db/app.db
```

### Проверить таблицу:

```sql
-- Проверить что таблица существует
.tables

-- Должна быть: a002_organization

-- Посмотреть схему таблицы
.schema a002_organization

-- Проверить данные
SELECT * FROM a002_organization;

-- Проверить количество записей
SELECT COUNT(*) FROM a002_organization;

-- Проверить только не удаленные
SELECT * FROM a002_organization WHERE is_deleted = 0;
```

---

## Возможные проблемы:

### 1. Данные есть, но is_deleted = 1

```sql
SELECT * FROM a002_organization WHERE is_deleted = 1;
```

**Решение:** Обновить флаг:
```sql
UPDATE a002_organization SET is_deleted = 0;
```

### 2. Пустая таблица

```sql
SELECT COUNT(*) FROM a002_organization;
-- 0
```

**Причина:** Ошибка при insert

**Решение:** Проверить логи backend, найти ERROR

### 3. Неправильный код (пустой или дубликат)

```sql
SELECT code, description FROM a002_organization;
```

**Проблема:** Если `code` пустой:
```
|| ООО "Рога и Копыта"
```

**Решение:** Проверить что OData возвращает `Code`:
```bash
curl "http://..../Catalog_Организации?\$top=1" -u login:password | jq '.value[0].Code'
```

### 4. Ошибка валидации

Если в логах есть:
```
ERROR Failed to process organization ORG-001: ИНН должен содержать 10 или 12 цифр
```

**Решение:** Данные пропускаются из-за валидации. Нужно либо:
- Исправить данные в 1С
- Ослабить валидацию в `Organization::validate()`

---

## Шаг 5: Проверить API endpoint

```bash
# Получить список организаций
curl http://localhost:3000/api/organization | jq '.'
```

**Ожидаемый ответ:**
```json
[
  {
    "id": "...",
    "code": "...",
    "description": "ООО \"Рога и Копыта\"",
    "fullName": "...",
    "inn": "1234567890",
    "kpp": "123456789"
  }
]
```

**Если пусто:**
```json
[]
```

---

## Шаг 6: Проверить frontend запрос

Откройте DevTools (F12) → Network tab

При открытии "Организации" должен быть запрос:
```
GET http://localhost:3000/api/organization
```

**Status:** 200 OK

**Response:** JSON array

---

## Частые причины проблемы:

### 1. Код пустой (Code = "")

OData модель:
```json
{
  "Code": "",
  "Description": "ООО \"Рога и Копыта\""
}
```

В таблице `code` будет пустой строкой `""`.

**Решение:** Использовать `Ref_Key` как код если `Code` пустой:

```rust
// from_ut_odata.rs
pub fn to_aggregate(&self) -> Result<Organization, String> {
    let code = if self.code.is_empty() {
        // Использовать первые 8 символов Ref_Key
        self.ref_key.chars().take(8).collect()
    } else {
        self.code.clone()
    };

    Organization::new_for_insert(
        code,
        // ...
    )
}
```

### 2. Ошибка валидации ИНН/КПП

```rust
// Organization::validate()
// Валидация ИНН
let inn_digits: String = self.inn.chars().filter(|c| c.is_ascii_digit()).collect();
if inn_digits.len() != 10 && inn_digits.len() != 12 {
    return Err("ИНН должен содержать 10 цифр (для ЮЛ) или 12 цифр (для ИП)".into());
}
```

**Если ИНН пустой или некорректный - запись НЕ сохранится!**

**Решение:** Сделать валидацию мягче:

```rust
// Валидация ИНН (только если не пустой)
if !self.inn.is_empty() {
    let inn_digits: String = self.inn.chars().filter(|c| c.is_ascii_digit()).collect();
    if inn_digits.len() != 10 && inn_digits.len() != 12 {
        return Err("ИНН должен содержать 10 цифр (для ЮЛ) или 12 цифр (для ИП)".into());
    }
}
```

### 3. Транзакции не коммитятся

Sea-ORM должен автоматически коммитить, но если есть проблемы:

```rust
// Добавить явный flush после insert/update
a002_organization::repository::insert(&new_org).await?;
// commit автоматический
```

---

## Тестовые SQL команды

### Вставить тестовую запись вручную:

```sql
INSERT INTO a002_organization (
    id, code, description, full_name, inn, kpp,
    is_deleted, is_posted, created_at, updated_at, version
) VALUES (
    '00000000-0000-0000-0000-000000000001',
    'TEST-001',
    'Тестовая организация',
    'ООО "Тестовая организация"',
    '1234567890',
    '123456789',
    0, 0,
    datetime('now'), datetime('now'), 0
);
```

Затем проверить через API:
```bash
curl http://localhost:3000/api/organization | jq '.[] | select(.code == "TEST-001")'
```

### Очистить таблицу:

```sql
DELETE FROM a002_organization;
```

### Сбросить счетчик autoincrement (не нужно для UUID):

```sql
-- Не применимо, т.к. id = TEXT (UUID)
```

---

## Контрольный список

- [ ] Запустить backend с `RUST_LOG=debug`
- [ ] Запустить импорт
- [ ] Проверить логи: найти `INFO Successfully inserted organization`
- [ ] Открыть БД: `sqlite3 target/db/app.db`
- [ ] Выполнить: `SELECT * FROM a002_organization;`
- [ ] Проверить количество: `SELECT COUNT(*) FROM a002_organization;`
- [ ] Проверить API: `curl http://localhost:3000/api/organization`
- [ ] Проверить frontend: DevTools → Network → GET /api/organization

---

## Быстрая диагностика

```bash
# 1. Проверить файл БД
ls -lh target/db/app.db

# 2. Проверить таблицу
sqlite3 target/db/app.db "SELECT COUNT(*) FROM a002_organization;"

# 3. Проверить данные
sqlite3 target/db/app.db "SELECT code, description, inn FROM a002_organization LIMIT 5;"

# 4. Проверить API
curl -s http://localhost:3000/api/organization | jq 'length'
```

Если все команды возвращают 0 или пусто - данные не сохраняются.

---

## Что делать если данные не сохраняются

1. **Найти ERROR в логах backend**
   ```
   grep ERROR backend.log
   ```

2. **Проверить что insert вызывается**
   ```
   grep "Successfully inserted" backend.log
   ```

3. **Проверить код организации**
   ```sql
   SELECT code, length(code), hex(code) FROM a002_organization;
   ```

4. **Добавить TRACE логи в repository::insert**
   ```rust
   pub async fn insert(aggregate: &Organization) -> anyhow::Result<Uuid> {
       let uuid = aggregate.base.id.value();
       tracing::info!("Repository::insert called for uuid={}", uuid);

       // ... existing code ...

       tracing::info!("Repository::insert completed for uuid={}", uuid);
       Ok(uuid)
   }
   ```

5. **Перезапустить backend и повторить импорт**

---

## Если проблема в валидации

Временно отключить валидацию для тестирования:

```rust
// Organization::validate()
pub fn validate(&self) -> Result<(), String> {
    // Временно вернуть Ok для всех записей
    Ok(())

    // Закомментировать остальное
    // if self.base.description.trim().is_empty() { ... }
}
```

Затем повторить импорт и проверить что данные сохранились.

Если данные появились - проблема в валидации. Нужно ослабить правила.
