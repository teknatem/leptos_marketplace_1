use chrono::Utc;
use contracts::domain::a007_marketplace_product::aggregate::{
    MarketplaceProduct, MarketplaceProductId,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set, Statement, Value};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a007_marketplace_product")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub marketplace_ref: String,
    pub connection_mp_ref: String,
    pub marketplace_sku: String,
    pub barcode: Option<String>,
    pub article: String,
    pub brand: Option<String>,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,
    pub nomenclature_ref: Option<String>,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for MarketplaceProduct {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        MarketplaceProduct {
            base: BaseAggregate::with_metadata(
                MarketplaceProductId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            marketplace_ref: m.marketplace_ref,
            connection_mp_ref: m.connection_mp_ref,
            marketplace_sku: m.marketplace_sku,
            barcode: m.barcode,
            article: m.article,
            brand: m.brand,
            category_id: m.category_id,
            category_name: m.category_name,
            last_update: m.last_update,
            nomenclature_ref: m.nomenclature_ref,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> anyhow::Result<Vec<MarketplaceProduct>> {
    let mut items: Vec<MarketplaceProduct> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    items.sort_by(|a, b| {
        a.base
            .description
            .to_lowercase()
            .cmp(&b.base.description.to_lowercase())
    });
    Ok(items)
}

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<MarketplaceProduct>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn insert(aggregate: &MarketplaceProduct) -> anyhow::Result<Uuid> {
    let uuid = aggregate.base.id.value();
    let active = ActiveModel {
        id: Set(uuid.to_string()),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        marketplace_ref: Set(aggregate.marketplace_ref.clone()),
        connection_mp_ref: Set(aggregate.connection_mp_ref.clone()),
        marketplace_sku: Set(aggregate.marketplace_sku.clone()),
        barcode: Set(aggregate.barcode.clone()),
        article: Set(aggregate.article.clone()),
        brand: Set(aggregate.brand.clone()),
        category_id: Set(aggregate.category_id.clone()),
        category_name: Set(aggregate.category_name.clone()),
        last_update: Set(aggregate.last_update),
        nomenclature_ref: Set(aggregate.nomenclature_ref.clone()),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        created_at: Set(Some(aggregate.base.metadata.created_at)),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
    };
    active.insert(conn()).await?;
    Ok(uuid)
}

pub async fn update(aggregate: &MarketplaceProduct) -> anyhow::Result<()> {
    let id = aggregate.base.id.value().to_string();
    let active = ActiveModel {
        id: Set(id),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        marketplace_ref: Set(aggregate.marketplace_ref.clone()),
        connection_mp_ref: Set(aggregate.connection_mp_ref.clone()),
        marketplace_sku: Set(aggregate.marketplace_sku.clone()),
        barcode: Set(aggregate.barcode.clone()),
        article: Set(aggregate.article.clone()),
        brand: Set(aggregate.brand.clone()),
        category_id: Set(aggregate.category_id.clone()),
        category_name: Set(aggregate.category_name.clone()),
        last_update: Set(aggregate.last_update),
        nomenclature_ref: Set(aggregate.nomenclature_ref.clone()),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
        created_at: sea_orm::ActiveValue::NotSet,
    };
    active.update(conn()).await?;
    Ok(())
}

pub async fn soft_delete(id: Uuid) -> anyhow::Result<bool> {
    use sea_orm::sea_query::Expr;
    let result = Entity::update_many()
        .col_expr(Column::IsDeleted, Expr::value(true))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(Column::Id.eq(id.to_string()))
        .exec(conn())
        .await?;
    Ok(result.rows_affected > 0)
}

pub async fn get_by_connection_and_sku(
    connection_mp_ref: &str,
    sku: &str,
) -> anyhow::Result<Option<MarketplaceProduct>> {
    let result = Entity::find()
        .filter(Column::ConnectionMpRef.eq(connection_mp_ref))
        .filter(Column::MarketplaceSku.eq(sku))
        .filter(Column::IsDeleted.eq(false))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}

pub async fn list_by_connection_and_article(
    connection_mp_ref: &str,
    article: &str,
) -> anyhow::Result<Vec<MarketplaceProduct>> {
    let article = article.trim();
    if article.is_empty() {
        return Ok(Vec::new());
    }

    let items: Vec<MarketplaceProduct> = Entity::find()
        .filter(Column::ConnectionMpRef.eq(connection_mp_ref))
        .filter(Column::Article.eq(article))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(items)
}

pub async fn get_unique_by_connection_and_article(
    connection_mp_ref: &str,
    article: &str,
) -> anyhow::Result<Option<MarketplaceProduct>> {
    let items = list_by_connection_and_article(connection_mp_ref, article).await?;
    if items.len() == 1 {
        Ok(items.into_iter().next())
    } else {
        Ok(None)
    }
}

pub async fn get_by_barcode(barcode: &str) -> anyhow::Result<Vec<MarketplaceProduct>> {
    let items: Vec<MarketplaceProduct> = Entity::find()
        .filter(Column::Barcode.eq(barcode))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

pub async fn list_by_marketplace_ref(
    marketplace_ref: &str,
) -> anyhow::Result<Vec<MarketplaceProduct>> {
    let items: Vec<MarketplaceProduct> = Entity::find()
        .filter(Column::MarketplaceRef.eq(marketplace_ref))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

pub async fn get_by_nomenclature_ref(
    nomenclature_ref: &str,
) -> anyhow::Result<Vec<MarketplaceProduct>> {
    let items: Vec<MarketplaceProduct> = Entity::find()
        .filter(Column::NomenclatureRef.eq(nomenclature_ref))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

#[derive(Debug, Clone)]
pub struct MarketplaceProductListQuery {
    pub marketplace_ref: Option<String>,
    pub connection_mp_ref: Option<String>,
    pub problems_only: bool,
    pub search: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

pub struct MarketplaceProductListResult {
    pub items: Vec<MarketplaceProduct>,
    pub total: usize,
}

#[derive(Debug, Clone)]
pub struct WbMappingProblemsQuery {
    pub date_from: String,
    pub date_to: String,
    pub connection_mp_ref: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct WbMappingProblemRow {
    pub problem_kind: String,
    pub connection_mp_ref: String,
    pub connection_name: Option<String>,
    pub nm_id: Option<i64>,
    pub supplier_article: Option<String>,
    pub marketplace_product_id: Option<String>,
    pub marketplace_sku: Option<String>,
    pub marketplace_article: Option<String>,
    pub marketplace_nomenclature_ref: Option<String>,
    pub nomenclature_name: Option<String>,
    pub nomenclature_article: Option<String>,
    pub p903_rows: i64,
    pub order_rows: i64,
    pub sale_rows: i64,
    pub missing_document_links: i64,
    pub mismatched_document_links: i64,
    pub article_match_count: i64,
}

pub struct WbMappingProblemsResult {
    pub items: Vec<WbMappingProblemRow>,
    pub total: usize,
}

fn sv(value: impl Into<String>) -> Value {
    Value::String(Some(Box::new(value.into())))
}

pub async fn list_for_matching(
    marketplace_ref: Option<&str>,
    unresolved_only: bool,
) -> anyhow::Result<Vec<MarketplaceProduct>> {
    use sea_orm::Condition;

    let mut select = Entity::find().filter(Column::IsDeleted.eq(false));

    if let Some(mp_ref) = marketplace_ref.filter(|value| !value.is_empty()) {
        select = select.filter(Column::MarketplaceRef.eq(mp_ref));
    }

    if unresolved_only {
        select = select.filter(
            Condition::any()
                .add(Column::NomenclatureRef.is_null())
                .add(Column::NomenclatureRef.eq("")),
        );
    }

    let items = select
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(items)
}

pub async fn list_paginated(
    query: MarketplaceProductListQuery,
) -> anyhow::Result<MarketplaceProductListResult> {
    use sea_orm::{Condition, QueryOrder, QuerySelect};

    let mut select = Entity::find().filter(Column::IsDeleted.eq(false));

    // Filtering
    if let Some(mp_ref) = query.marketplace_ref {
        if !mp_ref.is_empty() {
            select = select.filter(Column::MarketplaceRef.eq(mp_ref));
        }
    }

    if let Some(connection_mp_ref) = query.connection_mp_ref {
        if !connection_mp_ref.is_empty() {
            select = select.filter(Column::ConnectionMpRef.eq(connection_mp_ref));
        }
    }

    if query.problems_only {
        use sea_orm::sea_query::Expr;
        select = select.filter(
            Condition::any()
                .add(Column::NomenclatureRef.is_null())
                .add(Column::NomenclatureRef.eq(""))
                .add(Expr::cust(
                    "EXISTS (
                        SELECT 1
                        FROM a007_marketplace_product dup
                        WHERE dup.is_deleted = 0
                          AND dup.connection_mp_ref = a007_marketplace_product.connection_mp_ref
                          AND dup.article = a007_marketplace_product.article
                        GROUP BY dup.connection_mp_ref, dup.article
                        HAVING COUNT(*) > 1
                    )",
                ))
                .add(Expr::cust(
                    "EXISTS (
                        SELECT 1
                        FROM p903_wb_finance_report p903
                        WHERE p903.connection_mp_ref = a007_marketplace_product.connection_mp_ref
                          AND p903.sa_name = a007_marketplace_product.article
                          AND p903.nm_id IS NOT NULL
                          AND p903.nm_id > 0
                          AND CAST(p903.nm_id AS TEXT) <> a007_marketplace_product.marketplace_sku
                    )",
                )),
        );
    }

    if let Some(search) = query.search {
        if !search.is_empty() {
            let s = format!("%{}%", search);
            select = select.filter(
                Condition::any()
                    .add(Column::Code.like(s.clone()))
                    .add(Column::Description.like(s.clone()))
                    .add(Column::MarketplaceSku.like(s.clone()))
                    .add(Column::Article.like(s.clone()))
                    .add(Column::Barcode.like(s.clone())),
            );
        }
    }

    // Count total
    let total = select.clone().count(conn()).await? as usize;

    // Sorting
    let sort_col = match query.sort_by.as_str() {
        "code" => Column::Code,
        "description" => Column::Description,
        "marketplace_sku" => Column::MarketplaceSku,
        "article" => Column::Article,
        "barcode" => Column::Barcode,
        "created_at" => Column::CreatedAt,
        _ => Column::Code,
    };

    select = if query.sort_desc {
        select.order_by_desc(sort_col)
    } else {
        select.order_by_asc(sort_col)
    };

    // Pagination
    let items = select
        .limit(query.limit as u64)
        .offset(query.offset as u64)
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(MarketplaceProductListResult { items, total })
}

pub async fn list_wb_mapping_problems(
    query: WbMappingProblemsQuery,
) -> anyhow::Result<WbMappingProblemsResult> {
    let safe_limit = query.limit.clamp(1, 1000);
    let safe_offset = query.offset;
    let date_to_end = format!("{}T23:59:59", query.date_to);

    let mut p903_connection_filter = String::new();
    let mut a015_connection_filter = String::new();
    let mut a012_connection_filter = String::new();
    let connection_filter_value = query
        .connection_mp_ref
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let mut params = Vec::new();

    if connection_filter_value.is_some() {
        p903_connection_filter = " AND connection_mp_ref = ?".to_string();
        a015_connection_filter =
            " AND json_extract(header_json, '$.connection_id') = ?".to_string();
        a012_connection_filter = " AND connection_id = ?".to_string();
    }

    params.push(sv(query.date_from.clone()));
    params.push(sv(query.date_to.clone()));
    if let Some(connection_mp_ref) = &connection_filter_value {
        params.push(sv(connection_mp_ref.clone()));
    }
    params.push(sv(query.date_from.clone()));
    params.push(sv(query.date_to.clone()));
    if let Some(connection_mp_ref) = &connection_filter_value {
        params.push(sv(connection_mp_ref.clone()));
    }
    params.push(sv(query.date_from.clone()));
    params.push(sv(date_to_end));
    if let Some(connection_mp_ref) = &connection_filter_value {
        params.push(sv(connection_mp_ref.clone()));
    }

    let cte_sql = format!(
        r#"
        WITH source_rows AS (
            SELECT
                connection_mp_ref,
                nm_id,
                NULLIF(TRIM(COALESCE(sa_name, '')), '') AS supplier_article,
                NULLIF(TRIM(COALESCE(a004_nomenclature_ref, '')), '') AS document_nomenclature_ref,
                1 AS p903_rows,
                0 AS order_rows,
                0 AS sale_rows
            FROM p903_wb_finance_report
            WHERE rr_dt >= ? AND rr_dt <= ?
              {p903_connection_filter}

            UNION ALL

            SELECT
                json_extract(header_json, '$.connection_id') AS connection_mp_ref,
                CAST(json_extract(line_json, '$.nm_id') AS INTEGER) AS nm_id,
                NULLIF(TRIM(COALESCE(json_extract(line_json, '$.supplier_article'), '')), '') AS supplier_article,
                NULLIF(TRIM(COALESCE(nomenclature_ref, '')), '') AS document_nomenclature_ref,
                0 AS p903_rows,
                1 AS order_rows,
                0 AS sale_rows
            FROM a015_wb_orders
            WHERE is_deleted = 0
              AND document_date >= ? AND document_date <= ?
              {a015_connection_filter}

            UNION ALL

            SELECT
                connection_id AS connection_mp_ref,
                nm_id,
                NULLIF(TRIM(COALESCE(supplier_article, '')), '') AS supplier_article,
                NULLIF(TRIM(COALESCE(nomenclature_ref, '')), '') AS document_nomenclature_ref,
                0 AS p903_rows,
                0 AS order_rows,
                1 AS sale_rows
            FROM a012_wb_sales
            WHERE is_deleted = 0
              AND sale_date >= ? AND sale_date <= ?
              {a012_connection_filter}
        ),
        aggregated AS (
            SELECT
                connection_mp_ref,
                nm_id,
                supplier_article,
                SUM(p903_rows) AS p903_rows,
                SUM(order_rows) AS order_rows,
                SUM(sale_rows) AS sale_rows,
                SUM(CASE WHEN document_nomenclature_ref IS NULL THEN 1 ELSE 0 END) AS missing_document_links,
                GROUP_CONCAT(DISTINCT document_nomenclature_ref) AS document_refs
            FROM source_rows
            WHERE connection_mp_ref IS NOT NULL AND TRIM(connection_mp_ref) <> ''
            GROUP BY connection_mp_ref, nm_id, supplier_article
        ),
        article_counts AS (
            SELECT connection_mp_ref, article, COUNT(*) AS article_match_count
            FROM a007_marketplace_product
            WHERE is_deleted = 0
              AND NULLIF(TRIM(article), '') IS NOT NULL
            GROUP BY connection_mp_ref, article
        ),
        article_unique AS (
            SELECT connection_mp_ref, article, MIN(id) AS marketplace_product_id
            FROM a007_marketplace_product
            WHERE is_deleted = 0
              AND NULLIF(TRIM(article), '') IS NOT NULL
            GROUP BY connection_mp_ref, article
            HAVING COUNT(*) = 1
        ),
        joined AS (
            SELECT
                a.connection_mp_ref,
                c.description AS connection_name,
                a.nm_id,
                a.supplier_article,
                COALESCE(mp_sku.id, mp_article.id) AS marketplace_product_id,
                COALESCE(mp_sku.marketplace_sku, mp_article.marketplace_sku) AS marketplace_sku,
                COALESCE(mp_sku.article, mp_article.article) AS marketplace_article,
                NULLIF(TRIM(COALESCE(mp_sku.nomenclature_ref, mp_article.nomenclature_ref, '')), '') AS marketplace_nomenclature_ref,
                n.description AS nomenclature_name,
                n.article AS nomenclature_article,
                a.p903_rows,
                a.order_rows,
                a.sale_rows,
                a.missing_document_links,
                CASE
                    WHEN NULLIF(TRIM(COALESCE(mp_sku.nomenclature_ref, mp_article.nomenclature_ref, '')), '') IS NULL THEN 0
                    WHEN a.document_refs IS NULL THEN 0
                    WHEN instr(',' || a.document_refs || ',', ',' || NULLIF(TRIM(COALESCE(mp_sku.nomenclature_ref, mp_article.nomenclature_ref, '')), '') || ',') > 0 THEN 0
                    ELSE 1
                END AS mismatched_document_links,
                COALESCE(ac.article_match_count, 0) AS article_match_count
            FROM aggregated a
            LEFT JOIN a007_marketplace_product mp_sku
                   ON a.nm_id IS NOT NULL
                  AND a.nm_id > 0
                  AND mp_sku.connection_mp_ref = a.connection_mp_ref
                  AND mp_sku.marketplace_sku = CAST(a.nm_id AS TEXT)
                  AND mp_sku.is_deleted = 0
            LEFT JOIN article_unique au
                   ON mp_sku.id IS NULL
                  AND a.supplier_article IS NOT NULL
                  AND au.connection_mp_ref = a.connection_mp_ref
                  AND au.article = a.supplier_article
            LEFT JOIN a007_marketplace_product mp_article
                   ON mp_article.id = au.marketplace_product_id
                  AND mp_article.is_deleted = 0
            LEFT JOIN article_counts ac
                   ON ac.connection_mp_ref = a.connection_mp_ref
                  AND ac.article = a.supplier_article
            LEFT JOIN a004_nomenclature n
                   ON n.id = NULLIF(TRIM(COALESCE(mp_sku.nomenclature_ref, mp_article.nomenclature_ref, '')), '')
            LEFT JOIN a006_connection_mp c ON c.id = a.connection_mp_ref
        ),
        classified AS (
            SELECT
                CASE
                    -- Строки без валидного nm_id в принципе невозможно сопоставить с a007 —
                    -- это шум, не имеющий пути решения. В отчёт проблем такие не попадают.
                    WHEN nm_id IS NULL OR nm_id <= 0 THEN ''
                    -- article_ambiguous имеет смысл только когда a007 не нашёлся точно по
                    -- SKU и article — единственный способ сопоставления, но он не уникален.
                    WHEN marketplace_product_id IS NULL AND article_match_count > 1 THEN 'article_ambiguous'
                    WHEN marketplace_product_id IS NULL THEN 'missing_a007'
                    WHEN marketplace_nomenclature_ref IS NULL THEN 'missing_nomenclature'
                    WHEN missing_document_links > 0 OR mismatched_document_links > 0 THEN 'stale_document_link'
                    ELSE ''
                END AS problem_kind,
                *
            FROM joined
        )
        "#
    );

    let count_sql = format!("{cte_sql} SELECT COUNT(*) AS cnt FROM classified WHERE problem_kind <> ''");
    let count_row = conn()
        .query_one(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            &count_sql,
            params.clone(),
        ))
        .await?;
    let total = count_row
        .and_then(|row| row.try_get::<i64>("", "cnt").ok())
        .unwrap_or(0)
        .max(0) as usize;

    let data_sql = format!(
        r#"
        {cte_sql}
        SELECT
            problem_kind,
            connection_mp_ref,
            connection_name,
            nm_id,
            supplier_article,
            marketplace_product_id,
            marketplace_sku,
            marketplace_article,
            marketplace_nomenclature_ref,
            nomenclature_name,
            nomenclature_article,
            p903_rows,
            order_rows,
            sale_rows,
            missing_document_links,
            mismatched_document_links,
            article_match_count
        FROM classified
        WHERE problem_kind <> ''
        ORDER BY
            CASE problem_kind
                WHEN 'missing_a007' THEN 1
                WHEN 'missing_nomenclature' THEN 2
                WHEN 'stale_document_link' THEN 3
                WHEN 'article_ambiguous' THEN 4
                WHEN 'missing_nm_id' THEN 5
                ELSE 9
            END,
            connection_name,
            nm_id,
            supplier_article
        LIMIT {safe_limit} OFFSET {safe_offset}
        "#
    );

    let rows = conn()
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            &data_sql,
            params,
        ))
        .await?;

    let items = rows
        .into_iter()
        .map(|row| WbMappingProblemRow {
            problem_kind: row.try_get("", "problem_kind").unwrap_or_default(),
            connection_mp_ref: row.try_get("", "connection_mp_ref").unwrap_or_default(),
            connection_name: row.try_get("", "connection_name").ok(),
            nm_id: row.try_get("", "nm_id").ok(),
            supplier_article: row.try_get("", "supplier_article").ok(),
            marketplace_product_id: row.try_get("", "marketplace_product_id").ok(),
            marketplace_sku: row.try_get("", "marketplace_sku").ok(),
            marketplace_article: row.try_get("", "marketplace_article").ok(),
            marketplace_nomenclature_ref: row.try_get("", "marketplace_nomenclature_ref").ok(),
            nomenclature_name: row.try_get("", "nomenclature_name").ok(),
            nomenclature_article: row.try_get("", "nomenclature_article").ok(),
            p903_rows: row.try_get("", "p903_rows").unwrap_or(0),
            order_rows: row.try_get("", "order_rows").unwrap_or(0),
            sale_rows: row.try_get("", "sale_rows").unwrap_or(0),
            missing_document_links: row.try_get("", "missing_document_links").unwrap_or(0),
            mismatched_document_links: row.try_get("", "mismatched_document_links").unwrap_or(0),
            article_match_count: row.try_get("", "article_match_count").unwrap_or(0),
        })
        .collect();

    Ok(WbMappingProblemsResult { items, total })
}

#[derive(Debug, Clone)]
pub struct StalePostingRef {
    pub registrator_type: String,
    pub registrator_id: String,
    /// YYYY-MM-DD (день документа: sale_date / document_date / rr_dt).
    pub doc_date: String,
}

/// Список «устаревших» проводок для одной строки WB-проблем (`stale_document_link`).
///
/// Возвращает уникальные регистраторы (a012/a015 по UUID; p903 — по
/// `(connection_mp_ref, rr_dt)`, согласно его API rebuild_day_from_existing),
/// у которых `nomenclature_ref` пуст или не совпадает с `expected_ref` —
/// актуальным значением a007.nomenclature_ref, который должен резолвить
/// caller через `a007::service::resolve_wb_nomenclature_ref`.
/// Если `expected_ref` пуст — любая запись считается stale.
/// Список отсортирован по `doc_date ASC`.
pub async fn list_stale_postings_for_problem(
    connection_mp_ref: &str,
    nm_id: i64,
    date_from: &str,
    date_to: &str,
    expected_ref: Option<&str>,
) -> anyhow::Result<Vec<StalePostingRef>> {
    let expected_ref = expected_ref.map(str::trim).unwrap_or("").to_string();
    let date_to_end = format!("{} 23:59:59", date_to);

    let sql = r#"
        WITH stale_a012 AS (
            SELECT 'a012_wb_sales' AS registrator_type,
                   id AS registrator_id,
                   substr(sale_date, 1, 10) AS doc_date
            FROM a012_wb_sales
            WHERE is_deleted = 0
              AND connection_id = ?
              AND nm_id = ?
              AND sale_date >= ? AND sale_date <= ?
              AND (
                ? = ''
                OR nomenclature_ref IS NULL
                OR TRIM(nomenclature_ref) = ''
                OR nomenclature_ref <> ?
              )
        ),
        stale_a015 AS (
            SELECT 'a015_wb_orders' AS registrator_type,
                   id AS registrator_id,
                   substr(document_date, 1, 10) AS doc_date
            FROM a015_wb_orders
            WHERE is_deleted = 0
              AND json_extract(header_json, '$.connection_id') = ?
              AND CAST(json_extract(line_json, '$.nm_id') AS INTEGER) = ?
              AND document_date >= ? AND document_date <= ?
              AND (
                ? = ''
                OR nomenclature_ref IS NULL
                OR TRIM(nomenclature_ref) = ''
                OR nomenclature_ref <> ?
              )
        ),
        stale_p903 AS (
            SELECT DISTINCT 'p903_wb_finance_report' AS registrator_type,
                   connection_mp_ref || '|' || substr(rr_dt, 1, 10) AS registrator_id,
                   substr(rr_dt, 1, 10) AS doc_date
            FROM p903_wb_finance_report
            WHERE connection_mp_ref = ?
              AND nm_id = ?
              AND rr_dt >= ? AND rr_dt <= ?
              AND (
                ? = ''
                OR a004_nomenclature_ref IS NULL
                OR TRIM(a004_nomenclature_ref) = ''
                OR a004_nomenclature_ref <> ?
              )
        )
        SELECT registrator_type, registrator_id, doc_date FROM stale_a012
        UNION ALL
        SELECT registrator_type, registrator_id, doc_date FROM stale_a015
        UNION ALL
        SELECT registrator_type, registrator_id, doc_date FROM stale_p903
        ORDER BY doc_date ASC, registrator_type ASC, registrator_id ASC
    "#;

    let conn_v = sv(connection_mp_ref);
    let nm_id_v = Value::BigInt(Some(nm_id));
    let date_from_v = sv(date_from);
    let date_to_v = sv(date_to_end);
    let mp_nom_v = sv(expected_ref);

    let mut params: Vec<Value> = Vec::with_capacity(18);
    for _ in 0..3 {
        params.push(conn_v.clone());
        params.push(nm_id_v.clone());
        params.push(date_from_v.clone());
        params.push(date_to_v.clone());
        params.push(mp_nom_v.clone());
        params.push(mp_nom_v.clone());
    }

    let rows = conn()
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
            params,
        ))
        .await?;

    let items = rows
        .into_iter()
        .map(|row| StalePostingRef {
            registrator_type: row.try_get("", "registrator_type").unwrap_or_default(),
            registrator_id: row.try_get("", "registrator_id").unwrap_or_default(),
            doc_date: row.try_get("", "doc_date").unwrap_or_default(),
        })
        .collect();

    Ok(items)
}

#[cfg(test)]
fn classify_wb_mapping_problem(
    nm_id: Option<i64>,
    has_marketplace_product: bool,
    has_marketplace_nomenclature: bool,
    missing_document_links: i64,
    mismatched_document_links: i64,
    article_match_count: i64,
) -> Option<&'static str> {
    if nm_id.map_or(true, |value| value <= 0) {
        // Строки без валидного nm_id невозможно сопоставить — в отчёт не попадают.
        return None;
    }
    if !has_marketplace_product {
        if article_match_count > 1 {
            return Some("article_ambiguous");
        }
        return Some("missing_a007");
    }
    if !has_marketplace_nomenclature {
        return Some("missing_nomenclature");
    }
    if missing_document_links > 0 || mismatched_document_links > 0 {
        return Some("stale_document_link");
    }
    None
}

#[cfg(test)]
mod wb_mapping_problem_tests {
    use super::classify_wb_mapping_problem;

    #[test]
    fn nm_id_without_a007_is_missing_a007() {
        assert_eq!(
            classify_wb_mapping_problem(Some(123), false, false, 0, 0, 0),
            Some("missing_a007")
        );
    }

    #[test]
    fn a007_without_nomenclature_is_missing_nomenclature() {
        assert_eq!(
            classify_wb_mapping_problem(Some(123), true, false, 0, 0, 1),
            Some("missing_nomenclature")
        );
    }

    #[test]
    fn fixed_a007_with_old_empty_documents_is_stale_document_link() {
        assert_eq!(
            classify_wb_mapping_problem(Some(123), true, true, 2, 0, 1),
            Some("stale_document_link")
        );
    }

    /// Когда a007 не нашёлся точно по SKU и article в кабинете не уникален —
    /// документ не может однозначно сопоставиться, это и есть article_ambiguous.
    #[test]
    fn no_sku_match_with_non_unique_article_is_ambiguous() {
        assert_eq!(
            classify_wb_mapping_problem(Some(123), false, false, 0, 0, 2),
            Some("article_ambiguous")
        );
    }

    /// Когда a007 найден точно по SKU, повторяющийся article в кабинете —
    /// data hygiene issue для списка товаров, но не ambiguous для документа.
    #[test]
    fn sku_match_with_non_unique_article_is_not_ambiguous() {
        assert_eq!(
            classify_wb_mapping_problem(Some(123), true, true, 0, 0, 2),
            None
        );
    }

    #[test]
    fn correct_mapping_is_not_a_problem() {
        assert_eq!(
            classify_wb_mapping_problem(Some(123), true, true, 0, 0, 1),
            None
        );
    }
}
