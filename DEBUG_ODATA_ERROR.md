# Отладка ошибки "error decoding response body"

## Проблема

```
Import failed: error decoding response body
```

Это означает что сервер 1С вернул JSON, но его структура не совпадает с нашей OData моделью.

---

## Шаг 1: Запустить backend с DEBUG логами

```bash
RUST_LOG=debug cargo run --bin backend
```

Или в Windows PowerShell:
```powershell
$env:RUST_LOG="debug"
cargo run --bin backend
```

---

## Шаг 2: Запустить импорт

Через UI или curl:

```bash
curl -X POST http://localhost:3000/api/u501/import/start \
  -H "Content-Type: application/json" \
  -d '{
    "connection_id": "YOUR_UUID",
    "target_aggregates": ["a002_organization"]
  }'
```

---

## Шаг 3: Найти в логах backend

### Успешный запрос выглядит так:

```
INFO Fetching OData from: http://79.137.226.70:8787/TradeSklad/odata/standard.odata/Catalog_Организации?$top=100&$skip=0
DEBUG OData response preview: {"odata.metadata":"...","value":[{...}]}...
```

### Ошибка будет выглядеть так:

```
ERROR Failed to parse OData response. Error: missing field `Code` at line 1 column 123
ERROR Response body: {"odata.metadata":"http://...","value":[{"Ref_Key":"...","Наименование":"..."}]}
```

---

## Шаг 4: Скопировать JSON из логов

Найдите строку `ERROR Response body:` и скопируйте весь JSON.

Пример:
```json
{
  "odata.metadata": "http://...",
  "value": [
    {
      "Ref_Key": "12345678-1234-1234-1234-123456789012",
      "Наименование": "ООО Рога и Копыта",
      "ИНН": "1234567890",
      "КПП": "123456789"
    }
  ]
}
```

---

## Шаг 5: Сравнить с нашей моделью

### Наша модель ожидает:

```rust
{
  "Ref_Key": "...",      // UUID организации
  "Code": "...",         // Код
  "Description": "...",  // Наименование
  "ПолноеНаименование": "...",  // Опционально
  "ИНН": "...",          // Опционально
  "КПП": "...",          // Опционально
  "DeletionMark": false  // Опционально
}
```

### Частые проблемы:

#### 1. Поле называется иначе

**Проблема:**
```json
{"Наименование": "..."} вместо {"Description": "..."}
```

**Решение:** Добавить alias в `from_ut_odata.rs`:
```rust
#[serde(rename = "Description", alias = "Наименование")]
pub description: String,
```

#### 2. Отсутствует обязательное поле

**Проблема:**
```
Error: missing field `Code`
```

**Решение:** Сделать поле опциональным или с default:
```rust
#[serde(rename = "Code", default)]
pub code: String,
```

#### 3. Дополнительные поля в ответе

**Проблема:**
```json
{"DataVersion": "...", "Parent_Key": "...", ...}
```

**Решение:** Добавить `#[serde(flatten)]` или игнорировать:
```rust
#[serde(flatten)]
pub extra: std::collections::HashMap<String, serde_json::Value>,
```

---

## Шаг 6: Тестирование структуры вручную

### Получить один элемент через curl:

```bash
curl "http://79.137.226.70:8787/TradeSklad/odata/standard.odata/Catalog_Организации?\$top=1" \
  -u login:password \
  -H "Accept: application/json" | jq '.'
```

### Пример ответа:

```json
{
  "odata.metadata": "http://79.137.226.70:8787/TradeSklad/odata/standard.odata/$metadata#Catalog_Организации",
  "value": [
    {
      "Ref_Key": "12345678-1234-1234-1234-123456789012",
      "Code": "00-000001",
      "Description": "ООО \"Рога и Копыта\"",
      "ПолноеНаименование": "Общество с ограниченной ответственностью \"Рога и Копыта\"",
      "ИНН": "1234567890",
      "КПП": "123456789",
      "Parent_Key": "00000000-0000-0000-0000-000000000000",
      "Owner_Key": "00000000-0000-0000-0000-000000000000",
      "DeletionMark": false,
      "IsFolder": false,
      "Predefined": false,
      "PredefinedDataName": ""
    }
  ]
}
```

---

## Шаг 7: Обновить модель

Если поля называются иначе, обновите `from_ut_odata.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtOrganizationOData {
    #[serde(rename = "Ref_Key", default)]
    pub ref_key: String,

    #[serde(rename = "Code", default)]
    pub code: String,

    #[serde(rename = "Description", alias = "Наименование", default)]
    pub description: String,

    #[serde(rename = "ПолноеНаименование", default)]
    pub full_name: Option<String>,

    #[serde(rename = "ИНН", default)]
    pub inn: Option<String>,

    #[serde(rename = "КПП", default)]
    pub kpp: Option<String>,

    #[serde(rename = "DeletionMark", default)]
    pub deletion_mark: bool,

    // Игнорировать дополнительные поля
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}
```

---

## Шаг 8: Перекомпилировать и повторить

```bash
cargo run --bin backend
```

Затем повторить импорт.

---

## Типичные различия между конфигурациями 1С

### УТ 10 vs УТ 11

| Поле | УТ 10 | УТ 11 |
|------|-------|-------|
| Код | `Code` | `Code` |
| Наименование | `Description` | `Description` или `Наименование` |
| Полное наименование | `FullName` | `ПолноеНаименование` |
| ИНН | `INN` | `ИНН` |
| КПП | `KPP` | `КПП` |

### Управление торговлей vs Бухгалтерия

Справочник Организации может иметь разные поля в зависимости от конфигурации.

---

## Полезные команды для отладки

### 1. Получить метаданные коллекции:

```bash
curl "http://79.137.226.70:8787/TradeSklad/odata/standard.odata/\$metadata" \
  -u login:password | grep "Catalog_Организации" -A 50
```

### 2. Получить список доступных коллекций:

```bash
curl "http://79.137.226.70:8787/TradeSklad/odata/standard.odata/" \
  -u login:password | jq '.value[].name'
```

### 3. Проверить структуру ответа:

```bash
curl "http://79.137.226.70:8787/TradeSklad/odata/standard.odata/Catalog_Организации?\$top=1&\$format=json" \
  -u login:password | jq '.value[0] | keys'
```

Это покажет все доступные поля.

---

## Быстрое решение: Игнорировать лишние поля

Добавьте в `UtOrganizationOData`:

```rust
use std::collections::HashMap;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtOrganizationOData {
    // ... существующие поля ...

    // Игнорировать все остальные поля
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub other: HashMap<String, Value>,
}
```

Это позволит парсить JSON даже если есть дополнительные поля.

---

## Контрольный список

- [ ] Запустить backend с `RUST_LOG=debug`
- [ ] Скопировать JSON из логов (строка `ERROR Response body:`)
- [ ] Проверить структуру через curl
- [ ] Сравнить поля с моделью `UtOrganizationOData`
- [ ] Добавить `alias` для полей с другими именами
- [ ] Добавить `#[serde(default)]` для опциональных полей
- [ ] Добавить `#[serde(flatten)]` для игнорирования лишних полей
- [ ] Перекомпилировать и повторить импорт

---

## Что делать если не помогло

1. Скопируйте полный JSON ответ из логов
2. Проверьте что URL правильный (содержит `/odata/standard.odata/`)
3. Убедитесь что это именно УТ 11, а не другая конфигурация
4. Проверьте что `Accept: application/json` заголовок установлен
5. Попробуйте добавить `$format=json` к URL

---

## Пример исправления

Если в логах вы видите:

```json
{
  "value": [{
    "Ref_Key": "...",
    "Код": "...",           ← поле называется "Код" вместо "Code"
    "Наименование": "..."   ← поле называется "Наименование" вместо "Description"
  }]
}
```

Измените модель:

```rust
#[serde(rename = "Code", alias = "Код", default)]
pub code: String,

#[serde(rename = "Description", alias = "Наименование", default)]
pub description: String,
```

Это позволит парсить оба варианта.
