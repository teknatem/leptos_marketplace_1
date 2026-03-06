use contracts::domain::a002_organization::aggregate::Organization;
use serde::{Deserialize, Serialize};

/// OData модель справочника Организации из УТ 11
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtOrganizationOData {
    #[serde(rename = "Ref_Key", default)]
    pub ref_key: String,

    // Код может называться Code
    #[serde(rename = "Code", alias = "Код", default)]
    pub code: String,

    // Наименование может быть Description или Наименование
    #[serde(rename = "Description", alias = "Наименование", default)]
    pub description: String,

    #[serde(rename = "ПолноеНаименование", alias = "FullName", default)]
    pub full_name: Option<String>,

    #[serde(rename = "ИНН", alias = "INN", default)]
    pub inn: Option<String>,

    #[serde(rename = "КПП", alias = "KPP", default)]
    pub kpp: Option<String>,

    #[serde(rename = "DeletionMark", default)]
    pub deletion_mark: bool,

    // Игнорировать дополнительные поля, которые мы не используем
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

impl UtOrganizationOData {
    /// Преобразование OData модели в агрегат Organization
    pub fn to_aggregate(&self) -> Result<Organization, String> {
        use contracts::domain::a002_organization::aggregate::OrganizationId;
        use uuid::Uuid;

        tracing::debug!(
            "Converting Organization OData: ref_key={}, code={}, description={}, inn={:?}, kpp={:?}",
            self.ref_key,
            self.code,
            self.description,
            self.inn,
            self.kpp
        );

        let full_name = self
            .full_name
            .clone()
            .unwrap_or_else(|| self.description.clone());

        let inn = self.inn.clone().unwrap_or_default();
        let kpp = self.kpp.clone().unwrap_or_default();

        // Используем Ref_Key из 1С как UUID для организации
        let org_id = if !self.ref_key.is_empty() {
            // Парсим Ref_Key как UUID
            Uuid::parse_str(&self.ref_key)
                .map(OrganizationId)
                .unwrap_or_else(|_| {
                    tracing::warn!(
                        "Failed to parse Ref_Key as UUID: {}, generating new UUID",
                        self.ref_key
                    );
                    OrganizationId::new_v4()
                })
        } else {
            tracing::warn!(
                "Empty Ref_Key for organization {}, generating new UUID",
                self.description
            );
            OrganizationId::new_v4()
        };

        let mut org = Organization::new_with_id(
            org_id,
            self.code.clone(),
            self.description.clone(),
            full_name,
            inn,
            kpp,
            None,
        );
        // Применить признак удаления из источника
        org.base.metadata.is_deleted = self.deletion_mark;

        Ok(org)
    }

    /// Проверка, нужно ли обновлять существующий агрегат
    pub fn should_update(&self, existing: &Organization) -> bool {
        existing.base.code != self.code
            || existing.base.description != self.description
            || existing.full_name
                != self
                    .full_name
                    .clone()
                    .unwrap_or_else(|| self.description.clone())
            || existing.inn != self.inn.clone().unwrap_or_default()
            || existing.kpp != self.kpp.clone().unwrap_or_default()
            || existing.base.metadata.is_deleted != self.deletion_mark
    }
}

/// Ответ OData для списка организаций
#[derive(Debug, Deserialize)]
pub struct UtOrganizationListResponse {
    pub value: Vec<UtOrganizationOData>,
}
