use super::repository;
use contracts::domain::a025_bi_dashboard::aggregate::{
    BiDashboard, BiDashboardId, BiDashboardStatus, DashboardGroup, DashboardItem, DashboardLayout,
    GlobalFilter,
};
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// DTO для создания/обновления BI дашборда через API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiDashboardDto {
    pub id: Option<String>,
    pub code: Option<String>,
    pub description: String,
    pub comment: Option<String>,

    pub layout: Option<DashboardLayout>,
    pub global_filters: Option<Vec<GlobalFilter>>,

    pub status: Option<String>,
    pub owner_user_id: String,
    pub is_public: Option<bool>,
    pub rating: Option<u8>,
    pub updated_by: Option<String>,
}

// ============================================================================
// Service functions
// ============================================================================

/// Создание нового BI дашборда
pub async fn create(dto: BiDashboardDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("DASH-{}", &Uuid::new_v4().to_string()[..8].to_uppercase()));

    let mut dashboard = BiDashboard::new_for_insert(code, dto.description, dto.owner_user_id);

    dashboard.base.comment = dto.comment;

    if let Some(layout) = dto.layout {
        dashboard.layout = layout;
    }
    if let Some(filters) = dto.global_filters {
        dashboard.global_filters = filters;
    }
    if let Some(status_str) = &dto.status {
        dashboard.status =
            BiDashboardStatus::from_str(status_str).unwrap_or(BiDashboardStatus::Draft);
    }
    if let Some(is_public) = dto.is_public {
        dashboard.is_public = is_public;
    }
    if let Some(rating) = dto.rating {
        if rating >= 1 && rating <= 5 {
            dashboard.rating = Some(rating);
        }
    }
    if let Some(created_by) = dto.updated_by {
        dashboard.created_by = Some(created_by);
    }

    dashboard
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    dashboard.before_write();

    let id = dashboard.base.id.0;

    let db = crate::shared::data::db::get_connection();
    repository::insert(&db, &dashboard).await?;

    Ok(id)
}

/// Обновление BI дашборда
pub async fn update(dto: BiDashboardDto) -> anyhow::Result<()> {
    let id_str = dto
        .id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ID is required for update"))?;

    let dashboard_uuid = Uuid::parse_str(id_str)
        .map_err(|e| anyhow::anyhow!("Invalid dashboard ID: {}", e))?;
    let dashboard_id = BiDashboardId::new(dashboard_uuid);

    let db = crate::shared::data::db::get_connection();
    let mut dashboard = repository::find_by_id(&db, &dashboard_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("BI Dashboard not found: {}", id_str))?;

    if let Some(code) = dto.code {
        dashboard.base.code = code;
    }
    dashboard.base.description = dto.description;
    dashboard.base.comment = dto.comment;

    if let Some(layout) = dto.layout {
        dashboard.layout = layout;
    }
    if let Some(filters) = dto.global_filters {
        dashboard.global_filters = filters;
    }
    if let Some(status_str) = &dto.status {
        dashboard.status =
            BiDashboardStatus::from_str(status_str).unwrap_or(BiDashboardStatus::Draft);
    }
    if let Some(is_public) = dto.is_public {
        dashboard.is_public = is_public;
    }
    // rating: None в DTO означает "сбросить оценку", Some(x) — установить
    dashboard.rating = dto.rating.and_then(|r| if r >= 1 && r <= 5 { Some(r) } else { None });

    if let Some(updated_by) = dto.updated_by {
        dashboard.updated_by = Some(updated_by);
    }
    dashboard.owner_user_id = dto.owner_user_id;

    dashboard
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    dashboard.before_write();

    repository::update(&db, &dashboard).await?;

    Ok(())
}

/// Удаление дашборда (soft delete)
pub async fn delete(id: &str) -> anyhow::Result<()> {
    let dashboard_uuid = Uuid::parse_str(id)
        .map_err(|e| anyhow::anyhow!("Invalid dashboard ID: {}", e))?;
    let dashboard_id = BiDashboardId::new(dashboard_uuid);

    let db = crate::shared::data::db::get_connection();
    repository::soft_delete(&db, &dashboard_id).await?;

    Ok(())
}

/// Получить дашборд по ID
pub async fn get_by_id(id: &str) -> anyhow::Result<Option<BiDashboard>> {
    let dashboard_uuid = Uuid::parse_str(id)
        .map_err(|e| anyhow::anyhow!("Invalid dashboard ID: {}", e))?;
    let dashboard_id = BiDashboardId::new(dashboard_uuid);

    let db = crate::shared::data::db::get_connection();
    let dashboard = repository::find_by_id(&db, &dashboard_id).await?;

    Ok(dashboard)
}

/// Получить все дашборды (без пагинации)
pub async fn list_all() -> anyhow::Result<Vec<BiDashboard>> {
    let db = crate::shared::data::db::get_connection();
    let dashboards = repository::list_all(&db).await?;
    Ok(dashboards)
}

/// Получить список дашбордов с пагинацией
pub async fn list_paginated(
    page: u64,
    page_size: u64,
) -> anyhow::Result<(Vec<BiDashboard>, u64)> {
    let db = crate::shared::data::db::get_connection();
    let (dashboards, total) = repository::list_paginated(&db, page, page_size).await?;
    Ok((dashboards, total))
}

/// Получить дашборды конкретного владельца
pub async fn list_by_owner(owner_user_id: &str) -> anyhow::Result<Vec<BiDashboard>> {
    let db = crate::shared::data::db::get_connection();
    let dashboards = repository::list_by_owner(&db, owner_user_id).await?;
    Ok(dashboards)
}

/// Получить публичные дашборды
pub async fn list_public() -> anyhow::Result<Vec<BiDashboard>> {
    let db = crate::shared::data::db::get_connection();
    let dashboards = repository::list_public(&db).await?;
    Ok(dashboards)
}

/// Вставить тестовые данные (3 примера)
pub async fn insert_test_data() -> anyhow::Result<()> {
    use sea_orm::{ConnectionTrait, DbBackend, Statement};
    let db = crate::shared::data::db::get_connection();

    const TEST_OWNER: &str = "f2fc6986-855d-492b-acff-70c7cd8cdd34";

    // Дашборд 1: Операционный — содержит Выручку WB и Количество заказов
    let layout_ops = serde_json::json!({
        "groups": [
            {
                "id": "g-001-ops-main",
                "title": "Основные показатели",
                "sort_order": 0,
                "items": [
                    {
                        "indicator_id": "a024a024-0001-4001-a001-000000000001",
                        "sort_order": 0,
                        "col_class": "2x1",
                        "param_overrides": {}
                    },
                    {
                        "indicator_id": "a024a024-0003-4001-a001-000000000003",
                        "sort_order": 1,
                        "col_class": "1x1",
                        "param_overrides": {}
                    }
                ],
                "subgroups": [
                    {
                        "id": "g-001-ops-margin",
                        "title": "Маржинальность",
                        "sort_order": 0,
                        "items": [
                            {
                                "indicator_id": "a024a024-0002-4001-a001-000000000002",
                                "sort_order": 0,
                                "col_class": "1x1",
                                "param_overrides": {}
                            }
                        ],
                        "subgroups": []
                    }
                ]
            }
        ]
    });

    let filters_ops = serde_json::json!([
        {
            "key": "date_range",
            "label": "Период",
            "value": "last_30_days"
        }
    ]);

    // Дашборд 2: Финансовый — все индикаторы выручки
    let layout_fin = serde_json::json!({
        "groups": [
            {
                "id": "g-002-fin",
                "title": "Выручка по площадкам",
                "sort_order": 0,
                "items": [
                    {
                        "indicator_id": "a024a024-0001-4001-a001-000000000001",
                        "sort_order": 0,
                        "col_class": "2x1",
                        "param_overrides": {}
                    },
                    {
                        "indicator_id": "a024a024-0004-4001-a001-000000000004",
                        "sort_order": 1,
                        "col_class": "2x1",
                        "param_overrides": {}
                    }
                ],
                "subgroups": []
            }
        ]
    });

    let filters_fin = serde_json::json!([
        {
            "key": "date_range",
            "label": "Период",
            "value": "this_month"
        }
    ]);

    // Дашборд 3: Пустой шаблон для проверки empty-state
    let layout_empty = serde_json::json!({ "groups": [] });
    let filters_empty = serde_json::json!([]);

    let records: &[(&str, &str, &str, &str, &str, &str, i32, &str)] = &[
        (
            "a025a025-0001-4001-a001-000000000001",
            "DASH-OPS",
            "Операционный дашборд",
            "Основные операционные показатели: выручка, заказы, маржа",
            &layout_ops.to_string(),
            &filters_ops.to_string(),
            4,
            "active",
        ),
        (
            "a025a025-0002-4001-a001-000000000002",
            "DASH-FIN",
            "Финансовый дашборд",
            "Финансовые показатели по всем маркетплейсам",
            &layout_fin.to_string(),
            &filters_fin.to_string(),
            5,
            "active",
        ),
        (
            "a025a025-0003-4001-a001-000000000003",
            "DASH-EMPTY",
            "Шаблон дашборда",
            "Пустой дашборд для настройки",
            &layout_empty.to_string(),
            &filters_empty.to_string(),
            0,
            "draft",
        ),
    ];

    for (id, code, description, comment, layout_json, filters_json, rating, status) in records {
        let rating_val = if *rating > 0 {
            format!("{}", rating)
        } else {
            "NULL".to_string()
        };

        let sql = format!(
            "INSERT OR IGNORE INTO a025_bi_dashboard \
            (id, code, description, comment, layout_json, global_filters_json, \
             status, owner_user_id, is_public, rating, created_at, updated_at, version) \
            VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', 1, {}, datetime('now'), datetime('now'), 1)",
            id,
            code,
            description,
            comment,
            layout_json.replace('\'', "''"),
            filters_json.replace('\'', "''"),
            status,
            TEST_OWNER,
            rating_val,
        );
        db.execute(Statement::from_string(DbBackend::Sqlite, sql))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to insert test record {}: {}", id, e))?;
    }

    Ok(())
}
