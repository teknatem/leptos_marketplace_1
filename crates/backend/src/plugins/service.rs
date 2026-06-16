//! Business logic for plugin CRUD, invocation, validation, and demo data.

use super::repository;
use chrono::Utc;
use contracts::plugins::{
    DataBinding, PluginBundle, PluginDefinition, PluginError, PluginInvokeRequest, PluginManifest,
    PluginRuntime, PluginStatus, PluginUpsert, PluginValidateReport, ViewSpec,
};
use uuid::Uuid;

fn db() -> &'static sea_orm::DatabaseConnection {
    crate::shared::data::db::get_connection()
}

/// Проверить бандл без сохранения: статические правила + компиляция серверного
/// ES-модуля с перечнем экспортов (без вызова функций). `client_script` пока не
/// компилируется (исполняется в браузере) — TODO Plan 3.
pub async fn validate(bundle: &PluginBundle) -> PluginValidateReport {
    if let Err(message) = bundle.validate() {
        return PluginValidateReport {
            ok: false,
            server_exports: vec![],
            errors: vec![PluginError::new("manifest", message)],
        };
    }
    match &bundle.server_script {
        Some(script) => super::engine::validate_server_script(script).await,
        None => PluginValidateReport {
            ok: true,
            ..Default::default()
        },
    }
}

pub async fn upsert(dto: PluginUpsert) -> anyhow::Result<String> {
    dto.bundle
        .validate()
        .map_err(|error| anyhow::anyhow!("Validation failed: {error}"))?;

    let status = dto
        .status
        .as_deref()
        .map(PluginStatus::from_str)
        .unwrap_or(PluginStatus::Draft);

    match dto.id {
        Some(id) => {
            let mut existing = repository::find_by_id(db(), &id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Plugin not found: {id}"))?;

            if let Some(expected) = dto.version {
                if expected != existing.version {
                    return Err(anyhow::anyhow!(
                        "Version conflict for plugin {id}: expected {expected}, actual {}",
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

/// Найти плагин по бизнес-коду (`manifest.code`). Ключ переносимости и upsert-by-code.
pub async fn get_by_code(code: &str) -> anyhow::Result<Option<PluginDefinition>> {
    Ok(repository::find_by_code(db(), code).await?)
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

pub async fn invoke(
    id: &str,
    request: PluginInvokeRequest,
) -> anyhow::Result<(serde_json::Value, Vec<String>)> {
    let def = repository::find_by_id(db(), id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Plugin not found: {id}"))?;
    if !def.is_enabled || def.status != PluginStatus::Active {
        return Err(anyhow::anyhow!("Plugin is not active"));
    }

    let code = def.bundle.manifest.code.clone();
    let method = request.method.clone();
    let started = std::time::Instant::now();
    let result = super::engine::invoke_server_method(def, request).await;
    let duration_ms = started.elapsed().as_millis() as i64;

    // Зафиксировать запуск для наблюдаемости (Plan 4).
    let (status, error_stage, row_count) = match &result {
        Ok((value, _logs)) => ("ok", None, value.as_array().map(|a| a.len() as i64)),
        Err(error) => {
            let stage = error
                .downcast_ref::<PluginError>()
                .map(|e| e.stage.clone());
            let status = if stage.as_deref() == Some("timeout") {
                "timeout"
            } else {
                "error"
            };
            (status, stage, None)
        }
    };
    super::runs::record(
        id,
        &code,
        &method,
        duration_ms,
        status,
        error_stage.as_deref(),
        row_count,
        None,
    )
    .await;

    result
}

/// Статистика запусков плагина (сводка + последние запуски) за окно `days` дней.
pub async fn stats(id: &str, days: i64) -> anyhow::Result<contracts::plugins::PluginStats> {
    super::runs::stats(id, days, 50).await
}

/// Краткие сводки по всем плагинам за окно `days` дней (для реестра).
pub async fn runs_summary(days: i64) -> anyhow::Result<Vec<contracts::plugins::PluginRunBrief>> {
    super::runs::summary_all(days).await
}

// ============================================================================
// Переносимый файловый формат (export / import)
// ============================================================================

/// Экспортировать плагин как zip-архив переносимого бандла. Возвращает (имя файла, байты).
pub async fn export(id: &str) -> anyhow::Result<(String, Vec<u8>)> {
    let def = repository::find_by_id(db(), id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Plugin not found: {id}"))?;
    let bytes = super::package::export_bundle(&def.bundle)?;
    let filename = super::package::archive_filename(&def.bundle.manifest.code);
    Ok((filename, bytes))
}

/// Результат импорта: id сохранённого плагина (None если валидация не пройдена),
/// его code и отчёт валидации.
pub struct ImportOutcome {
    pub id: Option<String>,
    pub code: String,
    pub report: contracts::plugins::PluginValidateReport,
}

/// Импортировать плагин из zip-архива. **Атомарно**: bundle собирается целиком,
/// валидируется (Plan 1), и лишь при `ok` сохраняется одним upsert по `code`.
/// Идентичность — `manifest.code`: существующий плагин обновляется (статус/включённость
/// сохраняются), новый создаётся как `draft` и выключенным (админ включает вручную).
pub async fn import(bytes: &[u8]) -> anyhow::Result<ImportOutcome> {
    let bundle = super::package::import_archive(bytes)?;
    let code = bundle.manifest.code.clone();

    let report = validate(&bundle).await;
    if !report.ok {
        return Ok(ImportOutcome {
            id: None,
            code,
            report,
        });
    }

    let existing = get_by_code(&code).await?;
    let dto = match &existing {
        Some(current) => PluginUpsert {
            id: Some(current.id.clone()),
            bundle,
            status: Some(current.status.as_str().to_string()),
            is_enabled: Some(current.is_enabled),
            owner_user_id: None,
            created_by_agent_id: None,
            version: Some(current.version),
        },
        None => PluginUpsert {
            id: None,
            bundle,
            status: Some(PluginStatus::Draft.as_str().to_string()),
            is_enabled: Some(false),
            owner_user_id: None,
            created_by_agent_id: None,
            version: None,
        },
    };

    let id = upsert(dto).await?;
    Ok(ImportOutcome {
        id: Some(id),
        code,
        report,
    })
}

// ============================================================================
// Declarative DataView path
// ============================================================================

pub async fn run_data(
    id: &str,
    ctx: &contracts::plugins::PluginRunContext,
) -> anyhow::Result<contracts::shared::drilldown::DrilldownResponse> {
    use crate::data_view::DataViewRegistry;
    use contracts::shared::data_view::ViewContext;

    let def = repository::find_by_id(db(), id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Plugin not found: {id}"))?;

    let view_id = def
        .bundle
        .data
        .view_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Plugin {id} has no data.view_id binding"))?;
    let group_by = ctx
        .group_by
        .clone()
        .or_else(|| def.bundle.data.group_by.clone())
        .ok_or_else(|| anyhow::anyhow!("Plugin {id} has no group_by"))?;

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

    DataViewRegistry::new()
        .compute_drilldown(&view_id, &view_ctx, &group_by, &[])
        .await
        .map_err(|error| anyhow::anyhow!("DataView '{view_id}' drilldown error: {error}"))
}

// ============================================================================
// Demo JavaScript micro-application
// ============================================================================

const DEMO_CLIENT_SCRIPT: &str = r##"
function isoDate(date) {
  return date.toISOString().slice(0, 10);
}

function cell(value) {
  const td = document.createElement("td");
  td.textContent = value == null || value === "" ? "-" : String(value);
  return td;
}

export async function mount(root, host) {
  const today = new Date();
  const from = new Date(today);
  from.setDate(from.getDate() - 30);

  root.innerHTML = `
    <main class="report">
      <header class="report__header">
        <div>
          <h1>Заказанные позиции WB без номенклатуры 1С</h1>
          <p>Позиции из заказов Wildberries, для которых отсутствует действующая связь с a004_nomenclature.</p>
        </div>
      </header>
      <section class="filters">
        <label>Дата с <input id="date-from" type="date"></label>
        <label>Дата по <input id="date-to" type="date"></label>
        <button id="refresh" type="button">Обновить</button>
      </section>
      <section class="summary">
        <span>Несопоставленных позиций</span>
        <strong id="total">-</strong>
      </section>
      <div id="status" class="status"></div>
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>Кабинет</th>
              <th>Артикул</th>
              <th>SKU WB</th>
              <th>Наименование</th>
              <th class="number">Заказов</th>
              <th class="number">Количество</th>
              <th>Первый заказ</th>
              <th>Последний заказ</th>
            </tr>
          </thead>
          <tbody id="rows"></tbody>
        </table>
      </div>
    </main>`;

  const dateFrom = root.querySelector("#date-from");
  const dateTo = root.querySelector("#date-to");
  const refresh = root.querySelector("#refresh");
  const status = root.querySelector("#status");
  const total = root.querySelector("#total");
  const tbody = root.querySelector("#rows");
  dateFrom.value = isoDate(from);
  dateTo.value = isoDate(today);

  async function load() {
    refresh.disabled = true;
    status.className = "status";
    status.textContent = "Загрузка...";
    tbody.replaceChildren();

    try {
      const rows = await host.invoke("loadReport", {
        dateFrom: dateFrom.value,
        dateTo: dateTo.value
      });
      total.textContent = rows.length;

      if (rows.length === 0) {
        status.className = "status status--ok";
        status.textContent = "За выбранный период все заказанные позиции сопоставлены.";
        return;
      }

      status.textContent = `Найдено позиций: ${rows.length}`;
      for (const row of rows) {
        const tr = document.createElement("tr");
        tr.append(
          cell(row.connection_name),
          cell(row.article),
          cell(row.marketplace_sku),
          cell(row.product_name),
          cell(row.order_count),
          cell(row.ordered_qty),
          cell(row.first_order_date),
          cell(row.last_order_date)
        );
        tr.children[4].className = "number";
        tr.children[5].className = "number";
        tbody.append(tr);
      }
    } catch (error) {
      total.textContent = "-";
      status.className = "status status--error";
      status.textContent = error instanceof Error ? error.message : String(error);
    } finally {
      refresh.disabled = false;
    }
  }

  refresh.addEventListener("click", load);
  await load();
}
"##;

const DEMO_SERVER_SCRIPT: &str = r#"
export async function loadReport(args, host) {
  if (!args.dateFrom || !args.dateTo) {
    throw new Error("Необходимо указать обе границы периода");
  }

  host.log.info("Loading unmapped WB order positions", args.dateFrom, args.dateTo);
  return await host.db.queryResource(
    "unmappedOrderPositions",
    [args.dateFrom, args.dateTo]
  );
}
"#;

const DEMO_REPORT_SQL: &str = r#"
SELECT
  COALESCE(c.description, p.connection_mp_ref) AS connection_name,
  COALESCE(NULLIF(mp.article, ''), p.line_key) AS article,
  COALESCE(NULLIF(mp.marketplace_sku, ''), p.line_key) AS marketplace_sku,
  COALESCE(NULLIF(mp.description, ''), '(карточка товара не найдена)') AS product_name,
  COUNT(DISTINCT p.order_key) AS order_count,
  ROUND(SUM(p.amount), 3) AS ordered_qty,
  MIN(p.entry_date) AS first_order_date,
  MAX(p.entry_date) AS last_order_date
FROM p909_mp_order_line_turnovers p
LEFT JOIN a007_marketplace_product mp
  ON mp.id = p.marketplace_product_ref
 AND mp.is_deleted = 0
LEFT JOIN a004_nomenclature n
  ON n.id = mp.nomenclature_ref
 AND n.is_deleted = 0
LEFT JOIN a006_connection_mp c
  ON c.id = p.connection_mp_ref
 AND c.is_deleted = 0
WHERE p.turnover_code = 'qty_ordered'
  AND p.layer = 'oper'
  AND c.marketplace = 'WB'
  AND p.entry_date BETWEEN ? AND ?
  AND n.id IS NULL
GROUP BY
  p.connection_mp_ref,
  COALESCE(NULLIF(p.marketplace_product_ref, ''), p.line_key),
  mp.article,
  mp.marketplace_sku,
  mp.description
ORDER BY order_count DESC, article
LIMIT 1000
"#;

const DEMO_STYLES: &str = r#"
:root {
  color-scheme: light dark;
  font-family: Inter, "Segoe UI", sans-serif;
  color: #1f2937;
  background: #f6f7fb;
}
* { box-sizing: border-box; }
body { margin: 0; }
.report { padding: 24px; }
.report__header h1 { margin: 0; font-size: 24px; }
.report__header p { margin: 8px 0 0; color: #667085; }
.filters {
  display: flex;
  align-items: end;
  gap: 12px;
  flex-wrap: wrap;
  margin: 22px 0 16px;
}
.filters label {
  display: grid;
  gap: 6px;
  color: #475467;
  font-size: 12px;
  font-weight: 600;
}
.filters input, .filters button {
  height: 38px;
  border-radius: 8px;
  border: 1px solid #d0d5dd;
  padding: 0 12px;
  font: inherit;
}
.filters button {
  border-color: #2563eb;
  background: #2563eb;
  color: white;
  font-weight: 600;
  cursor: pointer;
}
.filters button:disabled { opacity: .55; cursor: default; }
.summary {
  display: inline-flex;
  align-items: baseline;
  gap: 12px;
  padding: 14px 18px;
  border: 1px solid #e4e7ec;
  border-radius: 12px;
  background: white;
}
.summary span { color: #667085; font-size: 13px; }
.summary strong { font-size: 24px; color: #b42318; }
.status { min-height: 42px; padding: 13px 0; color: #667085; }
.status--ok { color: #067647; }
.status--error { color: #b42318; }
.table-wrap {
  overflow: auto;
  border: 1px solid #e4e7ec;
  border-radius: 12px;
  background: white;
}
table { width: 100%; border-collapse: collapse; font-size: 13px; }
th {
  position: sticky;
  top: 0;
  text-align: left;
  padding: 11px 12px;
  background: #f9fafb;
  color: #475467;
  border-bottom: 1px solid #e4e7ec;
  white-space: nowrap;
}
td { padding: 10px 12px; border-bottom: 1px solid #f0f1f3; }
tr:last-child td { border-bottom: 0; }
.number { text-align: right; font-variant-numeric: tabular-nums; }
@media (prefers-color-scheme: dark) {
  :root { color: #f2f4f7; background: #111827; }
  .report__header p, .filters label, .status, .summary span { color: #98a2b3; }
  .filters input, .summary, .table-wrap { background: #1f2937; border-color: #344054; color: #f2f4f7; }
  th { background: #182230; color: #d0d5dd; border-color: #344054; }
  td { border-color: #273548; }
}
"#;

pub async fn insert_test_data() -> anyhow::Result<()> {
    use sea_orm::{ConnectionTrait, DbBackend, Statement};

    const TEST_ID: &str = "9f1c0a00-0000-4000-8000-000000000001";
    const TEST_CODE: &str = "PLG-WB-UNMAPPED-ORDERS";

    db().execute(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "DELETE FROM plugin WHERE code = ? OR id = ?",
        vec![TEST_CODE.into(), TEST_ID.into()],
    ))
    .await?;

    let now = Utc::now();
    let def = PluginDefinition {
        id: TEST_ID.to_string(),
        bundle: PluginBundle {
            manifest: PluginManifest {
                code: TEST_CODE.to_string(),
                title: "Заказанные позиции WB без номенклатуры 1С".to_string(),
                runtime: PluginRuntime::Hybrid,
                api_version: "2".to_string(),
                description: Some(
                    "Демонстрационное JS-микроприложение: клиентский ES-модуль строит интерфейс \
                     и вызывает серверный ES-модуль через host.invoke()."
                        .to_string(),
                ),
                capabilities: vec!["data:read".to_string()],
            },
            params: vec![],
            data: DataBinding::default(),
            client_script: Some(DEMO_CLIENT_SCRIPT.to_string()),
            server_script: Some(DEMO_SERVER_SCRIPT.to_string()),
            view_spec: ViewSpec::default(),
            styles: Some(DEMO_STYLES.to_string()),
            sql_resources: [(
                "unmappedOrderPositions".to_string(),
                DEMO_REPORT_SQL.trim().to_string(),
            )]
            .into_iter()
            .collect(),
            assets: Default::default(),
        },
        status: PluginStatus::Active,
        is_enabled: true,
        owner_user_id: None,
        created_by_agent_id: None,
        version: 1,
        created_at: now,
        updated_at: now,
    };
    repository::insert(db(), &def).await?;
    Ok(())
}
