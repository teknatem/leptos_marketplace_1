use super::repository;
use contracts::domain::a007_marketplace_product::aggregate::{
    MarketplaceProduct, MarketplaceProductDto,
};
use uuid::Uuid;

/// Создание нового товара маркетплейса
pub async fn create(dto: MarketplaceProductDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("MP-PROD-{}", Uuid::new_v4()));

    let mut aggregate = MarketplaceProduct::new_for_insert(
        code,
        dto.description,
        dto.marketplace_id,
        dto.connection_mp_id,
        dto.marketplace_sku,
        dto.barcode,
        dto.art,
        dto.product_name,
        dto.brand,
        dto.category_id,
        dto.category_name,
        dto.price,
        dto.stock,
        dto.last_update,
        dto.marketplace_url,
        dto.nomenclature_id,
        dto.comment,
    );

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    // Сохранение через repository
    repository::insert(&aggregate).await
}

/// Обновление существующего товара маркетплейса
pub async fn update(dto: MarketplaceProductDto) -> anyhow::Result<()> {
    let id = dto
        .id
        .as_ref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid ID"))?;

    let mut aggregate = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Not found"))?;

    aggregate.update(&dto);

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    // Сохранение
    repository::update(&aggregate).await
}

/// Мягкое удаление товара маркетплейса
pub async fn delete(id: Uuid) -> anyhow::Result<bool> {
    repository::soft_delete(id).await
}

/// Получение товара по ID
pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<MarketplaceProduct>> {
    repository::get_by_id(id).await
}

/// Получение списка всех товаров маркетплейсов
pub async fn list_all() -> anyhow::Result<Vec<MarketplaceProduct>> {
    repository::list_all().await
}

/// Получение товара по SKU маркетплейса
pub async fn get_by_marketplace_sku(
    marketplace_id: &str,
    sku: &str,
) -> anyhow::Result<Option<MarketplaceProduct>> {
    repository::get_by_marketplace_sku(marketplace_id, sku).await
}

/// Получение товаров по штрихкоду
pub async fn get_by_barcode(barcode: &str) -> anyhow::Result<Vec<MarketplaceProduct>> {
    repository::get_by_barcode(barcode).await
}

/// Получение товаров маркетплейса
pub async fn list_by_marketplace_id(
    marketplace_id: &str,
) -> anyhow::Result<Vec<MarketplaceProduct>> {
    repository::list_by_marketplace_id(marketplace_id).await
}

/// Вставка тестовых данных
pub async fn insert_test_data() -> anyhow::Result<()> {
    // Получаем ID маркетплейсов (предполагаем, что они уже созданы)
    let data = vec![
        MarketplaceProductDto {
            id: None,
            code: Some("mp-wb-001".into()),
            description: "Тестовый товар Wildberries".into(),
            marketplace_id: "marketplace-wb-id".into(), // Здесь нужен реальный ID из a005
            connection_mp_id: "connection-mp-id".into(), // Здесь нужен реальный ID из a006
            marketplace_sku: "WB12345678".into(),
            barcode: Some("4607012345678".into()),
            art: "ART-WB-001".into(),
            product_name: "Тестовый товар Wildberries".into(),
            brand: Some("Test Brand".into()),
            category_id: Some("CAT-123".into()),
            category_name: Some("Электроника".into()),
            price: Some(1299.99),
            stock: Some(150),
            last_update: Some(chrono::Utc::now()),
            marketplace_url: Some("https://www.wildberries.ru/catalog/12345678/detail.aspx".into()),
            nomenclature_id: None,
            comment: Some("Тестовый товар для демонстрации".into()),
        },
        MarketplaceProductDto {
            id: None,
            code: Some("mp-ozon-001".into()),
            description: "Тестовый товар Ozon".into(),
            marketplace_id: "marketplace-ozon-id".into(), // Здесь нужен реальный ID из a005
            connection_mp_id: "connection-mp-id".into(), // Здесь нужен реальный ID из a006
            marketplace_sku: "OZON87654321".into(),
            barcode: Some("4607087654321".into()),
            art: "ART-OZON-001".into(),
            product_name: "Тестовый товар Ozon".into(),
            brand: Some("Another Brand".into()),
            category_id: Some("CAT-456".into()),
            category_name: Some("Одежда".into()),
            price: Some(2499.50),
            stock: Some(75),
            last_update: Some(chrono::Utc::now()),
            marketplace_url: Some("https://www.ozon.ru/product/87654321".into()),
            nomenclature_id: None,
            comment: Some("Тестовый товар для демонстрации".into()),
        },
    ];

    for dto in data {
        create(dto).await?;
    }

    Ok(())
}
