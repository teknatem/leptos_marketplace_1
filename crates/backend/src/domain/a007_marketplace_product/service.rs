use super::repository;
use contracts::domain::a007_marketplace_product::aggregate::{
    MarketplaceProduct, MarketplaceProductDto,
};
use contracts::domain::common::AggregateId;
use uuid::Uuid;

/// Поиск и установка nomenclature_ref по артикулу
/// Возвращает true если nomenclature_ref был установлен, false если нет
pub async fn search_and_set_nomenclature(aggregate: &mut MarketplaceProduct) -> anyhow::Result<bool> {
    // Если nomenclature_ref уже заполнен, ничего не делаем
    if aggregate.nomenclature_ref.is_some() {
        return Ok(false);
    }

    let article = aggregate.article.trim();
    if article.is_empty() {
        return Ok(false);
    }

    // Ищем по артикулу
    let found_items = crate::domain::a004_nomenclature::repository::find_by_article(article).await?;

    // Если найдено ровно 1 - устанавливаем
    if found_items.len() == 1 {
        let nomenclature_id = found_items[0].base.id.as_string();
        aggregate.nomenclature_ref = Some(nomenclature_id);
        tracing::info!(
            "Auto-matched article '{}' to nomenclature '{}'",
            article,
            found_items[0].base.description
        );
        return Ok(true);
    }

    // В остальных случаях (0 или N) оставляем пустым
    if found_items.is_empty() {
        tracing::debug!("No nomenclature found for article '{}'", article);
    } else {
        tracing::warn!(
            "Found {} nomenclatures for article '{}', manual selection required",
            found_items.len(),
            article
        );
    }

    Ok(false)
}

/// Создание нового товара маркетплейса
pub async fn create(dto: MarketplaceProductDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("MP-PROD-{}", Uuid::new_v4()));

    let mut aggregate = MarketplaceProduct::new_for_insert(
        code,
        dto.description,
        dto.marketplace_ref,
        dto.connection_mp_ref,
        dto.marketplace_sku,
        dto.barcode,
        dto.article,
        dto.brand,
        dto.category_id,
        dto.category_name,
        dto.last_update,
        dto.nomenclature_ref,
        dto.comment,
    );

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Автоматический поиск номенклатуры если не задано
    search_and_set_nomenclature(&mut aggregate).await?;

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

    // Автоматический поиск номенклатуры если не задано
    search_and_set_nomenclature(&mut aggregate).await?;

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

/// Получение товара по connection_mp_ref и SKU
pub async fn get_by_connection_and_sku(
    connection_mp_ref: &str,
    sku: &str,
) -> anyhow::Result<Option<MarketplaceProduct>> {
    repository::get_by_connection_and_sku(connection_mp_ref, sku).await
}

/// Получение товаров по штрихкоду
pub async fn get_by_barcode(barcode: &str) -> anyhow::Result<Vec<MarketplaceProduct>> {
    repository::get_by_barcode(barcode).await
}

/// Получение товаров маркетплейса
pub async fn list_by_marketplace_ref(
    marketplace_ref: &str,
) -> anyhow::Result<Vec<MarketplaceProduct>> {
    repository::list_by_marketplace_ref(marketplace_ref).await
}

/// Параметры для поиска/создания товара при импорте продаж
pub struct FindOrCreateParams {
    pub marketplace_ref: String,
    pub connection_mp_ref: String,
    pub marketplace_sku: String,
    pub barcode: Option<String>,
    pub title: String,
}

/// Найти или создать a007_marketplace_product для регистра продаж
///
/// Алгоритм поиска:
/// 1. Поиск по (connection_mp_ref, marketplace_sku)
/// 2. Если не найден и есть barcode - поиск через p901 по штрихкоду маркетплейса
/// 3. Если не найден - создание нового a007 с комментарием
///
/// Возвращает UUID найденного или созданного товара
pub async fn find_or_create_for_sale(params: FindOrCreateParams) -> anyhow::Result<Uuid> {
    // Шаг 1: Поиск по (connection_mp_ref, marketplace_sku)
    if let Some(existing) = repository::get_by_connection_and_sku(
        &params.connection_mp_ref,
        &params.marketplace_sku,
    )
    .await?
    {
        return Ok(existing.base.id.value());
    }

    // Шаг 2: Если есть barcode - поиск через p901
    if let Some(ref barcode) = params.barcode {
        // Определяем источник для p901 по marketplace_ref
        let source = match params.marketplace_ref.as_str() {
            id if id.contains("ozon") => "OZON",
            id if id.contains("wb") => "WB",
            id if id.contains("ym") => "YM",
            _ => "UNKNOWN",
        };

        // Ищем nomenclature_ref через p901
        let nomenclature_ref =
            crate::projections::p901_nomenclature_barcodes::service::find_nomenclature_ref_by_barcode_from_marketplace(
                barcode,
                source,
            )
            .await?;

        // Если нашли nomenclature_ref - ищем a007 с этим nomenclature_ref
        if let Some(ref nom_ref) = nomenclature_ref {
            let products = repository::get_by_nomenclature_ref(nom_ref).await?;

            // Фильтруем по marketplace_ref, берем первый подходящий
            if let Some(existing) = products
                .into_iter()
                .find(|p| p.marketplace_ref == params.marketplace_ref)
            {
                return Ok(existing.base.id.value());
            }
        }
    }

    // Шаг 3: Не найден - создаем новый
    let now = chrono::Utc::now();
    let comment = format!(
        "Автоматически создано при импорте продаж [{}]",
        now.format("%Y-%m-%d %H:%M:%S UTC")
    );

    let dto = MarketplaceProductDto {
        id: None,
        code: Some(format!("MP-AUTO-{}", Uuid::new_v4())),
        description: params.title.clone(),
        marketplace_ref: params.marketplace_ref.clone(),
        connection_mp_ref: params.connection_mp_ref,
        marketplace_sku: params.marketplace_sku.clone(),
        barcode: params.barcode,
        article: params.marketplace_sku, // Используем marketplace_sku как артикул
        brand: None,
        category_id: None,
        category_name: None,
        last_update: Some(now),
        nomenclature_ref: None, // Сопоставление через u505
        comment: Some(comment),
    };

    create(dto).await
}

/// Вставка тестовых данных
pub async fn insert_test_data() -> anyhow::Result<()> {
    // Получаем ID маркетплейсов (предполагаем, что они уже созданы)
    let data = vec![
        MarketplaceProductDto {
            id: None,
            code: Some("mp-wb-001".into()),
            description: "Тестовый товар Wildberries".into(),
            marketplace_ref: "marketplace-wb-id".into(), // Здесь нужен реальный ID из a005
            connection_mp_ref: "connection-mp-id".into(), // Здесь нужен реальный ID из a006
            marketplace_sku: "WB12345678".into(),
            barcode: Some("4607012345678".into()),
            article: "ART-WB-001".into(),
            brand: Some("Test Brand".into()),
            category_id: Some("CAT-123".into()),
            category_name: Some("Электроника".into()),
            last_update: Some(chrono::Utc::now()),
            nomenclature_ref: None,
            comment: Some("Тестовый товар для демонстрации".into()),
        },
        MarketplaceProductDto {
            id: None,
            code: Some("mp-ozon-001".into()),
            description: "Тестовый товар Ozon".into(),
            marketplace_ref: "marketplace-ozon-id".into(), // Здесь нужен реальный ID из a005
            connection_mp_ref: "connection-mp-id".into(), // Здесь нужен реальный ID из a006
            marketplace_sku: "OZON87654321".into(),
            barcode: Some("4607087654321".into()),
            article: "ART-OZON-001".into(),
            brand: Some("Another Brand".into()),
            category_id: Some("CAT-456".into()),
            category_name: Some("Одежда".into()),
            last_update: Some(chrono::Utc::now()),
            nomenclature_ref: None,
            comment: Some("Тестовый товар для демонстрации".into()),
        },
    ];

    for dto in data {
        create(dto).await?;
    }

    Ok(())
}
