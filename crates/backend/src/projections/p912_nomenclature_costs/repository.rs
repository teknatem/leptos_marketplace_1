use anyhow::Result;
use chrono::{DateTime, Utc};
use contracts::projections::p912_nomenclature_costs::dto::NomenclatureCostDto;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, Set,
    Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p912_nomenclature_costs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub period: String,
    pub nomenclature_ref: String,
    pub cost: f64,
    pub quantity: Option<f64>,
    pub amount: Option<f64>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub line_no: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Clone)]
pub struct NomenclatureCostEntry {
    pub id: String,
    pub period: String,
    pub nomenclature_ref: String,
    pub cost: f64,
    pub quantity: Option<f64>,
    pub amount: Option<f64>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub line_no: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedCostRecord {
    pub nomenclature_ref: String,
    pub cost: f64,
    pub period: String,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub line_no: i32,
    pub updated_at: String,
}

impl NomenclatureCostEntry {
    pub fn to_active_model(&self) -> ActiveModel {
        ActiveModel {
            id: Set(self.id.clone()),
            period: Set(self.period.clone()),
            nomenclature_ref: Set(self.nomenclature_ref.clone()),
            cost: Set(self.cost),
            quantity: Set(self.quantity),
            amount: Set(self.amount),
            registrator_type: Set(self.registrator_type.clone()),
            registrator_ref: Set(self.registrator_ref.clone()),
            line_no: Set(self.line_no),
            created_at: Set(self.created_at.to_rfc3339()),
            updated_at: Set(self.updated_at.to_rfc3339()),
        }
    }
}

#[derive(Debug, Clone, FromQueryResult, Serialize, Deserialize)]
pub struct CostWithNomenclature {
    pub id: String,
    pub period: String,
    pub nomenclature_ref: String,
    pub cost: f64,
    pub quantity: Option<f64>,
    pub amount: Option<f64>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub line_no: i32,
    pub created_at: String,
    pub updated_at: String,
    pub nomenclature_name: Option<String>,
    pub nomenclature_article: Option<String>,
}

impl From<CostWithNomenclature> for NomenclatureCostDto {
    fn from(value: CostWithNomenclature) -> Self {
        Self {
            id: value.id,
            period: value.period,
            nomenclature_ref: value.nomenclature_ref,
            nomenclature_name: value.nomenclature_name,
            nomenclature_article: value.nomenclature_article,
            cost: value.cost,
            quantity: value.quantity,
            amount: value.amount,
            registrator_type: value.registrator_type,
            registrator_ref: value.registrator_ref,
            line_no: value.line_no,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn delete_by_registrator(registrator_type: &str, registrator_ref: &str) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::RegistratorType.eq(registrator_type))
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .exec(conn())
        .await?;
    Ok(result.rows_affected)
}

pub async fn replace_for_registrator(
    registrator_type: &str,
    registrator_ref: &str,
    entries: &[NomenclatureCostEntry],
) -> Result<()> {
    let db = conn();
    let txn = db.begin().await?;

    Entity::delete_many()
        .filter(Column::RegistratorType.eq(registrator_type))
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .exec(&txn)
        .await?;

    if !entries.is_empty() {
        Entity::insert_many(entries.iter().map(NomenclatureCostEntry::to_active_model))
            .exec(&txn)
            .await?;
    }

    txn.commit().await?;
    Ok(())
}

pub async fn get_by_registrator(
    registrator_type: &str,
    registrator_ref: &str,
) -> Result<Vec<NomenclatureCostDto>> {
    let items = Entity::find()
        .filter(Column::RegistratorType.eq(registrator_type))
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .order_by_asc(Column::LineNo)
        .all(conn())
        .await?;

    let nomenclature_ids: Vec<String> = items
        .iter()
        .map(|item| item.nomenclature_ref.clone())
        .collect();
    let nomenclature_map = if nomenclature_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        crate::domain::a004_nomenclature::repository::Entity::find()
            .filter(
                crate::domain::a004_nomenclature::repository::Column::Id
                    .is_in(nomenclature_ids.clone()),
            )
            .all(conn())
            .await?
            .into_iter()
            .map(|nom| (nom.id, (Some(nom.description), Some(nom.article))))
            .collect::<std::collections::HashMap<_, _>>()
    };

    Ok(items
        .into_iter()
        .map(|item| {
            let (nomenclature_name, nomenclature_article) = nomenclature_map
                .get(&item.nomenclature_ref)
                .cloned()
                .unwrap_or((None, None));
            NomenclatureCostDto {
                id: item.id,
                period: item.period,
                nomenclature_ref: item.nomenclature_ref,
                nomenclature_name,
                nomenclature_article,
                cost: item.cost,
                quantity: item.quantity,
                amount: item.amount,
                registrator_type: item.registrator_type,
                registrator_ref: item.registrator_ref,
                line_no: item.line_no,
                created_at: item.created_at,
                updated_at: item.updated_at,
            }
        })
        .collect())
}

pub async fn resolve_latest_cost_before_date(
    nomenclature_ref: &str,
    target_date: &str,
) -> Result<Option<ResolvedCostRecord>> {
    if nomenclature_ref.trim().is_empty() {
        return Ok(None);
    }

    let item = Entity::find()
        .filter(Column::NomenclatureRef.eq(nomenclature_ref.to_string()))
        .filter(Column::Period.lte(target_date.to_string()))
        .order_by_desc(Column::Period)
        .order_by_desc(Column::UpdatedAt)
        .order_by_desc(Column::LineNo)
        .one(conn())
        .await?;

    Ok(item.map(|row| ResolvedCostRecord {
        nomenclature_ref: row.nomenclature_ref,
        cost: row.cost,
        period: row.period,
        registrator_type: row.registrator_type,
        registrator_ref: row.registrator_ref,
        line_no: row.line_no,
        updated_at: row.updated_at,
    }))
}

pub async fn resolve_latest_costs_before_date(
    nomenclature_refs: &[String],
    target_date: &str,
) -> Result<std::collections::HashMap<String, ResolvedCostRecord>> {
    if nomenclature_refs.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let mut rows = Entity::find()
        .filter(Column::NomenclatureRef.is_in(nomenclature_refs.iter().cloned()))
        .filter(Column::Period.lte(target_date.to_string()))
        .order_by_asc(Column::NomenclatureRef)
        .order_by_desc(Column::Period)
        .order_by_desc(Column::UpdatedAt)
        .order_by_desc(Column::LineNo)
        .all(conn())
        .await?;

    let mut resolved = std::collections::HashMap::new();
    for row in rows.drain(..) {
        resolved
            .entry(row.nomenclature_ref.clone())
            .or_insert_with(|| ResolvedCostRecord {
                nomenclature_ref: row.nomenclature_ref,
                cost: row.cost,
                period: row.period,
                registrator_type: row.registrator_type,
                registrator_ref: row.registrator_ref,
                line_no: row.line_no,
                updated_at: row.updated_at,
            });
    }

    Ok(resolved)
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
    fn escape_like(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for ch in s.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '%' => out.push_str("\\%"),
                '_' => out.push_str("\\_"),
                _ => out.push(ch),
            }
        }
        out
    }

    let db = conn();
    let mut where_clauses = vec!["1=1".to_string()];
    let mut params: Vec<sea_orm::Value> = vec![];

    if let Some(period) = period.filter(|value| !value.trim().is_empty()) {
        where_clauses.push("p.period = ?".to_string());
        params.push(period.into());
    }
    if let Some(nomenclature_ref) = nomenclature_ref.filter(|value| !value.trim().is_empty()) {
        where_clauses.push("p.nomenclature_ref = ?".to_string());
        params.push(nomenclature_ref.into());
    }
    if let Some(registrator_type) = registrator_type.filter(|value| !value.trim().is_empty()) {
        where_clauses.push("p.registrator_type = ?".to_string());
        params.push(registrator_type.into());
    }
    if let Some(registrator_ref) = registrator_ref.filter(|value| !value.trim().is_empty()) {
        where_clauses.push("p.registrator_ref = ?".to_string());
        params.push(registrator_ref.into());
    }
    if let Some(q) = q.filter(|value| value.trim().len() >= 2) {
        let escaped = format!("%{}%", escape_like(&q.trim().to_lowercase()));
        where_clauses.push(
            "(lower(n.article) LIKE ? ESCAPE '\\' OR lower(n.description) LIKE ? ESCAPE '\\')"
                .to_string(),
        );
        params.push(escaped.clone().into());
        params.push(escaped.into());
    }

    let where_sql = where_clauses.join(" AND ");
    let count_sql = format!(
        "SELECT COUNT(*) as count \
         FROM p912_nomenclature_costs p \
         LEFT JOIN a004_nomenclature n ON p.nomenclature_ref = n.id \
         WHERE {where_sql}"
    );
    let count_stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &count_sql,
        params.iter().cloned(),
    );

    #[derive(Debug, FromQueryResult)]
    struct CountResult {
        count: i64,
    }

    let total_count = CountResult::find_by_statement(count_stmt)
        .one(db)
        .await?
        .map(|row| row.count)
        .unwrap_or(0);

    let mut list_sql = format!(
        "SELECT \
            p.id, p.period, p.nomenclature_ref, p.cost, p.quantity, p.amount, \
            p.registrator_type, p.registrator_ref, p.line_no, p.created_at, p.updated_at, \
            n.description as nomenclature_name, n.article as nomenclature_article \
         FROM p912_nomenclature_costs p \
         LEFT JOIN a004_nomenclature n ON p.nomenclature_ref = n.id \
         WHERE {where_sql} \
         ORDER BY p.period DESC, p.registrator_type ASC, p.registrator_ref ASC, p.line_no ASC"
    );

    if let Some(limit) = limit {
        list_sql.push_str(&format!(" LIMIT {}", limit.max(1)));
    }
    if let Some(offset) = offset {
        list_sql.push_str(&format!(" OFFSET {}", offset));
    }

    let list_stmt =
        Statement::from_sql_and_values(db.get_database_backend(), &list_sql, params.into_iter());
    let items = CostWithNomenclature::find_by_statement(list_stmt)
        .all(db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok((items, total_count))
}
