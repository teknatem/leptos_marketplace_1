//! Бизнес-логика подсистемы Plugins: CRUD, валидация, тестовые данные.

use super::repository;
use chrono::Utc;
use contracts::plugins::{
    DataBinding, PluginBundle, PluginDefinition, PluginManifest, PluginRuntime, PluginStatus,
    PluginUpsert, ViewSpec, Widget, WidgetKind,
};
use uuid::Uuid;

fn db() -> &'static sea_orm::DatabaseConnection {
    crate::shared::data::db::get_connection()
}

/// Валидация бандла без сохранения.
pub fn validate(bundle: &PluginBundle) -> Result<(), String> {
    bundle.validate()
}

/// Создание или обновление плагина по DTO. Возвращает id.
pub async fn upsert(dto: PluginUpsert) -> anyhow::Result<String> {
    dto.bundle
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    let status = dto
        .status
        .as_deref()
        .map(PluginStatus::from_str)
        .unwrap_or(PluginStatus::Draft);

    match dto.id {
        Some(id) => {
            let mut existing = repository::find_by_id(db(), &id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", id))?;

            if let Some(expected) = dto.version {
                if expected != existing.version {
                    return Err(anyhow::anyhow!(
                        "Version conflict for plugin {}: expected {}, actual {}",
                        id,
                        expected,
                        existing.version
                    ));
                }
            }

            existing.bundle = dto.bundle;
            existing.status = status;
            if let Some(enabled) = dto.is_enabled {
                existing.is_enabled = enabled;
            }
            if dto.owner_user_id.is_some() {
                existing.owner_user_id = dto.owner_user_id;
            }
            if dto.created_by_agent_id.is_some() {
                existing.created_by_agent_id = dto.created_by_agent_id;
            }
            existing.version += 1;
            existing.updated_at = Utc::now();

            repository::update(db(), &existing).await?;
            Ok(id)
        }
        None => {
            let id = Uuid::new_v4().to_string();
            let now = Utc::now();
            let def = PluginDefinition {
                id: id.clone(),
                bundle: dto.bundle,
                status,
                is_enabled: dto.is_enabled.unwrap_or(true),
                owner_user_id: dto.owner_user_id,
                created_by_agent_id: dto.created_by_agent_id,
                version: 1,
                created_at: now,
                updated_at: now,
            };
            repository::insert(db(), &def).await?;
            Ok(id)
        }
    }
}

pub async fn get_by_id(id: &str) -> anyhow::Result<Option<PluginDefinition>> {
    Ok(repository::find_by_id(db(), id).await?)
}

pub async fn list_all() -> anyhow::Result<Vec<PluginDefinition>> {
    Ok(repository::list_all(db()).await?)
}

pub async fn list_enabled() -> anyhow::Result<Vec<PluginDefinition>> {
    Ok(repository::list_enabled(db()).await?)
}

pub async fn delete(id: &str) -> anyhow::Result<()> {
    repository::soft_delete(db(), id).await?;
    Ok(())
}

/// Запустить `server_script` плагина (Rhai на сервере).
/// Возвращает (JSON-результат, строки вывода `print`).
pub async fn run_script(
    id: &str,
    ctx: &contracts::plugins::PluginRunContext,
) -> anyhow::Result<(serde_json::Value, Vec<String>)> {
    let def = repository::find_by_id(db(), id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", id))?;
    super::engine::run_server_script(def, ctx.clone()).await
}

// ============================================================================
// Данные плагина — декларативный путь через DataView (без Rhai)
// ============================================================================

/// Выполнить декларативную привязку данных плагина: по `data.view_id` строит
/// drilldown через `DataViewRegistry` и возвращает таблицу строк. Это путь для
/// `Client`-плагинов без скрипта (Фаза 1) и серверная основа для host.query.
pub async fn run_data(
    id: &str,
    ctx: &contracts::plugins::PluginRunContext,
) -> anyhow::Result<contracts::shared::drilldown::DrilldownResponse> {
    use crate::data_view::DataViewRegistry;
    use contracts::shared::data_view::ViewContext;

    let def = repository::find_by_id(db(), id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", id))?;

    let view_id = def
        .bundle
        .data
        .view_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Plugin {} has no data.view_id binding", id))?;

    let group_by = ctx
        .group_by
        .clone()
        .or_else(|| def.bundle.data.group_by.clone())
        .ok_or_else(|| anyhow::anyhow!("Plugin {} has no group_by", id))?;

    // Контекст: период + кабинеты + параметры формы + metric_id из привязки.
    let mut params = ctx.params.clone();
    if let Some(metric) = &def.bundle.data.metric_id {
        params.entry("metric".to_string()).or_insert(metric.clone());
    }

    let view_ctx = ViewContext {
        date_from: ctx.date_from.clone().unwrap_or_default(),
        date_to: ctx.date_to.clone().unwrap_or_default(),
        period2_from: None,
        period2_to: None,
        connection_mp_refs: ctx.connection_mp_refs.clone(),
        params,
    };

    let registry = DataViewRegistry::new();
    registry
        .compute_drilldown(&view_id, &view_ctx, &group_by, &[])
        .await
        .map_err(|e| anyhow::anyhow!("DataView '{}' drilldown error: {}", view_id, e))
}

// ============================================================================
// Тестовые данные — первый декларативный плагин-отчёт (без скриптов)
// ============================================================================

/// Вставить демонстрационный плагин: декларативный табличный отчёт по выручке,
/// использующий существующий DataView `dv001_revenue` (без Rhai).
pub async fn insert_test_data() -> anyhow::Result<()> {
    use sea_orm::{ConnectionTrait, DbBackend, Statement};

    const TEST_ID: &str = "9f1c0a00-0000-4000-8000-000000000001";
    const TEST_CODE: &str = "PLG-SALES-TABLE";

    let bundle = PluginBundle {
        manifest: PluginManifest {
            code: TEST_CODE.to_string(),
            title: "Отчёт по продажам (демо)".to_string(),
            runtime: PluginRuntime::Hybrid,
            api_version: "1".to_string(),
            description: Some(
                "Проверка концепции. Нажмите «Выполнить»: client_script вызывает call_server(), \
                 host исполняет server_script на бэкенде (host_read SQL → наименования подключений \
                 Яндекс.Маркета), результат возвращается в клиент и выводится в консоль через print(). \
                 Демонстрирует собственные функции (Rhai fn) на клиенте и сервере, вызов сервера из \
                 кода клиента и консоль — всё без пересборки проекта."
                    .to_string(),
            ),
            capabilities: vec!["data:read".to_string()],
        },
        params: vec![],
        data: DataBinding::default(),
        client_script: Some(
            r#"// КЛИЕНТ (Rhai в браузере).
// Платформенная функция: call_server("имя") — вызвать метод сервера.
let conns = call_server("get_yandex_connections");

// Своя клиентская функция форматирования строки:
fn bullet(i, name) { "  " + (i + 1) + ") " + name }

// Вывод в консоль:
print("Подключений Яндекс.Маркета: " + conns.len());
let out = [];
let i = 0;
for c in conns {
    print(bullet(i, c.name));
    out.push(#{ "Наименование": c.name });
    i += 1;
}
out"#
                .to_string(),
        ),
        server_script: Some(
            r#"// СЕРВЕР (Rhai на бэкенде). Платформенные функции: host_read / host_exec / host_query.
// Собственный метод сервера — пишется без пересборки проекта, вызывается с клиента
// через call_server("get_yandex_connections").
fn get_yandex_connections() {
    host_read(
        "SELECT c.description AS name " +
        "FROM a006_connection_mp c " +
        "JOIN a005_marketplace m ON m.id = c.marketplace " +
        "WHERE m.marketplace_type = 'Яндекс.Маркет' AND c.is_deleted = 0 " +
        "ORDER BY c.description"
    )
}"#
                .to_string(),
        ),
        view_spec: ViewSpec {
            widgets: vec![Widget {
                kind: WidgetKind::Table,
                title: Some("Подключения Яндекс.Маркета".to_string()),
                config: serde_json::json!({ "source": "connections" }),
            }],
            custom_html: None,
        },
        // Демо-плагин не несёт собственных стилей — используется оформление host.
        styles: None,
        assets: Default::default(),
    };

    let def = PluginDefinition {
        id: TEST_ID.to_string(),
        bundle,
        status: PluginStatus::Active,
        is_enabled: true,
        owner_user_id: None,
        created_by_agent_id: None,
        version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let b = &def.bundle;
    let manifest_json = serde_json::to_string(&b.manifest)?;
    let params_json = serde_json::to_string(&b.params)?;
    let data_json = serde_json::to_string(&b.data)?;
    let view_spec_json = serde_json::to_string(&b.view_spec)?;
    let assets_json = serde_json::to_string(&b.assets)?;
    let styles = b.styles.clone().unwrap_or_default();
    // NULL или экранированная строка-литерал для скриптов.
    let sql_opt = |o: &Option<String>| match o {
        Some(s) => format!("'{}'", s.replace('\'', "''")),
        None => "NULL".to_string(),
    };
    let client_sql = sql_opt(&b.client_script);
    let server_sql = sql_opt(&b.server_script);

    // Удаляем прежние демо-записи по коду (включая старый псевдо-UUID id).
    db().execute(Statement::from_string(
        DbBackend::Sqlite,
        format!("DELETE FROM plugin WHERE code = '{}'", TEST_CODE),
    ))
    .await
    .map_err(|e| anyhow::anyhow!("Failed to clear old demo plugin: {}", e))?;

    let sql = format!(
        "INSERT OR REPLACE INTO plugin \
         (id, code, title, runtime, status, manifest_json, params_json, data_json, \
          client_script, server_script, view_spec_json, styles, assets_json, \
          owner_user_id, created_by_agent_id, is_enabled, is_deleted, created_at, updated_at, version) \
         VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', {}, {}, '{}', '{}', '{}', \
          NULL, NULL, 1, 0, datetime('now'), datetime('now'), 1)",
        def.id,
        b.manifest.code,
        b.manifest.title,
        b.manifest.runtime.as_str(),
        def.status.as_str(),
        manifest_json.replace('\'', "''"),
        params_json.replace('\'', "''"),
        data_json.replace('\'', "''"),
        client_sql,
        server_sql,
        view_spec_json.replace('\'', "''"),
        styles.replace('\'', "''"),
        assets_json.replace('\'', "''"),
    );

    db().execute(Statement::from_string(DbBackend::Sqlite, sql))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to insert test plugin: {}", e))?;

    Ok(())
}
