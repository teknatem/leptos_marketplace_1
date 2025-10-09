use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::domain::common::{AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin};

// ============================================================================
// ID Type
// ============================================================================

/// Уникальный идентификатор товара маркетплейса
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarketplaceProductId(pub Uuid);

impl MarketplaceProductId {
    pub fn new(value: Uuid) -> Self {
        Self(value)
    }

    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl AggregateId for MarketplaceProductId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }

    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(MarketplaceProductId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

// ============================================================================
// Aggregate Root
// ============================================================================

/// Товар маркетплейса (консолидация номенклатуры)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceProduct {
    #[serde(flatten)]
    pub base: BaseAggregate<MarketplaceProductId>,

    /// ID маркетплейса (ссылка на a005_marketplace)
    #[serde(rename = "marketplaceId")]
    pub marketplace_id: String,

    /// Внутренний ID товара в маркетплейсе
    #[serde(rename = "marketplaceSku")]
    pub marketplace_sku: String,

    /// Штрихкод товара
    pub barcode: Option<String>,

    /// Артикул на маркетплейсе
    pub art: String,

    /// Наименование товара на маркетплейсе
    #[serde(rename = "productName")]
    pub product_name: String,

    /// Бренд товара
    pub brand: Option<String>,

    /// ID категории товара на маркетплейсе
    #[serde(rename = "categoryId")]
    pub category_id: Option<String>,

    /// Текстовое название категории
    #[serde(rename = "categoryName")]
    pub category_name: Option<String>,

    /// Текущая цена по маркетплейсу
    pub price: Option<f64>,

    /// Наличие на складе / остаток
    pub stock: Option<i32>,

    /// Дата последнего обновления информации с маркетплейса
    #[serde(rename = "lastUpdate")]
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,

    /// Ссылка на товар на маркетплейсе
    #[serde(rename = "marketplaceUrl")]
    pub marketplace_url: Option<String>,

    /// Уникальный ID товара в собственной базе (ссылка на a004_nomenclature)
    #[serde(rename = "nomenclatureId")]
    pub nomenclature_id: Option<String>,
}

impl MarketplaceProduct {
    /// Создать новый товар маркетплейса для вставки в БД
    #[allow(clippy::too_many_arguments)]
    pub fn new_for_insert(
        code: String,
        description: String,
        marketplace_id: String,
        marketplace_sku: String,
        barcode: Option<String>,
        art: String,
        product_name: String,
        brand: Option<String>,
        category_id: Option<String>,
        category_name: Option<String>,
        price: Option<f64>,
        stock: Option<i32>,
        last_update: Option<chrono::DateTime<chrono::Utc>>,
        marketplace_url: Option<String>,
        nomenclature_id: Option<String>,
        comment: Option<String>,
    ) -> Self {
        let mut base = BaseAggregate::new(
            MarketplaceProductId::new_v4(),
            code,
            description,
        );
        base.comment = comment;

        Self {
            base,
            marketplace_id,
            marketplace_sku,
            barcode,
            art,
            product_name,
            brand,
            category_id,
            category_name,
            price,
            stock,
            last_update,
            marketplace_url,
            nomenclature_id,
        }
    }

    /// Создать товар с заданным UUID
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_id(
        id: MarketplaceProductId,
        code: String,
        description: String,
        marketplace_id: String,
        marketplace_sku: String,
        barcode: Option<String>,
        art: String,
        product_name: String,
        brand: Option<String>,
        category_id: Option<String>,
        category_name: Option<String>,
        price: Option<f64>,
        stock: Option<i32>,
        last_update: Option<chrono::DateTime<chrono::Utc>>,
        marketplace_url: Option<String>,
        nomenclature_id: Option<String>,
        comment: Option<String>,
    ) -> Self {
        let mut base = BaseAggregate::new(
            id,
            code,
            description,
        );
        base.comment = comment;

        Self {
            base,
            marketplace_id,
            marketplace_sku,
            barcode,
            art,
            product_name,
            brand,
            category_id,
            category_name,
            price,
            stock,
            last_update,
            marketplace_url,
            nomenclature_id,
        }
    }

    /// Получить ID как строку
    pub fn to_string_id(&self) -> String {
        self.base.id.as_string()
    }

    /// Обновить timestamp
    pub fn touch_updated(&mut self) {
        self.base.touch();
    }

    /// Обновить данные из DTO
    pub fn update(&mut self, dto: &MarketplaceProductDto) {
        self.base.code = dto.code.clone().unwrap_or_default();
        self.base.description = dto.description.clone();
        self.base.comment = dto.comment.clone();
        self.marketplace_id = dto.marketplace_id.clone();
        self.marketplace_sku = dto.marketplace_sku.clone();
        self.barcode = dto.barcode.clone();
        self.art = dto.art.clone();
        self.product_name = dto.product_name.clone();
        self.brand = dto.brand.clone();
        self.category_id = dto.category_id.clone();
        self.category_name = dto.category_name.clone();
        self.price = dto.price;
        self.stock = dto.stock;
        self.last_update = dto.last_update;
        self.marketplace_url = dto.marketplace_url.clone();
        self.nomenclature_id = dto.nomenclature_id.clone();
    }

    /// Валидация данных
    pub fn validate(&self) -> Result<(), String> {
        if self.base.description.trim().is_empty() {
            return Err("Описание не может быть пустым".into());
        }
        if self.base.code.trim().is_empty() {
            return Err("Код не может быть пустым".into());
        }
        if self.marketplace_id.trim().is_empty() {
            return Err("ID маркетплейса не может быть пустым".into());
        }
        if self.marketplace_sku.trim().is_empty() {
            return Err("SKU маркетплейса не может быть пустым".into());
        }
        if self.art.trim().is_empty() {
            return Err("Артикул не может быть пустым".into());
        }
        if self.product_name.trim().is_empty() {
            return Err("Наименование товара не может быть пустым".into());
        }

        // Валидация URL если указан
        if let Some(url) = &self.marketplace_url {
            if !url.trim().is_empty()
                && !url.starts_with("http://")
                && !url.starts_with("https://")
            {
                return Err("URL должен начинаться с http:// или https://".into());
            }
        }

        // Валидация цены
        if let Some(price) = self.price {
            if price < 0.0 {
                return Err("Цена не может быть отрицательной".into());
            }
        }

        // Валидация остатка
        if let Some(stock) = self.stock {
            if stock < 0 {
                return Err("Остаток не может быть отрицательным".into());
            }
        }

        Ok(())
    }

    /// Хук перед записью
    pub fn before_write(&mut self) {
        self.touch_updated();
    }
}

impl AggregateRoot for MarketplaceProduct {
    type Id = MarketplaceProductId;

    fn id(&self) -> Self::Id {
        self.base.id
    }

    fn code(&self) -> &str {
        &self.base.code
    }

    fn description(&self) -> &str {
        &self.base.description
    }

    fn metadata(&self) -> &EntityMetadata {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut EntityMetadata {
        &mut self.base.metadata
    }

    fn events(&self) -> &EventStore {
        &self.base.events
    }

    fn events_mut(&mut self) -> &mut EventStore {
        &mut self.base.events
    }

    fn aggregate_index() -> &'static str {
        "a007"
    }

    fn collection_name() -> &'static str {
        "marketplace_product"
    }

    fn element_name() -> &'static str {
        "Товар маркетплейса"
    }

    fn list_name() -> &'static str {
        "Товары маркетплейсов"
    }

    fn origin() -> Origin {
        Origin::Marketplace
    }
}

// ============================================================================
// Forms / DTOs
// ============================================================================

/// DTO для создания/обновления товара маркетплейса
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketplaceProductDto {
    pub id: Option<String>,
    pub code: Option<String>,
    pub description: String,
    #[serde(rename = "marketplaceId")]
    pub marketplace_id: String,
    #[serde(rename = "marketplaceSku")]
    pub marketplace_sku: String,
    pub barcode: Option<String>,
    pub art: String,
    #[serde(rename = "productName")]
    pub product_name: String,
    pub brand: Option<String>,
    #[serde(rename = "categoryId")]
    pub category_id: Option<String>,
    #[serde(rename = "categoryName")]
    pub category_name: Option<String>,
    pub price: Option<f64>,
    pub stock: Option<i32>,
    #[serde(rename = "lastUpdate")]
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "marketplaceUrl")]
    pub marketplace_url: Option<String>,
    #[serde(rename = "nomenclatureId")]
    pub nomenclature_id: Option<String>,
    pub comment: Option<String>,
}
