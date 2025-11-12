# Исправление Импорта Штрихкодов из Yandex Market

## Найденная Проблема

**Причина**: Структура десериализации не соответствует фактическому ответу API Yandex Market.

### Что было

Код ожидал структуру:
```json
{
  "result": {
    "offerCards": [
      {
        "offerId": "...",
        "barcodes": ["..."],
        "mapping": {...}
      }
    ]
  }
}
```

### Что возвращает API

API фактически возвращает:
```json
{
  "result": {
    "offerMappings": [
      {
        "offer": {
          "offerId": "...",
          "barcodes": ["..."]
        },
        "mapping": {...}
      }
    ]
  }
}
```

### Результат

Из-за несоответствия структур:
- Поле `barcodes` не десериализовалось (использовался `#[serde(default)]` → пустой массив)
- Код видел `product.barcodes.is_empty() == true`
- В логах: "Product XXX has no barcodes, skipping barcode import"
- НО в файле `yandex_api_requests.log` видно, что API ВОЗВРАЩАЕТ штрихкоды

### Пример

Товар `991151К`:
- **В API логах**: `"barcodes": ["4603302903022"]` ✅
- **В коде**: `product.barcodes = []` ❌ (из-за неправильной десериализации)

## Внесенные Исправления

### 1. `crates/backend/src/usecases/u503_import_from_yandex/yandex_api_client.rs`

**Изменено**:

```rust
// БЫЛО:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexProductInfoResult {
    #[serde(rename = "offerCards")]
    pub offer_cards: Vec<YandexOfferCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexOfferCard {
    #[serde(rename = "offerId")]
    pub offer_id: String,
    #[serde(default)]
    pub mapping: Option<YandexOfferCardMapping>,
    // ...
}

// СТАЛО:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexProductInfoResult {
    #[serde(rename = "offerMappings")]
    pub offer_mappings: Vec<YandexOfferMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexOfferMapping {
    pub offer: YandexOfferCard,
    #[serde(default)]
    pub mapping: Option<YandexOfferCardMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexOfferCard {
    #[serde(rename = "offerId")]
    pub offer_id: String,
    // mapping убран из offer, теперь в YandexOfferMapping
    // ...
}
```

### 2. `crates/backend/src/usecases/u503_import_from_yandex/executor.rs`

**Изменено**:

```rust
// БЫЛО:
for offer_card in info_response.result.offer_cards {
    let product_name = offer_card.mapping.as_ref()...;
    match self.process_product(connection, &offer_card).await {

// СТАЛО:
for offer_mapping in info_response.result.offer_mappings {
    let offer_card = &offer_mapping.offer;
    let mapping = &offer_mapping.mapping;

    let product_name = mapping.as_ref()...;
    match self.process_product(connection, offer_card, mapping).await {
```

**Сигнатура метода**:
```rust
// БЫЛО:
async fn process_product(
    &self,
    connection: &ConnectionMP,
    product: &YandexOfferCard,
) -> Result<(bool, usize)>

// СТАЛО:
async fn process_product(
    &self,
    connection: &ConnectionMP,
    product: &YandexOfferCard,
    mapping: &Option<YandexOfferCardMapping>,  // Добавлен параметр
) -> Result<(bool, usize)>
```

**Использование mapping**:
```rust
// БЫЛО:
let product_name = product.mapping.as_ref()...;
let (category_id, category_name) = product.mapping.as_ref()...;

// СТАЛО:
let product_name = mapping.as_ref()...;
let (category_id, category_name) = mapping.as_ref()...;
```

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

## Проверка Корректности

После импорта проверьте:

1. **Логи содержат**:
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
