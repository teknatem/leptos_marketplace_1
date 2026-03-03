use super::repository;
use contracts::domain::a024_bi_indicator::aggregate::{
    BiIndicator, BiIndicatorId, BiIndicatorStatus, DataSpec, DrillSpec, ParamDef, ViewSpec,
};
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// DTO для создания/обновления BI индикатора через API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiIndicatorDto {
    pub id: Option<String>,
    pub code: Option<String>,
    pub description: String,
    pub comment: Option<String>,

    /// JSON-представление DataSpec
    pub data_spec: Option<DataSpec>,
    /// Список параметров
    pub params: Option<Vec<ParamDef>>,
    /// JSON-представление ViewSpec
    pub view_spec: Option<ViewSpec>,
    /// JSON-представление DrillSpec (опционально)
    pub drill_spec: Option<DrillSpec>,

    pub status: Option<String>,
    pub owner_user_id: String,
    pub is_public: Option<bool>,
    pub updated_by: Option<String>,
}

// ============================================================================
// HTML sanitization
// ============================================================================

/// Санитизация HTML для ViewSpec.custom_html.
///
/// Правила:
/// - Запрещены все `<script>` теги и их содержимое
/// - Запрещены все `on*` атрибуты (onclick, onload, etc.)
/// - Запрещены `javascript:` URI
/// - Разрешены подстановки {{value}}, {{delta}}, {{title}}
/// - Разрешены базовые HTML-теги и атрибуты `class` / `style`
pub fn sanitize_html(html: &str) -> String {
    ammonia::Builder::new()
        .tags(maplit::hashset![
            "div", "span", "p", "h1", "h2", "h3", "h4", "h5", "h6",
            "ul", "ol", "li", "strong", "em", "b", "i", "small", "sup", "sub",
            "table", "thead", "tbody", "tr", "td", "th",
            "br", "hr", "section", "article", "aside", "header", "footer",
        ])
        .generic_attributes(maplit::hashset!["class", "style", "id", "title"])
        .clean(html)
        .to_string()
}

// ============================================================================
// Service functions
// ============================================================================

/// Создание нового BI индикатора
pub async fn create(dto: BiIndicatorDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("IND-{}", &Uuid::new_v4().to_string()[..8].to_uppercase()));

    let mut indicator = BiIndicator::new_for_insert(code, dto.description, dto.owner_user_id);

    // Применяем поля из DTO
    indicator.base.comment = dto.comment;

    if let Some(data_spec) = dto.data_spec {
        indicator.data_spec = data_spec;
    }
    if let Some(params) = dto.params {
        indicator.params = params;
    }
    if let Some(mut view_spec) = dto.view_spec {
        sanitize_view_spec_html(&mut view_spec);
        indicator.view_spec = view_spec;
    }
    if let Some(drill_spec) = dto.drill_spec {
        indicator.drill_spec = Some(drill_spec);
    }
    if let Some(status_str) = &dto.status {
        indicator.status =
            BiIndicatorStatus::from_str(status_str).unwrap_or(BiIndicatorStatus::Draft);
    }
    if let Some(is_public) = dto.is_public {
        indicator.is_public = is_public;
    }
    if let Some(updated_by) = dto.updated_by {
        indicator.created_by = Some(updated_by);
    }

    indicator
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    indicator.before_write();

    let id = indicator.base.id.0;

    let db = crate::shared::data::db::get_connection();
    repository::insert(&db, &indicator).await?;

    Ok(id)
}

/// Обновление BI индикатора
pub async fn update(dto: BiIndicatorDto) -> anyhow::Result<()> {
    let id_str = dto
        .id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ID is required for update"))?;

    let indicator_uuid = Uuid::parse_str(id_str)
        .map_err(|e| anyhow::anyhow!("Invalid indicator ID: {}", e))?;
    let indicator_id = BiIndicatorId::new(indicator_uuid);

    let db = crate::shared::data::db::get_connection();
    let mut indicator = repository::find_by_id(&db, &indicator_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("BI Indicator not found: {}", id_str))?;

    // Обновляем поля
    if let Some(code) = dto.code {
        indicator.base.code = code;
    }
    indicator.base.description = dto.description;
    indicator.base.comment = dto.comment;

    if let Some(data_spec) = dto.data_spec {
        indicator.data_spec = data_spec;
    }
    if let Some(params) = dto.params {
        indicator.params = params;
    }
    if let Some(mut view_spec) = dto.view_spec {
        sanitize_view_spec_html(&mut view_spec);
        indicator.view_spec = view_spec;
    }
    indicator.drill_spec = dto.drill_spec;

    if let Some(status_str) = &dto.status {
        indicator.status =
            BiIndicatorStatus::from_str(status_str).unwrap_or(BiIndicatorStatus::Draft);
    }
    if let Some(is_public) = dto.is_public {
        indicator.is_public = is_public;
    }
    if let Some(updated_by) = dto.updated_by {
        indicator.updated_by = Some(updated_by);
    }
    indicator.owner_user_id = dto.owner_user_id;

    indicator
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    indicator.before_write();

    repository::update(&db, &indicator).await?;

    Ok(())
}

/// Удаление индикатора (soft delete)
pub async fn delete(id: &str) -> anyhow::Result<()> {
    let indicator_uuid = Uuid::parse_str(id)
        .map_err(|e| anyhow::anyhow!("Invalid indicator ID: {}", e))?;
    let indicator_id = BiIndicatorId::new(indicator_uuid);

    let db = crate::shared::data::db::get_connection();
    repository::soft_delete(&db, &indicator_id).await?;

    Ok(())
}

/// Получить индикатор по ID
pub async fn get_by_id(id: &str) -> anyhow::Result<Option<BiIndicator>> {
    let indicator_uuid = Uuid::parse_str(id)
        .map_err(|e| anyhow::anyhow!("Invalid indicator ID: {}", e))?;
    let indicator_id = BiIndicatorId::new(indicator_uuid);

    let db = crate::shared::data::db::get_connection();
    let indicator = repository::find_by_id(&db, &indicator_id).await?;

    Ok(indicator)
}

/// Получить все индикаторы (без пагинации)
pub async fn list_all() -> anyhow::Result<Vec<BiIndicator>> {
    let db = crate::shared::data::db::get_connection();
    let indicators = repository::list_all(&db).await?;
    Ok(indicators)
}

/// Получить список индикаторов с пагинацией
pub async fn list_paginated(
    page: u64,
    page_size: u64,
) -> anyhow::Result<(Vec<BiIndicator>, u64)> {
    let db = crate::shared::data::db::get_connection();
    let (indicators, total) = repository::list_paginated(&db, page, page_size).await?;
    Ok((indicators, total))
}

/// Получить индикаторы конкретного владельца
pub async fn list_by_owner(owner_user_id: &str) -> anyhow::Result<Vec<BiIndicator>> {
    let db = crate::shared::data::db::get_connection();
    let indicators = repository::list_by_owner(&db, owner_user_id).await?;
    Ok(indicators)
}

/// Получить публичные индикаторы
pub async fn list_public() -> anyhow::Result<Vec<BiIndicator>> {
    let db = crate::shared::data::db::get_connection();
    let indicators = repository::list_public(&db).await?;
    Ok(indicators)
}

/// Вставить тестовые данные (5 предварительно разработанных индикаторов)
pub async fn insert_test_data() -> anyhow::Result<()> {
    use sea_orm::{ConnectionTrait, DbBackend, Statement};
    let db = crate::shared::data::db::get_connection();

    const TEST_OWNER: &str = "f2fc6986-855d-492b-acff-70c7cd8cdd34";

    let records: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
        (
            "a024a024-0001-4001-a001-000000000001",
            "IND-REVENUE-WB",
            "Выручка WB",
            "Тестовый индикатор: суммарная выручка Wildberries за выбранный период.",
            r#"[{"key":"period","param_type":"date_range","label":"Период","default_value":null,"required":false,"global_filter_key":"date_range"}]"#,
            r#"{"custom_html":"<div class=\"kpi\"><div class=\"kpi__label\">{{title}}</div><div class=\"kpi__value\">{{value}}</div><div class=\"kpi__delta\">{{delta}}</div></div>","custom_css":".kpi{display:flex;flex-direction:column;gap:8px;height:100%;padding:4px}.kpi__label{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.6px}.kpi__value{font-size:2.4rem;font-weight:800;color:var(--bi-text);line-height:1}.kpi__delta{font-size:14px;font-weight:600;color:var(--bi-success);background:rgba(34,197,94,.12);padding:2px 10px;border-radius:12px;display:inline-block}","format":{"kind":"Money","currency":"RUB"},"thresholds":[]}"#,
            "active",
        ),
        (
            "a024a024-0002-4001-a001-000000000002",
            "IND-MARGIN",
            "Маржинальность",
            "Тестовый индикатор: процент маржи. Кольцеобразный дизайн, пороги зелёный/красный.",
            "[]",
            r#"{"custom_html":"<div class=\"ring-kpi\"><div class=\"ring-kpi__ring\"><span class=\"ring-kpi__num\">{{value}}</span></div><div class=\"ring-kpi__info\"><div class=\"ring-kpi__title\">{{title}}</div><div class=\"ring-kpi__delta\">{{delta}}</div></div></div>","custom_css":".ring-kpi{display:flex;align-items:center;gap:16px;height:100%}.ring-kpi__ring{width:76px;height:76px;border-radius:50%;border:6px solid var(--bi-primary);display:flex;align-items:center;justify-content:center;flex-shrink:0}.ring-kpi__num{font-size:1rem;font-weight:800;color:var(--bi-primary)}.ring-kpi__info{display:flex;flex-direction:column;gap:6px}.ring-kpi__title{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.5px}.ring-kpi__delta{font-size:14px;font-weight:600;color:var(--bi-success)}","format":{"kind":"Percent","decimals":1},"thresholds":[{"condition":"> 25","color":"rgb(34,197,94)","label":"Высокая"},{"condition":"< 10","color":"rgb(239,68,68)","label":"Низкая"}]}"#,
            "active",
        ),
        (
            "a024a024-0003-4001-a001-000000000003",
            "IND-ORDERS",
            "Количество заказов",
            "Тестовый индикатор: количество заказов за период. Карточка с маркером.",
            r#"[{"key":"period","param_type":"date_range","label":"Период","default_value":null,"required":false,"global_filter_key":"date_range"}]"#,
            r#"{"custom_html":"<div class=\"cnt-kpi\"><span class=\"cnt-kpi__dot\"></span><div class=\"cnt-kpi__body\"><div class=\"cnt-kpi__title\">{{title}}</div><div class=\"cnt-kpi__value\">{{value}}</div><div class=\"cnt-kpi__delta\">{{delta}}</div></div></div>","custom_css":".cnt-kpi{display:flex;align-items:flex-start;gap:12px;height:100%;padding:4px}.cnt-kpi__dot{width:10px;height:10px;border-radius:50%;background:var(--bi-primary);flex-shrink:0;margin-top:4px}.cnt-kpi__body{display:flex;flex-direction:column;gap:4px}.cnt-kpi__title{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.5px}.cnt-kpi__value{font-size:2.2rem;font-weight:800;color:var(--bi-text);line-height:1}.cnt-kpi__delta{font-size:13px;font-weight:600;color:var(--bi-success)}","format":{"kind":"Integer"},"thresholds":[]}"#,
            "active",
        ),
        (
            "a024a024-0004-4001-a001-000000000004",
            "IND-REVENUE-OZON",
            "Выручка Ozon",
            "Тестовый индикатор (draft): выручка Ozon. Статус draft для проверки фильтрации.",
            r#"[{"key":"period","param_type":"date_range","label":"Период","default_value":null,"required":false,"global_filter_key":"date_range"}]"#,
            r#"{"custom_html":"<div class=\"kpi\"><div class=\"kpi__label\">{{title}}</div><div class=\"kpi__value\">{{value}}</div><div class=\"kpi__delta\">{{delta}}</div></div>","custom_css":".kpi{display:flex;flex-direction:column;gap:8px;height:100%;padding:4px}.kpi__label{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.6px}.kpi__value{font-size:2.4rem;font-weight:800;color:var(--bi-text);line-height:1}.kpi__delta{font-size:14px;font-weight:600;color:var(--bi-success);background:rgba(34,197,94,.12);padding:2px 10px;border-radius:12px;display:inline-block}","format":{"kind":"Money","currency":"RUB"},"thresholds":[]}"#,
            "draft",
        ),
        (
            "a024a024-0005-4001-a001-000000000005",
            "IND-EMPTY",
            "Новый индикатор (без шаблона)",
            "Тестовый индикатор без HTML/CSS — для проверки empty-state на вкладке Превью.",
            "[]",
            r#"{"custom_html":null,"custom_css":null,"format":{"kind":"Integer"},"thresholds":[]}"#,
            "draft",
        ),
    ];

    let data_spec_empty = r#"{"schema_id":"","query_config":{"data_source":"","selected_fields":[],"groupings":[],"filters":{}},"sql_artifact_id":null}"#;

    for (id, code, description, comment, params_json, view_spec_json, status) in records {
        let sql = format!(
            "INSERT OR IGNORE INTO a024_bi_indicator \
            (id, code, description, comment, data_spec_json, params_json, view_spec_json, \
             status, owner_user_id, is_public, created_at, updated_at, version) \
            VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', 1, datetime('now'), datetime('now'), 1)",
            id, code, description, comment, data_spec_empty, params_json, view_spec_json,
            status, TEST_OWNER
        );
        db.execute(Statement::from_string(DbBackend::Sqlite, sql)).await
            .map_err(|e| anyhow::anyhow!("Failed to insert test record {}: {}", id, e))?;
    }

    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

fn sanitize_view_spec_html(view_spec: &mut ViewSpec) {
    if let Some(html) = &view_spec.custom_html {
        let sanitized = sanitize_html(html);
        view_spec.custom_html = Some(sanitized);
    }
}
