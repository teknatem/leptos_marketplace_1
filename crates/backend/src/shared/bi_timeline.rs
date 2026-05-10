use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::Result;
use chrono::NaiveDate;
use contracts::domain::a024_bi_indicator::aggregate::{BiIndicator, BiIndicatorStatus};
use contracts::domain::common::AggregateId;
use contracts::shared::bi_timeline::{
    BiTimelineError, BiTimelineIndicatorInfo, BiTimelineIndicatorsResponse, BiTimelinePoint,
    BiTimelineRequest, BiTimelineResponse, BiTimelineSeries,
};
use contracts::shared::data_view::{DataViewMeta, ViewContext};
use contracts::shared::drilldown::{DrilldownResponse, DrilldownRow};

use crate::data_view::DataViewRegistry;
use crate::domain::a024_bi_indicator;

const PRIORITY_CODES: &[&str] = &[
    "IND-WB-ADS-SPEND",
    "REVENUE",
    "IND-REVENUE-WB",
    "IND-ORDERS",
];

fn priority_rank(code: &str) -> usize {
    PRIORITY_CODES
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(code))
        .unwrap_or(usize::MAX)
}

fn aliases_for_code(code: &str) -> Vec<&'static str> {
    match code.trim().to_ascii_uppercase().as_str() {
        "REVENUE" => vec!["REVENUE", "IND-REVENUE-WB"],
        "IND-REVENUE-WB" => vec!["IND-REVENUE-WB", "REVENUE"],
        "IND-WB-ADS-SPEND" => vec!["IND-WB-ADS-SPEND"],
        "IND-ORDERS" => vec!["IND-ORDERS"],
        _ => vec![],
    }
}

fn is_priority(code: &str) -> bool {
    PRIORITY_CODES
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(code))
}

fn find_day_dimension(meta: &DataViewMeta) -> Option<String> {
    meta.available_dimensions
        .iter()
        .find(|dim| dim.id == "date")
        .or_else(|| {
            meta.available_dimensions
                .iter()
                .find(|dim| dim.id == "entry_date")
        })
        .or_else(|| {
            meta.available_dimensions.iter().find(|dim| {
                dim.db_column
                    .as_deref()
                    .map(|column| column == "date" || column == "entry_date")
                    .unwrap_or(false)
            })
        })
        .map(|dim| dim.id.clone())
}

fn indicator_info(indicator: &BiIndicator, registry: &DataViewRegistry) -> BiTimelineIndicatorInfo {
    let view_id = indicator
        .data_spec
        .view_id
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let (compatible, day_dimension, reason) = match view_id.as_deref() {
        Some(view_id) => match registry.get_meta(view_id) {
            Some(meta) => match find_day_dimension(meta) {
                Some(day_dimension) => (true, Some(day_dimension), None),
                None => (
                    false,
                    None,
                    Some("DataView не объявляет дневное измерение".to_string()),
                ),
            },
            None => (
                false,
                None,
                Some(format!("DataView '{}' не найден", view_id)),
            ),
        },
        None => (
            false,
            None,
            Some("У индикатора не задан data_spec.view_id".to_string()),
        ),
    };

    BiTimelineIndicatorInfo {
        id: indicator.base.id.as_string(),
        code: indicator.base.code.clone(),
        description: indicator.base.description.clone(),
        comment: indicator.base.comment.clone(),
        view_id,
        metric_id: indicator.data_spec.metric_id.clone(),
        day_dimension,
        compatible,
        reason,
        priority: is_priority(&indicator.base.code),
        format: indicator.view_spec.format.clone(),
    }
}

fn build_view_context(
    indicator: &BiIndicator,
    base_ctx: &ViewContext,
    request_params: &HashMap<String, String>,
) -> ViewContext {
    let mut params = HashMap::new();

    for param in &indicator.params {
        if let Some(default_value) = param
            .default_value
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            params.insert(param.key.clone(), default_value.to_string());
        }
    }

    params.extend(base_ctx.params.clone());
    params.extend(request_params.clone());

    if let Some(metric_id) = indicator
        .data_spec
        .metric_id
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        params.insert("metric".to_string(), metric_id.to_string());
    }

    ViewContext {
        date_from: base_ctx.date_from.clone(),
        date_to: base_ctx.date_to.clone(),
        period2_from: base_ctx.period2_from.clone(),
        period2_to: base_ctx.period2_to.clone(),
        connection_mp_refs: base_ctx.connection_mp_refs.clone(),
        params,
    }
}

fn day_offset_from_date(value: &str) -> Option<i64> {
    let date_part = value.split('T').next().unwrap_or(value);
    let mut parts = date_part.split('-');
    let _year = parts.next()?;
    let _month = parts.next()?;
    let day = parts.next()?.parse::<i64>().ok()?;
    (1..=31).contains(&day).then_some(day - 1)
}

fn day_offset_from_row(row: &DrilldownRow, fallback_idx: usize) -> i64 {
    row.label
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|day| (1..=31).contains(day))
        .map(|day| day - 1)
        .or_else(|| day_offset_from_date(&row.group_key))
        .or_else(|| {
            row.group_key
                .trim()
                .parse::<i64>()
                .ok()
                .filter(|offset| (0..=370).contains(offset))
        })
        .unwrap_or(fallback_idx as i64)
}

fn points_from_drilldown_rows(
    response: &DrilldownResponse,
    ctx: &ViewContext,
) -> (Vec<BiTimelinePoint>, Vec<BiTimelinePoint>) {
    let mut by_day: BTreeMap<i64, (String, f64, f64)> = BTreeMap::new();
    let p1_days = inclusive_days(Some(&ctx.date_from), Some(&ctx.date_to)).unwrap_or(31);
    let p2_days =
        inclusive_days(ctx.period2_from.as_deref(), ctx.period2_to.as_deref()).unwrap_or(p1_days);

    for (idx, row) in response.rows.iter().enumerate() {
        if row.group_key == "__other__" {
            continue;
        }
        let offset = day_offset_from_row(row, idx);
        let label = (offset + 1).to_string();
        let entry = by_day.entry(offset).or_insert((label, 0.0, 0.0));
        entry.1 += row.value1;
        entry.2 += row.value2;
    }

    let mut p1 = Vec::with_capacity(by_day.len());
    let mut p2 = Vec::with_capacity(by_day.len());
    for (offset, (label, value1, value2)) in by_day {
        if offset < p1_days as i64 && value1.abs() >= 0.000_001 {
            p1.push(BiTimelinePoint {
                offset,
                label: label.clone(),
                value: value1,
            });
        }
        if offset < p2_days as i64 && value2.abs() >= 0.000_001 {
            p2.push(BiTimelinePoint {
                offset,
                label,
                value: value2,
            });
        }
    }

    (p1, p2)
}

fn parse_ymd(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn inclusive_days(from: Option<&str>, to: Option<&str>) -> Option<usize> {
    let from = from.and_then(parse_ymd)?;
    let to = to.and_then(parse_ymd)?;
    Some((to - from).num_days().max(0) as usize + 1)
}

pub async fn list_compatible_indicators() -> Result<BiTimelineIndicatorsResponse> {
    let registry = DataViewRegistry::new();
    let mut indicators: Vec<BiTimelineIndicatorInfo> = a024_bi_indicator::service::list_all()
        .await?
        .into_iter()
        .filter(|indicator| indicator.status == BiIndicatorStatus::Active)
        .map(|indicator| indicator_info(&indicator, &registry))
        .filter(|info| info.compatible)
        .collect();

    indicators.sort_by(|left, right| {
        priority_rank(&left.code)
            .cmp(&priority_rank(&right.code))
            .then_with(|| left.description.cmp(&right.description))
            .then_with(|| left.code.cmp(&right.code))
    });

    Ok(BiTimelineIndicatorsResponse { indicators })
}

fn resolve_requested_indicators(
    all: Vec<BiIndicator>,
    indicator_ids: &[String],
    indicator_codes: &[String],
) -> Vec<BiIndicator> {
    let requested_ids: HashSet<String> = indicator_ids.iter().cloned().collect();
    let requested_codes: Vec<String> = indicator_codes
        .iter()
        .flat_map(|code| {
            let aliases = aliases_for_code(code);
            if aliases.is_empty() {
                vec![code.as_str()]
            } else {
                aliases
            }
        })
        .map(|code| code.to_ascii_uppercase())
        .collect();

    let mut seen = HashSet::new();
    all.into_iter()
        .filter(|indicator| {
            requested_ids.contains(&indicator.base.id.as_string())
                || requested_codes
                    .iter()
                    .any(|code| indicator.base.code.eq_ignore_ascii_case(code))
        })
        .filter(|indicator| seen.insert(indicator.base.id.as_string()))
        .collect()
}

pub async fn build_timeline(req: BiTimelineRequest) -> Result<BiTimelineResponse> {
    let registry = DataViewRegistry::new();
    let all_indicators = a024_bi_indicator::service::list_all().await?;
    let mut selected =
        resolve_requested_indicators(all_indicators, &req.indicator_ids, &req.indicator_codes);

    if selected.is_empty() {
        selected = a024_bi_indicator::service::list_all()
            .await?
            .into_iter()
            .filter(|indicator| indicator.status == BiIndicatorStatus::Active)
            .filter(|indicator| is_priority(&indicator.base.code))
            .collect();
    }

    let mut items = Vec::new();
    let mut errors = Vec::new();

    for indicator in selected {
        let info = indicator_info(&indicator, &registry);
        if !info.compatible {
            errors.push(BiTimelineError {
                indicator_id: info.id,
                message: info
                    .reason
                    .unwrap_or_else(|| "Индикатор несовместим с BI Timeline".to_string()),
            });
            continue;
        }

        let Some(view_id) = info.view_id.clone() else {
            continue;
        };
        let Some(day_dimension) = info.day_dimension.clone() else {
            continue;
        };

        let view_ctx = build_view_context(&indicator, &req.context, &req.params);
        match registry
            .compute_drilldown(&view_id, &view_ctx, &day_dimension, &[])
            .await
        {
            Ok(response) => {
                let (series_p1, series_p2) = points_from_drilldown_rows(&response, &view_ctx);
                items.push(BiTimelineSeries {
                    indicator: info,
                    period1_label: response.period1_label,
                    period2_label: response.period2_label,
                    series_p1,
                    series_p2,
                });
            }
            Err(err) => {
                errors.push(BiTimelineError {
                    indicator_id: info.id,
                    message: err.to_string(),
                });
            }
        }
    }

    items.sort_by(|left, right| {
        priority_rank(&left.indicator.code)
            .cmp(&priority_rank(&right.indicator.code))
            .then_with(|| left.indicator.description.cmp(&right.indicator.description))
    });

    Ok(BiTimelineResponse { items, errors })
}
