# Инструкция по Проверке Данных

## Проблема
После импорта товаров из Yandex Market в таблицу p901_nomenclature_barcodes не попали штрихкоды с source='YM'.

## Возможные Причины

### 1. У товаров нет штрихкодов
Yandex API может возвращать пустой массив `barcodes: []` для некоторых товаров.

**Как проверить:**
- Посмотреть в логах backend строки: `"Product XXX has no barcodes, skipping barcode import"`
- Проверить API response от Yandex в логах

### 2. Backend не был перезапущен после изменений
Старая версия кода без импорта штрихкодов все еще работает.

**Решение:**
```bash
# Остановить текущий backend процесс
# В Windows через Task Manager найти backend.exe и завершить

# Пересобрать и запустить
cd e:\dev\rust\leptos_marketplace_1
cargo build --bin backend
cargo run --bin backend
```

### 3. Ошибка при импорте штрихкодов
Функция `import_barcodes_to_p901` вызывается, но падает с ошибкой.

**Как проверить логи:**
После перезапуска backend с новым кодом, при импорте должны появиться строки:
- `"Importing N barcode(s) for product XXX (source: YM)"`
- `"✓ Imported barcode XXX (source: YM, article: XXX, nomenclature_ref: None)"`
- `"Finished importing barcodes for product XXX: N barcode(s) imported"`

Если видны ошибки:
- `"Failed to create entry for barcode XXX"` - проблема с валидацией
- `"Failed to upsert barcode XXX to database"` - проблема с БД

### 4. Импорт товаров произошел ДО внесения изменений
Товары были импортированы старой версией кода без функции импорта штрихкодов.

**Решение:**
Запустить повторный импорт товаров через UI:
1. Открыть UI приложения
2. Перейти в раздел импорта u503 Yandex Market
3. Запустить импорт заново
4. Следить за логами backend

## Рекомендуемый План Действий

1. **Остановить backend процесс**
   - Task Manager → найти `backend.exe` → End Task

2. **Пересобрать с новым кодом**
   ```bash
   cd e:\dev\rust\leptos_marketplace_1
   cargo build --bin backend
   ```

3. **Запустить backend с логами**
   ```bash
   cargo run --bin backend
   ```
   Или через PowerShell:
   ```powershell
   $env:RUST_LOG="info,backend=debug"
   cargo run --bin backend
   ```

4. **Запустить НОВЫЙ импорт товаров YM**
   - Открыть UI
   - Запустить импорт u503
   - Следить за логами в консоли backend

5. **Проверить результат**
   - Открыть UI → Штрихкоды номенклатуры (p901)
   - Отфильтровать по source = "YM"
   - Должны появиться новые записи

## Проверка через SQL (если есть sqlite3)

```sql
-- Количество штрихкодов YM
SELECT COUNT(*) FROM p901_nomenclature_barcodes WHERE source = 'YM';

-- Примеры штрихкодов YM
SELECT barcode, article, nomenclature_ref, created_at
FROM p901_nomenclature_barcodes
WHERE source = 'YM'
LIMIT 10;

-- Товары YM из a007
SELECT marketplace_sku, barcode, product_name
FROM a007_marketplace_product
WHERE marketplace_id = 'YM'
LIMIT 10;
```

## Примеры Логов

**Успешный импорт:**
```
[INFO] Importing 3 barcode(s) for product SKU-123 (source: YM)
[INFO] ✓ Imported barcode 4680038522234 (source: YM, article: SKU-123, nomenclature_ref: None)
[INFO] ✓ Imported barcode 4680038522241 (source: YM, article: SKU-123, nomenclature_ref: None)
[INFO] ✓ Imported barcode 4680038522258 (source: YM, article: SKU-123, nomenclature_ref: None)
[INFO] Finished importing barcodes for product SKU-123: 3 barcode(s) imported
```

**Товар без штрихкодов:**
```
[INFO] Product SKU-456 has no barcodes, skipping barcode import
```

**Ошибка валидации:**
```
[ERROR] Failed to create entry for barcode invalid@barcode: Barcode contains invalid characters
```
