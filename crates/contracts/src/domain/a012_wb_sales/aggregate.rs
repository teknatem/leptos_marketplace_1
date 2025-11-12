use crate::domain::common::{
    AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ID типа для документа Wildberries Sales
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WbSalesId(pub Uuid);

impl WbSalesId {
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

impl AggregateId for WbSalesId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }
    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(WbSalesId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

/// Заголовочные поля документа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesHeader {
    /// Номер документа (srid из WB API - уникальный ID строки события)
    pub document_no: String,
    /// ID подключения маркетплейса
    pub connection_id: String,
    /// ID организации
    pub organization_id: String,
    /// ID маркетплейса
    pub marketplace_id: String,
}

/// Строка документа (в WB одна строка = одна продажа)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesLine {
    /// ID строки (совпадает с srid в WB)
    pub line_id: String,
    /// Артикул продавца
    pub supplier_article: String,
    /// nmId (ID номенклатуры WB)
    pub nm_id: i64,
    /// Баркод
    pub barcode: String,
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
    /// Код валюты (обычно пусто, т.к. всё в рублях)
    pub currency_code: Option<String>,
}

/// Статусы и временные метки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesState {
    /// Тип события (sale/return)
    pub event_type: String,
    /// Нормализованный статус (DELIVERED для sale)
    pub status_norm: String,
    /// Дата/время продажи
    pub sale_dt: DateTime<Utc>,
    /// Дата/время последнего изменения
    pub last_change_dt: Option<DateTime<Utc>>,
}

/// Служебные метаданные
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesSourceMeta {
    /// Ссылка на сырой JSON (ID в document_raw_storage)
    pub raw_payload_ref: String,
    /// Дата/время получения из API
    pub fetched_at: DateTime<Utc>,
    /// Версия документа (для отслеживания изменений)
    pub document_version: i32,
}

/// Документ Wildberries Sales (агрегат)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSales {
    #[serde(flatten)]
    pub base: BaseAggregate<WbSalesId>,

    /// Заголовок документа
    pub header: WbSalesHeader,

    /// Строка документа (в WB всегда одна строка)
    pub line: WbSalesLine,

    /// Статусы и временные метки
    pub state: WbSalesState,

    /// Служебные метаданные
    pub source_meta: WbSalesSourceMeta,

    /// Флаг проведения документа (для формирования проекций)
    pub is_posted: bool,
}

impl WbSales {
    pub fn new_for_insert(
        code: String,
        description: String,
        header: WbSalesHeader,
        line: WbSalesLine,
        state: WbSalesState,
        source_meta: WbSalesSourceMeta,
        is_posted: bool,
    ) -> Self {
        let base = BaseAggregate::new(WbSalesId::new_v4(), code, description);
        Self {
            base,
            header,
            line,
            state,
            source_meta,
            is_posted,
        }
    }

    pub fn new_with_id(
        id: WbSalesId,
        code: String,
        description: String,
        header: WbSalesHeader,
        line: WbSalesLine,
        state: WbSalesState,
        source_meta: WbSalesSourceMeta,
        is_posted: bool,
    ) -> Self {
        let base = BaseAggregate::new(id, code, description);
        Self {
            base,
            header,
            line,
            state,
            source_meta,
            is_posted,
        }
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
            return Err("Номер документа (srid) обязателен".into());
        }
        if self.header.connection_id.trim().is_empty() {
            return Err("Подключение обязательно".into());
        }
        Ok(())
    }

    pub fn before_write(&mut self) {
        self.touch_updated();
    }
}

impl AggregateRoot for WbSales {
    type Id = WbSalesId;
    
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
        "a012"
    }
    
    fn collection_name() -> &'static str {
        "wb_sales"
    }
    
    fn element_name() -> &'static str {
        "Документ WB Продажи"
    }
    
    fn list_name() -> &'static str {
        "Документы WB Продажи"
    }
    
    fn origin() -> Origin {
        Origin::Marketplace
    }
}

