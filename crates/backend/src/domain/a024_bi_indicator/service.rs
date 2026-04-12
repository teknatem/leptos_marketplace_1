use super::repository;
use contracts::domain::a024_bi_indicator::aggregate::{
    BiIndicator, BiIndicatorId, BiIndicatorStatus, DataSpec, DrillSpec, ParamDef, ParamType,
    Threshold, ViewSpec,
};
use contracts::shared::analytics::{IndicatorContext, IndicatorValue, ValueFormat};
use contracts::shared::drilldown::DrilldownResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub version: Option<i64>,
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
    let description = persisted_description(&dto);

    let mut indicator = BiIndicator::new_for_insert(code, description, dto.owner_user_id);

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
    let description = persisted_description(&dto);

    let db = crate::shared::data::db::get_connection();
    let mut indicator = repository::find_by_id(&db, &indicator_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("BI Indicator not found: {}", id_str))?;

    if let Some(expected_version) = dto.version {
        let current_version = indicator.base.metadata.version as i64;
        if expected_version != current_version {
            return Err(anyhow::anyhow!(
                "Version conflict for BI indicator {}: expected {}, actual {}",
                id_str,
                expected_version,
                current_version
            ));
        }
    }

    // Обновляем поля
    if let Some(code) = dto.code {
        indicator.base.code = code;
    }
    indicator.base.description = description;
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

/// Получить набор индикаторов по списку id
pub async fn list_by_ids(ids: &[String]) -> anyhow::Result<Vec<BiIndicator>> {
    let db = crate::shared::data::db::get_connection();
    let indicators = repository::list_by_ids(&db, ids).await?;
    Ok(indicators)
}

fn serialize_json<T: Serialize>(value: &T) -> anyhow::Result<String> {
    serde_json::to_string(value).map_err(|e| anyhow::anyhow!("JSON serialize error: {}", e))
}

fn build_view_ctx(
    indicator: &BiIndicator,
    ctx: &IndicatorContext,
) -> contracts::shared::data_view::ViewContext {
    use contracts::shared::data_view::ViewContext;

    let mut view_ctx = ViewContext {
        date_from: ctx.date_from.clone(),
        date_to: ctx.date_to.clone(),
        period2_from: ctx.extra.get("period2_from").cloned(),
        period2_to: ctx.extra.get("period2_to").cloned(),
        connection_mp_refs: ctx.connection_mp_refs.clone(),
        params: HashMap::new(),
    };

    for param in &indicator.params {
        if let Some(default_value) = param
            .default_value
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            view_ctx
                .params
                .insert(param.key.clone(), default_value.to_string());
        }
    }

    for (key, value) in &ctx.extra {
        view_ctx.params.insert(key.clone(), value.clone());
    }

    if let Some(metric_id) = indicator
        .data_spec
        .metric_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        view_ctx
            .params
            .insert("metric".to_string(), metric_id.clone());
    }

    view_ctx
}

const INDICATOR_STACK_KEY: &str = "__indicator_stack";

fn indicator_stack(ctx: &IndicatorContext) -> Vec<String> {
    ctx.extra
        .get(INDICATOR_STACK_KEY)
        .map(String::as_str)
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn extend_indicator_stack(
    ctx: &IndicatorContext,
    indicator_code: &str,
) -> anyhow::Result<IndicatorContext> {
    let mut next = ctx.clone();
    let mut stack = indicator_stack(ctx);
    if stack
        .iter()
        .any(|value| value.eq_ignore_ascii_case(indicator_code))
    {
        return Err(anyhow::anyhow!(
            "Circular BI indicator dependency detected for '{}'",
            indicator_code
        ));
    }
    stack.push(indicator_code.to_string());
    next.extra
        .insert(INDICATOR_STACK_KEY.to_string(), stack.join(","));
    Ok(next)
}

async fn compute_indicator_model(
    indicator: &BiIndicator,
    ctx: &IndicatorContext,
) -> anyhow::Result<IndicatorValue> {
    let stacked_ctx = extend_indicator_stack(ctx, &indicator.base.code)?;

    if let Some(view_id) = &indicator.data_spec.view_id {
        use crate::data_view::DataViewRegistry;
        let registry = DataViewRegistry::new();
        let view_ctx = build_view_ctx(indicator, &stacked_ctx);
        return registry
            .compute_scalar(view_id, &view_ctx)
            .await
            .map_err(|e| anyhow::anyhow!("DataView '{}' compute error: {}", view_id, e));
    }

    Err(anyhow::anyhow!(
        "BI indicator {} has no supported compute source. Expected view_id.",
        indicator.base.code
    ))
}

fn default_date_params(include_connections: bool) -> Vec<ParamDef> {
    let mut params = vec![
        ParamDef {
            key: "date_from".to_string(),
            param_type: ParamType::Date,
            label: "Начало периода".to_string(),
            default_value: None,
            required: false,
            global_filter_key: Some("date_from".to_string()),
        },
        ParamDef {
            key: "date_to".to_string(),
            param_type: ParamType::Date,
            label: "Конец периода".to_string(),
            default_value: None,
            required: false,
            global_filter_key: Some("date_to".to_string()),
        },
    ];

    if include_connections {
        params.push(ParamDef {
            key: "connection_ids".to_string(),
            param_type: ParamType::Ref,
            label: "Кабинеты МП".to_string(),
            default_value: None,
            required: false,
            global_filter_key: Some("connection_ids".to_string()),
        });
    }

    params
}

fn custom_view_spec(
    html: &str,
    css: &str,
    format: ValueFormat,
    thresholds: Vec<Threshold>,
) -> ViewSpec {
    ViewSpec {
        style_name: "custom".to_string(),
        custom_html: Some(html.to_string()),
        custom_css: Some(css.to_string()),
        format,
        thresholds,
        preview_values: HashMap::new(),
    }
}

/// Вставить тестовые данные (6 предварительно разработанных индикаторов)
pub async fn insert_test_data() -> anyhow::Result<()> {
    use sea_orm::{ConnectionTrait, DbBackend, Statement};
    let db = crate::shared::data::db::get_connection();

    const TEST_OWNER: &str = "f2fc6986-855d-492b-acff-70c7cd8cdd34";

    // (id, code, description, comment, data_spec, params, view_spec, status)
    let records: Vec<(
        &str,
        &str,
        &str,
        &str,
        DataSpec,
        Vec<ParamDef>,
        ViewSpec,
        &str,
    )> = vec![
        (
            "a024a024-0001-4001-a001-000000000001",
            "IND-REVENUE-WB",
            "Выручка WB",
            "Сквозной пример: суммарная выручка по выбранным кабинетам WB за период. DataView=dv001_revenue.",
            DataSpec {
                view_id: Some("dv001_revenue".to_string()),
                metric_id: Some("revenue".to_string()),
            },
            default_date_params(true),
            custom_view_spec(
                "<div class=\"kpi\"><div class=\"kpi__label\">{{title}}</div><div class=\"kpi__value\">{{value}}</div><div class=\"kpi__delta\">{{delta}}</div></div>",
                ".kpi{display:flex;flex-direction:column;gap:8px;height:100%;padding:4px}.kpi__label{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.6px}.kpi__value{font-size:2.4rem;font-weight:800;color:var(--bi-text);line-height:1}.kpi__delta{font-size:14px;font-weight:600;color:var(--bi-success);background:rgba(34,197,94,.12);padding:2px 10px;border-radius:12px;display:inline-block}",
                ValueFormat::Money {
                    currency: "RUB".to_string(),
                    scale: None,
                    decimals: None,
                },
                vec![],
            ),
            "active",
        ),
        (
            "a024a024-0002-4001-a001-000000000002",
            "IND-MARGIN",
            "Маржинальность",
            "Тестовый индикатор: процент маржи. Кольцеобразный дизайн, пороги зелёный/красный.",
            DataSpec::default(),
            vec![],
            custom_view_spec(
                "<div class=\"ring-kpi\"><div class=\"ring-kpi__ring\"><span class=\"ring-kpi__num\">{{value}}</span></div><div class=\"ring-kpi__info\"><div class=\"ring-kpi__title\">{{title}}</div><div class=\"ring-kpi__delta\">{{delta}}</div></div></div>",
                ".ring-kpi{display:flex;align-items:center;gap:16px;height:100%}.ring-kpi__ring{width:76px;height:76px;border-radius:50%;border:6px solid var(--bi-primary);display:flex;align-items:center;justify-content:center;flex-shrink:0}.ring-kpi__num{font-size:1rem;font-weight:800;color:var(--bi-primary)}.ring-kpi__info{display:flex;flex-direction:column;gap:6px}.ring-kpi__title{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.5px}.ring-kpi__delta{font-size:14px;font-weight:600;color:var(--bi-success)}",
                ValueFormat::Percent { decimals: 1 },
                vec![
                    Threshold {
                        condition: "> 25".to_string(),
                        color: "rgb(34,197,94)".to_string(),
                        label: Some("Высокая".to_string()),
                    },
                    Threshold {
                        condition: "< 10".to_string(),
                        color: "rgb(239,68,68)".to_string(),
                        label: Some("Низкая".to_string()),
                    },
                ],
            ),
            "active",
        ),
        (
            "a024a024-0003-4001-a001-000000000003",
            "IND-ORDERS",
            "Количество заказов",
            "Тестовый индикатор: количество заказов за период. DataView=dv001_revenue, metric=order_count.",
            DataSpec {
                view_id: Some("dv001_revenue".to_string()),
                metric_id: Some("order_count".to_string()),
            },
            default_date_params(true),
            custom_view_spec(
                "<div class=\"cnt-kpi\"><span class=\"cnt-kpi__dot\"></span><div class=\"cnt-kpi__body\"><div class=\"cnt-kpi__title\">{{title}}</div><div class=\"cnt-kpi__value\">{{value}}</div><div class=\"cnt-kpi__delta\">{{delta}}</div></div></div>",
                ".cnt-kpi{display:flex;align-items:flex-start;gap:12px;height:100%;padding:4px}.cnt-kpi__dot{width:10px;height:10px;border-radius:50%;background:var(--bi-primary);flex-shrink:0;margin-top:4px}.cnt-kpi__body{display:flex;flex-direction:column;gap:4px}.cnt-kpi__title{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.5px}.cnt-kpi__value{font-size:2.2rem;font-weight:800;color:var(--bi-text);line-height:1}.cnt-kpi__delta{font-size:13px;font-weight:600;color:var(--bi-success)}",
                ValueFormat::Integer,
                vec![],
            ),
            "active",
        ),
        (
            "a024a024-0004-4001-a001-000000000004",
            "IND-REVENUE-OZON",
            "Выручка Ozon",
            "Тестовый индикатор (draft): выручка Ozon.",
            DataSpec::default(),
            default_date_params(false),
            custom_view_spec(
                "<div class=\"kpi\"><div class=\"kpi__label\">{{title}}</div><div class=\"kpi__value\">{{value}}</div><div class=\"kpi__delta\">{{delta}}</div></div>",
                ".kpi{display:flex;flex-direction:column;gap:8px;height:100%;padding:4px}.kpi__label{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.6px}.kpi__value{font-size:2.4rem;font-weight:800;color:var(--bi-text);line-height:1}.kpi__delta{font-size:14px;font-weight:600;color:var(--bi-success);background:rgba(34,197,94,.12);padding:2px 10px;border-radius:12px;display:inline-block}",
                ValueFormat::Money {
                    currency: "RUB".to_string(),
                    scale: None,
                    decimals: None,
                },
                vec![],
            ),
            "draft",
        ),
        (
            "a024a024-0006-4001-a001-000000000006",
            "IND-PROFIT-D",
            "Прибыль (дилер)",
            "Тестовый индикатор: dealer profit за период. DataView=dv001_revenue, metric=profit_d.",
            DataSpec {
                view_id: Some("dv001_revenue".to_string()),
                metric_id: Some("profit_d".to_string()),
            },
            default_date_params(true),
            custom_view_spec(
                "<div class=\"kpi\"><div class=\"kpi__label\">{{title}}</div><div class=\"kpi__value\">{{value}}</div><div class=\"kpi__delta\">{{delta}}</div></div>",
                ".kpi{display:flex;flex-direction:column;gap:8px;height:100%;padding:4px}.kpi__label{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.6px}.kpi__value{font-size:2.4rem;font-weight:800;color:var(--bi-text);line-height:1}.kpi__delta{font-size:14px;font-weight:600;color:var(--bi-success);background:rgba(34,197,94,.12);padding:2px 10px;border-radius:12px;display:inline-block}",
                ValueFormat::Money {
                    currency: "RUB".to_string(),
                    scale: None,
                    decimals: None,
                },
                vec![],
            ),
            "active",
        ),
        (
            "a024a024-0007-4001-a001-000000000007",
            "IND-AVG-CHECK",
            "Средний чек",
            "Тестовый индикатор: средний чек за период. Формула согласована с revenue / order_count. DataView=dv001_revenue, metric=avg_check.",
            DataSpec {
                view_id: Some("dv006_indicator_ratio_percent".to_string()),
                metric_id: Some("ratio".to_string()),
            },
            vec![
                ParamDef {
                    key: "numerator_indicator_code".to_string(),
                    param_type: ParamType::String,
                    label: "Numerator indicator code".to_string(),
                    default_value: Some("IND-MP-REV-PRICE".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "denominator_indicator_code".to_string(),
                    param_type: ParamType::String,
                    label: "Denominator indicator code".to_string(),
                    default_value: Some("IND-ORDERS".to_string()),
                    required: true,
                    global_filter_key: None,
                },
            ],
            custom_view_spec(
                "<div class=\"kpi\"><div class=\"kpi__label\">{{title}}</div><div class=\"kpi__value\">{{value}}</div><div class=\"kpi__delta\">{{delta}}</div></div>",
                ".kpi{display:flex;flex-direction:column;gap:8px;height:100%;padding:4px}.kpi__label{font-size:11px;font-weight:600;color:var(--bi-text-secondary);text-transform:uppercase;letter-spacing:.6px}.kpi__value{font-size:2.4rem;font-weight:800;color:var(--bi-text);line-height:1}.kpi__delta{font-size:14px;font-weight:600;color:var(--bi-success);background:rgba(34,197,94,.12);padding:2px 10px;border-radius:12px;display:inline-block}",
                ValueFormat::Money {
                    currency: "RUB".to_string(),
                    scale: None,
                    decimals: Some(2),
                },
                vec![],
            ),
            "active",
        ),
        (
            "a024a024-0005-4001-a001-000000000005",
            "IND-EMPTY",
            "Новый индикатор (без шаблона)",
            "Тестовый индикатор без HTML/CSS — для проверки empty-state на вкладке Превью.",
            DataSpec::default(),
            vec![],
            ViewSpec {
                style_name: "classic".to_string(),
                custom_html: None,
                custom_css: None,
                format: ValueFormat::Integer,
                thresholds: vec![],
                preview_values: HashMap::new(),
            },
            "draft",
        ),
        (
            "a024a024-0016-4001-a001-000000000016",
            "IND-GL-MP-ACQ-FACT",
            "MP acquiring (fact)",
            "Test indicator for dv004_general_ledger_turnovers with turnover_code=mp_acquiring and layer=fact.",
            DataSpec {
                view_id: Some("dv004_general_ledger_turnovers".to_string()),
                metric_id: Some("amount".to_string()),
            },
            vec![
                ParamDef {
                    key: "turnover_code".to_string(),
                    param_type: ParamType::String,
                    label: "Turnover code".to_string(),
                    default_value: Some("mp_acquiring".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "layer".to_string(),
                    param_type: ParamType::String,
                    label: "Layer".to_string(),
                    default_value: Some("fact".to_string()),
                    required: true,
                    global_filter_key: None,
                },
            ],
            ViewSpec {
                style_name: "classic".to_string(),
                custom_html: None,
                custom_css: None,
                format: ValueFormat::Money {
                    currency: "RUB".to_string(),
                    scale: None,
                    decimals: None,
                },
                thresholds: vec![],
                preview_values: HashMap::new(),
            },
            "active",
        ),
        (
            "a024a024-0017-4001-a001-000000000017",
            "IND-GL-MP-PENALTY-FACT",
            "MP penalty (fact)",
            "Test indicator for dv004_general_ledger_turnovers with turnover_code=mp_penalty and layer=fact.",
            DataSpec {
                view_id: Some("dv004_general_ledger_turnovers".to_string()),
                metric_id: Some("amount".to_string()),
            },
            vec![
                ParamDef {
                    key: "turnover_code".to_string(),
                    param_type: ParamType::String,
                    label: "Turnover code".to_string(),
                    default_value: Some("mp_penalty".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "layer".to_string(),
                    param_type: ParamType::String,
                    label: "Layer".to_string(),
                    default_value: Some("fact".to_string()),
                    required: true,
                    global_filter_key: None,
                },
            ],
            ViewSpec {
                style_name: "classic".to_string(),
                custom_html: None,
                custom_css: None,
                format: ValueFormat::Money {
                    currency: "RUB".to_string(),
                    scale: None,
                    decimals: None,
                },
                thresholds: vec![],
                preview_values: HashMap::new(),
            },
            "active",
        ),
        (
            "a024a024-0018-4001-a001-000000000018",
            "IND-GL-MP-LOGISTICS-FACT",
            "MP logistics (fact)",
            "Ready-to-use BI indicator on DataView dv004_general_ledger_turnovers with turnover_items=mp_ppvz_reward, mp_ppvz_reward_nm, mp_rebill_logistic_cost, mp_rebill_logistic_cost_nm and layer=fact.",
            DataSpec {
                view_id: Some("dv004_general_ledger_turnovers".to_string()),
                metric_id: Some("amount".to_string()),
            },
            vec![
                ParamDef {
                    key: "turnover_items".to_string(),
                    param_type: ParamType::String,
                    label: "GL turnovers".to_string(),
                    default_value: Some(
                        "mp_ppvz_reward, mp_ppvz_reward_nm, mp_rebill_logistic_cost, mp_rebill_logistic_cost_nm"
                            .to_string(),
                    ),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "turnover_code".to_string(),
                    param_type: ParamType::String,
                    label: "Turnover code".to_string(),
                    default_value: Some("mp_ppvz_reward".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "layer".to_string(),
                    param_type: ParamType::String,
                    label: "Layer".to_string(),
                    default_value: Some("fact".to_string()),
                    required: true,
                    global_filter_key: None,
                },
            ],
            ViewSpec {
                style_name: "classic".to_string(),
                custom_html: None,
                custom_css: None,
                format: ValueFormat::Money {
                    currency: "RUB".to_string(),
                    scale: None,
                    decimals: None,
                },
                thresholds: vec![],
                preview_values: HashMap::new(),
            },
            "active",
        ),
        (
            "a024a024-0019-4001-a001-000000000019",
            "IND-GL-7609-MAIN-BALANCE",
            "Итог 7609 (основные обороты)",
            "Ready-to-use BI indicator on DataView dv005_gl_account_view_total with account=7609, section=main, metric=balance.",
            DataSpec {
                view_id: Some("dv005_gl_account_view_total".to_string()),
                metric_id: Some("balance".to_string()),
            },
            vec![
                ParamDef {
                    key: "account".to_string(),
                    param_type: ParamType::String,
                    label: "Account".to_string(),
                    default_value: Some("7609".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "section".to_string(),
                    param_type: ParamType::String,
                    label: "Section".to_string(),
                    default_value: Some("main".to_string()),
                    required: true,
                    global_filter_key: None,
                },
            ],
            ViewSpec {
                style_name: "classic".to_string(),
                custom_html: None,
                custom_css: None,
                format: ValueFormat::Money {
                    currency: "RUB".to_string(),
                    scale: None,
                    decimals: None,
                },
                thresholds: vec![],
                preview_values: HashMap::new(),
            },
            "active",
        ),
        (
            "a024a024-0020-4001-a001-000000000020",
            "IND-REV-TO-PRICE-PCT",
            "Выручка к прайсу, %",
            "Ready-to-use BI indicator on DataView dv006_indicator_ratio_percent with numerator=REVENUE and denominator=IND-MP-REV-PRICE.",
            DataSpec {
                view_id: Some("dv006_indicator_ratio_percent".to_string()),
                metric_id: Some("ratio_percent".to_string()),
            },
            vec![
                ParamDef {
                    key: "numerator_indicator_code".to_string(),
                    param_type: ParamType::String,
                    label: "Numerator indicator code".to_string(),
                    default_value: Some("REVENUE".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "denominator_indicator_code".to_string(),
                    param_type: ParamType::String,
                    label: "Denominator indicator code".to_string(),
                    default_value: Some("IND-MP-REV-PRICE".to_string()),
                    required: true,
                    global_filter_key: None,
                },
            ],
            ViewSpec {
                style_name: "classic".to_string(),
                custom_html: None,
                custom_css: None,
                format: ValueFormat::Percent { decimals: 1 },
                thresholds: vec![],
                preview_values: HashMap::new(),
            },
            "active",
        ),
        (
            "a024a024-0021-4001-a001-000000000021",
            "IND-GL-7609-TO-PRICE-PCT",
            "7609 к прайсу, %",
            "Ready-to-use BI indicator on DataView dv006_indicator_ratio_percent with numerator=IND-GL-7609-MAIN-BALANCE and denominator=IND-MP-REV-PRICE.",
            DataSpec {
                view_id: Some("dv006_indicator_ratio_percent".to_string()),
                metric_id: Some("ratio_percent".to_string()),
            },
            vec![
                ParamDef {
                    key: "numerator_indicator_code".to_string(),
                    param_type: ParamType::String,
                    label: "Numerator indicator code".to_string(),
                    default_value: Some("IND-GL-7609-MAIN-BALANCE".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "denominator_indicator_code".to_string(),
                    param_type: ParamType::String,
                    label: "Denominator indicator code".to_string(),
                    default_value: Some("IND-MP-REV-PRICE".to_string()),
                    required: true,
                    global_filter_key: None,
                },
            ],
            ViewSpec {
                style_name: "classic".to_string(),
                custom_html: None,
                custom_css: None,
                format: ValueFormat::Percent { decimals: 1 },
                thresholds: vec![],
                preview_values: HashMap::new(),
            },
            "active",
        ),
        (
            "a024a024-0022-4001-a001-000000000022",
            "IND-MP-RETURNS-COUNT",
            "РљРѕР»РёС‡РµСЃС‚РІРѕ РІРѕР·РІСЂР°С‚РѕРІ",
            "Ready-to-use BI indicator on DataView dv004_general_ledger_turnovers with metric=entry_count, turnover_code=customer_revenue_pl_storno and layer=oper.",
            DataSpec {
                view_id: Some("dv004_general_ledger_turnovers".to_string()),
                metric_id: Some("entry_count".to_string()),
            },
            vec![
                ParamDef {
                    key: "turnover_code".to_string(),
                    param_type: ParamType::String,
                    label: "Turnover code".to_string(),
                    default_value: Some("customer_revenue_pl_storno".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "layer".to_string(),
                    param_type: ParamType::String,
                    label: "Layer".to_string(),
                    default_value: Some("oper".to_string()),
                    required: true,
                    global_filter_key: None,
                },
            ],
            ViewSpec {
                style_name: "classic".to_string(),
                custom_html: None,
                custom_css: None,
                format: ValueFormat::Integer,
                thresholds: vec![],
                preview_values: HashMap::new(),
            },
            "active",
        ),
        (
            "a024a024-0023-4001-a001-000000000023",
            "IND-MP-RETURNS-TO-REV-PCT",
            "Р’РѕР·РІСЂР°С‚С‹ Рє СЂРµР°Р»РёР·Р°С†РёРё, %",
            "Ready-to-use BI indicator on DataView dv007_gl_turnover_ratio_percent with numerator=customer_revenue_pl_storno/oper and denominator=customer_revenue_pl/oper.",
            DataSpec {
                view_id: Some("dv007_gl_turnover_ratio_percent".to_string()),
                metric_id: Some("ratio_percent".to_string()),
            },
            vec![
                ParamDef {
                    key: "numerator_turnover_items".to_string(),
                    param_type: ParamType::String,
                    label: "Numerator GL turnovers".to_string(),
                    default_value: Some("-customer_revenue_pl_storno".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "numerator_turnover_code".to_string(),
                    param_type: ParamType::String,
                    label: "Numerator turnover code".to_string(),
                    default_value: Some("customer_revenue_pl_storno".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "numerator_layer".to_string(),
                    param_type: ParamType::String,
                    label: "Numerator layer".to_string(),
                    default_value: Some("oper".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "denominator_turnover_code".to_string(),
                    param_type: ParamType::String,
                    label: "Denominator turnover code".to_string(),
                    default_value: Some("customer_revenue_pl".to_string()),
                    required: true,
                    global_filter_key: None,
                },
                ParamDef {
                    key: "denominator_layer".to_string(),
                    param_type: ParamType::String,
                    label: "Denominator layer".to_string(),
                    default_value: Some("oper".to_string()),
                    required: true,
                    global_filter_key: None,
                },
            ],
            ViewSpec {
                style_name: "classic".to_string(),
                custom_html: None,
                custom_css: None,
                format: ValueFormat::Percent { decimals: 1 },
                thresholds: vec![],
                preview_values: HashMap::new(),
            },
            "active",
        ),
    ];

    for (id, code, description, comment, data_spec, params, view_spec, status) in records {
        let data_spec_json = serialize_json(&data_spec)?;
        let params_json = serialize_json(&params)?;
        let view_spec_json = serialize_json(&view_spec)?;
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
/// Источник вычисления индикатора.
/// Поддерживается только путь через `data_spec.view_id`.
pub async fn compute_indicator(id: &str, ctx: &IndicatorContext) -> anyhow::Result<IndicatorValue> {
    let indicator_uuid =
        Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid indicator ID: {}", e))?;
    let indicator_id = BiIndicatorId::new(indicator_uuid);

    let db = crate::shared::data::db::get_connection();
    let indicator = repository::find_by_id(&db, &indicator_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("BI Indicator not found: {}", id))?;

    compute_indicator_model(&indicator, ctx).await
}

/// Вычислить значение индикатора по его code.
pub async fn compute_indicator_by_code(
    code: &str,
    ctx: &IndicatorContext,
) -> anyhow::Result<IndicatorValue> {
    let db = crate::shared::data::db::get_connection();
    let indicator = repository::find_by_code(&db, code)
        .await?
        .ok_or_else(|| anyhow::anyhow!("BI Indicator not found by code: {}", code))?;

    compute_indicator_model(&indicator, ctx).await
}

/// Выполнить drilldown для индикатора.
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
        let registry = DataViewRegistry::new();
        let view_ctx = build_view_ctx(&indicator, ctx);
        return registry
            .compute_drilldown(view_id, &view_ctx, &group_by, &[])
            .await
            .map_err(|e| anyhow::anyhow!("DataView '{}' drilldown error: {}", view_id, e));
    }

    Err(anyhow::anyhow!(
        "BI indicator {} has no supported drilldown source. Expected view_id.",
        id
    ))
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

fn persisted_description(dto: &BiIndicatorDto) -> String {
    dto.view_spec
        .as_ref()
        .and_then(|view_spec| view_spec.preview_values.get("name"))
        .filter(|name| !name.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| dto.description.clone())
}

#[cfg(test)]
mod indicator_stack_tests {
    use super::{extend_indicator_stack, indicator_stack, INDICATOR_STACK_KEY};
    use contracts::shared::analytics::IndicatorContext;
    use std::collections::HashMap;

    #[test]
    fn indicator_stack_detects_circular_dependency() {
        let mut ctx = IndicatorContext {
            date_from: "2026-04-01".to_string(),
            date_to: "2026-04-30".to_string(),
            organization_ref: None,
            marketplace: None,
            connection_mp_refs: vec![],
            extra: HashMap::new(),
        };
        ctx.extra.insert(
            INDICATOR_STACK_KEY.to_string(),
            "REVENUE,IND-MP-REV-PRICE".to_string(),
        );

        let err = extend_indicator_stack(&ctx, "REVENUE").unwrap_err();
        assert!(err
            .to_string()
            .contains("Circular BI indicator dependency detected"));
    }

    #[test]
    fn indicator_stack_appends_new_indicator_code() {
        let ctx = IndicatorContext {
            date_from: "2026-04-01".to_string(),
            date_to: "2026-04-30".to_string(),
            organization_ref: None,
            marketplace: None,
            connection_mp_refs: vec![],
            extra: HashMap::new(),
        };

        let next = extend_indicator_stack(&ctx, "IND-GL-7609-MAIN-BALANCE").unwrap();
        assert_eq!(
            indicator_stack(&next),
            vec!["IND-GL-7609-MAIN-BALANCE".to_string()]
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{build_view_ctx, persisted_description, BiIndicatorDto};
    use contracts::domain::a024_bi_indicator::aggregate::{
        BiIndicator, DataSpec, ParamDef, ParamType, ViewSpec,
    };
    use contracts::shared::analytics::IndicatorContext;
    use std::collections::HashMap;

    #[test]
    fn indicator_param_defaults_are_merged_and_request_overrides_them() {
        let mut indicator = BiIndicator::new_for_insert(
            "IND-TEST".to_string(),
            "Test".to_string(),
            "owner".to_string(),
        );
        indicator.params = vec![
            ParamDef {
                key: "turnover_code".to_string(),
                param_type: ParamType::String,
                label: "Turnover".to_string(),
                default_value: Some("mp_acquiring".to_string()),
                required: true,
                global_filter_key: None,
            },
            ParamDef {
                key: "layer".to_string(),
                param_type: ParamType::String,
                label: "Layer".to_string(),
                default_value: Some("fact".to_string()),
                required: true,
                global_filter_key: None,
            },
        ];
        indicator.data_spec.metric_id = Some("amount".to_string());

        let mut extra = HashMap::new();
        extra.insert("layer".to_string(), "oper".to_string());
        extra.insert("custom".to_string(), "value".to_string());
        let ctx = IndicatorContext {
            date_from: "2026-03-01".to_string(),
            date_to: "2026-03-31".to_string(),
            organization_ref: None,
            marketplace: None,
            connection_mp_refs: vec!["cab-1".to_string()],
            extra,
        };

        let view_ctx = build_view_ctx(&indicator, &ctx);
        assert_eq!(
            view_ctx.params.get("turnover_code").map(String::as_str),
            Some("mp_acquiring")
        );
        assert_eq!(
            view_ctx.params.get("layer").map(String::as_str),
            Some("oper")
        );
        assert_eq!(
            view_ctx.params.get("metric").map(String::as_str),
            Some("amount")
        );
        assert_eq!(
            view_ctx.params.get("custom").map(String::as_str),
            Some("value")
        );
    }

    #[test]
    fn persisted_description_prefers_preview_title_when_present() {
        let mut preview_values = HashMap::new();
        preview_values.insert("name".to_string(), "Маржинальность".to_string());

        let dto = BiIndicatorDto {
            id: None,
            code: Some("IND-TEST".to_string()),
            description: "Старое имя".to_string(),
            comment: None,
            data_spec: Some(DataSpec::default()),
            params: None,
            view_spec: Some(ViewSpec {
                preview_values,
                ..ViewSpec::default()
            }),
            drill_spec: None,
            status: None,
            owner_user_id: "owner".to_string(),
            is_public: None,
            updated_by: None,
        };

        assert_eq!(persisted_description(&dto), "Маржинальность");
    }

    #[test]
    fn persisted_description_falls_back_to_description_when_preview_title_is_blank() {
        let mut preview_values = HashMap::new();
        preview_values.insert("name".to_string(), "   ".to_string());

        let dto = BiIndicatorDto {
            id: None,
            code: Some("IND-TEST".to_string()),
            description: "Выручка".to_string(),
            comment: None,
            data_spec: Some(DataSpec::default()),
            params: None,
            view_spec: Some(ViewSpec {
                preview_values,
                ..ViewSpec::default()
            }),
            drill_spec: None,
            status: None,
            owner_user_id: "owner".to_string(),
            is_public: None,
            updated_by: None,
        };

        assert_eq!(persisted_description(&dto), "Выручка");
    }
}
