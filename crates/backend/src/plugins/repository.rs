//! SeaORM-доступ к таблице `plugin`. Бандл хранится JSON-колонками (как у a024).

use chrono::Utc;
use contracts::plugins::{
    DataBinding, ParamSpec, PluginBundle, PluginDefinition, PluginManifest, PluginRuntime,
    PluginStatus, ViewSpec,
};
use sea_orm::entity::prelude::*;
use sea_orm::prelude::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::collections::HashMap;

mod plugin {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "plugin")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub code: String,
        pub title: String,
        pub runtime: String,
        pub status: String,
        pub manifest_json: String,
        pub params_json: String,
        pub data_json: String,
        pub client_script: Option<String>,
        pub server_script: Option<String>,
        pub view_spec_json: String,
        pub styles: Option<String>,
        pub assets_json: String,
        pub owner_user_id: Option<String>,
        pub created_by_agent_id: Option<String>,
        pub is_enabled: bool,
        pub is_deleted: bool,
        pub created_at: Option<chrono::DateTime<chrono::Utc>>,
        pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
        pub version: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

impl From<plugin::Model> for PluginDefinition {
    fn from(m: plugin::Model) -> Self {
        let manifest: PluginManifest =
            serde_json::from_str(&m.manifest_json).unwrap_or_else(|_| PluginManifest {
                code: m.code.clone(),
                title: m.title.clone(),
                runtime: PluginRuntime::from_str(&m.runtime),
                api_version: "1".to_string(),
                description: None,
                capabilities: vec![],
            });
        let params: Vec<ParamSpec> = serde_json::from_str(&m.params_json).unwrap_or_default();
        let data: DataBinding = serde_json::from_str(&m.data_json).unwrap_or_default();
        let view_spec: ViewSpec = serde_json::from_str(&m.view_spec_json).unwrap_or_default();
        let assets: HashMap<String, String> =
            serde_json::from_str(&m.assets_json).unwrap_or_default();

        let bundle = PluginBundle {
            manifest,
            params,
            data,
            client_script: m.client_script,
            server_script: m.server_script,
            view_spec,
            styles: m.styles,
            assets,
        };

        PluginDefinition {
            id: m.id,
            bundle,
            status: PluginStatus::from_str(&m.status),
            is_enabled: m.is_enabled,
            owner_user_id: m.owner_user_id,
            created_by_agent_id: m.created_by_agent_id,
            version: m.version,
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
        }
    }
}

fn to_active(def: &PluginDefinition, is_insert: bool) -> plugin::ActiveModel {
    let now = Utc::now();
    let b = &def.bundle;
    plugin::ActiveModel {
        id: Set(def.id.clone()),
        code: Set(b.manifest.code.clone()),
        title: Set(b.manifest.title.clone()),
        runtime: Set(b.manifest.runtime.as_str().to_string()),
        status: Set(def.status.as_str().to_string()),
        manifest_json: Set(serde_json::to_string(&b.manifest).unwrap_or_else(|_| "{}".into())),
        params_json: Set(serde_json::to_string(&b.params).unwrap_or_else(|_| "[]".into())),
        data_json: Set(serde_json::to_string(&b.data).unwrap_or_else(|_| "{}".into())),
        client_script: Set(b.client_script.clone()),
        server_script: Set(b.server_script.clone()),
        view_spec_json: Set(serde_json::to_string(&b.view_spec).unwrap_or_else(|_| "{}".into())),
        styles: Set(b.styles.clone()),
        assets_json: Set(serde_json::to_string(&b.assets).unwrap_or_else(|_| "{}".into())),
        owner_user_id: Set(def.owner_user_id.clone()),
        created_by_agent_id: Set(def.created_by_agent_id.clone()),
        is_enabled: Set(def.is_enabled),
        is_deleted: Set(false),
        created_at: Set(Some(if is_insert { now } else { def.created_at })),
        updated_at: Set(Some(now)),
        version: Set(def.version),
    }
}

// ============================================================================
// Repository functions
// ============================================================================

pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<PluginDefinition>, DbErr> {
    let models = plugin::Entity::find()
        .filter(plugin::Column::IsDeleted.eq(false))
        .order_by_desc(plugin::Column::UpdatedAt)
        .all(db)
        .await?;
    Ok(models.into_iter().map(Into::into).collect())
}

pub async fn list_enabled(db: &DatabaseConnection) -> Result<Vec<PluginDefinition>, DbErr> {
    let models = plugin::Entity::find()
        .filter(plugin::Column::IsDeleted.eq(false))
        .filter(plugin::Column::IsEnabled.eq(true))
        .order_by_asc(plugin::Column::Title)
        .all(db)
        .await?;
    Ok(models.into_iter().map(Into::into).collect())
}

pub async fn find_by_id(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<PluginDefinition>, DbErr> {
    let model = plugin::Entity::find_by_id(id.to_string()).one(db).await?;
    Ok(model.filter(|m| !m.is_deleted).map(Into::into))
}

pub async fn insert(db: &DatabaseConnection, def: &PluginDefinition) -> Result<(), DbErr> {
    to_active(def, true).insert(db).await?;
    Ok(())
}

pub async fn update(db: &DatabaseConnection, def: &PluginDefinition) -> Result<(), DbErr> {
    plugin::Entity::update(to_active(def, false))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn soft_delete(db: &DatabaseConnection, id: &str) -> Result<(), DbErr> {
    plugin::Entity::update_many()
        .col_expr(plugin::Column::IsDeleted, Expr::value(true))
        .col_expr(plugin::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(plugin::Column::Id.eq(id.to_string()))
        .exec(db)
        .await?;
    Ok(())
}
