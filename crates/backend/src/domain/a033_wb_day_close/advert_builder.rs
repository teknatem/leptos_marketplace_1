/// Строитель рекламных снапшотов для a033_wb_day_close.
///
/// Загружает строки p911 (advert_clicks_no_order) и p913 (advert_clicks_order_accrual)
/// за (connection_id, business_date), обогащает sa_name из a004 и order_id/order_date из a015.
use anyhow::Result;
use contracts::domain::a033_wb_day_close::{
    WbDayCloseAdvertNoOrderLine, WbDayCloseAdvertOrderAccrualLine,
};
use sea_orm::{ConnectionTrait, Statement, Value};
use std::collections::HashMap;

use crate::shared::data::db::get_connection;

fn conn() -> &'static sea_orm::DatabaseConnection {
    get_connection()
}

fn sv(value: impl Into<String>) -> Value {
    Value::String(Some(Box::new(value.into())))
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

pub struct AdvertBuildResult {
    pub no_order_lines: Vec<WbDayCloseAdvertNoOrderLine>,
    pub order_accrual_lines: Vec<WbDayCloseAdvertOrderAccrualLine>,
    /// GL-итоги из sys_general_ledger (то, что реально проведено по бухучёту)
    pub gl_no_order: f64,
    pub gl_order_accrual: f64,
    pub gl_order_expense: f64,
    /// Итог по p913 (advert_clicks_order_expense) за эту дату — снапшот проекции
    pub snap_order_expense: f64,
}

pub async fn build(connection_id: &str, business_date: &str) -> Result<AdvertBuildResult> {
    let (
        no_order_raw,
        order_accrual_raw,
        snap_order_expense,
        (gl_no_order, gl_order_accrual, gl_order_expense),
    ) = tokio::try_join!(
        fetch_p911(connection_id, business_date),
        fetch_p913_accrual(connection_id, business_date),
        fetch_p913_expense_total(connection_id, business_date),
        fetch_sys_gl_advert_totals(connection_id, business_date),
    )?;

    // Собираем все nomenclature_ref для batch-запроса sa_name
    let mut nref_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for r in &no_order_raw {
        if let Some(nref) = &r.nomenclature_ref {
            nref_set.insert(nref.clone());
        }
    }
    for r in &order_accrual_raw {
        if let Some(nref) = &r.nomenclature_ref {
            nref_set.insert(nref.clone());
        }
    }
    let sa_names = fetch_sa_names(&nref_set.into_iter().collect::<Vec<_>>()).await?;

    // Собираем все order_key из p913 для batch-resolve a015
    let order_keys: Vec<String> = order_accrual_raw
        .iter()
        .map(|r| r.order_key.clone())
        .filter(|k| !k.is_empty())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let order_map = fetch_a015_for_order_keys(&order_keys).await?;

    // Собираем все campaign_code из обоих источников для batch-resolve a030
    let campaign_codes: Vec<String> = no_order_raw
        .iter()
        .map(|r| r.campaign_code.clone())
        .chain(order_accrual_raw.iter().map(|r| r.campaign_code.clone()))
        .filter(|c| !c.is_empty())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let campaign_map = fetch_a030_by_campaign_codes(connection_id, &campaign_codes).await?;

    // Строим итоговые векторы
    let no_order_lines: Vec<WbDayCloseAdvertNoOrderLine> = no_order_raw
        .into_iter()
        .map(|r| {
            let sa_name = r
                .nomenclature_ref
                .as_deref()
                .and_then(|nref| sa_names.get(nref))
                .cloned();
            let campaign_ref = campaign_map.get(&r.campaign_code).cloned();
            WbDayCloseAdvertNoOrderLine {
                projection_ref_id: r.id,
                nomenclature_ref: r.nomenclature_ref,
                sa_name,
                amount: r.amount,
                general_ledger_ref: r.general_ledger_ref,
                campaign_code: r.campaign_code,
                campaign_ref,
            }
        })
        .collect();

    let order_accrual_lines: Vec<WbDayCloseAdvertOrderAccrualLine> = order_accrual_raw
        .into_iter()
        .map(|r| {
            let sa_name = r
                .nomenclature_ref
                .as_deref()
                .and_then(|nref| sa_names.get(nref))
                .cloned();
            let order_info = order_map.get(&r.order_key);
            let campaign_ref = campaign_map.get(&r.campaign_code).cloned();
            WbDayCloseAdvertOrderAccrualLine {
                projection_ref_id: r.id,
                nomenclature_ref: r.nomenclature_ref,
                sa_name,
                amount: r.amount,
                order_key: r.order_key,
                order_id: order_info.map(|(id, _)| id.clone()),
                order_date: order_info.and_then(|(_, date)| {
                    if date.is_empty() {
                        None
                    } else {
                        Some(date.clone())
                    }
                }),
                campaign_code: r.campaign_code,
                campaign_ref,
            }
        })
        .collect();

    Ok(AdvertBuildResult {
        no_order_lines,
        order_accrual_lines,
        gl_no_order,
        gl_order_accrual,
        gl_order_expense,
        snap_order_expense,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Raw rows
// ─────────────────────────────────────────────────────────────────────────────

struct P911Raw {
    id: String,
    nomenclature_ref: Option<String>,
    amount: f64,
    general_ledger_ref: Option<String>,
    campaign_code: String,
}

struct P913Raw {
    id: String,
    nomenclature_ref: Option<String>,
    amount: f64,
    order_key: String,
    campaign_code: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Fetchers
// ─────────────────────────────────────────────────────────────────────────────

async fn fetch_p911(connection_id: &str, business_date: &str) -> Result<Vec<P911Raw>> {
    let exists =
        crate::projections::p911_wb_advert_by_items::repository::sql_a026_document_exists("p");
    let sql = format!(
        r#"
        SELECT p.id, p.nomenclature_ref, p.amount, p.general_ledger_ref, p.wb_advert_campaign_code
        FROM p911_wb_advert_by_items p
        WHERE p.connection_mp_ref = ?
          AND p.entry_date = ?
          AND p.turnover_code = 'advert_clicks_no_order'
          AND p.registrator_type = 'a026_wb_advert_daily'
          AND {exists}
        ORDER BY p.amount DESC, p.id
    "#
    );
    let stmt = Statement::from_sql_and_values(
        conn().get_database_backend(),
        sql,
        vec![sv(connection_id), sv(business_date)],
    );
    let rows = conn().query_all(stmt).await?;
    Ok(rows
        .into_iter()
        .map(|row| P911Raw {
            id: row.try_get("", "id").unwrap_or_default(),
            nomenclature_ref: row
                .try_get("", "nomenclature_ref")
                .ok()
                .filter(|s: &String| !s.is_empty()),
            amount: row.try_get("", "amount").unwrap_or(0.0),
            general_ledger_ref: row
                .try_get("", "general_ledger_ref")
                .ok()
                .filter(|s: &String| !s.is_empty()),
            campaign_code: row
                .try_get("", "wb_advert_campaign_code")
                .unwrap_or_default(),
        })
        .collect())
}

async fn fetch_p913_accrual(connection_id: &str, business_date: &str) -> Result<Vec<P913Raw>> {
    let sql = r#"
        SELECT p.id, p.nomenclature_ref, p.amount, p.order_key, p.wb_advert_campaign_code
        FROM p913_wb_advert_order_attr p
        INNER JOIN a026_wb_advert_daily a ON a.id = p.registrator_ref
        WHERE p.connection_mp_ref = ?
          AND p.entry_date = ?
          AND p.turnover_code = 'advert_clicks_order_accrual'
          AND p.registrator_type = 'a026_wb_advert_daily'
        ORDER BY p.amount DESC, p.id
    "#;
    let stmt = Statement::from_sql_and_values(
        conn().get_database_backend(),
        sql,
        vec![sv(connection_id), sv(business_date)],
    );
    let rows = conn().query_all(stmt).await?;
    Ok(rows
        .into_iter()
        .map(|row| P913Raw {
            id: row.try_get("", "id").unwrap_or_default(),
            nomenclature_ref: row
                .try_get("", "nomenclature_ref")
                .ok()
                .filter(|s: &String| !s.is_empty()),
            amount: row.try_get("", "amount").unwrap_or(0.0),
            order_key: row.try_get("", "order_key").unwrap_or_default(),
            campaign_code: row
                .try_get("", "wb_advert_campaign_code")
                .unwrap_or_default(),
        })
        .collect())
}

/// GL-итоги по трём рекламным оборотам из sys_general_ledger (layer=oper).
/// Возвращает (no_order, order_accrual, order_expense).
async fn fetch_sys_gl_advert_totals(
    connection_id: &str,
    business_date: &str,
) -> Result<(f64, f64, f64)> {
    let sql = r#"
        SELECT
            turnover_code,
            COALESCE(SUM(amount), 0.0) AS total
        FROM sys_general_ledger
        WHERE connection_mp_ref = ?
          AND entry_date = ?
          AND layer = 'oper'
          AND turnover_code IN (
              'advert_clicks_no_order',
              'advert_clicks_order_accrual',
              'advert_clicks_order_expense'
          )
        GROUP BY turnover_code
    "#;
    let stmt = Statement::from_sql_and_values(
        conn().get_database_backend(),
        sql,
        vec![sv(connection_id), sv(business_date)],
    );
    let rows = conn().query_all(stmt).await?;
    let mut no_order = 0.0_f64;
    let mut order_accrual = 0.0_f64;
    let mut order_expense = 0.0_f64;
    for row in rows {
        let code: String = row.try_get("", "turnover_code").unwrap_or_default();
        let total: f64 = row.try_get("", "total").unwrap_or(0.0);
        match code.as_str() {
            "advert_clicks_no_order" => no_order = total,
            "advert_clicks_order_accrual" => order_accrual = total,
            "advert_clicks_order_expense" => order_expense = total,
            _ => {}
        }
    }
    Ok((no_order, order_accrual, order_expense))
}

/// Суммарный итог advert_clicks_order_expense из p913 за business_date.
/// Дата берётся из GL (дата реализации a012), а не из p913.entry_date (legacy: дата reserve a026).
async fn fetch_p913_expense_total(connection_id: &str, business_date: &str) -> Result<f64> {
    let sql = r#"
        SELECT COALESCE(SUM(p.amount), 0.0) AS total
        FROM p913_wb_advert_order_attr p
        INNER JOIN sys_general_ledger g ON g.id = p.general_ledger_ref
        WHERE p.connection_mp_ref = ?
          AND g.entry_date = ?
          AND g.layer = 'oper'
          AND g.turnover_code = 'advert_clicks_order_expense'
          AND p.turnover_code = 'advert_clicks_order_expense'
    "#;
    let stmt = Statement::from_sql_and_values(
        conn().get_database_backend(),
        sql,
        vec![sv(connection_id), sv(business_date)],
    );
    let rows = conn().query_all(stmt).await?;
    Ok(rows
        .first()
        .and_then(|row| row.try_get::<f64>("", "total").ok())
        .unwrap_or(0.0))
}

/// Batch-resolve campaign_code (числовой advert_id) → UUID записи a030_wb_advert_campaign.
async fn fetch_a030_by_campaign_codes(
    connection_id: &str,
    codes: &[String],
) -> Result<HashMap<String, String>> {
    if codes.is_empty() {
        return Ok(HashMap::new());
    }
    let mut map = HashMap::with_capacity(codes.len());
    const CHUNK: usize = 400;
    for chunk in codes.chunks(CHUNK) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT id, CAST(advert_id AS TEXT) AS advert_code \
             FROM a030_wb_advert_campaign \
             WHERE connection_id = ? AND CAST(advert_id AS TEXT) IN ({placeholders}) AND is_deleted = 0"
        );
        let mut params: Vec<Value> = vec![sv(connection_id)];
        params.extend(chunk.iter().map(|s| sv(s)));
        let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
        let rows = conn().query_all(stmt).await?;
        for row in rows {
            let uuid: String = row.try_get("", "id").unwrap_or_default();
            let code: String = row.try_get("", "advert_code").unwrap_or_default();
            if !uuid.is_empty() && !code.is_empty() {
                map.insert(code, uuid);
            }
        }
    }
    Ok(map)
}

/// Batch-загрузка article (= sa_name) из a004_nomenclature по списку id.
async fn fetch_sa_names(nrefs: &[String]) -> Result<HashMap<String, String>> {
    if nrefs.is_empty() {
        return Ok(HashMap::new());
    }
    let mut map = HashMap::with_capacity(nrefs.len());
    const CHUNK: usize = 400;
    for chunk in nrefs.chunks(CHUNK) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT id, article FROM a004_nomenclature WHERE id IN ({placeholders}) AND is_deleted = 0"
        );
        let params: Vec<Value> = chunk.iter().map(|s| sv(s)).collect();
        let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
        let rows = conn().query_all(stmt).await?;
        for row in rows {
            let id: String = row.try_get("", "id").unwrap_or_default();
            let article: String = row.try_get("", "article").unwrap_or_default();
            if !id.is_empty() {
                map.insert(id, article);
            }
        }
    }
    Ok(map)
}

/// Batch-resolve order_key → (a015.id, order_date).
async fn fetch_a015_for_order_keys(
    order_keys: &[String],
) -> Result<HashMap<String, (String, String)>> {
    if order_keys.is_empty() {
        return Ok(HashMap::new());
    }
    let mut map = HashMap::with_capacity(order_keys.len());
    const CHUNK: usize = 400;
    for chunk in order_keys.chunks(CHUNK) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            r#"
            SELECT
                id,
                document_no AS order_key,
                SUBSTR(COALESCE(json_extract(state_json, '$.order_dt'), ''), 1, 10) AS order_date
            FROM a015_wb_orders
            WHERE document_no IN ({placeholders})
              AND is_deleted = 0
            "#
        );
        let params: Vec<Value> = chunk.iter().map(|s| sv(s)).collect();
        let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
        let rows = conn().query_all(stmt).await?;
        for row in rows {
            let key: String = row.try_get("", "order_key").unwrap_or_default();
            if key.is_empty() {
                continue;
            }
            let id: String = row.try_get("", "id").unwrap_or_default();
            let date: String = row.try_get("", "order_date").unwrap_or_default();
            map.insert(key, (id, date));
        }
    }
    Ok(map)
}

// ─────────────────────────────────────────────────────────────────────────────
// Diagnostic: breakdown advert_clicks_order_accrual по registrator_ref
// ─────────────────────────────────────────────────────────────────────────────

pub struct RegistratorDiagRow {
    pub registrator_ref: String,
    pub p913_sum: f64,
    pub p913_rows: u64,
    pub gl_sum: f64,
}

pub struct AccrualDiagnostic {
    pub total_rows: u64,
    pub gl_entries: u64,
    pub per_registrator: Vec<RegistratorDiagRow>,
}

/// Поразрядная диагностика advert_clicks_order_accrual:
/// сравниваем p913 vs sys_general_ledger на уровне отдельных a026-документов.
pub async fn fetch_accrual_diagnostic(
    connection_id: &str,
    business_date: &str,
) -> Result<AccrualDiagnostic> {
    // p913: только строки с живым a026-документом (без «осиротевших» registrator_ref)
    let sql_p913 = r#"
        SELECT
            p.registrator_ref,
            p.registrator_type,
            COALESCE(SUM(p.amount), 0.0) AS p913_sum,
            COUNT(*) AS p913_rows
        FROM p913_wb_advert_order_attr p
        INNER JOIN a026_wb_advert_daily a ON a.id = p.registrator_ref
        WHERE p.connection_mp_ref = ?
          AND p.entry_date = ?
          AND p.turnover_code = 'advert_clicks_order_accrual'
          AND p.registrator_type = 'a026_wb_advert_daily'
        GROUP BY p.registrator_ref, p.registrator_type
        ORDER BY p913_sum DESC
    "#;
    let stmt = Statement::from_sql_and_values(
        conn().get_database_backend(),
        sql_p913,
        vec![sv(connection_id), sv(business_date)],
    );
    let p913_rows_raw = conn().query_all(stmt).await?;

    let total_rows: u64 = p913_rows_raw
        .iter()
        .map(|r| r.try_get::<i64>("", "p913_rows").unwrap_or(0) as u64)
        .sum();

    // GL: группируем по registrator_ref
    let sql_gl = r#"
        SELECT
            registrator_ref,
            COALESCE(SUM(amount), 0.0) AS gl_sum,
            COUNT(*) AS gl_entries
        FROM sys_general_ledger
        WHERE connection_mp_ref = ?
          AND entry_date = ?
          AND layer = 'oper'
          AND turnover_code = 'advert_clicks_order_accrual'
        GROUP BY registrator_ref
    "#;
    let stmt_gl = Statement::from_sql_and_values(
        conn().get_database_backend(),
        sql_gl,
        vec![sv(connection_id), sv(business_date)],
    );
    let gl_rows_raw = conn().query_all(stmt_gl).await?;

    let gl_entries: u64 = gl_rows_raw
        .iter()
        .map(|r| r.try_get::<i64>("", "gl_entries").unwrap_or(0) as u64)
        .sum();

    // Map GL по registrator_ref
    let gl_map: HashMap<String, f64> = gl_rows_raw
        .into_iter()
        .filter_map(|row| {
            let rref: String = row.try_get("", "registrator_ref").ok()?;
            let sum: f64 = row.try_get("", "gl_sum").unwrap_or(0.0);
            Some((rref, sum))
        })
        .collect();

    // Соединяем p913 и GL по registrator_ref
    let mut per_registrator: Vec<RegistratorDiagRow> = p913_rows_raw
        .into_iter()
        .map(|row| {
            let rref: String = row.try_get("", "registrator_ref").unwrap_or_default();
            let p913_sum: f64 = row.try_get("", "p913_sum").unwrap_or(0.0);
            let rows: i64 = row.try_get("", "p913_rows").unwrap_or(0);
            let gl_sum = *gl_map.get(&rref).unwrap_or(&0.0);
            RegistratorDiagRow {
                registrator_ref: rref,
                p913_sum,
                p913_rows: rows as u64,
                gl_sum,
            }
        })
        .collect();

    // Добавляем GL-регистраторы которых нет в p913
    for (rref, gl_sum) in &gl_map {
        if !per_registrator.iter().any(|r| &r.registrator_ref == rref) {
            per_registrator.push(RegistratorDiagRow {
                registrator_ref: rref.clone(),
                p913_sum: 0.0,
                p913_rows: 0,
                gl_sum: *gl_sum,
            });
        }
    }

    per_registrator.sort_by(|a, b| {
        b.p913_sum
            .partial_cmp(&a.p913_sum)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(AccrualDiagnostic {
        total_rows,
        gl_entries,
        per_registrator,
    })
}
