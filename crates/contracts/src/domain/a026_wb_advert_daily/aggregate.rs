use crate::domain::common::{
    AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WbAdvertDailyId(pub Uuid);

impl WbAdvertDailyId {
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

impl AggregateId for WbAdvertDailyId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }

    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(WbAdvertDailyId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertDailyHeader {
    pub document_no: String,
    pub document_date: String,
    /// Идентификатор рекламной кампании WB; один документ = одна дата + один advert_id.
    #[serde(default)]
    pub advert_id: i64,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WbAdvertDailyMetrics {
    pub views: i64,
    pub clicks: i64,
    pub ctr: f64,
    pub cpc: f64,
    pub atbs: i64,
    pub orders: i64,
    pub shks: i64,
    pub sum: f64,
    pub sum_price: f64,
    pub cr: f64,
    pub canceled: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertDailyLine {
    pub nm_id: i64,
    pub nm_name: String,
    pub nomenclature_ref: Option<String>,
    pub advert_ids: Vec<i64>,
    #[serde(default)]
    pub app_types: Vec<i32>,
    #[serde(default)]
    pub placements: Vec<String>,
    pub metrics: WbAdvertDailyMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertDailySourceMeta {
    pub source: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertDaily {
    #[serde(flatten)]
    pub base: BaseAggregate<WbAdvertDailyId>,
    pub header: WbAdvertDailyHeader,
    pub totals: WbAdvertDailyMetrics,
    pub unattributed_totals: WbAdvertDailyMetrics,
    pub lines: Vec<WbAdvertDailyLine>,
    pub source_meta: WbAdvertDailySourceMeta,
    pub is_posted: bool,
}

impl WbAdvertDaily {
    pub fn new_for_insert(
        header: WbAdvertDailyHeader,
        totals: WbAdvertDailyMetrics,
        unattributed_totals: WbAdvertDailyMetrics,
        lines: Vec<WbAdvertDailyLine>,
        source_meta: WbAdvertDailySourceMeta,
    ) -> Self {
        let description = format!(
            "Статистика рекламы WB advert_id={} за {}",
            header.advert_id, header.document_date
        );
        let base = BaseAggregate::new(
            WbAdvertDailyId::new_v4(),
            header.document_no.clone(),
            description,
        );

        Self {
            base,
            header,
            totals,
            unattributed_totals,
            lines,
            source_meta,
            is_posted: false,
        }
    }

    pub fn touch_updated(&mut self) {
        self.base.touch();
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.header.document_no.trim().is_empty() {
            return Err("Номер документа обязателен".into());
        }
        if self.header.document_date.trim().is_empty() {
            return Err("Дата документа обязательна".into());
        }
        if self.header.connection_id.trim().is_empty() {
            return Err("Подключение обязательно".into());
        }
        if self.header.advert_id <= 0 {
            return Err("advert_id должен быть положительным".into());
        }
        Ok(())
    }

    pub fn before_write(&mut self) {
        self.touch_updated();
    }
}

impl AggregateRoot for WbAdvertDaily {
    type Id = WbAdvertDailyId;

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
        "a026"
    }

    fn collection_name() -> &'static str {
        "wb_advert_daily"
    }

    fn element_name() -> &'static str {
        "Статистика рекламы WB"
    }

    fn list_name() -> &'static str {
        "Статистика рекламы WB"
    }

    fn origin() -> Origin {
        Origin::Marketplace
    }
}
