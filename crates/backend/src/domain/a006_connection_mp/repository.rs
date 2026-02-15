use chrono::Utc;
use contracts::domain::a006_connection_mp::aggregate::{
    ConnectionMP, ConnectionMPId, AuthorizationType,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sea_orm::entity::prelude::*;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a006_connection_mp")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub marketplace: String,
    // Deprecated legacy field, kept for backward DB compatibility.
    pub organization: String,
    pub organization_ref: String,
    pub api_key: String,
    pub supplier_id: Option<String>,
    pub application_id: Option<String>,
    pub is_used: bool,
    pub business_account_id: Option<String>,
    pub api_key_stats: Option<String>,
    pub test_mode: bool,
    pub planned_commission_percent: Option<f64>,
    pub authorization_type: String,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for ConnectionMP {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        let authorization_type = match m.authorization_type.as_str() {
            "API Key" => AuthorizationType::ApiKey,
            "OAuth 2.0" => AuthorizationType::OAuth2,
            "Basic Auth" => AuthorizationType::BasicAuth,
            _ => AuthorizationType::ApiKey,
        };

        ConnectionMP {
            base: BaseAggregate::with_metadata(
                ConnectionMPId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            marketplace_id: m.marketplace,
            organization_ref: m.organization_ref,
            api_key: m.api_key,
            supplier_id: m.supplier_id,
            application_id: m.application_id,
            is_used: m.is_used,
            business_account_id: m.business_account_id,
            api_key_stats: m.api_key_stats,
            test_mode: m.test_mode,
            planned_commission_percent: m.planned_commission_percent,
            authorization_type,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> anyhow::Result<Vec<ConnectionMP>> {
    let mut items: Vec<ConnectionMP> = Entity::find()
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

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<ConnectionMP>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn insert(aggregate: &ConnectionMP) -> anyhow::Result<Uuid> {
    let uuid = aggregate.base.id.value();
    let active = ActiveModel {
        id: Set(uuid.to_string()),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        marketplace: Set(aggregate.marketplace_id.clone()),
        organization: Set(String::new()),
        organization_ref: Set(aggregate.organization_ref.clone()),
        api_key: Set(aggregate.api_key.clone()),
        supplier_id: Set(aggregate.supplier_id.clone()),
        application_id: Set(aggregate.application_id.clone()),
        is_used: Set(aggregate.is_used),
        business_account_id: Set(aggregate.business_account_id.clone()),
        api_key_stats: Set(aggregate.api_key_stats.clone()),
        test_mode: Set(aggregate.test_mode),
        planned_commission_percent: Set(aggregate.planned_commission_percent),
        authorization_type: Set(aggregate.authorization_type.as_str().to_string()),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        created_at: Set(Some(aggregate.base.metadata.created_at)),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
    };
    active.insert(conn()).await?;
    Ok(uuid)
}

pub async fn update(aggregate: &ConnectionMP) -> anyhow::Result<()> {
    let id = aggregate.base.id.value().to_string();
    let active = ActiveModel {
        id: Set(id),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        marketplace: Set(aggregate.marketplace_id.clone()),
        organization: Set(String::new()),
        organization_ref: Set(aggregate.organization_ref.clone()),
        api_key: Set(aggregate.api_key.clone()),
        supplier_id: Set(aggregate.supplier_id.clone()),
        application_id: Set(aggregate.application_id.clone()),
        is_used: Set(aggregate.is_used),
        business_account_id: Set(aggregate.business_account_id.clone()),
        api_key_stats: Set(aggregate.api_key_stats.clone()),
        test_mode: Set(aggregate.test_mode),
        planned_commission_percent: Set(aggregate.planned_commission_percent),
        authorization_type: Set(aggregate.authorization_type.as_str().to_string()),
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
