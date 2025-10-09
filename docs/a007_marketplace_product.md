# Aggregate a007_marketplace_product

## Описание

Агрегат для консолидации данных по номенклатуре со всех маркетплейсов. Позволяет загружать информацию о товарах с различных торговых площадок и сопоставлять их с номенклатурой 1С.

## Структура данных

| Поле                | Тип          | Обязательное | Описание                                                |
|---------------------|--------------|--------------|--------------------------------------------------------|
| id                  | UUID         | Да           | Уникальный идентификор записи                           |
| code                | String       | Да           | Код товара                                              |
| description         | String       | Да           | Краткое описание                                        |
| comment             | String       | Нет          | Комментарий                                             |
| marketplace_id      | String       | Да           | ID маркетплейса (связь с a005_marketplace)              |
| marketplace_sku     | String       | Да           | Внутренний ID товара в маркетплейсе                     |
| barcode             | String       | Нет          | Штрихкод товара                                         |
| art                 | String       | Да           | Артикул на маркетплейсе                                 |
| product_name        | String       | Да           | Наименование товара на маркетплейсе                     |
| brand               | String       | Нет          | Бренд товара                                            |
| category_id         | String       | Нет          | ID категории товара на маркетплейсе                     |
| category_name       | String       | Нет          | Текстовое название категории                            |
| price               | f64          | Нет          | Текущая цена по маркетплейсу                            |
| stock               | i32          | Нет          | Наличие на складе / остаток                             |
| last_update         | DateTime     | Нет          | Дата последнего обновления информации с маркетплейса    |
| marketplace_url     | String       | Нет          | Ссылка на товар на маркетплейсе                         |
| nomenclature_id     | String       | Нет          | ID товара в собственной базе (связь с a004_nomenclature)|

## Валидация

- Описание, код, marketplace_id, marketplace_sku, артикул и наименование товара не могут быть пустыми
- URL должен начинаться с http:// или https:// (если указан)
- Цена не может быть отрицательной
- Остаток не может быть отрицательным

## REST API Endpoints

### Получить все товары
```
GET /api/marketplace_product
```

### Получить товар по ID
```
GET /api/marketplace_product/:id
```

### Создать/обновить товар
```
POST /api/marketplace_product
Content-Type: application/json

{
  "id": "optional-uuid",
  "code": "optional-code",
  "description": "Название товара",
  "marketplaceId": "uuid-маркетплейса",
  "marketplaceSku": "SKU123456",
  "barcode": "4607012345678",
  "art": "ART-001",
  "productName": "Название на маркетплейсе",
  "brand": "Бренд",
  "categoryId": "CAT-123",
  "categoryName": "Категория",
  "price": 1299.99,
  "stock": 100,
  "lastUpdate": "2025-01-01T12:00:00Z",
  "marketplaceUrl": "https://marketplace.com/product/123",
  "nomenclatureId": "uuid-номенклатуры",
  "comment": "Комментарий"
}
```

### Удалить товар (мягкое удаление)
```
DELETE /api/marketplace_product/:id
```

### Вставить тестовые данные
```
POST /api/marketplace_product/testdata
```

## Дополнительные методы сервиса

- `get_by_marketplace_sku(marketplace_id, sku)` - поиск товара по SKU маркетплейса
- `get_by_barcode(barcode)` - поиск товаров по штрихкоду
- `list_by_marketplace_id(marketplace_id)` - получить все товары конкретного маркетплейса

## Связи с другими агрегатами

- **a005_marketplace**: Связь через поле `marketplace_id` - определяет с какого маркетплейса загружен товар
- **a004_nomenclature**: Связь через поле `nomenclature_id` - сопоставление с номенклатурой 1С

## Происхождение данных (Origin)

`Origin::Marketplace` - данные загружаются с маркетплейсов

## Файлы

### Contracts (Shared types)
- `crates/contracts/src/domain/a007_marketplace_product/aggregate.rs` - Aggregate root и DTO
- `crates/contracts/src/domain/a007_marketplace_product/mod.rs` - Module export

### Backend
- `crates/backend/src/domain/a007_marketplace_product/repository.rs` - Sea-ORM модель и CRUD операции
- `crates/backend/src/domain/a007_marketplace_product/service.rs` - Бизнес-логика
- `crates/backend/src/domain/a007_marketplace_product/mod.rs` - Module export

### Database
- Таблица: `a007_marketplace_product` в SQLite
- Создание схемы: `crates/backend/src/shared/data/db.rs`

### REST API
- Endpoints: `crates/backend/src/main.rs`

## Примечания

- Агрегат использует стандартный паттерн BaseAggregate с метаданными (created_at, updated_at, is_deleted, version)
- Поддерживается мягкое удаление (soft delete)
- Timestamp last_update обновляется при загрузке данных с маркетплейса
- Связь с маркетплейсами и номенклатурой 1С реализована через строковые UUID поля
