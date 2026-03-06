use contracts::domain::a003_counterparty::aggregate::Counterparty;
use serde::{Deserialize, Serialize};

/// OData модель справочника Контрагенты из УТ 11
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtCounterpartyOData {
    #[serde(rename = "Ref_Key", default)]
    pub ref_key: String,

    // Код может быть Code/Код
    #[serde(rename = "Code", alias = "Код", default)]
    pub code: String,

    // Наименование может быть Description/Наименование
    #[serde(rename = "Description", alias = "Наименование", default)]
    pub description: String,

    // Признак папки
    #[serde(rename = "IsFolder", alias = "ЭтоГруппа", default)]
    pub is_folder: bool,

    // Родительский элемент может быть Parent_Key/ParentKey
    #[serde(rename = "Parent_Key", alias = "ParentKey", default)]
    pub parent_key: Option<String>,

    #[serde(rename = "DeletionMark", default)]
    pub deletion_mark: bool,

    // Tax identifiers
    #[serde(rename = "ИНН", alias = "INN", default)]
    pub inn: Option<String>,
    #[serde(rename = "КПП", alias = "KPP", default)]
    pub kpp: Option<String>,

    // Игнорировать дополнительные поля
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

impl UtCounterpartyOData {
    fn normalize_parent_id(parent_key: &Option<String>) -> Option<String> {
        use uuid::Uuid;
        match parent_key
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            Some(raw) => Uuid::parse_str(raw).ok().map(|u| u.to_string()),
            None => None,
        }
    }

    /// Преобразование OData модели в агрегат Counterparty
    pub fn to_aggregate(&self) -> Result<Counterparty, String> {
        use contracts::domain::a003_counterparty::aggregate::CounterpartyId;
        use uuid::Uuid;

        tracing::debug!(
            "Converting Counterparty OData: ref_key={}, code={}, description={}, is_folder={}, parent_key={:?}, inn={:?}, kpp={:?}",
            self.ref_key,
            self.code,
            self.description,
            self.is_folder,
            self.parent_key,
            self.inn,
            self.kpp
        );

        let id = if !self.ref_key.is_empty() {
            Uuid::parse_str(&self.ref_key)
                .map(CounterpartyId)
                .unwrap_or_else(|_| CounterpartyId::new_v4())
        } else {
            CounterpartyId::new_v4()
        };

        let parent_id = Self::normalize_parent_id(&self.parent_key);

        let mut agg = Counterparty::new_with_id(
            id,
            self.code.clone(),
            self.description.clone(),
            self.is_folder,
            parent_id,
            self.inn.clone().unwrap_or_default(),
            self.kpp.clone().unwrap_or_default(),
            None,
        );
        // Применить признак удаления из источника
        agg.base.metadata.is_deleted = self.deletion_mark;
        Ok(agg)
    }

    /// Проверка, нужно ли обновлять существующий агрегат
    pub fn should_update(&self, existing: &Counterparty) -> bool {
        let normalized_parent = Self::normalize_parent_id(&self.parent_key);
        existing.base.code != self.code
            || existing.base.description != self.description
            || existing.is_folder != self.is_folder
            || existing.parent_id != normalized_parent
            || existing.base.metadata.is_deleted != self.deletion_mark
            || existing.inn != self.inn.clone().unwrap_or_default()
            || existing.kpp != self.kpp.clone().unwrap_or_default()
    }
}

/// Ответ OData для списка контрагентов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtCounterpartyListResponse {
    pub value: Vec<UtCounterpartyOData>,
}
