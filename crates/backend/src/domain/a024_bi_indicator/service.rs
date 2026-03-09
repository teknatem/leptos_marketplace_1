use super::repository;
use contracts::domain::a024_bi_indicator::aggregate::{
    BiIndicator, BiIndicatorId, BiIndicatorStatus, DataSpec, DrillSpec, ParamDef, ViewSpec,
};
use contracts::domain::common::AggregateId;
use contracts::shared::drilldown::{DrilldownRequest, DrilldownResponse};
use contracts::shared::indicators::{IndicatorContext, IndicatorId, IndicatorValue};
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
            "div", "span", "p", "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "li", "strong",
            "em", "b", "i", "small", "sup", "sub", "table", "thead", "tbody", "tr", "td", "th",
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

    let indicator_uuid =
        Uuid::parse_str(id_str).map_err(|e| anyhow::anyhow!("Invalid indicator ID: {}", e))?;
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
    let indicator_uuid =
        Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid indicator ID: {}", e))?;
    let indicator_id = BiIndicatorId::new(indicator_uuid);

    let db = crate::shared::data::db::get_connection();
    repository::soft_delete(&db, &indicator_id).await?;

    Ok(())
}

/// Получить индикатор по ID
pub async fn get_by_id(id: &str) -> anyhow::Result<Option<BiIndicator>> {
    let indicator_uuid =
        Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid indicator ID: {}", e))?;
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
    sort_by: &str,
    sort_desc: bool,
    q: Option<&str>,
) -> anyhow::Result<(Vec<BiIndicator>, u64)> {
    let db = crate::shared::data::db::get_connection();
    let (indicators, total) =
        repository::list_paginated(&db, page, page_size, sort_by, sort_desc, q).await?;
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

    // (id, code, description, comment, data_spec_json, params_json, view_spec_json, status)
    let records: &[(&str, &str, &str, &str, &str, &str, &str, &str)] = &[
        (
            "a024a024-0001-4001-a001-000000000001",
            "IND-REVENUE-WB",
            "Выручка WB",
            "Сквозной пример: суммарная выручка по выбранным кабинетам WB за период. schema_id=sales_revenue.",
            r#"{"schema_id":"sales_revenue","view_id":"dv001_revenue","query_config":{"data_source":"p904_sales_data","selected_fields":["revenue"],"groupings":[],"filters":{},"display_fields":[],"sort":{"field":"","ascending":true},"enabled_fields":[]},"sql_artifact_id":null}"#,
            r#"[{"key":"date_from","param_type":"date","label":"Начало периода","default_value":null,"required":false,"global_filter_key":"date_from"},{"key":"date_to","param_type":"date","label":"Конец периода","default_value":null,"required":false,"global_filter_key":"date_to"},{"key":"connection_ids","param_type":"ref","label":"Кабинеты МП","default_value":null,"required":false,"global_filter_key":"connection_ids"}]"#,
            r#"{"style_name":"custom","custom_html":"<div class=\"kpi\"><div class=\"kpi__label\">{{title}}</div><div class=\"kpi__value\">{{value}}</div><div class=\"kpi__delta\">{{delta}}</div></div>","custom_css":".kpi{display:flex;flex-direction:column;gap:8px;height:100%;padding:4px}.kpi__label{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.6px}.kpi__value{font-size:2.4rem;font-weight:800;color:var(--bi-text);line-height:1}.kpi__delta{font-size:14px;font-weight:600;color:var(--bi-success);background:rgba(34,197,94,.12);padding:2px 10px;border-radius:12px;display:inline-block}","format":{"kind":"Money","currency":"RUB"},"thresholds":[]}"#,
            "active",
        ),
        (
            "a024a024-0002-4001-a001-000000000002",
            "IND-MARGIN",
            "Маржинальность",
            "Тестовый индикатор: процент маржи. Кольцеобразный дизайн, пороги зелёный/красный.",
            r#"{"schema_id":"","query_config":{"data_source":"","selected_fields":[],"groupings":[],"filters":{}},"sql_artifact_id":null}"#,
            "[]",
            r#"{"style_name":"custom","custom_html":"<div class=\"ring-kpi\"><div class=\"ring-kpi__ring\"><span class=\"ring-kpi__num\">{{value}}</span></div><div class=\"ring-kpi__info\"><div class=\"ring-kpi__title\">{{title}}</div><div class=\"ring-kpi__delta\">{{delta}}</div></div></div>","custom_css":".ring-kpi{display:flex;align-items:center;gap:16px;height:100%}.ring-kpi__ring{width:76px;height:76px;border-radius:50%;border:6px solid var(--bi-primary);display:flex;align-items:center;justify-content:center;flex-shrink:0}.ring-kpi__num{font-size:1rem;font-weight:800;color:var(--bi-primary)}.ring-kpi__info{display:flex;flex-direction:column;gap:6px}.ring-kpi__title{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.5px}.ring-kpi__delta{font-size:14px;font-weight:600;color:var(--bi-success)}","format":{"kind":"Percent","decimals":1},"thresholds":[{"condition":"> 25","color":"rgb(34,197,94)","label":"Высокая"},{"condition":"< 10","color":"rgb(239,68,68)","label":"Низкая"}]}"#,
            "active",
        ),
        (
            "a024a024-0003-4001-a001-000000000003",
            "IND-ORDERS",
            "Количество заказов",
            "Тестовый индикатор: количество заказов за период. schema_id=sales_order_count.",
            r#"{"schema_id":"sales_order_count","query_config":{"data_source":"p904_sales_data","selected_fields":["order_count"],"groupings":[],"filters":{}},"sql_artifact_id":null}"#,
            r#"[{"key":"date_from","param_type":"date","label":"Начало периода","default_value":null,"required":false,"global_filter_key":"date_from"},{"key":"date_to","param_type":"date","label":"Конец периода","default_value":null,"required":false,"global_filter_key":"date_to"},{"key":"connection_ids","param_type":"ref","label":"Кабинеты МП","default_value":null,"required":false,"global_filter_key":"connection_ids"}]"#,
            r#"{"style_name":"custom","custom_html":"<div class=\"cnt-kpi\"><span class=\"cnt-kpi__dot\"></span><div class=\"cnt-kpi__body\"><div class=\"cnt-kpi__title\">{{title}}</div><div class=\"cnt-kpi__value\">{{value}}</div><div class=\"cnt-kpi__delta\">{{delta}}</div></div></div>","custom_css":".cnt-kpi{display:flex;align-items:flex-start;gap:12px;height:100%;padding:4px}.cnt-kpi__dot{width:10px;height:10px;border-radius:50%;background:var(--bi-primary);flex-shrink:0;margin-top:4px}.cnt-kpi__body{display:flex;flex-direction:column;gap:4px}.cnt-kpi__title{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.5px}.cnt-kpi__value{font-size:2.2rem;font-weight:800;color:var(--bi-text);line-height:1}.cnt-kpi__delta{font-size:13px;font-weight:600;color:var(--bi-success)}","format":{"kind":"Integer"},"thresholds":[]}"#,
            "active",
        ),
        (
            "a024a024-0004-4001-a001-000000000004",
            "IND-REVENUE-OZON",
            "Выручка Ozon",
            "Тестовый индикатор (draft): выручка Ozon.",
            r#"{"schema_id":"","query_config":{"data_source":"","selected_fields":[],"groupings":[],"filters":{}},"sql_artifact_id":null}"#,
            r#"[{"key":"date_from","param_type":"date","label":"Начало периода","default_value":null,"required":false,"global_filter_key":"date_from"},{"key":"date_to","param_type":"date","label":"Конец периода","default_value":null,"required":false,"global_filter_key":"date_to"}]"#,
            r#"{"style_name":"custom","custom_html":"<div class=\"kpi\"><div class=\"kpi__label\">{{title}}</div><div class=\"kpi__value\">{{value}}</div><div class=\"kpi__delta\">{{delta}}</div></div>","custom_css":".kpi{display:flex;flex-direction:column;gap:8px;height:100%;padding:4px}.kpi__label{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.6px}.kpi__value{font-size:2.4rem;font-weight:800;color:var(--bi-text);line-height:1}.kpi__delta{font-size:14px;font-weight:600;color:var(--bi-success);background:rgba(34,197,94,.12);padding:2px 10px;border-radius:12px;display:inline-block}","format":{"kind":"Money","currency":"RUB"},"thresholds":[]}"#,
            "draft",
        ),
        (
            "a024a024-0005-4001-a001-000000000005",
            "IND-EMPTY",
            "Новый индикатор (без шаблона)",
            "Тестовый индикатор без HTML/CSS — для проверки empty-state на вкладке Превью.",
            r#"{"schema_id":"","query_config":{"data_source":"","selected_fields":[],"groupings":[],"filters":{}},"sql_artifact_id":null}"#,
            "[]",
            r#"{"style_name":"classic","custom_html":null,"custom_css":null,"format":{"kind":"Integer"},"thresholds":[]}"#,
            "draft",
        ),
    ];

    for (id, code, description, comment, data_spec_json, params_json, view_spec_json, status) in
        records
    {
        let sql = format!(
            "INSERT OR REPLACE INTO a024_bi_indicator \
            (id, code, description, comment, data_spec_json, params_json, view_spec_json, \
             status, owner_user_id, is_public, created_at, updated_at, version) \
            VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', 1, datetime('now'), datetime('now'), 1)",
            id, code, description, comment, data_spec_json, params_json, view_spec_json,
            status, TEST_OWNER
        );
        db.execute(Statement::from_string(DbBackend::Sqlite, sql))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to insert test record {}: {}", id, e))?;
    }

    Ok(())
}

// ============================================================================
// Schema-based indicator compute
// ============================================================================

/// Вычислить значение индикатора по его ID.
///
/// Приоритет диспетчера:
/// 1. `data_spec.data_source_config` — новый универсальный путь (IndicatorDataSourceConfig)
/// 2. `data_spec.schema_query` — устаревший p904-специфичный путь (backward compat)
/// 3. IndicatorRegistry — legacy-путь по schema_id строке
pub async fn compute_indicator(
    id: &str,
    ctx: &IndicatorContext,
) -> anyhow::Result<IndicatorValue> {
    let indicator_uuid =
        Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid indicator ID: {}", e))?;
    let indicator_id = BiIndicatorId::new(indicator_uuid);

    let db = crate::shared::data::db::get_connection();
    let indicator = repository::find_by_id(&db, &indicator_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("BI Indicator not found: {}", id))?;

    // DataView path (highest priority)
    if let Some(view_id) = &indicator.data_spec.view_id {
        use crate::data_view::DataViewRegistry;
        use contracts::shared::data_view::ViewContext;
        let registry = DataViewRegistry::new();
        let view_ctx = ViewContext::from(ctx);
        return registry
            .compute_scalar(view_id, &view_ctx)
            .await
            .map_err(|e| anyhow::anyhow!("DataView '{}' compute error: {}", view_id, e));
    }

    if let Some(dsc) = &indicator.data_spec.data_source_config {
        // New universal path
        crate::shared::indicators::schema_executor::compute_from_data_source(
            dsc,
            ctx,
            IndicatorId::new(id),
        )
        .await
        .map_err(|e| anyhow::anyhow!("DataSourceConfig compute error: {}", e))
    } else if let Some(schema_config) = &indicator.data_spec.schema_query {
        // Legacy p904 schema path (backward compat)
        crate::shared::indicators::schema_executor::compute_p904(
            schema_config,
            ctx,
            IndicatorId::new(id),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Schema compute error: {}", e))
    } else {
        // Legacy registry path
        use crate::shared::indicators::registry::IndicatorRegistry;
        let schema_id = &indicator.data_spec.schema_id;
        let ind_id = IndicatorId::new(schema_id);
        let registry = IndicatorRegistry::new();
        let results = registry.compute(&[ind_id.clone()], ctx).await;
        results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No result from registry for schema_id={}", schema_id))
    }
}

/// Выполнить drilldown для индикатора.
///
/// Schema_id берётся из `data_spec.schema_id` (или "ds03_p904_sales" если задан schema_query).
pub async fn get_indicator_drilldown(
    id: &str,
    group_by: String,
    ctx: &IndicatorContext,
) -> anyhow::Result<DrilldownResponse> {
    let indicator_uuid =
        Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid indicator ID: {}", e))?;
    let indicator_id = BiIndicatorId::new(indicator_uuid);

    let db = crate::shared::data::db::get_connection();
    let indicator = repository::find_by_id(&db, &indicator_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("BI Indicator not found: {}", id))?;

    // DataView path (highest priority)
    if let Some(view_id) = &indicator.data_spec.view_id {
        use crate::data_view::DataViewRegistry;
        use contracts::shared::data_view::ViewContext;
        let registry = DataViewRegistry::new();
        let view_ctx = ViewContext::from(ctx);
        return registry
            .compute_drilldown(view_id, &view_ctx, &group_by)
            .await
            .map_err(|e| anyhow::anyhow!("DataView '{}' drilldown error: {}", view_id, e));
    }

    // Determine schema_id and metric_column — prefer new data_source_config
    let (schema_id, metric_column) = if let Some(dsc) = &indicator.data_spec.data_source_config {
        (dsc.schema_id.clone(), dsc.metric_field_id.clone())
    } else if indicator.data_spec.schema_query.is_some() {
        let metric = indicator
            .data_spec
            .schema_query
            .as_ref()
            .map(|sq| sq.metric.column_name().to_string())
            .unwrap_or_else(|| "customer_in".to_string());
        ("ds03_p904_sales".to_string(), metric)
    } else {
        (indicator.data_spec.schema_id.clone(), "customer_in".to_string())
    };

    let req = DrilldownRequest {
        schema_id,
        group_by,
        date_from: ctx.date_from.clone(),
        date_to: ctx.date_to.clone(),
        period2_from: ctx.extra.get("period2_from").cloned(),
        period2_to: ctx.extra.get("period2_to").cloned(),
        connection_mp_refs: ctx.connection_mp_refs.clone(),
        extra_filters: std::collections::HashMap::new(),
        metric_column,
    };

    crate::shared::indicators::schema_executor::execute_drilldown(&req)
        .await
        .map_err(|e| anyhow::anyhow!("Drilldown error: {}", e))
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
