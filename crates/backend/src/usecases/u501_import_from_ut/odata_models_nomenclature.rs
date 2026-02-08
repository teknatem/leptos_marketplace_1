use contracts::domain::a004_nomenclature::aggregate::Nomenclature;
use serde::{Deserialize, Serialize};

/// OData модель справочника Номенклатура из УТ 11
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtNomenclatureOData {
    #[serde(rename = "Ref_Key", default)]
    pub ref_key: String,

    // Код может быть Code/Код
    #[serde(rename = "Code", alias = "Код", default)]
    pub code: String,

    // Наименование может быть Description/Наименование
    #[serde(rename = "Description", alias = "Наименование", default)]
    pub description: String,

    // Полное наименование
    #[serde(rename = "ПолноеНаименование", alias = "FullDescription", default)]
    pub full_description: Option<String>,

    // Признак папки
    #[serde(rename = "IsFolder", alias = "ЭтоГруппа", default)]
    pub is_folder: bool,

    // Родительский элемент может быть Parent_Key/ParentKey
    #[serde(rename = "Parent_Key", alias = "ParentKey", default)]
    pub parent_key: Option<String>,

    #[serde(rename = "DeletionMark", default)]
    pub deletion_mark: bool,

    // Артикул
    #[serde(rename = "Артикул", alias = "Article", default)]
    pub article: Option<String>,

    // Требуется сборка
    #[serde(rename = "ТребуетсяСборка", default)]
    pub is_assembly: Option<bool>,

    // Номенклатура для остатков (ссылка на другую номенклатуру)
    #[serde(rename = "НоменклатураДляОстатков_Key", default)]
    pub base_nomenclature_key: Option<String>,

    // Игнорировать дополнительные поля
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

impl UtNomenclatureOData {
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

    /// Преобразование OData модели в агрегат Nomenclature
    pub fn to_aggregate(&self) -> Result<Nomenclature, String> {
        use contracts::domain::a004_nomenclature::aggregate::NomenclatureId;
        use uuid::Uuid;

        tracing::debug!(
            "Converting Nomenclature OData: ref_key={}, code={}, description={}, full_description={:?}, is_folder={}, parent_key={:?}, article={:?}",
            self.ref_key,
            self.code,
            self.description,
            self.full_description,
            self.is_folder,
            self.parent_key,
            self.article
        );

        let id = if !self.ref_key.is_empty() {
            Uuid::parse_str(&self.ref_key)
                .map(NomenclatureId)
                .unwrap_or_else(|_| NomenclatureId::new_v4())
        } else {
            NomenclatureId::new_v4()
        };

        let parent_id = Self::normalize_parent_id(&self.parent_key);
        let base_nomenclature_ref = Self::normalize_parent_id(&self.base_nomenclature_key);

        let mut agg = Nomenclature::new_with_id(
            id,
            self.code.clone(),
            self.description.clone(),
            self.full_description.clone().unwrap_or_default(),
            self.is_folder,
            parent_id,
            self.article.clone().unwrap_or_default(),
            None,
        );
        // Применить новые поля
        agg.is_assembly = self.is_assembly.unwrap_or(false);
        agg.base_nomenclature_ref = base_nomenclature_ref;
        // Используем метод compute_is_derivative вместо дублирования логики
        agg.is_derivative = agg.compute_is_derivative();
        // Применить признак удаления из источника
        agg.base.metadata.is_deleted = self.deletion_mark;
        Ok(agg)
    }

    /// Проверка, нужно ли обновлять существующий агрегат
    pub fn should_update(&self, existing: &Nomenclature) -> bool {
        let normalized_parent = Self::normalize_parent_id(&self.parent_key);
        let normalized_base_ref = Self::normalize_parent_id(&self.base_nomenclature_key);
        existing.base.code != self.code
            || existing.base.description != self.description
            || existing.full_description != self.full_description.clone().unwrap_or_default()
            || existing.is_folder != self.is_folder
            || existing.parent_id != normalized_parent
            || existing.base.metadata.is_deleted != self.deletion_mark
            || existing.article != self.article.clone().unwrap_or_default()
            || existing.is_assembly != self.is_assembly.unwrap_or(false)
            || existing.base_nomenclature_ref != normalized_base_ref
    }
}

/// Ответ OData для списка номенклатуры
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtNomenclatureListResponse {
    pub value: Vec<UtNomenclatureOData>,
}
