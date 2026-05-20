//! Единый реестр измерений для GL-drilldown и UI-каталога измерений.
//!
//! Источник истины по составу измерений оборота хранится в metadata оборотов
//! (`TurnoverClassDef.available_dimensions`). Этот модуль:
//! - даёт UI-ready `GlDimensionDef` по ids измерений
//! - собирает сигнатуру измерений оборота
//! - строит плоский каталог измерений с usage по оборотам

use std::collections::HashMap;

use contracts::general_ledger::{GlDimensionCatalogItem, GlDimensionDef, GlDimensionUsageRef};

use super::turnover_registry::{get_turnover_class, TURNOVER_CLASSES};

const COMMON_DIMENSION_IDS: &[&str] = &[
    "entry_date",
    "connection_mp_ref",
    "registrator_type",
    "layer",
    "registrator_ref",
];

const NOMENCLATURE_DIMENSION_IDS: &[&str] = &[
    "nomenclature",
    "dim1_category",
    "dim2_line",
    "dim3_model",
    "dim4_format",
    "dim5_sink",
    "dim6_size",
];

#[derive(Debug, Clone, Copy)]
struct DimensionSeed {
    id: &'static str,
    label: &'static str,
    code_main: &'static str,
    code_suffix: Option<&'static str>,
    parent_id: Option<&'static str>,
    sort_order: usize,
    /// Квалифицированное имя поля в БД: таблица.колонка.
    db_field: &'static str,
}

const DIMENSION_SEEDS: &[DimensionSeed] = &[
    DimensionSeed {
        id: "entry_date",
        label: "По дням",
        code_main: "Day",
        code_suffix: None,
        parent_id: None,
        sort_order: 10,
        db_field: "sys_gl.entry_date",
    },
    DimensionSeed {
        id: "connection_mp_ref",
        label: "По кабинету МП",
        code_main: "Cab",
        code_suffix: None,
        parent_id: None,
        sort_order: 20,
        db_field: "sys_gl.connection_mp_ref",
    },
    DimensionSeed {
        id: "registrator_type",
        label: "По типу регистратора",
        code_main: "RegType",
        code_suffix: None,
        parent_id: None,
        sort_order: 30,
        db_field: "sys_gl.registrator_type",
    },
    DimensionSeed {
        id: "layer",
        label: "По слою",
        code_main: "Layer",
        code_suffix: None,
        parent_id: None,
        sort_order: 40,
        db_field: "sys_gl.layer",
    },
    DimensionSeed {
        id: "registrator_ref",
        label: "По регистратору",
        code_main: "RegRef",
        code_suffix: None,
        parent_id: None,
        sort_order: 50,
        db_field: "sys_gl.registrator_ref",
    },
    DimensionSeed {
        id: "nomenclature",
        label: "По номенклатуре",
        code_main: "Nom",
        code_suffix: None,
        parent_id: None,
        sort_order: 60,
        db_field: "sys_gl.nomenclature → a004_nm",
    },
    DimensionSeed {
        id: "dim1_category",
        label: "По категории",
        code_main: "Nom",
        code_suffix: Some("01"),
        parent_id: Some("nomenclature"),
        sort_order: 61,
        db_field: "a004_nm.dim1_category",
    },
    DimensionSeed {
        id: "dim2_line",
        label: "По линейке",
        code_main: "Nom",
        code_suffix: Some("02"),
        parent_id: Some("nomenclature"),
        sort_order: 62,
        db_field: "a004_nm.dim2_line",
    },
    DimensionSeed {
        id: "dim3_model",
        label: "По модели",
        code_main: "Nom",
        code_suffix: Some("03"),
        parent_id: Some("nomenclature"),
        sort_order: 63,
        db_field: "a004_nm.dim3_model",
    },
    DimensionSeed {
        id: "dim4_format",
        label: "По формату",
        code_main: "Nom",
        code_suffix: Some("04"),
        parent_id: Some("nomenclature"),
        sort_order: 64,
        db_field: "a004_nm.dim4_format",
    },
    DimensionSeed {
        id: "dim5_sink",
        label: "По назначению",
        code_main: "Nom",
        code_suffix: Some("05"),
        parent_id: Some("nomenclature"),
        sort_order: 65,
        db_field: "a004_nm.dim5_sink",
    },
    DimensionSeed {
        id: "dim6_size",
        label: "По размеру",
        code_main: "Nom",
        code_suffix: Some("06"),
        parent_id: Some("nomenclature"),
        sort_order: 66,
        db_field: "a004_nm.dim6_size",
    },
];

pub fn common_dimensions() -> Vec<GlDimensionDef> {
    dimensions_by_ids(COMMON_DIMENSION_IDS)
}

pub fn nomenclature_dimensions() -> Vec<GlDimensionDef> {
    dimensions_by_ids(NOMENCLATURE_DIMENSION_IDS)
}

pub fn dimensions_catalog() -> Vec<GlDimensionCatalogItem> {
    let usage_map = dimension_usage_map();
    let mut items = DIMENSION_SEEDS
        .iter()
        .map(|seed| build_catalog_item(*seed, &usage_map))
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.code.cmp(&right.code))
    });

    items
}

fn dimensions_by_ids(ids: &[&str]) -> Vec<GlDimensionDef> {
    ids.iter()
        .filter_map(|id| dimension_def(id))
        .collect::<Vec<_>>()
}

fn dimension_def(dimension_id: &str) -> Option<GlDimensionDef> {
    let seed = dimension_seed(dimension_id)?;
    Some(GlDimensionDef {
        id: seed.id.to_string(),
        label: seed.label.to_string(),
        code: seed.code().to_string(),
        code_main: seed.code_main.to_string(),
        code_suffix: seed.code_suffix.map(ToString::to_string),
        parent_id: seed.parent_id.map(ToString::to_string),
        db_field: seed.db_field.to_string(),
    })
}

/// Возвращает список доступных измерений для конкретного вида оборота.
///
/// Источник истины — `TurnoverClassDef.available_dimensions`.
/// Если оборот неизвестен, оставляем безопасный fallback только на common dimensions.
pub fn dimensions_for_turnover(turnover_code: &str) -> Vec<GlDimensionDef> {
    get_turnover_class(turnover_code)
        .map(|tc| dimensions_by_ids(tc.available_dimensions))
        .unwrap_or_else(common_dimensions)
}

pub fn dimension_signature_for_turnover(turnover_code: &str) -> String {
    get_turnover_class(turnover_code)
        .map(|tc| dimension_signature_from_ids(tc.available_dimensions))
        .unwrap_or_else(|| dimension_signature_from_ids(COMMON_DIMENSION_IDS))
}

pub fn dimension_signature_from_ids(ids: &[&str]) -> String {
    ids.iter()
        .filter_map(|id| dimension_seed(id).map(DimensionSeed::code))
        .collect::<Vec<_>>()
        .join(".")
}

/// Метка измерения по id (для заголовка drilldown-таблицы).
pub fn dimension_label(dimension_id: &str) -> Option<&'static str> {
    dimension_seed(dimension_id).map(|seed| seed.label)
}

/// Определяет, является ли измерение номенклатурным (требует JOIN a004).
pub fn is_nomenclature_dimension(dimension_id: &str) -> bool {
    dimension_seed(dimension_id)
        .map(|seed| seed.code_main == "Nom")
        .unwrap_or(false)
}

fn dimension_seed(dimension_id: &str) -> Option<&'static DimensionSeed> {
    DIMENSION_SEEDS.iter().find(|seed| seed.id == dimension_id)
}

fn build_catalog_item(
    seed: DimensionSeed,
    usage_map: &HashMap<&'static str, Vec<GlDimensionUsageRef>>,
) -> GlDimensionCatalogItem {
    let path = build_seed_path(seed);
    let root = path.first().copied().unwrap_or(seed);
    let used_by_turnovers = usage_map.get(seed.id).cloned().unwrap_or_default();

    GlDimensionCatalogItem {
        id: seed.id.to_string(),
        label: seed.label.to_string(),
        code: seed.code().to_string(),
        code_main: seed.code_main.to_string(),
        code_suffix: seed.code_suffix.map(ToString::to_string),
        parent_id: seed.parent_id.map(ToString::to_string),
        root_id: root.id.to_string(),
        depth: path.len().saturating_sub(1),
        sort_order: seed.sort_order,
        path_ids: path.iter().map(|item| item.id.to_string()).collect(),
        path_codes: path.iter().map(|item| item.code().to_string()).collect(),
        turnover_count: used_by_turnovers.len(),
        used_by_turnovers,
        db_field: seed.db_field.to_string(),
    }
}

fn build_seed_path(seed: DimensionSeed) -> Vec<DimensionSeed> {
    let mut path = vec![seed];
    let mut current = seed;

    while let Some(parent_id) = current.parent_id {
        let parent = dimension_seed(parent_id)
            .copied()
            .unwrap_or_else(|| panic!("Unknown parent dimension id: {parent_id}"));
        path.push(parent);
        current = parent;
    }

    path.reverse();
    path
}

fn dimension_usage_map() -> HashMap<&'static str, Vec<GlDimensionUsageRef>> {
    let mut usage_map: HashMap<&'static str, Vec<GlDimensionUsageRef>> = HashMap::new();

    for turnover in TURNOVER_CLASSES {
        let usage_ref = GlDimensionUsageRef {
            turnover_code: turnover.code.to_string(),
            turnover_name: turnover.name.to_string(),
            report_group: turnover.report_group.as_str().to_string(),
        };

        for dimension_id in turnover.available_dimensions {
            usage_map
                .entry(*dimension_id)
                .or_default()
                .push(usage_ref.clone());
        }
    }

    for usages in usage_map.values_mut() {
        usages.sort_by(|left, right| left.turnover_code.cmp(&right.turnover_code));
    }

    usage_map
}

impl DimensionSeed {
    fn code(&self) -> &'static str {
        match self.id {
            "entry_date" => "Day",
            "connection_mp_ref" => "Cab",
            "registrator_type" => "RegType",
            "layer" => "Layer",
            "registrator_ref" => "RegRef",
            "nomenclature" => "Nom",
            "dim1_category" => "Nom01",
            "dim2_line" => "Nom02",
            "dim3_model" => "Nom03",
            "dim4_format" => "Nom04",
            "dim5_sink" => "Nom05",
            "dim6_size" => "Nom06",
            _ => unreachable!("dimension registry must cover all supported ids"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn explicit_dimensions_distinguish_nm_and_no_nm_turnovers() {
        let base = dimensions_for_turnover("mp_commission_adjustment");
        let base_ids = base.iter().map(|item| item.id.as_str()).collect::<Vec<_>>();
        assert!(!base_ids.contains(&"nomenclature"));
        assert!(base_ids.contains(&"registrator_ref"));

        let with_nm = dimensions_for_turnover("mp_commission_adjustment_nm");
        let with_nm_ids = with_nm
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>();
        assert!(with_nm_ids.contains(&"nomenclature"));
        assert!(with_nm_ids.contains(&"dim1_category"));
    }

    #[test]
    fn dimension_codes_are_unique() {
        let items = dimensions_catalog();
        let codes = items
            .iter()
            .map(|item| item.code.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(codes.len(), items.len());
    }

    #[test]
    fn nomenclature_children_have_global_fixed_codes() {
        let items = dimensions_catalog();
        let codes_by_id = items
            .iter()
            .map(|item| (item.id.as_str(), item.code.as_str()))
            .collect::<HashMap<_, _>>();

        assert_eq!(codes_by_id.get("nomenclature"), Some(&"Nom"));
        assert_eq!(codes_by_id.get("dim1_category"), Some(&"Nom01"));
        assert_eq!(codes_by_id.get("dim2_line"), Some(&"Nom02"));
        assert_eq!(codes_by_id.get("dim3_model"), Some(&"Nom03"));
        assert_eq!(codes_by_id.get("dim4_format"), Some(&"Nom04"));
        assert_eq!(codes_by_id.get("dim5_sink"), Some(&"Nom05"));
        assert_eq!(codes_by_id.get("dim6_size"), Some(&"Nom06"));
    }

    #[test]
    fn dimension_catalog_builds_hierarchy_and_paths() {
        let item = dimensions_catalog()
            .into_iter()
            .find(|item| item.id == "dim3_model")
            .expect("dim3_model not found");

        assert_eq!(item.parent_id.as_deref(), Some("nomenclature"));
        assert_eq!(item.root_id, "nomenclature");
        assert_eq!(item.depth, 1);
        assert_eq!(item.path_ids, vec!["nomenclature", "dim3_model"]);
        assert_eq!(item.path_codes, vec!["Nom", "Nom03"]);
    }

    #[test]
    fn dimension_signature_matches_profiles() {
        assert_eq!(
            dimension_signature_from_ids(COMMON_DIMENSION_IDS),
            "Day.Cab.RegType.Layer.RegRef"
        );
        assert_eq!(
            dimension_signature_from_ids(&[
                "entry_date",
                "connection_mp_ref",
                "registrator_type",
                "layer",
                "registrator_ref",
                "nomenclature",
                "dim1_category",
                "dim2_line",
                "dim3_model",
                "dim4_format",
                "dim5_sink",
                "dim6_size",
            ]),
            "Day.Cab.RegType.Layer.RegRef.Nom.Nom01.Nom02.Nom03.Nom04.Nom05.Nom06"
        );
    }

    #[test]
    fn dimension_catalog_contains_turnover_usage() {
        let item = dimensions_catalog()
            .into_iter()
            .find(|item| item.id == "entry_date")
            .expect("entry_date not found");

        assert!(item.turnover_count > 0);
        assert!(item
            .used_by_turnovers
            .iter()
            .any(|usage| usage.turnover_code == "qty_ordered"));
    }
}
