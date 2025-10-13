use crate::domain::common::{
    AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ID типа для записи возвратов OZON
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OzonReturnsId(pub Uuid);

impl OzonReturnsId {
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

impl AggregateId for OzonReturnsId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }
    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(OzonReturnsId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

/// Возврат товара с OZON (агрегат)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonReturns {
    #[serde(flatten)]
    pub base: BaseAggregate<OzonReturnsId>,

    /// Ссылка на подключение МП (a006_connection_mp.id)
    #[serde(rename = "connectionId")]
    pub connection_id: String,

    /// Ссылка на организацию (a002_organization.id)
    #[serde(rename = "organizationId")]
    pub organization_id: String,

    /// Ссылка на маркетплейс (a005_marketplace.id)
    #[serde(rename = "marketplaceId")]
    pub marketplace_id: String,

    /// ID возврата из OZON
    #[serde(rename = "returnId")]
    pub return_id: String,

    /// Дата возврата (YYYY-MM-DD)
    #[serde(with = "serde_date")]
    #[serde(rename = "returnDate")]
    pub return_date: chrono::NaiveDate,

    /// Причина возврата
    #[serde(rename = "returnReasonName")]
    pub return_reason_name: String,

    /// Тип возврата (FullReturn, PartialReturn и т.д.)
    #[serde(rename = "returnType")]
    pub return_type: String,

    /// ID заказа
    #[serde(rename = "orderId")]
    pub order_id: String,

    /// Номер заказа
    #[serde(rename = "orderNumber")]
    pub order_number: String,

    /// Артикул товара (SKU)
    pub sku: String,

    /// Название товара
    #[serde(rename = "productName")]
    pub product_name: String,

    /// Цена товара
    pub price: f64,

    /// Количество возвращенных единиц
    pub quantity: i32,

    /// Номер отправления
    #[serde(rename = "postingNumber")]
    pub posting_number: String,

    /// ID клиринга
    #[serde(rename = "clearingId")]
    pub clearing_id: Option<String>,

    /// ID клиринга возврата
    #[serde(rename = "returnClearingId")]
    pub return_clearing_id: Option<String>,
}

impl OzonReturns {
    #[allow(clippy::too_many_arguments)]
    pub fn new_for_insert(
        code: String,
        description: String,
        connection_id: String,
        organization_id: String,
        marketplace_id: String,
        return_id: String,
        return_date: chrono::NaiveDate,
        return_reason_name: String,
        return_type: String,
        order_id: String,
        order_number: String,
        sku: String,
        product_name: String,
        price: f64,
        quantity: i32,
        posting_number: String,
        clearing_id: Option<String>,
        return_clearing_id: Option<String>,
        comment: Option<String>,
    ) -> Self {
        let mut base = BaseAggregate::new(OzonReturnsId::new_v4(), code, description);
        base.comment = comment;
        Self {
            base,
            connection_id,
            organization_id,
            marketplace_id,
            return_id,
            return_date,
            return_reason_name,
            return_type,
            order_id,
            order_number,
            sku,
            product_name,
            price,
            quantity,
            posting_number,
            clearing_id,
            return_clearing_id,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_id(
        id: OzonReturnsId,
        code: String,
        description: String,
        connection_id: String,
        organization_id: String,
        marketplace_id: String,
        return_id: String,
        return_date: chrono::NaiveDate,
        return_reason_name: String,
        return_type: String,
        order_id: String,
        order_number: String,
        sku: String,
        product_name: String,
        price: f64,
        quantity: i32,
        posting_number: String,
        clearing_id: Option<String>,
        return_clearing_id: Option<String>,
        comment: Option<String>,
    ) -> Self {
        let mut base = BaseAggregate::new(id, code, description);
        base.comment = comment;
        Self {
            base,
            connection_id,
            organization_id,
            marketplace_id,
            return_id,
            return_date,
            return_reason_name,
            return_type,
            order_id,
            order_number,
            sku,
            product_name,
            price,
            quantity,
            posting_number,
            clearing_id,
            return_clearing_id,
        }
    }

    pub fn to_string_id(&self) -> String {
        self.base.id.as_string()
    }

    pub fn touch_updated(&mut self) {
        self.base.touch();
    }

    pub fn update(&mut self, dto: &OzonReturnsDto) {
        self.base.code = dto.code.clone().unwrap_or_default();
        self.base.description = dto.description.clone();
        self.base.comment = dto.comment.clone();
        self.connection_id = dto.connection_id.clone();
        self.organization_id = dto.organization_id.clone();
        self.marketplace_id = dto.marketplace_id.clone();
        self.return_id = dto.return_id.clone();
        self.return_date = dto.return_date;
        self.return_reason_name = dto.return_reason_name.clone();
        self.return_type = dto.return_type.clone();
        self.order_id = dto.order_id.clone();
        self.order_number = dto.order_number.clone();
        self.sku = dto.sku.clone();
        self.product_name = dto.product_name.clone();
        self.price = dto.price;
        self.quantity = dto.quantity;
        self.posting_number = dto.posting_number.clone();
        self.clearing_id = dto.clearing_id.clone();
        self.return_clearing_id = dto.return_clearing_id.clone();
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.base.description.trim().is_empty() {
            return Err("Описание не может быть пустым".into());
        }
        if self.base.code.trim().is_empty() {
            return Err("Код не может быть пустым".into());
        }
        if self.connection_id.trim().is_empty() {
            return Err("Подключение обязательно".into());
        }
        if self.organization_id.trim().is_empty() {
            return Err("Организация обязательна".into());
        }
        if self.marketplace_id.trim().is_empty() {
            return Err("Маркетплейс обязателен".into());
        }
        if self.return_id.trim().is_empty() {
            return Err("ID возврата обязателен".into());
        }
        // SKU и название товара могут быть пустыми, если возврат без товаров
        if self.quantity < 0 {
            return Err("Количество не может быть отрицательным".into());
        }
        if self.price < 0.0 {
            return Err("Цена не может быть отрицательной".into());
        }
        Ok(())
    }

    pub fn before_write(&mut self) {
        self.touch_updated();
    }
}

impl AggregateRoot for OzonReturns {
    type Id = OzonReturnsId;
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
        "a009"
    }
    fn collection_name() -> &'static str {
        "ozon_returns"
    }
    fn element_name() -> &'static str {
        "Возврат OZON"
    }
    fn list_name() -> &'static str {
        "Возвраты OZON"
    }
    fn origin() -> Origin {
        Origin::Marketplace
    }
}

// =============================================================================
// DTO
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OzonReturnsDto {
    pub id: Option<String>,
    pub code: Option<String>,
    pub description: String,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "organizationId")]
    pub organization_id: String,
    #[serde(rename = "marketplaceId")]
    pub marketplace_id: String,
    #[serde(rename = "returnId")]
    pub return_id: String,
    #[serde(with = "serde_date")]
    #[serde(rename = "returnDate")]
    pub return_date: chrono::NaiveDate,
    #[serde(rename = "returnReasonName")]
    pub return_reason_name: String,
    #[serde(rename = "returnType")]
    pub return_type: String,
    #[serde(rename = "orderId")]
    pub order_id: String,
    #[serde(rename = "orderNumber")]
    pub order_number: String,
    pub sku: String,
    #[serde(rename = "productName")]
    pub product_name: String,
    pub price: f64,
    pub quantity: i32,
    #[serde(rename = "postingNumber")]
    pub posting_number: String,
    #[serde(rename = "clearingId")]
    pub clearing_id: Option<String>,
    #[serde(rename = "returnClearingId")]
    pub return_clearing_id: Option<String>,
    pub comment: Option<String>,
}

// Local serde helper for NaiveDate as YYYY-MM-DD
mod serde_date {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d";

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.format(FORMAT).to_string();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}
