use contracts::domain::a002_organization::aggregate::Organization;
use serde::{Deserialize, Serialize};

/// OData модель справочника Организации из УТ 11
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtOrganizationOData {
    #[serde(rename = "Ref_Key", default)]
    pub ref_key: String,

    #[serde(rename = "Code", default)]
    pub code: String,

    #[serde(rename = "Description", default)]
    pub description: String,

    #[serde(rename = "ПолноеНаименование", default)]
    pub full_name: Option<String>,

    #[serde(rename = "ИНН", default)]
    pub inn: Option<String>,

    #[serde(rename = "КПП", default)]
    pub kpp: Option<String>,

    #[serde(rename = "DeletionMark", default)]
    pub deletion_mark: bool,
}

impl UtOrganizationOData {
    /// Преобразование OData модели в агрегат Organization
    pub fn to_aggregate(&self) -> Result<Organization, String> {
        use uuid::Uuid;
        use contracts::domain::a002_organization::aggregate::OrganizationId;

        let full_name = self.full_name.clone()
            .unwrap_or_else(|| self.description.clone());

        let inn = self.inn.clone().unwrap_or_default();
        let kpp = self.kpp.clone().unwrap_or_default();

        // Используем Ref_Key из 1С как UUID для организации
        let org_id = if !self.ref_key.is_empty() {
            // Парсим Ref_Key как UUID
            Uuid::parse_str(&self.ref_key)
                .map(OrganizationId)
                .unwrap_or_else(|_| {
                    tracing::warn!("Failed to parse Ref_Key as UUID: {}, generating new UUID", self.ref_key);
                    OrganizationId::new_v4()
                })
        } else {
            tracing::warn!("Empty Ref_Key for organization {}, generating new UUID", self.description);
            OrganizationId::new_v4()
        };

        let org = Organization::new_with_id(
            org_id,
            self.code.clone(),
            self.description.clone(),
            full_name,
            inn,
            kpp,
            None,
        );

        Ok(org)
    }

    /// Проверка, нужно ли обновлять существующий агрегат
    pub fn should_update(&self, existing: &Organization) -> bool {
        existing.base.code != self.code
            || existing.base.description != self.description
            || existing.full_name != self.full_name.clone().unwrap_or_else(|| self.description.clone())
            || existing.inn != self.inn.clone().unwrap_or_default()
            || existing.kpp != self.kpp.clone().unwrap_or_default()
    }
}

/// Ответ OData для списка организаций
#[derive(Debug, Deserialize)]
pub struct UtOrganizationListResponse {
    pub value: Vec<UtOrganizationOData>,
}
