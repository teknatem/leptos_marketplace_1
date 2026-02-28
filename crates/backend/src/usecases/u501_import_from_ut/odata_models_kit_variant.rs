use contracts::domain::a022_kit_variant::aggregate::{GoodsItem, KitVariant, KitVariantId};
use serde::{Deserialize, Serialize};

/// OData модель строки табличной части Товары
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UtKitVariantGoodsItemOData {
    #[serde(rename = "Номенклатура_Key", default)]
    pub nomenclature_key: String,

    #[serde(rename = "Количество", default)]
    pub quantity: f64,

    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

/// OData модель справочника ВариантыКомплектацииНоменклатуры из УТ 11
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtKitVariantOData {
    #[serde(rename = "Ref_Key", default)]
    pub ref_key: String,

    #[serde(rename = "Code", alias = "Код", default)]
    pub code: String,

    #[serde(rename = "Description", alias = "Наименование", default)]
    pub description: String,

    /// Владелец — ссылка на номенклатуру (английское имя в OData)
    #[serde(rename = "Owner_Key", default)]
    pub owner_key: String,

    #[serde(rename = "DeletionMark", default)]
    pub deletion_mark: bool,

    /// Признак: является ли этот вариант основным
    #[serde(rename = "Основной", default)]
    pub is_main: bool,

    /// Табличная часть Товары (загружается через $expand=Товары)
    #[serde(rename = "Товары", default)]
    pub goods: Vec<UtKitVariantGoodsItemOData>,

    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

impl UtKitVariantOData {
    /// Преобразование OData модели в агрегат KitVariant
    pub fn to_aggregate(&self, connection_id: &str) -> Result<KitVariant, String> {
        use uuid::Uuid;

        let id = if !self.ref_key.is_empty() {
            Uuid::parse_str(&self.ref_key)
                .map(KitVariantId::new)
                .unwrap_or_else(|_| KitVariantId::new_v4())
        } else {
            KitVariantId::new_v4()
        };

        let owner_ref = if !self.owner_key.is_empty() {
            Uuid::parse_str(&self.owner_key)
                .ok()
                .map(|u| u.to_string())
        } else {
            None
        };

        let goods: Vec<GoodsItem> = self
            .goods
            .iter()
            .filter_map(|g| {
                let nom_ref = Uuid::parse_str(&g.nomenclature_key)
                    .ok()
                    .map(|u| u.to_string())?;
                Some(GoodsItem {
                    nomenclature_ref: nom_ref,
                    quantity: g.quantity,
                })
            })
            .collect();

        let goods_json = if goods.is_empty() {
            None
        } else {
            serde_json::to_string(&goods).ok()
        };

        let mut agg = KitVariant::new_from_odata(
            id.value(),
            self.code.clone(),
            self.description.clone(),
            owner_ref,
            goods_json,
            connection_id.to_string(),
        );
        agg.base.metadata.is_deleted = self.deletion_mark;

        Ok(agg)
    }
}

/// Ответ OData для списка вариантов комплектации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtKitVariantListResponse {
    pub value: Vec<UtKitVariantOData>,
}
