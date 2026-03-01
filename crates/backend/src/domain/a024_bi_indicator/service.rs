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

// ============================================================================
// Helpers
// ============================================================================

fn sanitize_view_spec_html(view_spec: &mut ViewSpec) {
    if let Some(html) = &view_spec.custom_html {
        let sanitized = sanitize_html(html);
        view_spec.custom_html = Some(sanitized);
    }
}
