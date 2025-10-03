use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Connection1CDatabaseId(pub i32);

impl Connection1CDatabaseId {
    pub fn new(value: i32) -> Self {
        Self(value)
    }
    pub fn value(&self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_deleted: bool,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseAggregate<Id> {
    pub id: Id,
    pub metadata: EntityMetadata,
    pub events: EventStore,
}

impl<Id> BaseAggregate<Id> {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            metadata: EntityMetadata {
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                is_deleted: false,
                version: 0,
            },
            events: EventStore::default(),
        }
    }
    pub fn with_metadata(id: Id, metadata: EntityMetadata) -> Self {
        Self {
            id,
            metadata,
            events: EventStore::default(),
        }
    }
}

pub trait AggregateRoot {
    type Id;
    fn id(&self) -> Self::Id;
    fn metadata(&self) -> &EntityMetadata;
    fn metadata_mut(&mut self) -> &mut EntityMetadata;
    fn aggregate_type() -> &'static str;
    fn events(&self) -> &EventStore;
    fn events_mut(&mut self) -> &mut EventStore;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection1CDatabase {
    #[serde(flatten)]
    pub base: BaseAggregate<Connection1CDatabaseId>,
    pub description: String,
    pub url: String,
    pub comment: Option<String>,
    pub login: String,
    pub password: String,
    #[serde(rename = "isPrimary", default)]
    pub is_primary: bool,
}

impl Connection1CDatabase {
    pub fn new_for_insert(
        description: String,
        url: String,
        comment: Option<String>,
        login: String,
        password: String,
        is_primary: bool,
    ) -> Self {
        Self {
            base: BaseAggregate::new(Connection1CDatabaseId::new(0)),
            description,
            url,
            comment,
            login,
            password,
            is_primary,
        }
    }
    pub fn to_string_id(&self) -> String {
        self.base.id.0.to_string()
    }
    pub fn touch_updated(&mut self) {
        self.base.metadata.updated_at = chrono::Utc::now();
    }

    pub fn update_from_form(&mut self, form: &Connection1CDatabaseForm) {
        self.description = form.description.clone();
        self.url = form.url.clone();
        self.comment = form.comment.clone();
        self.login = form.login.clone();
        self.password = form.password.clone();
        self.is_primary = form.is_primary;
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.url.trim().is_empty() {
            return Err("URL не может быть пустым".into());
        }
        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err("URL должен начинаться с http:// или https://".into());
        }
        if self.description.trim().is_empty() {
            return Err("Описание не может быть пустым".into());
        }
        Ok(())
    }

    pub fn before_write(&mut self) {
        self.touch_updated();
    }
}

impl AggregateRoot for Connection1CDatabase {
    type Id = Connection1CDatabaseId;
    fn id(&self) -> Self::Id {
        self.base.id
    }
    fn metadata(&self) -> &EntityMetadata {
        &self.base.metadata
    }
    fn metadata_mut(&mut self) -> &mut EntityMetadata {
        &mut self.base.metadata
    }
    fn aggregate_type() -> &'static str {
        "Connection1CDatabase"
    }
    fn events(&self) -> &EventStore {
        &self.base.events
    }
    fn events_mut(&mut self) -> &mut EventStore {
        &mut self.base.events
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Connection1CDatabaseForm {
    pub id: Option<String>,
    pub description: String,
    pub url: String,
    pub comment: Option<String>,
    pub login: String,
    pub password: String,
    #[serde(rename = "isPrimary", default)]
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub duration_ms: u64,
    pub tested_at: chrono::DateTime<chrono::Utc>,
}
