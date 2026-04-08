use anyhow::Result;
use chrono::Utc;
use contracts::domain::a021_production_output::aggregate::ProductionOutput;
use contracts::domain::a023_purchase_of_goods::aggregate::PurchaseOfGoods;
use contracts::projections::p912_nomenclature_costs::dto::NomenclatureCostDto;
use std::collections::HashMap;

use super::repository::{self, NomenclatureCostEntry, ResolvedCostRecord};

const A021_REGISTRATOR_TYPE: &str = "a021_production_output";
const A023_REGISTRATOR_TYPE: &str = "a023_purchase_of_goods";

fn make_entry_id(registrator_type: &str, registrator_ref: &str, line_no: i32) -> String {
    format!("{registrator_type}:{registrator_ref}:{line_no}")
}

fn production_entries(document: &ProductionOutput) -> Vec<NomenclatureCostEntry> {
    let Some(nomenclature_ref) = document.nomenclature_ref.clone() else {
        return Vec::new();
    };
    let Some(cost) = document.cost_of_production.filter(|value| *value > 0.0) else {
        return Vec::new();
    };
    let registrator_ref = document.to_string_id();
    let now = Utc::now();

    vec![NomenclatureCostEntry {
        id: make_entry_id(A021_REGISTRATOR_TYPE, &registrator_ref, 0),
        period: document.document_date.clone(),
        nomenclature_ref,
        cost,
        quantity: Some(document.count as f64),
        amount: Some(document.amount),
        registrator_type: A021_REGISTRATOR_TYPE.to_string(),
        registrator_ref,
        line_no: 0,
        created_at: now,
        updated_at: now,
    }]
}

fn purchase_entries(document: &PurchaseOfGoods) -> Vec<NomenclatureCostEntry> {
    let registrator_ref = document.to_string_id();
    let now = Utc::now();

    document
        .parse_lines()
        .into_iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            if line.nomenclature_key.trim().is_empty() || line.price <= 0.0 {
                return None;
            }
            Some(NomenclatureCostEntry {
                id: make_entry_id(A023_REGISTRATOR_TYPE, &registrator_ref, idx as i32),
                period: document.document_date.clone(),
                nomenclature_ref: line.nomenclature_key,
                cost: line.price,
                quantity: Some(line.quantity),
                amount: Some(line.amount_with_vat),
                registrator_type: A023_REGISTRATOR_TYPE.to_string(),
                registrator_ref: registrator_ref.clone(),
                line_no: idx as i32,
                created_at: now,
                updated_at: now,
            })
        })
        .collect()
}

pub async fn project_production_output(document: &ProductionOutput) -> Result<()> {
    repository::replace_for_registrator(
        A021_REGISTRATOR_TYPE,
        &document.to_string_id(),
        &production_entries(document),
    )
    .await
}

pub async fn project_purchase_of_goods(document: &PurchaseOfGoods) -> Result<()> {
    repository::replace_for_registrator(
        A023_REGISTRATOR_TYPE,
        &document.to_string_id(),
        &purchase_entries(document),
    )
    .await
}

pub async fn remove_by_registrator(registrator_type: &str, registrator_ref: &str) -> Result<u64> {
    repository::delete_by_registrator(registrator_type, registrator_ref).await
}

pub async fn get_by_registrator(
    registrator_type: &str,
    registrator_ref: &str,
) -> Result<Vec<NomenclatureCostDto>> {
    repository::get_by_registrator(registrator_type, registrator_ref).await
}

pub async fn resolve_latest_cost_before_date(
    nomenclature_ref: &str,
    target_date: &str,
) -> Result<Option<ResolvedCostRecord>> {
    repository::resolve_latest_cost_before_date(nomenclature_ref, target_date).await
}

pub async fn resolve_latest_costs_before_date(
    nomenclature_refs: &[String],
    target_date: &str,
) -> Result<HashMap<String, ResolvedCostRecord>> {
    repository::resolve_latest_costs_before_date(nomenclature_refs, target_date).await
}

pub async fn list_with_filters(
    period: Option<String>,
    nomenclature_ref: Option<String>,
    registrator_type: Option<String>,
    registrator_ref: Option<String>,
    q: Option<String>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<(Vec<NomenclatureCostDto>, i64)> {
    repository::list_with_filters(
        period,
        nomenclature_ref,
        registrator_type,
        registrator_ref,
        q,
        limit,
        offset,
    )
    .await
}
