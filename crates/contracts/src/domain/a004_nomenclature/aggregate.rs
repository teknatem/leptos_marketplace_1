use crate::domain::common::{
    AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// ID Type
// ============================================================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NomenclatureId(pub Uuid);

impl NomenclatureId {
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

impl AggregateId for NomenclatureId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }

    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(NomenclatureId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

// ============================================================================
// Aggregate Root
// ============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nomenclature {
    #[serde(flatten)]
    pub base: BaseAggregate<NomenclatureId>,

    #[serde(rename = "fullDescription")]
    pub full_description: String,

    #[serde(rename = "isFolder", default)]
    pub is_folder: bool,

    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,

    #[serde(rename = "article")]
    pub article: String,

    #[serde(rename = "mpRefCount", default)]
    pub mp_ref_count: i32,
}

impl Nomenclature {
    pub fn new_for_insert(
        code: String,
        description: String,
        full_description: String,
        is_folder: bool,
        parent_id: Option<String>,
        article: String,
        comment: Option<String>,
    ) -> Self {
        let mut base = BaseAggregate::new(NomenclatureId::new_v4(), code, description);
        base.comment = comment;

        Self {
            base,
            full_description,
            is_folder,
            parent_id,
            article,
            mp_ref_count: 0,
        }
    }

    pub fn new_with_id(
        id: NomenclatureId,
        code: String,
        description: String,
        full_description: String,
        is_folder: bool,
        parent_id: Option<String>,
        article: String,
        comment: Option<String>,
    ) -> Self {
        let mut base = BaseAggregate::new(id, code, description);
        base.comment = comment;

        Self {
            base,
            full_description,
            is_folder,
            parent_id,
            article,
            mp_ref_count: 0,
        }
    }

    pub fn to_string_id(&self) -> String {
        self.base.id.as_string()
    }

    pub fn touch_updated(&mut self) {
        self.base.touch();
    }

    pub fn update(&mut self, dto: &NomenclatureDto) {
        self.base.code = dto.code.clone().unwrap_or_default();
        self.base.description = dto.description.clone();
        self.base.comment = dto.comment.clone();
        self.full_description = dto.full_description.clone().unwrap_or_default();
        self.is_folder = dto.is_folder;
        self.parent_id = dto.parent_id.clone();
        self.article = dto.article.clone().unwrap_or_default();
        // mp_ref_count обновляется только автоматически при сопоставлении
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.base.description.trim().is_empty() {
            return Err("Описание не может быть пустым".into());
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

impl AggregateRoot for Nomenclature {
    type Id = NomenclatureId;

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
        "a004"
    }

    fn collection_name() -> &'static str {
        "nomenclature"
    }

    fn element_name() -> &'static str {
        "Номенклатура"
    }

    fn list_name() -> &'static str {
        "Номенклатура"
    }

    fn origin() -> Origin {
        Origin::C1
    }
}

// ============================================================================
// DTO
// ============================================================================
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NomenclatureDto {
    pub id: Option<String>,
    pub code: Option<String>,
    pub description: String,
    #[serde(rename = "fullDescription")]
    pub full_description: Option<String>,
    #[serde(rename = "isFolder", default)]
    pub is_folder: bool,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub article: Option<String>,
    pub comment: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "mpRefCount", default)]
    pub mp_ref_count: i32,
}
