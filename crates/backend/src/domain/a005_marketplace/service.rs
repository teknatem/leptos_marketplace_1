use super::repository;
use contracts::domain::a005_marketplace::aggregate::{Marketplace, MarketplaceDto};
use contracts::enums::marketplace_type::MarketplaceType;
use uuid::Uuid;

/// Создание нового маркетплейса
pub async fn create(dto: MarketplaceDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("MP-{}", Uuid::new_v4()));
    let mut aggregate = Marketplace::new_for_insert(
        code,
        dto.description,
        dto.url,
        dto.logo_path,
        dto.marketplace_type,
        dto.comment,
        dto.acquiring_fee_pro,
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

/// Обновление существующего маркетплейса
pub async fn update(dto: MarketplaceDto) -> anyhow::Result<()> {
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

/// Мягкое удаление маркетплейса
pub async fn delete(id: Uuid) -> anyhow::Result<bool> {
    repository::soft_delete(id).await
}

/// Получение маркетплейса по ID
pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<Marketplace>> {
    repository::get_by_id(id).await
}

/// Получение списка всех маркетплейсов
pub async fn list_all() -> anyhow::Result<Vec<Marketplace>> {
    repository::list_all().await
}

/// Вставка тестовых данных
pub async fn insert_test_data() -> anyhow::Result<()> {
    let data = vec![
        MarketplaceDto {
            id: None,
            code: Some("mp-wb".into()),
            description: "Wildberries".into(),
            url: "https://www.wildberries.ru".into(),
            logo_path: Some("/assets/images/Wildberries.svg".into()),
            marketplace_type: Some(MarketplaceType::Wildberries),
            comment: Some("Крупнейший маркетплейс России".into()),
            acquiring_fee_pro: 0.0,
        },
        MarketplaceDto {
            id: None,
            code: Some("mp-ozon".into()),
            description: "Ozon".into(),
            url: "https://www.ozon.ru".into(),
            logo_path: Some("/assets/images/OZON.svg".into()),
            marketplace_type: Some(MarketplaceType::Ozon),
            comment: Some("Один из ведущих маркетплейсов".into()),
            acquiring_fee_pro: 0.0,
        },
        MarketplaceDto {
            id: None,
            code: Some("mp-ym".into()),
            description: "Яндекс.Маркет".into(),
            url: "https://market.yandex.ru".into(),
            logo_path: Some("/assets/images/Yandex.svg".into()),
            marketplace_type: Some(MarketplaceType::YandexMarket),
            comment: None,
            acquiring_fee_pro: 0.0,
        },
        MarketplaceDto {
            id: None,
            code: Some("mp-kuper".into()),
            description: "Kuper".into(),
            url: "https://kuper.ru".into(),
            logo_path: Some("/assets/images/Kuper.svg".into()),
            marketplace_type: Some(MarketplaceType::Kuper),
            comment: None,
            acquiring_fee_pro: 0.0,
        },
        MarketplaceDto {
            id: None,
            code: Some("mp-lemana".into()),
            description: "Lemana Pro".into(),
            url: "https://lemanapro.ru".into(),
            logo_path: Some("/assets/images/lemanapro.svg".into()),
            marketplace_type: Some(MarketplaceType::LemanaPro),
            comment: None,
            acquiring_fee_pro: 0.0,
        },
    ];

    for dto in data {
        create(dto).await?;
    }

    Ok(())
}
