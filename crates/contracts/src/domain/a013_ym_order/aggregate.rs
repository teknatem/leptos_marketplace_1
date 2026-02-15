use crate::domain::common::{
    AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ID типа для документа Yandex Market Order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct YmOrderId(pub Uuid);

impl YmOrderId {
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

impl AggregateId for YmOrderId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }
    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(YmOrderId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

/// Заголовочные поля документа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderHeader {
    /// Номер документа (orderId из Yandex Market API)
    pub document_no: String,
    /// ID подключения маркетплейса
    pub connection_id: String,
    /// ID организации
    pub organization_id: String,
    /// ID маркетплейса
    pub marketplace_id: String,
    /// ID кампании в Yandex Market
    pub campaign_id: String,
    /// Общая сумма заказа из API (total)
    pub total_amount: Option<f64>,
    /// Валюта заказа
    pub currency: Option<String>,
    /// Платеж покупателя (itemsTotal) - общая стоимость товаров включая НДС, без доставки
    #[serde(default)]
    pub items_total: Option<f64>,
    /// Стоимость доставки (deliveryTotal)
    #[serde(default)]
    pub delivery_total: Option<f64>,
    /// Субсидии от Маркета (JSON массив OrderSubsidyDTO)
    #[serde(default)]
    pub subsidies_json: Option<String>,
}

/// Строка документа (позиция)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderLine {
    /// ID строки (itemId из YM)
    pub line_id: String,
    /// shopSku (артикул продавца)
    pub shop_sku: String,
    /// offerId
    pub offer_id: String,
    /// Название товара
    pub name: String,
    /// Количество
    pub qty: f64,
    /// Цена до скидок
    pub price_list: Option<f64>,
    /// Сумма скидок
    pub discount_total: Option<f64>,
    /// Цена после скидок (за единицу)
    pub price_effective: Option<f64>,
    /// Сумма за строку
    pub amount_line: Option<f64>,
    /// Код валюты
    pub currency_code: Option<String>,
    /// Цена товара после всех скидок (buyerPrice)
    #[serde(default)]
    pub buyer_price: Option<f64>,
    /// Субсидии на уровне товара (JSON массив OrderItemSubsidyDTO)
    #[serde(default)]
    pub subsidies_json: Option<String>,
    /// Статус товара в заказе
    #[serde(default)]
    pub status: Option<String>,
    /// Плановая цена (пока константа = 0)
    #[serde(default)]
    pub price_plan: Option<f64>,
    /// Ссылка на товар маркетплейса (a007_marketplace_product)
    #[serde(default)]
    pub marketplace_product_ref: Option<String>,
    /// Ссылка на номенклатуру 1С (a004_nomenclature)
    #[serde(default)]
    pub nomenclature_ref: Option<String>,
}

/// Статусы и временные метки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderState {
    /// Исходный статус из API
    pub status_raw: String,
    /// Подстатус
    pub substatus_raw: Option<String>,
    /// Нормализованный статус (DELIVERED/RECEIVED)
    pub status_norm: String,
    /// Дата/время изменения статуса на DELIVERED
    pub status_changed_at: Option<DateTime<Utc>>,
    /// Дата/время обновления заказа в источнике
    pub updated_at_source: Option<DateTime<Utc>>,
    /// Дата создания заказа (creationDate из API)
    pub creation_date: Option<DateTime<Utc>>,
    /// Дата доставки (deliveryDate из API)
    pub delivery_date: Option<DateTime<Utc>>,
}

/// Служебные метаданные
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderSourceMeta {
    /// Ссылка на сырой JSON (ID в document_raw_storage)
    pub raw_payload_ref: String,
    /// Дата/время получения из API
    pub fetched_at: DateTime<Utc>,
    /// Версия документа (для отслеживания изменений)
    pub document_version: i32,
}

/// Документ Yandex Market Order (агрегат)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrder {
    #[serde(flatten)]
    pub base: BaseAggregate<YmOrderId>,

    /// Заголовок документа
    pub header: YmOrderHeader,

    /// Строки документа
    pub lines: Vec<YmOrderLine>,

    /// Статусы и временные метки
    pub state: YmOrderState,

    /// Служебные метаданные
    pub source_meta: YmOrderSourceMeta,

    /// Флаг проведения документа (для формирования проекций)
    pub is_posted: bool,

    /// Флаг ошибки (ненулевой при отсутствии сопоставления номенклатуры в строках)
    #[serde(default)]
    pub is_error: bool,
}

impl YmOrder {
    pub fn new_for_insert(
        code: String,
        description: String,
        header: YmOrderHeader,
        lines: Vec<YmOrderLine>,
        state: YmOrderState,
        source_meta: YmOrderSourceMeta,
        is_posted: bool,
    ) -> Self {
        let base = BaseAggregate::new(YmOrderId::new_v4(), code, description);
        Self {
            base,
            header,
            lines,
            state,
            source_meta,
            is_posted,
            is_error: false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_id(
        id: YmOrderId,
        code: String,
        description: String,
        header: YmOrderHeader,
        lines: Vec<YmOrderLine>,
        state: YmOrderState,
        source_meta: YmOrderSourceMeta,
        is_posted: bool,
    ) -> Self {
        let base = BaseAggregate::new(id, code, description);
        Self {
            base,
            header,
            lines,
            state,
            source_meta,
            is_posted,
            is_error: false,
        }
    }

    /// Обновление флага is_error на основе строк документа
    /// Ошибкой считается отсутствие nomenclature_ref в любой строке
    pub fn update_is_error(&mut self) {
        self.is_error = self
            .lines
            .iter()
            .any(|line| line.nomenclature_ref.is_none());
    }

    /// Пересчет итогов по строкам документа
    pub fn recalculate_totals(&mut self) {
        let mut _total_qty = 0.0;
        let mut total_amount = 0.0;

        for line in &self.lines {
            _total_qty += line.qty;
            if let Some(amount) = line.amount_line {
                total_amount += amount;
            }
        }

        self.header.items_total = Some(total_amount);
        // total_qty пока не используется, но может пригодиться для валидации
        // total_amount в header.total_amount может содержать другую сумму (из API),
        // поэтому не перезаписываем её
    }

    pub fn to_string_id(&self) -> String {
        self.base.id.as_string()
    }

    pub fn touch_updated(&mut self) {
        self.base.touch();
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.base.description.trim().is_empty() {
            return Err("Описание не может быть пустым".into());
        }
        if self.base.code.trim().is_empty() {
            return Err("Код не может быть пустым".into());
        }
        if self.header.document_no.trim().is_empty() {
            return Err("Номер заказа обязателен".into());
        }
        if self.header.connection_id.trim().is_empty() {
            return Err("Подключение обязательно".into());
        }
        if self.lines.is_empty() {
            return Err("Заказ должен содержать хотя бы одну строку".into());
        }
        Ok(())
    }

    pub fn before_write(&mut self) {
        self.touch_updated();
    }
}

impl AggregateRoot for YmOrder {
    type Id = YmOrderId;

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
        "a013"
    }

    fn collection_name() -> &'static str {
        "ym_order"
    }

    fn element_name() -> &'static str {
        "Заказ Yandex Market"
    }

    fn list_name() -> &'static str {
        "Заказы Yandex Market"
    }

    fn origin() -> Origin {
        Origin::Marketplace
    }
}

// =============================================================================
// List DTO for frontend (flat structure for list views)
// =============================================================================

/// DTO для списка заказов (минимальные поля для list view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderListDto {
    pub id: String,
    pub document_no: String,
    #[serde(default)]
    pub status_changed_at: String,
    #[serde(default)]
    pub creation_date: String,
    #[serde(default)]
    pub delivery_date: String,
    #[serde(default)]
    pub campaign_id: String,
    #[serde(default)]
    pub status_norm: String,
    #[serde(default)]
    pub total_qty: f64,
    #[serde(default)]
    pub total_amount: f64,
    pub total_amount_api: Option<f64>,
    #[serde(default)]
    pub lines_count: usize,
    pub delivery_total: Option<f64>,
    #[serde(default)]
    pub subsidies_total: f64,
    #[serde(default)]
    pub is_posted: bool,
    #[serde(default)]
    pub is_error: bool,
    pub organization_name: Option<String>,
}
