use crate::domain::common::{
    AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin,
};
use crate::shared::indicators::ValueFormat;
use crate::shared::universal_dashboard::config::DashboardConfig;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// ID типа для агрегата BI Indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BiIndicatorId(pub Uuid);

impl BiIndicatorId {
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

impl AggregateId for BiIndicatorId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }
    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(BiIndicatorId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

// ============================================================================
// P904 Schema Query — schema-based indicator config (первый случай: p904_sales_data)
// ============================================================================

/// Измерения (группировки) для детализации индикатора на основе p904
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum P904DrillDimension {
    /// Группировка по дате (день)
    Date,
    /// Группировка по артикулу продавца
    Article,
    /// Группировка по кабинету МП (connection_mp)
    ConnectionMp,
    /// Группировка по маркетплейсу (WB / OZON / YM / ...)
    Marketplace,
    /// Группировка по типу регистратора
    RegistratorType,
    /// Группировка по номенклатуре
    NomenclatureRef,
}

impl P904DrillDimension {
    pub fn label(&self) -> &str {
        match self {
            P904DrillDimension::Date => "По дню",
            P904DrillDimension::Article => "По артикулу",
            P904DrillDimension::ConnectionMp => "По кабинету",
            P904DrillDimension::Marketplace => "По маркетплейсу",
            P904DrillDimension::RegistratorType => "По типу",
            P904DrillDimension::NomenclatureRef => "По номенклатуре",
        }
    }

    pub fn field_id(&self) -> &str {
        match self {
            P904DrillDimension::Date => "date",
            P904DrillDimension::Article => "article",
            P904DrillDimension::ConnectionMp => "connection_mp_ref",
            P904DrillDimension::Marketplace => "marketplace",
            P904DrillDimension::RegistratorType => "registrator_type",
            P904DrillDimension::NomenclatureRef => "nomenclature_ref",
        }
    }
}

/// Колонка-метрика для агрегации p904
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum P904MetricColumn {
    CustomerIn,
    CustomerOut,
    SellerOut,
    OrderCount,
}

impl P904MetricColumn {
    pub fn column_name(&self) -> &str {
        match self {
            P904MetricColumn::CustomerIn => "customer_in",
            P904MetricColumn::CustomerOut => "customer_out",
            P904MetricColumn::SellerOut => "seller_out",
            P904MetricColumn::OrderCount => "order_count",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            P904MetricColumn::CustomerIn => "Сумма выручка",
            P904MetricColumn::CustomerOut => "Сумма возвраты",
            P904MetricColumn::SellerOut => "Сумма продавцу",
            P904MetricColumn::OrderCount => "Кол-во заказов",
        }
    }
}

impl Default for P904MetricColumn {
    fn default() -> Self {
        P904MetricColumn::CustomerIn
    }
}

/// Конфигурация schema-based запроса для индикаторов на основе p904_sales_data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P904SchemaConfig {
    /// Метрика агрегации
    #[serde(default)]
    pub metric: P904MetricColumn,
    /// Доступные измерения для детализации (drilldown)
    #[serde(default)]
    pub available_dimensions: Vec<P904DrillDimension>,
}

impl Default for P904SchemaConfig {
    fn default() -> Self {
        Self {
            metric: P904MetricColumn::CustomerIn,
            available_dimensions: vec![
                P904DrillDimension::Date,
                P904DrillDimension::Article,
                P904DrillDimension::ConnectionMp,
                P904DrillDimension::Marketplace,
            ],
        }
    }
}

// ============================================================================
// IndicatorDataSourceConfig — универсальный конфиг источника данных
// ============================================================================

/// Универсальный конфиг источника данных для индикатора.
///
/// Явно связывает схему из SchemaRegistry с метрикой, измерениями для
/// drilldown и фильтрами контекста. Заменяет P904SchemaConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorDataSourceConfig {
    /// ID схемы из SchemaRegistry (ds03_p904_sales, ds01_wb_finance_report, ...)
    pub schema_id: String,
    /// ID поля-метрики для агрегации (must: can_aggregate=true в FieldDef)
    pub metric_field_id: String,
    /// ID полей для drilldown-группировки (must: can_group=true в FieldDef)
    #[serde(default)]
    pub available_dimensions: Vec<String>,
    /// ID полей контекстных фильтров (из schema_filters схемы).
    /// Обычно: connection_mp_ref, marketplace — передаются из контекста дашборда.
    #[serde(default)]
    pub context_filter_fields: Vec<String>,
}

// ============================================================================
// DataSpec — откуда и как получать данные
// ============================================================================

/// Спецификация источника данных для индикатора
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSpec {
    /// Идентификатор схемы данных (schema_id из SchemaRegistry)
    pub schema_id: String,
    /// Конфигурация запроса — совместима с Universal Dashboard QueryBuilder
    #[serde(default)]
    pub query_config: DashboardConfig,
    /// Опциональная ссылка на SQL-артефакт (a019_llm_artifact)
    pub sql_artifact_id: Option<String>,
    /// ID DataView из DataViewRegistry (например "dv001_revenue").
    /// Наивысший приоритет при вычислении индикатора и drilldown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_id: Option<String>,
    /// Универсальный конфиг источника данных (заменяет schema_query).
    /// Приоритет над schema_query при вычислении индикатора.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_source_config: Option<IndicatorDataSourceConfig>,
    /// Schema-based конфигурация для индикаторов на основе p904 (устарело).
    /// Используйте data_source_config. Оставлено для обратной совместимости.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_query: Option<P904SchemaConfig>,
}

impl Default for DataSpec {
    fn default() -> Self {
        Self {
            schema_id: String::new(),
            query_config: DashboardConfig {
                data_source: String::new(),
                selected_fields: vec![],
                groupings: vec![],
                display_fields: vec![],
                #[allow(deprecated)]
                filters: Default::default(),
                sort: Default::default(),
                enabled_fields: vec![],
            },
            sql_artifact_id: None,
            view_id: None,
            data_source_config: None,
            schema_query: None,
        }
    }
}

// ============================================================================
// Params — типизированные параметры индикатора
// ============================================================================

/// Тип параметра
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    Date,
    DateRange,
    String,
    Integer,
    Float,
    Boolean,
    Ref,
}

impl ParamType {
    pub fn as_str(&self) -> &str {
        match self {
            ParamType::Date => "date",
            ParamType::DateRange => "date_range",
            ParamType::String => "string",
            ParamType::Integer => "integer",
            ParamType::Float => "float",
            ParamType::Boolean => "boolean",
            ParamType::Ref => "ref",
        }
    }
}

/// Определение параметра индикатора
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    /// Уникальный ключ параметра (используется для подстановки в запрос)
    pub key: std::string::String,
    /// Тип параметра
    pub param_type: ParamType,
    /// Человекочитаемое название
    pub label: std::string::String,
    /// Значение по умолчанию
    pub default_value: Option<std::string::String>,
    /// Обязательный ли параметр
    pub required: bool,
    /// Ключ для связки с глобальными фильтрами дашборда
    pub global_filter_key: Option<std::string::String>,
}

// ============================================================================
// ViewSpec — как отображать индикатор
// ============================================================================

/// Порог/алерт для индикатора
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threshold {
    /// Условие в виде строки, например "< 10" или "> 100"
    pub condition: std::string::String,
    /// CSS-цвет, например "#ff0000" или "red"
    pub color: std::string::String,
    /// Опциональная метка
    pub label: Option<std::string::String>,
}

fn default_style_name() -> std::string::String {
    "classic".to_string()
}

/// Спецификация отображения индикатора
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewSpec {
    /// Стиль карточки: "classic" | "modern" | "custom"
    #[serde(default = "default_style_name")]
    pub style_name: std::string::String,
    /// Пользовательский HTML (только для style_name = "custom")
    pub custom_html: Option<std::string::String>,
    /// Пользовательский CSS (только для style_name = "custom")
    pub custom_css: Option<std::string::String>,
    /// Формат числового значения
    pub format: ValueFormat,
    /// Пороги/алерты
    pub thresholds: Vec<Threshold>,
    /// Значения параметров для превью/тестирования (сохраняются с записью)
    #[serde(default)]
    pub preview_values: HashMap<std::string::String, std::string::String>,
}

impl Default for ViewSpec {
    fn default() -> Self {
        Self {
            style_name: default_style_name(),
            custom_html: None,
            custom_css: None,
            format: ValueFormat::Number { decimals: 2 },
            thresholds: vec![],
            preview_values: HashMap::new(),
        }
    }
}

// ============================================================================
// DrillSpec — провал в детализацию
// ============================================================================

/// Тип цели при drill-down
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DrillTarget {
    Explore,
    SavedReport,
    Schema,
}

/// Спецификация drill-down навигации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrillSpec {
    /// Куда переходить
    pub target_type: DrillTarget,
    /// ID цели (saved_report ID, schema_id, etc.)
    pub target_id: std::string::String,
    /// Маппинг фильтров: ключ_клика -> фильтр_цели
    pub filter_mapping: HashMap<std::string::String, std::string::String>,
}

// ============================================================================
// Status
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BiIndicatorStatus {
    Draft,
    Active,
    Archived,
}

impl BiIndicatorStatus {
    pub fn from_str(s: &str) -> Result<Self, std::string::String> {
        match s {
            "draft" => Ok(BiIndicatorStatus::Draft),
            "active" => Ok(BiIndicatorStatus::Active),
            "archived" => Ok(BiIndicatorStatus::Archived),
            _ => Err(format!("Unknown BI indicator status: {}", s)),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            BiIndicatorStatus::Draft => "draft",
            BiIndicatorStatus::Active => "active",
            BiIndicatorStatus::Archived => "archived",
        }
    }
}

// ============================================================================
// Aggregate
// ============================================================================

/// Агрегат BI Indicator
///
/// Один агрегат = один индикатор BI-дашборда.
/// Содержит спецификации данных, параметров, отображения и drill-down.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiIndicator {
    #[serde(flatten)]
    pub base: BaseAggregate<BiIndicatorId>,

    /// Откуда и как получать данные
    pub data_spec: DataSpec,
    /// Параметры индикатора (типизированные, с дефолтами)
    pub params: Vec<ParamDef>,
    /// Как отображать индикатор
    pub view_spec: ViewSpec,
    /// Как проваливаться (опционально)
    pub drill_spec: Option<DrillSpec>,

    /// Статус: Draft | Active | Archived
    pub status: BiIndicatorStatus,
    /// Владелец индикатора
    pub owner_user_id: std::string::String,
    /// Публичный ли (доступен другим пользователям)
    pub is_public: bool,
    /// Кто создал
    pub created_by: Option<std::string::String>,
    /// Кто обновил
    pub updated_by: Option<std::string::String>,
}

impl BiIndicator {
    pub fn new_for_insert(
        code: std::string::String,
        description: std::string::String,
        owner_user_id: std::string::String,
    ) -> Self {
        let base = BaseAggregate::new(BiIndicatorId::new_v4(), code, description);
        Self {
            base,
            data_spec: DataSpec::default(),
            params: vec![],
            view_spec: ViewSpec::default(),
            drill_spec: None,
            status: BiIndicatorStatus::Draft,
            owner_user_id,
            is_public: false,
            created_by: None,
            updated_by: None,
        }
    }

    pub fn to_string_id(&self) -> std::string::String {
        self.base.id.as_string()
    }

    pub fn touch_updated(&mut self) {
        self.base.touch();
    }

    pub fn validate(&self) -> Result<(), std::string::String> {
        if self.base.description.trim().is_empty() {
            return Err("Наименование не может быть пустым".into());
        }
        if self.base.code.trim().is_empty() {
            return Err("Код не может быть пустым".into());
        }
        Ok(())
    }

    pub fn before_write(&mut self) {
        self.touch_updated();
    }
}

impl AggregateRoot for BiIndicator {
    type Id = BiIndicatorId;

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
        "a024"
    }

    fn collection_name() -> &'static str {
        "bi_indicator"
    }

    fn element_name() -> &'static str {
        "BI Индикатор"
    }

    fn list_name() -> &'static str {
        "BI Индикаторы"
    }

    fn origin() -> Origin {
        Origin::Self_
    }
}

// ============================================================================
// DTO для списка
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiIndicatorListItem {
    pub id: std::string::String,
    pub code: std::string::String,
    pub description: std::string::String,
    pub comment: Option<std::string::String>,
    pub status: std::string::String,
    pub owner_user_id: std::string::String,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// DTOs для LLM-генерации ViewSpec
// ============================================================================

/// Запрос на генерацию HTML/CSS для индикатора через LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateViewRequest {
    /// Текстовое описание желаемого вида индикатора
    pub prompt: String,
    /// Текущий HTML (для итеративной доработки)
    pub current_html: Option<String>,
    /// Текущий CSS
    pub current_css: Option<String>,
    /// Описание индикатора (контекст для LLM)
    pub indicator_description: String,
    /// ID агента (a017), если не задан — используется primary agent
    pub agent_id: Option<String>,
}

/// Ответ с сгенерированным HTML/CSS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateViewResponse {
    pub custom_html: String,
    pub custom_css: String,
    pub explanation: String,
}

impl From<BiIndicator> for BiIndicatorListItem {
    fn from(ind: BiIndicator) -> Self {
        Self {
            id: ind.base.id.as_string(),
            code: ind.base.code,
            description: ind.base.description,
            comment: ind.base.comment,
            status: ind.status.as_str().to_string(),
            owner_user_id: ind.owner_user_id,
            is_public: ind.is_public,
            created_at: ind.base.metadata.created_at,
            updated_at: ind.base.metadata.updated_at,
        }
    }
}
