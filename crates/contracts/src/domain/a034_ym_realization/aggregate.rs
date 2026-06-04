use crate::domain::common::{
    AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct YmRealizationId(pub Uuid);

fn fnv1a64(input: &str) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for byte in input.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

fn stable_uuid_bytes(key: &str) -> [u8; 16] {
    let h1 = fnv1a64(key);
    let h2 = fnv1a64(&format!("{key}\0salt"));
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&h1.to_le_bytes());
    bytes[8..].copy_from_slice(&h2.to_le_bytes());
    bytes
}

impl YmRealizationId {
    pub fn new(value: Uuid) -> Self {
        Self(value)
    }

    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }

    /// Детерминированный id документа по (connection, date). Один и тот же
    /// суточный отчёт о реализации всегда получает один UUID — перепроведение и
    /// replace_for_period не плодят осиротевшие GL-проводки от случайных id.
    pub fn stable_for_header(header: &YmRealizationHeader) -> Self {
        let key = format!(
            "a034_ym_realization:{}:{}",
            header.connection_id, header.document_date,
        );
        Self(Uuid::from_bytes(stable_uuid_bytes(&key)))
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl AggregateId for YmRealizationId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }

    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(YmRealizationId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmRealizationHeader {
    pub document_no: String,
    /// День реализации (YYYY-MM-DD). Один документ = один кабинет + одна дата.
    pub document_date: String,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
}

/// Строка отчёта о реализации YM (по SKU). Поля с запасом — отчёт несёт больше
/// данных, чем нужно для выручки; их можно использовать в будущих проекциях.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmRealizationLine {
    pub shop_sku: String,
    #[serde(default)]
    pub market_sku: Option<i64>,
    #[serde(default)]
    pub offer_name: String,
    #[serde(default)]
    pub quantity: f64,
    /// Выручка по покупателю (положительная). Знак операции несёт `is_return`.
    pub revenue_amount: f64,
    /// true — строка возврата (уменьшает выручку), false — продажа.
    #[serde(default)]
    pub is_return: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct YmRealizationTotals {
    /// Σ выручки по строкам-продажам.
    pub sales_revenue: f64,
    /// Σ выручки по строкам-возвратам (положительная величина).
    pub return_revenue: f64,
    /// Нетто-выручка = sales_revenue − return_revenue.
    pub net_revenue: f64,
    /// Σ количества по строкам-продажам.
    #[serde(default)]
    pub sales_qty: f64,
    /// Σ количества по строкам-возвратам (положительная величина).
    #[serde(default)]
    pub return_qty: f64,
    /// Нетто-количество = sales_qty − return_qty.
    #[serde(default)]
    pub net_qty: f64,
}

impl YmRealizationTotals {
    pub fn from_lines(lines: &[YmRealizationLine]) -> Self {
        let mut sales_revenue = 0.0;
        let mut return_revenue = 0.0;
        let mut sales_qty = 0.0;
        let mut return_qty = 0.0;
        for line in lines {
            if line.is_return {
                return_revenue += line.revenue_amount;
                return_qty += line.quantity;
            } else {
                sales_revenue += line.revenue_amount;
                sales_qty += line.quantity;
            }
        }
        Self {
            sales_revenue,
            return_revenue,
            net_revenue: sales_revenue - return_revenue,
            sales_qty,
            return_qty,
            net_qty: sales_qty - return_qty,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmRealizationSourceMeta {
    pub source: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmRealization {
    #[serde(flatten)]
    pub base: BaseAggregate<YmRealizationId>,
    pub header: YmRealizationHeader,
    pub totals: YmRealizationTotals,
    pub lines: Vec<YmRealizationLine>,
    pub source_meta: YmRealizationSourceMeta,
    pub is_posted: bool,
}

impl YmRealization {
    pub fn new_for_insert(
        header: YmRealizationHeader,
        lines: Vec<YmRealizationLine>,
        source_meta: YmRealizationSourceMeta,
    ) -> Self {
        let totals = YmRealizationTotals::from_lines(&lines);
        let description = format!(
            "Реализация YM за {} (кабинет {})",
            header.document_date, header.connection_id
        );
        let base = BaseAggregate::new(
            YmRealizationId::stable_for_header(&header),
            header.document_no.clone(),
            description,
        );

        Self {
            base,
            header,
            totals,
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
        Ok(())
    }

    pub fn before_write(&mut self) {
        self.touch_updated();
    }
}

impl AggregateRoot for YmRealization {
    type Id = YmRealizationId;

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
        "a034"
    }

    fn collection_name() -> &'static str {
        "ym_realization"
    }

    fn element_name() -> &'static str {
        "Реализация YM"
    }

    fn list_name() -> &'static str {
        "Реализация YM"
    }

    fn origin() -> Origin {
        Origin::Marketplace
    }
}
