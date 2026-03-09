use super::repository;
use crate::domain::a024_bi_indicator;
use contracts::shared::data_view::FilterRef;
use contracts::domain::a024_bi_indicator::aggregate::BiIndicatorId;
use contracts::domain::a025_bi_dashboard::aggregate::{
    BiDashboard, BiDashboardId, BiDashboardStatus, DashboardLayout,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// DTO для создания/обновления BI дашборда через API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiDashboardDto {
    pub id: Option<String>,
    pub code: Option<String>,
    pub description: String,
    pub comment: Option<String>,

    pub layout: Option<DashboardLayout>,
    pub filters: Option<Vec<FilterRef>>,

    pub status: Option<String>,
    pub owner_user_id: String,
    pub is_public: Option<bool>,
    pub rating: Option<u8>,
    pub updated_by: Option<String>,
}

fn collect_indicator_ids_from_layout(
    groups: &[contracts::domain::a025_bi_dashboard::aggregate::DashboardGroup],
    out: &mut HashSet<String>,
) {
    for group in groups {
        for item in &group.items {
            out.insert(item.indicator_id.clone());
        }
        collect_indicator_ids_from_layout(&group.subgroups, out);
    }
}

async fn validate_indicator_refs(layout: &DashboardLayout) -> anyhow::Result<()> {
    let mut ids = HashSet::new();
    collect_indicator_ids_from_layout(&layout.groups, &mut ids);
    if ids.is_empty() {
        return Ok(());
    }

    let db = crate::shared::data::db::get_connection();
    for indicator_id in ids {
        let uuid = Uuid::parse_str(&indicator_id)
            .map_err(|e| anyhow::anyhow!("Invalid indicator_id `{}`: {}", indicator_id, e))?;
        let id = BiIndicatorId::new(uuid);
        let exists = a024_bi_indicator::repository::find_by_id(&db, &id).await?;
        if exists.is_none() {
            return Err(anyhow::anyhow!(
                "Indicator not found for dashboard layout: {}",
                indicator_id
            ));
        }
    }

    Ok(())
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
        validate_indicator_refs(&layout).await?;
        dashboard.layout = layout;
    }
    if let Some(filters) = dto.filters {
        dashboard.filters = filters;
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

    let dashboard_uuid =
        Uuid::parse_str(id_str).map_err(|e| anyhow::anyhow!("Invalid dashboard ID: {}", e))?;
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
        validate_indicator_refs(&layout).await?;
        dashboard.layout = layout;
    }
    if let Some(filters) = dto.filters {
        dashboard.filters = filters;
    }
    if let Some(status_str) = &dto.status {
        dashboard.status =
            BiDashboardStatus::from_str(status_str).unwrap_or(BiDashboardStatus::Draft);
    }
    if let Some(is_public) = dto.is_public {
        dashboard.is_public = is_public;
    }
    // rating: None в DTO означает "сбросить оценку", Some(x) — установить
    dashboard.rating = dto
        .rating
        .and_then(|r| if r >= 1 && r <= 5 { Some(r) } else { None });

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
    let dashboard_uuid =
        Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid dashboard ID: {}", e))?;
    let dashboard_id = BiDashboardId::new(dashboard_uuid);

    let db = crate::shared::data::db::get_connection();
    repository::soft_delete(&db, &dashboard_id).await?;

    Ok(())
}

/// Получить дашборд по ID
pub async fn get_by_id(id: &str) -> anyhow::Result<Option<BiDashboard>> {
    let dashboard_uuid =
        Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid dashboard ID: {}", e))?;
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
    sort_by: &str,
    sort_desc: bool,
    q: Option<&str>,
) -> anyhow::Result<(Vec<BiDashboard>, u64)> {
    let db = crate::shared::data::db::get_connection();
    let (dashboards, total) =
        repository::list_paginated(&db, page, page_size, sort_by, sort_desc, q).await?;
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
            "filter_id": "date_range_1",
            "required": true,
            "order": 0,
            "default_value": "2025-01-01,2025-01-31",
            "label_override": "Период"
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
            "filter_id": "date_range_1",
            "required": true,
            "order": 0,
            "default_value": "2025-03-01,2025-03-31",
            "label_override": "Период"
        }
    ]);

    // Дашборд 3: Пустой шаблон для проверки empty-state
    let layout_empty = serde_json::json!({ "groups": [] });
    let filters_empty = serde_json::json!([]);

    // Дашборд 4: Сквозной пример — Выручка по кабинетам с мульти-выбором и датами
    let layout_revenue = serde_json::json!({
        "groups": [
            {
                "id": "g-004-revenue",
                "title": "Продажи WB",
                "sort_order": 0,
                "items": [
                    {
                        "indicator_id": "a024a024-0001-4001-a001-000000000001",
                        "indicator_name": "Выручка WB",
                        "sort_order": 0,
                        "col_class": "2x1",
                        "param_overrides": {}
                    },
                    {
                        "indicator_id": "a024a024-0003-4001-a001-000000000003",
                        "indicator_name": "Количество заказов",
                        "sort_order": 1,
                        "col_class": "1x1",
                        "param_overrides": {}
                    }
                ],
                "subgroups": []
            }
        ]
    });

    // Сквозные фильтры через реестр FilterDef / FilterRef
    let filters_revenue = serde_json::json!([
        {
            "filter_id": "date_range_1",
            "required": true,
            "order": 0,
            "default_value": "2025-01-01,2025-03-31"
        },
        {
            "filter_id": "connection_mp_refs",
            "required": false,
            "order": 1,
            "default_value": ""
        }
    ]);

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
        (
            "a025a025-0004-4001-a001-000000000004",
            "DASH-REVENUE",
            "Выручка по кабинетам",
            "Сквозной пример: выручка WB с фильтром по кабинетам (мульти-выбор) и периоду (дата-пикеры).",
            &layout_revenue.to_string(),
            &filters_revenue.to_string(),
            5,
            "active",
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
