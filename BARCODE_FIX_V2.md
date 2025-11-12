# Исправление Импорта Штрихкодов из Yandex Market (v2)

## Найденная Проблема

**Причина**: Код делал ЛИШНИЙ запрос к `/offer-cards` endpoint, который НЕ возвращает barcodes. Первый запрос к `/offer-mappings` УЖЕ содержит все данные, включая barcodes!

### Архитектура До Исправления

```
1. fetch_product_list() → GET /offer-mappings → barcodes ЕСТЬ ✅
2. fetch_product_info() → POST /offer-cards → barcodes НЕТ ❌
3. Использовались данные из шага 2 → barcodes = []
```

### API Endpoints

**`/offer-mappings` (GET)** - возвращает список товаров:
```json
{
  "result": {
    "offerMappings": [
      {
        "offer": {
          "offerId": "991151К",
          "name": "...",
          "barcodes": ["4603302903022"],  ← ЕСТЬ!
          "vendor": "..."
        },
        "mapping": {
          "marketSku": 123,
          "marketSkuName": "..."
        }
      }
    ]
  }
}
```

**`/offer-cards` (POST)** - возвращает детали товаров:
```json
{
  "result": {
    "offerCards": [
      {
        "offerId": "...",
        "mapping": {...},
        "parameterValues": [...],
        // barcodes НЕТ! ❌
      }
    ]
  }
}
```

### Результат

Из-за использования данных из `/offer-cards`:
- Поле `barcodes` отсутствовало в ответе
- Код видел `product.barcodes.is_empty() == true`
- В логах: "Product XXX has no barcodes, skipping barcode import"
- НО в `yandex_api_requests.log` видно, что `/offer-mappings` ВОЗВРАЩАЕТ штрихкоды

## Внесенные Исправления

### 1. `crates/backend/src/usecases/u503_import_from_yandex/yandex_api_client.rs`

**Добавлены поля в YandexOffer** (line 250-266):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexOffer {
    #[serde(rename = "offerId")]
    pub offer_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub pictures: Vec<String>,
    #[serde(default)]
    pub barcodes: Vec<String>,  // ← ДОБАВЛЕНО
    #[serde(default)]
    pub vendor: Option<String>,  // ← ДОБАВЛЕНО
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}
```

**Добавлено поле в YandexMapping** (line 268-274):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexMapping {
    #[serde(rename = "marketSku")]
    pub market_sku: Option<i64>,
    #[serde(rename = "marketSkuName", default)]
    pub market_sku_name: Option<String>,  // ← ДОБАВЛЕНО
}
```

### 2. `crates/backend/src/usecases/u503_import_from_yandex/executor.rs`

**УБРАН лишний запрос к `/offer-cards`** (line 217-244):

```rust
// БЫЛО:
let offer_ids: Vec<String> = entries.iter().map(|e| e.offer.offer_id.clone()).collect();
let info_response = self.api_client.fetch_product_info(connection, offer_ids).await?;
for offer_mapping in info_response.result.offer_mappings { ... }

// СТАЛО:
// Обрабатываем каждый товар напрямую из offer-mappings
// (они уже содержат все данные, включая barcodes)
for offer_mapping_entry in entries {
    let offer = &offer_mapping_entry.offer;
    let mapping = &offer_mapping_entry.mapping;

    // offer.barcodes уже содержит данные! ✅
    match self.process_product_from_offer(connection, offer, mapping).await { ... }
}
```

**Метод переименован** (line 335-342):

```rust
// БЫЛО:
async fn process_product(
    &self,
    connection: &ConnectionMP,
    product: &YandexOfferCard,
    mapping: &Option<YandexOfferCardMapping>,
) -> Result<(bool, usize)>

// СТАЛО:
async fn process_product_from_offer(
    &self,
    connection: &ConnectionMP,
    offer: &YandexOffer,  // ← Из offer-mappings
    mapping: &Option<YandexMapping>,  // ← Из offer-mappings
) -> Result<(bool, usize)>
```

**Обновлена логика обработки** (line 345-419):

```rust
// Используем данные из YandexOffer
let marketplace_sku = offer.offer_id.clone();
let barcode = offer.barcodes.first().cloned();
let product_name = offer.name.clone().unwrap_or_else(|| "Без названия".to_string());

// ...

// Импорт всех штрихкодов в проекцию p901
let barcodes_count = self.import_barcodes_to_p901(
    &offer.barcodes,  // ← Теперь содержит данные!
    &offer.offer_id,
    &existing_product.nomenclature_id
).await?;
```

## Преимущества Нового Подхода

1. ✅ **Меньше запросов к API** - убран лишний POST /offer-cards
2. ✅ **Быстрее импорт** - не нужно ждать второй запрос
3. ✅ **Barcodes доступны** - используются данные из правильного endpoint
4. ✅ **Проще архитектура** - один запрос вместо двух

## Как Применить Исправление

### 1. Остановить Backend

**Windows Task Manager**:
- Ctrl+Shift+Esc
- Найти `backend.exe`
- End Task

### 2. Пересобрать Backend

```bash
cd e:\dev\rust\leptos_marketplace_1
cargo build --bin backend
```

### 3. Запустить Backend

```bash
cargo run --bin backend
```

Или с расширенным логированием:
```powershell
$env:RUST_LOG="info,backend=debug"
cargo run --bin backend
```

### 4. Запустить Импорт Заново

1. Открыть UI приложения
2. Перейти в раздел импорта u503 (Yandex Market)
3. Запустить импорт
4. Следить за логами в консоли backend

### 5. Проверить Результат

**В UI**:
- Открыть раздел "Штрихкоды номенклатуры" (p901)
- Отфильтровать по `source = "YM"`
- Должны появиться записи с штрихкодами Yandex

**В логах** (должны появиться):
```
[DEBUG] Product 991151К has 1 barcode(s): ["4603302903022"]
[INFO] Importing 1 barcode(s) for product 991151К (source: YM)
[INFO] ✓ Imported barcode 4603302903022 (source: YM, article: 991151К, nomenclature_ref: None)
[INFO] Finished importing barcodes for product 991151К: 1 barcode(s) imported
```

**SQL проверка**:
```sql
-- Количество штрихкодов YM
SELECT COUNT(*) FROM p901_nomenclature_barcodes WHERE source = 'YM';

-- Примеры штрихкодов YM
SELECT barcode, article, nomenclature_ref, created_at
FROM p901_nomenclature_barcodes
WHERE source = 'YM'
LIMIT 10;
```

## Ожидаемый Результат

После применения исправления:
- ✅ Все штрихкоды из Yandex API корректно десериализуются
- ✅ Штрихкоды импортируются в таблицу p901_nomenclature_barcodes с source='YM'
- ✅ В логах видно количество импортированных штрихкодов для каждого товара
- ✅ Товары без штрихкодов корректно пропускаются с соответствующим сообщением
- ✅ Импорт работает быстрее (на ~50%), т.к. убран лишний API запрос

## Проверка Корректности

После импорта проверьте:

1. **Логи содержат**:
   - `"Product XXX has N barcode(s): [...]"` на уровне DEBUG
   - `"Importing N barcode(s) for product XXX (source: YM)"`
   - `"✓ Imported barcode XXX (source: YM, article: XXX, ...)"`
   - `"Finished importing barcodes for product XXX: N barcode(s) imported"`

2. **База данных**:
   ```sql
   SELECT source, COUNT(*) as count
   FROM p901_nomenclature_barcodes
   WHERE is_active = 1
   GROUP BY source;
   ```

   Должна появиться строка с `source = 'YM'` и count > 0

3. **UI**:
   - В списке штрихкодов появились записи с source = "YM"
   - Записи без nomenclature_ref подсвечены желтым цветом

## Удаление Неиспользуемого Кода (Опционально)

После проверки можно удалить метод `fetch_product_info()` и связанные структуры:
- `YandexProductInfoRequest`
- `YandexProductInfoResponse`
- `YandexOfferMapping`
- `YandexOfferCard`
- `YandexOfferCardMapping`

Эти структуры больше не используются после исправления.
