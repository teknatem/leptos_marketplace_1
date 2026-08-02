# Доработки UI для a012_wb_sales_detail

## Выполненные изменения

### 1. Товар маркетплейса - гиперссылка с выравниванием по левому краю ✅

**Файл**: `crates/frontend/src/domain/a012_wb_sales/ui/details/tabs/general.rs`

**Изменение**: Заменил кнопку `Button` на обычную гиперссылку `<a>` с:

- Выравниванием по левому краю
- Стилем бренда (`color: var(--color-brand)`)
- Курсором pointer
- Отображением "—" если ссылка отсутствует

### 2. Номенклатура 1С - гиперссылка с выравниванием по левому краю ✅

**Файл**: `crates/frontend/src/domain/a012_wb_sales/ui/details/tabs/general.rs`

**Изменение**: Аналогично п.1 - заменил кнопку на гиперссылку.

### 3. Проверка заполнения 'Дилерская цена УТ' ✅

**Проблема**: Поле не заполняется, хотя по артикулу ТВИЗШ70-445 цена должна быть.

**Решение**: Добавлена кнопка обновления дилерской цены с функционалом:

#### Backend изменения:

1. **Новая функция в service** (`crates/backend/src/domain/a012_wb_sales/service.rs`):

   ```rust
   pub async fn refresh_dealer_price(id: Uuid) -> Result<()>
   ```

   - Получает документ по ID
   - Вызывает `fill_dealer_price()` для обновления цены
   - Сохраняет документ обратно в базу

2. **Новый API endpoint** (`crates/backend/src/api/handlers/a012_wb_sales.rs`):
   ```rust
   pub async fn refresh_dealer_price(...)
   ```
   - POST `/api/a012/wb-sales/:id/refresh-dealer-price`
3. **Роут** (`crates/backend/src/api/routes.rs`):
   - Добавлен роут для нового endpoint

#### Frontend изменения:

1. **API функция** (`crates/frontend/src/domain/a012_wb_sales/ui/details/model.rs`):

   ```rust
   pub async fn refresh_dealer_price(id: &str) -> Result<(), String>
   ```

2. **ViewModel метод** (`crates/frontend/src/domain/a012_wb_sales/ui/details/view_model.rs`):

   - Добавлено состояние `refreshing_price: RwSignal<bool>`
   - Метод `refresh_dealer_price()` для вызова API и перезагрузки данных

3. **UI кнопка** (`crates/frontend/src/domain/a012_wb_sales/ui/details/tabs/line.rs`):
   - Кнопка "↻" рядом с полем "Дилерская цена УТ"
   - Показывает "..." во время обновления
   - Имеет tooltip с пояснением

## Логика заполнения дилерской цены

Функция `fill_dealer_price()` в `crates/backend/src/domain/a012_wb_sales/service.rs`:

1. Проверяет наличие `nomenclature_ref` в документе
2. Получает дату продажи из документа
3. Ищет цену в `p906_nomenclature_prices` по формуле:
   ```sql
   SELECT price FROM p906_nomenclature_prices
   WHERE nomenclature_ref = ? AND period <= sale_date
   ORDER BY period DESC
   LIMIT 1
   ```
4. Если не найдено, пробует найти по `base_nomenclature_ref`
5. Сохраняет найденную цену в поле `dealer_price_ut`

## Диагностика проблемы с артикулом ТВИЗШ70-445

Создан SQL файл `check_dealer_price_ТВИЗШ70-445.sql` с запросами для проверки:

1. Номенклатура с артикулом ТВИЗШ70-445
2. Marketplace product с этим артикулом
3. WB Sales документы с этим артикулом
4. Цены в p906_nomenclature_prices для этой номенклатуры
5. Цены для base_nomenclature_ref (если есть)
6. Объединенный запрос для полной диагностики

**Рекомендуется**:

1. Запустить SQL запросы из файла
2. Проверить, есть ли цены в `p906_nomenclature_prices`
3. Если цен нет - запустить импорт из УТ (u501_import_from_ut)
4. Нажать кнопку "↻" для обновления дилерской цены в документе

## Тестирование

1. Открыть документ WB Sales с артикулом ТВИЗШ70-445
2. Перейти на вкладку "Общие" - проверить гиперссылки
3. Перейти на вкладку "Подробно" - проверить поле "Дилерская цена УТ"
4. Нажать кнопку "↻" для обновления цены
5. Проверить, что цена обновилась (если есть данные в p906)

## Примечания

- Дилерская цена заполняется автоматически при импорте документов из WB API
- Если цена не заполнена, это означает:
  - Нет данных в `p906_nomenclature_prices` для этой номенклатуры
  - Нет данных для `base_nomenclature_ref`
  - Не заполнен `nomenclature_ref` в документе
- Кнопка обновления позволяет перезапросить цену после импорта данных в p906
