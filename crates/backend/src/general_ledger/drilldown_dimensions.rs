//! Реестр измерений для GL-drilldown.
//!
//! Источник истины по доступным измерениям теперь хранится прямо в metadata
//! оборота (`TurnoverClassDef.available_dimensions`). Этот модуль только
//! преобразует ids измерений в UI-ready `GlDimensionDef`.

use contracts::general_ledger::GlDimensionDef;

use super::turnover_registry::get_turnover_class;

pub fn common_dimensions() -> Vec<GlDimensionDef> {
    dimensions_by_ids(&[
        "entry_date",
        "connection_mp_ref",
        "registrator_type",
        "layer",
        "registrator_ref",
    ])
}

pub fn nomenclature_dimensions() -> Vec<GlDimensionDef> {
    dimensions_by_ids(&[
        "nomenclature",
        "dim1_category",
        "dim2_line",
        "dim3_model",
        "dim4_format",
        "dim5_sink",
        "dim6_size",
    ])
}

fn dimensions_by_ids(ids: &[&str]) -> Vec<GlDimensionDef> {
    ids.iter()
        .filter_map(|id| dimension_def(id))
        .collect::<Vec<_>>()
}

fn dimension_def(dimension_id: &str) -> Option<GlDimensionDef> {
    Some(GlDimensionDef {
        id: dimension_id.to_string(),
        label: dimension_label(dimension_id)?.to_string(),
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

/// Метка измерения по id (для заголовка drilldown-таблицы).
pub fn dimension_label(dimension_id: &str) -> Option<&'static str> {
    match dimension_id {
        "entry_date" => Some("По дням"),
        "connection_mp_ref" => Some("По кабинету МП"),
        "registrator_type" => Some("По типу документа"),
        "layer" => Some("По слою"),
        "registrator_ref" => Some("По документу"),
        "nomenclature" => Some("По номенклатуре"),
        "dim1_category" => Some("По категории"),
        "dim2_line" => Some("По линейке"),
        "dim3_model" => Some("По модели"),
        "dim4_format" => Some("По формату"),
        "dim5_sink" => Some("По назначению"),
        "dim6_size" => Some("По размеру"),
        _ => None,
    }
}

/// Определяет, является ли измерение номенклатурным (требует JOIN a004).
pub fn is_nomenclature_dimension(dimension_id: &str) -> bool {
    matches!(
        dimension_id,
        "nomenclature"
            | "dim1_category"
            | "dim2_line"
            | "dim3_model"
            | "dim4_format"
            | "dim5_sink"
            | "dim6_size"
    )
}

#[cfg(test)]
mod tests {
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
}
