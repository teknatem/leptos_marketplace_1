/// Строитель строк для a033_wb_day_close.
///
/// Шаги:
///   1. Агрегация p903_wb_finance_report по (srid, nm_id, nomenclature_ref, sa_name, supplier_oper_name)
///      Включает строки БЕЗ srid (хранение, штрафы, возмещение ПВЗ, приёмка).
///   2. p913 — reserve по order_key; expense в колонке «Реклама» = advert_clicks_order_expense matched a012
///   3. a012_wb_sales — по srid из p903, sale_date<=business_date+1д (лаг WB), знак суммы; fallback p912
///   4. a015_wb_orders — дата и статус отмены заказа
///   5. In-memory join, классификация LineKind/LineDetail, вычисление 10 колонок
use anyhow::Result;
use chrono::{Days, NaiveDate};
use contracts::domain::a033_wb_day_close::aggregate::{
    LineDetail, LineKind, ProblemSeverity, SaleEvent, WbDayCloseLine, WbDayCloseProblem,
};
use sea_orm::{ConnectionTrait, Statement, Value};
use std::collections::{HashMap, HashSet};

/// sale_date в a012 может быть на сутки позже `rr_dt` в p903 (особенность WB).
pub(crate) const A012_SALE_DATE_LAG_DAYS: i64 = 1;

/// Верхняя граница `sale_date` (YYYY-MM-DD) для привязки a012 к закрытию дня `business_date`.
pub(crate) fn a012_sale_date_upper_bound(business_date: &str) -> String {
    NaiveDate::parse_from_str(business_date, "%Y-%m-%d")
        .ok()
        .and_then(|d| {
            d.checked_add_days(Days::new(A012_SALE_DATE_LAG_DAYS.try_into().unwrap_or(1)))
        })
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| business_date.to_string())
}

use crate::shared::data::db::get_connection;

fn conn() -> &'static sea_orm::DatabaseConnection {
    get_connection()
}

fn sv(value: impl Into<String>) -> Value {
    Value::String(Some(Box::new(value.into())))
}

// ─────────────────────────────────────────────────────────────────────────────
// Промежуточные структуры
// ─────────────────────────────────────────────────────────────────────────────

/// Агрегат строки из p903 по (srid, nm_id, nomenclature_ref, sa_name, supplier_oper_name).
/// Для строк без srid — srid = "".
#[derive(Debug, Default)]
pub(crate) struct P903Row {
    pub srid: String,
    pub nm_id: Option<i64>,
    pub nomenclature_ref: Option<String>,
    pub sa_name: Option<String>,
    pub supplier_oper_name: Option<String>,
    pub retail_amount: f64,
    pub return_amount: f64,
    pub acquiring_fee: f64,
    pub ppvz_vw: f64,
    pub ppvz_vw_nds: f64,
    pub ppvz_sales_commission: f64,
    pub delivery_rub: f64,
    pub rebill_logistic_cost: f64,
    pub storage_fee: f64,
    pub penalty: f64,
    pub additional_payment: f64,
    pub cashback_amount: f64,
    pub delivery_amount: f64,
    #[allow(dead_code)]
    pub ppvz_for_pay: f64,
    pub qty_sold: i64,
    pub qty_returned: i64,
    /// UUID первой строки p903 в группе (MIN(id)).
    pub p903_ref_id: Option<String>,
    /// rrd_id первой строки p903 в группе (MIN(rrd_id)).
    pub rrd_id: Option<i64>,
    /// true если в группе встретились разные supplier_oper_name (неоднозначная классификация).
    pub kind_ambiguous: bool,
}

/// Информация о рекламе из p913 для одного srid.
#[derive(Debug, Default)]
struct P913AdvertRow {
    pub advert_expense: f64,
    pub advert_reserve: f64,
}

/// Строка a012_wb_sales.
#[derive(Debug, Clone)]
pub(crate) struct A012Row {
    pub id: String,
    pub document_no: String,
    pub sale_id: Option<String>,
    pub sale_date: String,
    /// COALESCE(amount_line, finished_price, total_price) — реализация > 0, возврат < 0.
    pub line_amount: f64,
    pub dealer_total: f64,
    pub is_posted: bool,
    pub event_type: String,
}

fn a012_is_sale(row: &A012Row) -> bool {
    row.line_amount > 0.0
}

fn a012_is_return(row: &A012Row) -> bool {
    row.line_amount < 0.0
}

fn a012_rows_for_kind<'a>(rows: &'a [A012Row], kind: &LineKind) -> Vec<&'a A012Row> {
    match kind {
        LineKind::Sale => rows.iter().filter(|r| a012_is_sale(r)).collect(),
        LineKind::Return => rows.iter().filter(|r| a012_is_return(r)).collect(),
        _ => vec![],
    }
}

/// Предпочитаем a012 с sale_date на дату закрытия; при лаге WB — ближайшую не позже upper bound.
fn sort_a012_for_pick(docs: &mut [A012Row], business_date: &str) {
    docs.sort_by(|a, b| {
        let a_on_close = a.sale_date.as_str() <= business_date;
        let b_on_close = b.sale_date.as_str() <= business_date;
        match (a_on_close, b_on_close) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.sale_date.cmp(&b.sale_date),
        }
    });
}

/// Строка a015_wb_orders.
#[derive(Debug, Clone)]
pub(crate) struct A015Row {
    pub id: String,
    pub order_date: String,
    pub is_cancel: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Классификатор LineKind
// ─────────────────────────────────────────────────────────────────────────────

const OP_SALE: &str = "Продажа";
const OP_RETURN: &str = "Возврат";
const OP_STORAGE: &str = "Хранение";
const OP_PENALTY: &str = "Штраф";
const OP_PPVZ_REWARD: &str = "Возмещение за выдачу и возврат товаров на ПВЗ";
const OP_VOLUNTARY_RETURN_COMPENSATION: &str = "Добровольная компенсация при возврате";
const OP_TRANSPORT_STORAGE_REIMBURSEMENT: &str =
    "Возмещение издержек по перевозке/по складским операциям с товаром";
const OP_LOGISTICS: &str = "Логистика";

fn classify_kind(row: &P903Row) -> (LineKind, bool) {
    let oper = row.supplier_oper_name.as_deref().unwrap_or("").trim();
    let has_srid = !row.srid.is_empty();

    // Приоритет 1: явное имя операции — не смешивать с суммовым fallback.
    // Важно: проверяем имя ДО суммовых условий, иначе строка «Возврат»
    // с retail_amount > 0 (WB иногда ставит сумму возврата в retail_amount)
    // ошибочно классифицируется как Sale.
    if oper == OP_SALE {
        return (LineKind::Sale, false);
    }
    if oper == OP_RETURN {
        return (LineKind::Return, false);
    }
    if oper == OP_STORAGE {
        return (LineKind::Storage, false);
    }
    if oper == OP_PENALTY {
        return (LineKind::Penalty, false);
    }
    if oper == OP_PPVZ_REWARD {
        return (LineKind::PpvzReward, false);
    }
    if oper == OP_VOLUNTARY_RETURN_COMPENSATION {
        return (LineKind::VoluntaryReturnCompensation, false);
    }
    if oper == OP_TRANSPORT_STORAGE_REIMBURSEMENT {
        return (LineKind::TransportStorageReimbursement, false);
    }
    if oper == OP_LOGISTICS {
        return (LineKind::Logistics, false);
    }

    // Приоритет 2: суммовой fallback — только когда имя операции пустое или нераспознано.
    if row.retail_amount > 0.0 && row.return_amount == 0.0 && has_srid {
        return (LineKind::Sale, false);
    }
    if row.return_amount > 0.0 && row.retail_amount == 0.0 && has_srid {
        return (LineKind::Return, false);
    }

    // Приёмка: без srid и есть delivery_amount
    if !has_srid && row.delivery_amount.abs() > f64::EPSILON {
        return (LineKind::Acceptance, false);
    }

    // Корректировка комиссии: есть srid, не продажа/возврат, есть ppvz-суммы
    if has_srid {
        let commission_total =
            row.ppvz_vw.abs() + row.ppvz_vw_nds.abs() + row.ppvz_sales_commission.abs();
        if commission_total > f64::EPSILON {
            return (LineKind::CommissionAdjustment, false);
        }
    }

    // Смешанный случай (retail + return одновременно) — маркируем как Other с unknown_flag
    if row.retail_amount > 0.0 && row.return_amount > 0.0 {
        return (LineKind::Other, true);
    }

    (LineKind::Other, true)
}

fn classify_detail(row: &P903Row) -> LineDetail {
    let has_srid = !row.srid.is_empty();
    let has_nm = row.nm_id.is_some() || row.nomenclature_ref.is_some();
    match (has_srid, has_nm) {
        (true, true) => LineDetail::OrderAndNomenclature,
        (true, false) => LineDetail::OrderOnly,
        (false, true) => LineDetail::NomenclatureOnly,
        (false, false) => LineDetail::General,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Шаг 1: p903 агрегация (включает строки без srid)
// ─────────────────────────────────────────────────────────────────────────────

/// Непустые srid из уже загруженных строк p903.
pub(crate) fn p903_srids_from_rows(p903_rows: &[P903Row]) -> Vec<String> {
    p903_rows
        .iter()
        .map(|r| r.srid.clone())
        .filter(|s| !s.is_empty())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Загружает агрегированные строки p903 за день (один SQL).
pub(crate) async fn load_p903_day(
    connection_id: &str,
    business_date: &str,
) -> Result<Vec<P903Row>> {
    fetch_p903_rows(connection_id, business_date).await
}

async fn fetch_p903_rows(connection_id: &str, business_date: &str) -> Result<Vec<P903Row>> {
    // GROUP BY включает supplier_oper_name, поэтому каждая группа имеет ровно одно значение
    // этого поля — так разные типы операций (Продажа/Возврат) не смешиваются.
    // Следствие: COUNT(DISTINCT supplier_oper_name) внутри группы = всегда 1,
    // а oper_name_count > 1 никогда не выполняется через SQL-путь.
    // kind_ambiguous срабатывает только через unknown_flag из classify_kind — это корректно.
    let sql = r#"
        SELECT
            COALESCE(srid, '')                           AS srid,
            nm_id,
            a004_nomenclature_ref,
            sa_name,
            MIN(supplier_oper_name)                      AS supplier_oper_name,
            COUNT(DISTINCT COALESCE(supplier_oper_name,'')) AS oper_name_count,
            MIN(id)                                      AS p903_ref_id,
            MIN(rrd_id)                                  AS rrd_id,
            SUM(COALESCE(retail_amount, 0))              AS retail_amount,
            SUM(COALESCE(return_amount, 0))              AS return_amount,
            SUM(COALESCE(acquiring_fee, 0))              AS acquiring_fee,
            SUM(COALESCE(ppvz_vw, 0))                   AS ppvz_vw,
            SUM(COALESCE(ppvz_vw_nds, 0))               AS ppvz_vw_nds,
            SUM(COALESCE(ppvz_sales_commission, 0))      AS ppvz_sales_commission,
            SUM(COALESCE(delivery_rub, 0))               AS delivery_rub,
            SUM(COALESCE(rebill_logistic_cost, 0))       AS rebill_logistic_cost,
            SUM(COALESCE(storage_fee, 0))                AS storage_fee,
            SUM(COALESCE(penalty, 0))                    AS penalty,
            SUM(COALESCE(additional_payment, 0))         AS additional_payment,
            SUM(COALESCE(cashback_amount, 0))            AS cashback_amount,
            SUM(COALESCE(delivery_amount, 0))            AS delivery_amount,
            SUM(COALESCE(ppvz_for_pay, 0))               AS ppvz_for_pay,
            SUM(CASE WHEN COALESCE(quantity, 0) > 0 THEN COALESCE(quantity, 0) ELSE 0 END) AS qty_sold,
            SUM(CASE WHEN COALESCE(quantity, 0) < 0 THEN -COALESCE(quantity, 0) ELSE 0 END) AS qty_returned
        FROM p903_wb_finance_report
        WHERE rr_dt = ?
          AND connection_mp_ref = ?
          AND (supplier_oper_name IS NULL OR supplier_oper_name <> 'Комиссия за организацию платежа с НДС')
        GROUP BY COALESCE(srid, ''), nm_id, a004_nomenclature_ref, sa_name, supplier_oper_name
    "#;

    let stmt = Statement::from_sql_and_values(
        conn().get_database_backend(),
        sql,
        vec![sv(business_date), sv(connection_id)],
    );

    let rows = conn().query_all(stmt).await?;
    let mut result = Vec::with_capacity(rows.len());

    for row in rows {
        let srid: String = row.try_get("", "srid").unwrap_or_default();
        let oper_name_count: i64 = row.try_get("", "oper_name_count").unwrap_or(1);
        result.push(P903Row {
            srid,
            nm_id: row.try_get("", "nm_id").ok(),
            nomenclature_ref: row
                .try_get("", "a004_nomenclature_ref")
                .ok()
                .filter(|s: &String| !s.is_empty()),
            sa_name: row
                .try_get("", "sa_name")
                .ok()
                .filter(|s: &String| !s.is_empty()),
            supplier_oper_name: row
                .try_get("", "supplier_oper_name")
                .ok()
                .filter(|s: &String| !s.is_empty()),
            retail_amount: row.try_get("", "retail_amount").unwrap_or(0.0),
            return_amount: row.try_get("", "return_amount").unwrap_or(0.0),
            acquiring_fee: row.try_get("", "acquiring_fee").unwrap_or(0.0),
            ppvz_vw: row.try_get("", "ppvz_vw").unwrap_or(0.0),
            ppvz_vw_nds: row.try_get("", "ppvz_vw_nds").unwrap_or(0.0),
            ppvz_sales_commission: row.try_get("", "ppvz_sales_commission").unwrap_or(0.0),
            delivery_rub: row.try_get("", "delivery_rub").unwrap_or(0.0),
            rebill_logistic_cost: row.try_get("", "rebill_logistic_cost").unwrap_or(0.0),
            storage_fee: row.try_get("", "storage_fee").unwrap_or(0.0),
            penalty: row.try_get("", "penalty").unwrap_or(0.0),
            additional_payment: row.try_get("", "additional_payment").unwrap_or(0.0),
            cashback_amount: row.try_get("", "cashback_amount").unwrap_or(0.0),
            delivery_amount: row.try_get("", "delivery_amount").unwrap_or(0.0),
            ppvz_for_pay: row.try_get("", "ppvz_for_pay").unwrap_or(0.0),
            qty_sold: row.try_get("", "qty_sold").unwrap_or(0i64),
            qty_returned: row.try_get("", "qty_returned").unwrap_or(0i64),
            p903_ref_id: row
                .try_get("", "p903_ref_id")
                .ok()
                .filter(|s: &String| !s.is_empty()),
            rrd_id: row.try_get::<i64>("", "rrd_id").ok(),
            kind_ambiguous: oper_name_count > 1,
        });
    }

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Шаг 2: p913 реклама per-srid
// ─────────────────────────────────────────────────────────────────────────────

async fn fetch_p913_advert(srids: &[String]) -> Result<HashMap<String, P913AdvertRow>> {
    if srids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = srids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        r#"
        SELECT
            order_key AS srid,
            SUM(CASE WHEN turnover_code = 'advert_clicks_order_accrual' THEN amount ELSE 0 END) AS reserve_amount,
            SUM(CASE WHEN turnover_code = 'advert_clicks_order_expense' THEN amount ELSE 0 END) AS expense_amount
        FROM p913_wb_advert_order_attr
        WHERE order_key IN ({})
        GROUP BY order_key
        "#,
        placeholders
    );

    let params: Vec<Value> = srids.iter().map(|s| sv(s)).collect();
    let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
    let rows = conn().query_all(stmt).await?;

    let mut map = HashMap::with_capacity(rows.len());
    for row in rows {
        let srid: String = row.try_get("", "srid").unwrap_or_default();
        if srid.is_empty() {
            continue;
        }
        map.insert(
            srid,
            P913AdvertRow {
                advert_expense: row.try_get("", "expense_amount").unwrap_or(0.0),
                advert_reserve: row.try_get("", "reserve_amount").unwrap_or(0.0),
            },
        );
    }
    Ok(map)
}

/// Фактически списанная реклама (advert_clicks_order_expense) по registrator_ref a012.
async fn fetch_p913_expense_by_a012_ids(ids: &[String]) -> Result<HashMap<String, f64>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut map = HashMap::with_capacity(ids.len());
    const CHUNK: usize = 400;
    for chunk in ids.chunks(CHUNK) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            r#"
            SELECT registrator_ref, COALESCE(SUM(amount), 0.0) AS expense_amount
            FROM p913_wb_advert_order_attr
            WHERE registrator_ref IN ({placeholders})
              AND turnover_code = 'advert_clicks_order_expense'
            GROUP BY registrator_ref
            "#,
        );
        let params: Vec<Value> = chunk.iter().map(|s| sv(s)).collect();
        let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
        let rows = conn().query_all(stmt).await?;
        for row in rows {
            let id: String = row.try_get("", "registrator_ref").unwrap_or_default();
            if id.is_empty() {
                continue;
            }
            map.insert(id, row.try_get("", "expense_amount").unwrap_or(0.0));
        }
    }
    Ok(map)
}

/// Сумма advert_clicks_order_expense по списку a012 id.
fn sum_a012_advert_expense(ids: &[String], map: &HashMap<String, f64>) -> Option<f64> {
    if ids.is_empty() {
        return None;
    }
    Some(
        ids.iter()
            .map(|id| map.get(id).copied().unwrap_or(0.0))
            .sum(),
    )
}

/// Реклама для колонки строки: advert_clicks_order_expense matched a012 (как в карточке a012).
fn advert_expense_from_matched_a012(
    matched_a012: &Option<A012Row>,
    expense_by_a012: &HashMap<String, f64>,
) -> f64 {
    matched_a012
        .as_ref()
        .and_then(|a| expense_by_a012.get(&a.id).copied())
        .unwrap_or(0.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Шаг 3: a012 per srid (sale_date <= business_date + лаг; знак суммы — в join по LineKind)
// ─────────────────────────────────────────────────────────────────────────────

/// a012 кабинета по document_no (= srid), `sale_date` не позже `a012_sale_date_upper_bound`.
async fn fetch_a012_for_srids(
    connection_id: &str,
    business_date: &str,
    srids: &[String],
) -> Result<HashMap<String, Vec<A012Row>>> {
    if srids.is_empty() {
        return Ok(HashMap::new());
    }

    let sale_date_to = a012_sale_date_upper_bound(business_date);
    const CHUNK: usize = 400;
    let mut map: HashMap<String, Vec<A012Row>> = HashMap::new();

    for chunk in srids.chunks(CHUNK) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            r#"
            SELECT
                id,
                document_no,
                sale_id,
                SUBSTR(COALESCE(sale_date, ''), 1, 10) AS sale_date,
                COALESCE(amount_line, finished_price, total_price, 0.0) AS line_amount,
                COALESCE(dealer_price_ut, 0.0) * COALESCE(ABS(qty), 1.0) AS dealer_total,
                is_posted,
                COALESCE(event_type, 'sale') AS event_type
            FROM a012_wb_sales
            WHERE connection_id = ?
              AND substr(COALESCE(sale_date, ''), 1, 10) <= ?
              AND document_no IN ({placeholders})
              AND is_deleted = 0
            "#,
        );

        let mut params: Vec<Value> = vec![sv(connection_id), sv(&sale_date_to)];
        params.extend(chunk.iter().map(|s| sv(s)));

        let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
        let rows = conn().query_all(stmt).await?;

        for row in rows {
            let doc_no: String = row.try_get("", "document_no").unwrap_or_default();
            if doc_no.is_empty() {
                continue;
            }
            let event_type: String = row
                .try_get("", "event_type")
                .unwrap_or_else(|_| "sale".to_string());
            map.entry(doc_no.clone()).or_default().push(A012Row {
                id: row.try_get("", "id").unwrap_or_default(),
                document_no: doc_no,
                sale_id: row
                    .try_get("", "sale_id")
                    .ok()
                    .filter(|s: &String| !s.is_empty()),
                sale_date: row.try_get("", "sale_date").unwrap_or_default(),
                line_amount: row.try_get("", "line_amount").unwrap_or(0.0),
                dealer_total: row.try_get("", "dealer_total").unwrap_or(0.0),
                is_posted: row.try_get("", "is_posted").unwrap_or(false),
                event_type,
            });
        }
    }

    Ok(map)
}

// ─────────────────────────────────────────────────────────────────────────────
// Шаг 4: a015 per srid (дата и статус отмены)
// ─────────────────────────────────────────────────────────────────────────────

async fn fetch_a015_for_srids(srids: &[String]) -> Result<HashMap<String, A015Row>> {
    if srids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = srids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    // order_dt хранится внутри state_json; is_cancel — прямая колонка.
    let sql = format!(
        r#"
        SELECT
            id,
            document_no AS srid,
            SUBSTR(COALESCE(json_extract(state_json, '$.order_dt'), ''), 1, 10) AS order_date,
            COALESCE(is_cancel, 0) AS is_cancel
        FROM a015_wb_orders
        WHERE document_no IN ({})
          AND is_deleted = 0
        "#,
        placeholders
    );

    let params: Vec<Value> = srids.iter().map(|s| sv(s)).collect();
    let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
    let rows = conn().query_all(stmt).await?;

    let mut map = HashMap::with_capacity(rows.len());
    for row in rows {
        let srid: String = row.try_get("", "srid").unwrap_or_default();
        if srid.is_empty() {
            continue;
        }
        let is_cancel: i32 = row.try_get("", "is_cancel").unwrap_or(0);
        map.insert(
            srid,
            A015Row {
                id: row.try_get("", "id").unwrap_or_default(),
                order_date: row.try_get("", "order_date").unwrap_or_default(),
                is_cancel: is_cancel != 0,
            },
        );
    }
    Ok(map)
}

// ─────────────────────────────────────────────────────────────────────────────
// Детектор: a012 sale_date ≠ p903 rr_dt (обратное направление)
// ─────────────────────────────────────────────────────────────────────────────

struct A012FinDateMismatch {
    id: String,
    srid: String,
    sale_date: String,
    fin_report_date: String,
}

/// a012 с sale_date = business_date, но без p903 на эту дату (fin report на другой rr_dt).
async fn fetch_a012_sale_on_close_without_fin_report(
    connection_id: &str,
    business_date: &str,
) -> Result<Vec<A012FinDateMismatch>> {
    let sql = r#"
        SELECT
            a.id,
            a.document_no AS srid,
            SUBSTR(COALESCE(a.sale_date, ''), 1, 10) AS sale_date,
            (
                SELECT MIN(p.rr_dt)
                FROM p903_wb_finance_report p
                WHERE p.srid = a.document_no
                  AND p.connection_mp_ref = a.connection_id
                  AND p.rr_dt != ?
            ) AS fin_report_date
        FROM a012_wb_sales a
        WHERE a.connection_id = ?
          AND SUBSTR(COALESCE(a.sale_date, ''), 1, 10) = ?
          AND a.is_deleted = 0
          AND a.is_posted = 1
          AND NOT EXISTS (
              SELECT 1
              FROM p903_wb_finance_report p2
              WHERE p2.srid = a.document_no
                AND p2.connection_mp_ref = a.connection_id
                AND p2.rr_dt = ?
          )
          AND EXISTS (
              SELECT 1
              FROM p903_wb_finance_report p3
              WHERE p3.srid = a.document_no
                AND p3.connection_mp_ref = a.connection_id
          )
    "#;
    let stmt = Statement::from_sql_and_values(
        conn().get_database_backend(),
        sql,
        vec![
            sv(business_date),
            sv(connection_id),
            sv(business_date),
            sv(business_date),
        ],
    );
    let rows = conn().query_all(stmt).await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let id: String = row.try_get("", "id").ok()?;
            let srid: String = row.try_get("", "srid").ok()?;
            let sale_date: String = row.try_get("", "sale_date").unwrap_or_default();
            let fin_report_date: String = row.try_get("", "fin_report_date").unwrap_or_default();
            if id.is_empty() || srid.is_empty() || fin_report_date.is_empty() {
                return None;
            }
            Some(A012FinDateMismatch {
                id,
                srid,
                sale_date,
                fin_report_date,
            })
        })
        .collect())
}

fn dedupe_fin_date_mismatch_problems(problems: Vec<WbDayCloseProblem>) -> Vec<WbDayCloseProblem> {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    problems
        .into_iter()
        .filter(|p| {
            if p.code != "a012_sale_date_mismatch_fin_report" {
                return true;
            }
            let key = (
                p.code.clone(),
                p.a012_ids
                    .first()
                    .cloned()
                    .or_else(|| p.srid.clone())
                    .unwrap_or_default(),
            );
            if key.1.is_empty() {
                return true;
            }
            seen.insert(key)
        })
        .collect()
}

fn append_reverse_fin_date_mismatch_problems(
    mut problems: Vec<WbDayCloseProblem>,
    rows: Vec<A012FinDateMismatch>,
    business_date: &str,
    expense_by_a012: &HashMap<String, f64>,
) -> Vec<WbDayCloseProblem> {
    let mut seen: HashSet<String> = problems
        .iter()
        .filter(|p| p.code == "a012_sale_date_mismatch_fin_report")
        .flat_map(|p| p.a012_ids.iter().cloned())
        .collect();

    for row in rows {
        if !seen.insert(row.id.clone()) {
            continue;
        }
        problems.push(WbDayCloseProblem {
            code: "a012_sale_date_mismatch_fin_report".to_string(),
            severity: ProblemSeverity::Warn,
            srid: Some(row.srid.clone()),
            nomenclature_ref: None,
            a012_ids: vec![row.id.clone()],
            a012_advert_expense: expense_by_a012.get(&row.id).copied(),
            message: format!(
                "srid={}: sale_date={}, fin_report(rr_dt)={} — нет p903 на дату закрытия {}",
                row.srid, row.sale_date, row.fin_report_date, business_date
            ),
        });
    }

    dedupe_fin_date_mismatch_problems(problems)
}

// ─────────────────────────────────────────────────────────────────────────────
// Шаг 5: p912 dealer prices — пакетная загрузка
// ─────────────────────────────────────────────────────────────────────────────

/// Пакетная загрузка актуальных цен p912 для нескольких nomenclature_ref за одним SQL.
/// Возвращает map: nomenclature_ref → cost (последняя цена с period <= business_date).
async fn fetch_p912_dealer_prices_batch(
    nrefs: &[String],
    business_date: &str,
) -> Result<HashMap<String, f64>> {
    if nrefs.is_empty() {
        return Ok(HashMap::new());
    }
    const CHUNK: usize = 400;
    let mut map: HashMap<String, f64> = HashMap::new();
    for chunk in nrefs.chunks(CHUNK) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            r#"
            WITH ranked AS (
                SELECT nomenclature_ref, cost,
                       ROW_NUMBER() OVER (
                           PARTITION BY nomenclature_ref
                           ORDER BY period DESC
                       ) AS rn
                FROM p912_nomenclature_costs
                WHERE nomenclature_ref IN ({placeholders})
                  AND period <= ?
            )
            SELECT nomenclature_ref, cost FROM ranked WHERE rn = 1
            "#,
        );
        let mut params: Vec<Value> = chunk.iter().map(|s| sv(s)).collect();
        params.push(sv(business_date));
        let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
        let rows = conn().query_all(stmt).await?;
        for row in rows {
            let nref: String = row.try_get("", "nomenclature_ref").unwrap_or_default();
            let cost: f64 = row.try_get("", "cost").unwrap_or(0.0);
            if !nref.is_empty() {
                map.insert(nref, cost);
            }
        }
    }
    Ok(map)
}

// ─────────────────────────────────────────────────────────────────────────────
// Чистая вычислительная функция (тестируемая без DB)
// ─────────────────────────────────────────────────────────────────────────────

/// Входные данные для построения одной строки документа a033.
/// Все DB-зависимости уже разрешены к этому моменту.
pub(crate) struct LineComputeInput<'a> {
    pub p903: &'a P903Row,
    /// Классифицированный тип строки.
    pub kind: LineKind,
    /// Уровень детализации.
    pub detail: LineDetail,
    /// true если классификация неоднозначна (разные supplier_oper_name в группе).
    pub kind_ambiguous: bool,
    /// Фактически списанная реклама для колонки: advert_clicks_order_expense matched a012.
    pub advert_expense: f64,
    /// Списанная реклама по order_key=srid (для детекторов резерва без expense).
    pub advert_order_expense: f64,
    /// Зарезервированная реклама из p913 (turnover_code='advert_clicks_order_accrual').
    pub advert_reserve: f64,
    /// p913 advert_clicks_order_expense по id a012 (для проблем и колонки).
    pub expense_by_a012: &'a HashMap<String, f64>,
    /// Суммарная дилерская стоимость (dealer_price_ut × qty) из a012 или p912.
    pub dealer_total: f64,
    /// Документ a012, соответствующий типу строки (sale или return).
    pub matched_a012: Option<A012Row>,
    /// Лишние a012 того же типа для этого srid (должен быть 0, иначе проблема).
    pub extra_a012_ids: Vec<String>,
    /// a012 того же типа, что и строка p903 (по знаку суммы; для детектора unposted).
    pub all_a012_for_srid: Vec<A012Row>,
    /// Данные заказа из a015.
    pub order: Option<A015Row>,
    /// Дата закрытия дня (= p903.rr_dt для строк документа).
    pub business_date: &'a str,
}

/// Вычисляет строку документа и список проблем для одного srid.
/// Чистая функция: не обращается к DB.
pub(crate) fn compute_line_and_problems(
    input: LineComputeInput<'_>,
) -> (WbDayCloseLine, Vec<WbDayCloseProblem>) {
    let p903 = input.p903;
    let srid = &p903.srid;

    // ── 10 колонок (доход +, расход −) ───────────────────────────────────────
    //
    // Возврат (LineKind::Return) — особые правила знаков:
    //   WB хранит суммы возврата в retail_amount (return_amount = 0), а не в return_amount.
    //   acquiring_fee и ppvz-поля содержат суммы, которые WB ВОЗВРАЩАЕТ продавцу (или берёт
    //   обратно при отрицательном значении ppvz — случай сторно соинвеста).
    //   Поэтому для возвратов знак колонок «revenue», «acquiring» и «commission» инвертируется
    //   относительно формулы для продаж — это согласовано с логикой GL-строителя (p903 → GL).
    let is_return = matches!(input.kind, LineKind::Return);

    // 1. Реализация.
    //    Продажа:  retail_amount − return_amount  (обычно return_amount = 0)
    //    Возврат:  WB кладёт сумму в retail_amount (return_amount = 0), знак инвертируем.
    //              Если return_amount > 0 (стандартный формат) — тоже инвертируем.
    let revenue = if is_return {
        if p903.return_amount.abs() > f64::EPSILON {
            -p903.return_amount
        } else {
            -p903.retail_amount
        }
    } else {
        p903.retail_amount - p903.return_amount
    };

    let advertising = -input.advert_expense;
    let logistics = -(p903.delivery_rub + p903.rebill_logistic_cost + p903.storage_fee);

    // 4. Эквайринг.
    //    Продажа:  −acquiring_fee  (расход)
    //    Возврат:  +acquiring_fee  (WB возвращает комиссию эквайрера → доход/сторно)
    //    Знак ppvz не влияет на acquiring — он всегда неотрицателен в WB-данных.
    let acquiring = if is_return {
        p903.acquiring_fee
    } else {
        -p903.acquiring_fee
    };

    // 5. Комиссия.
    //    Sale/Return:  ±(ppvz_vw + ppvz_vw_nds)
    //      — ppvz_sales_commission НЕ входит: GL-строитель для sale/return использует только
    //        ppvz_vw + ppvz_vw_nds в turnover mp_commission; ppvz_sales_commission идёт только
    //        в mp_commission_adjustment для прочих операций (CommissionAdjustment и др.).
    //    Прочие:  −(ppvz_vw + ppvz_vw_nds + ppvz_sales_commission)
    //    • ppvz > 0 в строке возврата → WB возвращает комиссию → колонка положительная (доход).
    //    • ppvz < 0 в строке возврата → WB берёт обратно соинвест → колонка отрицательная (расход).
    let is_sale_or_return = matches!(input.kind, LineKind::Sale | LineKind::Return);
    let commission_sales_adj = if is_sale_or_return {
        0.0
    } else {
        p903.ppvz_sales_commission
    };
    let commission = if is_return {
        p903.ppvz_vw + p903.ppvz_vw_nds + commission_sales_adj
    } else {
        -(p903.ppvz_vw + p903.ppvz_vw_nds + commission_sales_adj)
    };
    let penalty = -p903.penalty;
    let other = p903.additional_payment + p903.cashback_amount;
    let result = revenue + advertising + logistics + acquiring + commission + penalty + other;
    // 9. ЦенаДилер.
    //    Продажа:  −dealer_total (расход — себестоимость отгруженного товара).
    //    Возврат:  +dealer_total (доход — себестоимость возвращённого товара приходит обратно).
    let dealer_price = if input.dealer_total > 0.0 {
        if is_return {
            input.dealer_total
        } else {
            -input.dealer_total
        }
    } else {
        0.0
    };
    let margin_diff = result + dealer_price;

    // ── Legacy SaleEvent (обратная совместимость) ─────────────────────────────
    let event = match (&input.kind, p903.qty_sold > 0, p903.qty_returned > 0) {
        (LineKind::Sale, _, _) => SaleEvent::Sale,
        (LineKind::Return, _, _) => SaleEvent::Return,
        (_, true, true) => SaleEvent::Mixed,
        (_, false, true) => SaleEvent::Return,
        _ => SaleEvent::Sale,
    };

    // ── Связи ─────────────────────────────────────────────────────────────────
    let (order_id, order_date, order_is_cancelled) = match &input.order {
        Some(o) => (
            Some(o.id.clone()),
            Some(o.order_date.clone()).filter(|s| !s.is_empty()),
            o.is_cancel,
        ),
        None => (None, None, false),
    };

    let (sales_doc_id, sales_doc_no, sales_event_type, sales_sale_id, sales_doc_date) =
        match &input.matched_a012 {
            Some(a) => (
                Some(a.id.clone()),
                Some(a.document_no.clone()),
                Some(a.event_type.clone()),
                a.sale_id.clone(),
                Some(a.sale_date.clone()).filter(|s| !s.is_empty()),
            ),
            None => (None, None, None, None, None),
        };

    // ── Рeclassify Other→Info если все финансовые колонки нулевые ────────────
    let kind = if input.kind == LineKind::Other {
        let financial_sum = revenue.abs()
            + input.advert_expense.abs()
            + logistics.abs()
            + acquiring.abs()
            + commission.abs()
            + penalty.abs()
            + other.abs();
        if financial_sum < 0.001 {
            LineKind::Info
        } else {
            input.kind
        }
    } else {
        input.kind
    };

    // ── Проблемы (информационные строки не проверяются) ───────────────────────
    let mut line_problem_codes: Vec<String> = Vec::new();
    let mut problems: Vec<WbDayCloseProblem> = Vec::new();

    if kind.is_info() {
        let line = WbDayCloseLine {
            srid: srid.clone(),
            nomenclature_ref: p903.nomenclature_ref.clone(),
            nm_id: p903.nm_id,
            sa_name: p903.sa_name.clone(),
            event,
            kind,
            detail: input.detail,
            qty_sold: p903.qty_sold,
            qty_returned: p903.qty_returned,
            order_id,
            order_date,
            order_is_cancelled,
            sales_doc_id,
            sales_doc_no,
            sales_doc_date,
            sales_event_type,
            sales_sale_id,
            sales_extra_ids: input.extra_a012_ids,
            p903_ref_id: p903.p903_ref_id.clone(),
            p903_rrd_id: p903.rrd_id,
            revenue,
            advertising,
            logistics,
            acquiring,
            commission,
            penalty,
            other,
            result,
            dealer_price,
            margin_diff,
            problem_codes: line_problem_codes,
        };
        return (line, problems);
    }

    let make_problem = |code: &'static str, msg: String, a012_ids: Vec<String>| {
        let severity = match code {
            "advert_clicks_order_accrual_without_expense"
            | "column_invariant_mismatch"
            | "multiple_a012_for_srid"
            | "mixed_sale_and_return_for_srid" => ProblemSeverity::Block,
            _ => ProblemSeverity::Warn,
        };
        WbDayCloseProblem {
            code: code.to_string(),
            severity,
            srid: if srid.is_empty() {
                None
            } else {
                Some(srid.clone())
            },
            nomenclature_ref: p903.nomenclature_ref.clone(),
            a012_ids: a012_ids.clone(),
            a012_advert_expense: sum_a012_advert_expense(&a012_ids, input.expense_by_a012),
            message: msg,
        }
    };

    // 1. Неизвестный тип строки (Info — не ошибка, это пустые строки без сумм)
    if input.kind_ambiguous || kind == LineKind::Other {
        line_problem_codes.push("unknown_line_type".to_string());
        problems.push(make_problem(
            "unknown_line_type",
            format!(
                "srid={}: oper='{}', retail={:.2}, return={:.2}",
                srid,
                p903.supplier_oper_name.as_deref().unwrap_or(""),
                p903.retail_amount,
                p903.return_amount
            ),
            vec![],
        ));
    }

    // 2. Смешанная продажа+возврат для Sale/Return типа
    if matches!(kind, LineKind::Sale | LineKind::Return)
        && p903.qty_sold > 0
        && p903.qty_returned > 0
    {
        line_problem_codes.push("mixed_sale_and_return_for_srid".to_string());
        problems.push(make_problem(
            "mixed_sale_and_return_for_srid",
            format!(
                "srid={}: qty_sold={}, qty_returned={}",
                srid, p903.qty_sold, p903.qty_returned
            ),
            input
                .all_a012_for_srid
                .iter()
                .map(|r| r.id.clone())
                .collect(),
        ));
    }

    // 3. Резерв без expense (по order_key, не по GL-aligned колонке)
    if input.advert_reserve > 0.0 && input.advert_order_expense == 0.0 {
        line_problem_codes.push("advert_clicks_order_accrual_without_expense".to_string());
        problems.push(make_problem(
            "advert_clicks_order_accrual_without_expense",
            format!(
                "srid={}: advert_clicks_order_accrual={:.2}, advert_clicks_order_expense=0",
                srid, input.advert_reserve
            ),
            input.matched_a012.iter().map(|r| r.id.clone()).collect(),
        ));
    }

    // 4. Реклама на отменённый заказ
    if order_is_cancelled && input.advert_reserve > 0.0 {
        line_problem_codes.push("advert_attributed_to_cancelled_order".to_string());
        problems.push(make_problem(
            "advert_attributed_to_cancelled_order",
            format!(
                "srid={}: заказ отменён, но advert_clicks_order_accrual={:.2}",
                srid, input.advert_reserve
            ),
            input
                .all_a012_for_srid
                .iter()
                .map(|r| r.id.clone())
                .collect(),
        ));
    }

    // 5. Несколько a012 одного типа для srid
    if !input.extra_a012_ids.is_empty() {
        line_problem_codes.push("multiple_a012_for_srid".to_string());
        let mut ids = input
            .matched_a012
            .iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>();
        ids.extend(input.extra_a012_ids.clone());
        problems.push(make_problem(
            "multiple_a012_for_srid",
            format!(
                "srid={}: найдено {} документов a012 одного типа",
                srid,
                1 + input.extra_a012_ids.len()
            ),
            ids,
        ));
    }

    // 6. Заказ a015 не найден (только для Sale/Return)
    if kind.requires_order() && input.order.is_none() && !srid.is_empty() {
        line_problem_codes.push("a015_order_missing".to_string());
        problems.push(make_problem(
            "a015_order_missing",
            format!("srid={}: заказ не найден в a015_wb_orders", srid),
            vec![],
        ));
    }

    // 7. Нет реализации a012 нужного типа
    if kind == LineKind::Sale && input.matched_a012.is_none() && !srid.is_empty() {
        line_problem_codes.push("a012_sale_missing".to_string());
        problems.push(make_problem(
            "a012_sale_missing",
            format!(
                "srid={}: не найден a012-реализация (сумма>0, sale_date<=даты закрытия+{}д)",
                srid, A012_SALE_DATE_LAG_DAYS
            ),
            vec![],
        ));
    }
    if kind == LineKind::Return && input.matched_a012.is_none() && !srid.is_empty() {
        line_problem_codes.push("a012_return_missing".to_string());
        problems.push(make_problem(
            "a012_return_missing",
            format!(
                "srid={}: не найден a012-возврат (сумма<0, sale_date<=даты закрытия+{}д)",
                srid, A012_SALE_DATE_LAG_DAYS
            ),
            vec![],
        ));
    }

    // 8. Непроведённый a012
    let all_unposted =
        !input.all_a012_for_srid.is_empty() && input.all_a012_for_srid.iter().all(|r| !r.is_posted);
    if all_unposted {
        line_problem_codes.push("a012_unposted_for_p903_row".to_string());
        problems.push(make_problem(
            "a012_unposted_for_p903_row",
            format!("srid={}: a012 не проведён", srid),
            input
                .all_a012_for_srid
                .iter()
                .map(|r| r.id.clone())
                .collect(),
        ));
    }

    // 9. sale_date a012 ≠ rr_dt fin report (одна из дат = дата закрытия)
    if let Some(a012) = &input.matched_a012 {
        if !a012.sale_date.is_empty() && a012.sale_date != input.business_date {
            line_problem_codes.push("a012_sale_date_mismatch_fin_report".to_string());
            problems.push(make_problem(
                "a012_sale_date_mismatch_fin_report",
                format!(
                    "srid={}: sale_date={}, fin_report(rr_dt)={} — GL advert_clicks_order_expense по sale_date",
                    srid, a012.sale_date, input.business_date
                ),
                vec![a012.id.clone()],
            ));
        }
    }

    // 10. Отсутствует дилерская цена (только для Sale/Return)
    if kind.requires_sales_doc() && input.dealer_total < 0.001 && !srid.is_empty() {
        line_problem_codes.push("dealer_price_missing".to_string());
        problems.push(make_problem(
            "dealer_price_missing",
            format!("srid={}: dealer_price не найден", srid),
            vec![],
        ));
    }

    // 11. Инвариант колонок
    let invariant_expected =
        revenue + advertising + logistics + acquiring + commission + penalty + other;
    if (invariant_expected - result).abs() > 0.001 {
        line_problem_codes.push("column_invariant_mismatch".to_string());
        problems.push(make_problem(
            "column_invariant_mismatch",
            format!(
                "srid={}: columns_sum={:.4} != result={:.4}",
                srid, invariant_expected, result
            ),
            vec![],
        ));
    }

    let line = WbDayCloseLine {
        srid: srid.clone(),
        nomenclature_ref: p903.nomenclature_ref.clone(),
        nm_id: p903.nm_id,
        sa_name: p903.sa_name.clone(),
        event,
        kind,
        detail: input.detail,
        qty_sold: p903.qty_sold,
        qty_returned: p903.qty_returned,
        order_id,
        order_date,
        order_is_cancelled,
        sales_doc_id,
        sales_doc_no,
        sales_doc_date,
        sales_event_type,
        sales_sale_id,
        sales_extra_ids: input.extra_a012_ids,
        p903_ref_id: p903.p903_ref_id.clone(),
        p903_rrd_id: p903.rrd_id,
        revenue,
        advertising,
        logistics,
        acquiring,
        commission,
        penalty,
        other,
        result,
        dealer_price,
        margin_diff,
        problem_codes: line_problem_codes,
    };

    (line, problems)
}

// ─────────────────────────────────────────────────────────────────────────────
// Основная точка входа
// ─────────────────────────────────────────────────────────────────────────────

/// Собирает строки и проблемы для документа a033 (загружает p903).
pub async fn build(
    connection_id: &str,
    business_date: &str,
) -> Result<(Vec<WbDayCloseLine>, Vec<WbDayCloseProblem>)> {
    let p903_rows = load_p903_day(connection_id, business_date).await?;
    build_with_p903_rows(connection_id, business_date, p903_rows).await
}

/// Собирает строки и проблемы по уже загруженным строкам p903 (без повторного SQL p903).
pub(crate) async fn build_with_p903_rows(
    connection_id: &str,
    business_date: &str,
    p903_rows: Vec<P903Row>,
) -> Result<(Vec<WbDayCloseLine>, Vec<WbDayCloseProblem>)> {
    tracing::debug!(
        target: "a033_wb_day_close",
        connection_id,
        business_date,
        p903_line_groups = p903_rows.len(),
        "p903 aggregated line groups for day close"
    );
    if p903_rows.is_empty() {
        return Ok((vec![], vec![]));
    }

    let srids = p903_srids_from_rows(&p903_rows);

    // Шаги 2-4 параллельно
    let p913_map = fetch_p913_advert(&srids).await?;
    let a012_map = fetch_a012_for_srids(connection_id, business_date, &srids).await?;
    let a015_map = fetch_a015_for_srids(&srids).await?;

    let a012_ids: Vec<String> = a012_map
        .values()
        .flat_map(|rows| rows.iter().map(|r| r.id.clone()))
        .collect();
    let expense_by_a012 = fetch_p913_expense_by_a012_ids(&a012_ids).await?;

    // Пакетная загрузка p912 для всех nomenclature_ref (fallback дилерской цены).
    // Это устраняет N+1 запросов внутри цикла обработки строк p903.
    let all_nrefs: Vec<String> = p903_rows
        .iter()
        .filter_map(|r| r.nomenclature_ref.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let p912_price_map = fetch_p912_dealer_prices_batch(&all_nrefs, business_date).await?;

    let mut lines: Vec<WbDayCloseLine> = Vec::with_capacity(p903_rows.len());
    let mut problems: Vec<WbDayCloseProblem> = Vec::new();

    for p903 in &p903_rows {
        let srid = &p903.srid;

        // ── Классификация ─────────────────────────────────────────────────────
        let (kind, unknown_flag) = classify_kind(p903);
        let detail = classify_detail(p903);
        let kind_ambiguous = p903.kind_ambiguous || unknown_flag;

        // ── a012: реализация (сумма>0) / возврат (сумма<0), sale_date <= business_date ──
        let all_a012_raw = a012_map.get(srid.as_str()).cloned().unwrap_or_default();
        let relevant_a012: Vec<A012Row> = a012_rows_for_kind(&all_a012_raw, &kind)
            .into_iter()
            .cloned()
            .collect();

        let mut matched_a012_docs = relevant_a012.clone();
        sort_a012_for_pick(&mut matched_a012_docs, business_date);
        let matched_a012 = if !matched_a012_docs.is_empty() {
            Some(matched_a012_docs.remove(0))
        } else {
            None
        };
        let extra_a012_ids: Vec<String> = matched_a012_docs.iter().map(|r| r.id.clone()).collect();

        // ── Реклама ───────────────────────────────────────────────────────────
        let advert_info = p913_map.get(srid.as_str());
        let advert_order_expense = advert_info.map(|a| a.advert_expense).unwrap_or(0.0);
        let advert_reserve = advert_info.map(|a| a.advert_reserve).unwrap_or(0.0);
        let advert_expense = advert_expense_from_matched_a012(&matched_a012, &expense_by_a012);

        // ── Dealer price ──────────────────────────────────────────────────────
        let dealer_total = if !relevant_a012.is_empty() {
            relevant_a012.iter().map(|r| r.dealer_total).sum()
        } else {
            // Fallback: p912 (цена из заранее загруженного пакетного запроса)
            if let Some(nr) = &p903.nomenclature_ref {
                p912_price_map.get(nr.as_str()).copied().unwrap_or(0.0)
                    * (p903.qty_sold.saturating_sub(p903.qty_returned).max(0) as f64)
            } else {
                0.0
            }
        };

        // ── a015 ──────────────────────────────────────────────────────────────
        let order = a015_map.get(srid.as_str()).cloned();

        let input = LineComputeInput {
            p903,
            kind,
            detail,
            kind_ambiguous,
            advert_expense,
            advert_order_expense,
            advert_reserve,
            expense_by_a012: &expense_by_a012,
            dealer_total,
            matched_a012,
            extra_a012_ids,
            all_a012_for_srid: relevant_a012,
            order,
            business_date,
        };

        let (line, line_problems) = compute_line_and_problems(input);
        problems.extend(line_problems);
        lines.push(line);
    }

    let reverse_mismatches =
        fetch_a012_sale_on_close_without_fin_report(connection_id, business_date).await?;
    problems = append_reverse_fin_date_mismatch_problems(
        problems,
        reverse_mismatches,
        business_date,
        &expense_by_a012,
    );
    problems = dedupe_fin_date_mismatch_problems(problems);

    // Сортируем: сначала строки с srid (по srid), затем строки без srid
    lines.sort_by(|a, b| {
        let a_has = !a.srid.is_empty();
        let b_has = !b.srid.is_empty();
        match (a_has, b_has) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.srid.cmp(&b.srid),
        }
    });

    Ok((lines, problems))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn p903_sale(srid: &str, retail: f64) -> P903Row {
        P903Row {
            srid: srid.to_string(),
            nm_id: Some(1001),
            nomenclature_ref: Some("nm-001".to_string()),
            sa_name: Some("Артикул-А".to_string()),
            supplier_oper_name: Some(OP_SALE.to_string()),
            retail_amount: retail,
            qty_sold: 1,
            ..P903Row::default()
        }
    }

    fn p903_return(srid: &str, return_amt: f64) -> P903Row {
        P903Row {
            srid: srid.to_string(),
            nm_id: Some(1001),
            nomenclature_ref: Some("nm-001".to_string()),
            sa_name: Some("Артикул-А".to_string()),
            supplier_oper_name: Some(OP_RETURN.to_string()),
            return_amount: return_amt,
            qty_returned: 1,
            ..P903Row::default()
        }
    }

    fn p903_storage() -> P903Row {
        P903Row {
            srid: String::new(),
            supplier_oper_name: Some(OP_STORAGE.to_string()),
            storage_fee: 100.0,
            ..P903Row::default()
        }
    }

    fn p903_penalty() -> P903Row {
        P903Row {
            srid: String::new(),
            supplier_oper_name: Some(OP_PENALTY.to_string()),
            penalty: 50.0,
            ..P903Row::default()
        }
    }

    fn default_input(row: &P903Row) -> LineComputeInput<'_> {
        static EMPTY_EXPENSE: std::sync::OnceLock<HashMap<String, f64>> =
            std::sync::OnceLock::new();
        let (kind, unknown) = classify_kind(row);
        let detail = classify_detail(row);
        LineComputeInput {
            p903: row,
            kind,
            detail,
            kind_ambiguous: unknown,
            advert_expense: 0.0,
            advert_order_expense: 0.0,
            advert_reserve: 0.0,
            expense_by_a012: EMPTY_EXPENSE.get_or_init(HashMap::new),
            dealer_total: 0.0,
            matched_a012: None,
            extra_a012_ids: vec![],
            all_a012_for_srid: vec![],
            order: None,
            business_date: "2026-05-17",
        }
    }

    // ── Классификатор LineKind ────────────────────────────────────────────────

    #[test]
    fn classify_sale_by_oper_name() {
        let row = p903_sale("S001", 1000.0);
        let (kind, unknown) = classify_kind(&row);
        assert_eq!(kind, LineKind::Sale);
        assert!(!unknown);
    }

    #[test]
    fn classify_return_by_oper_name() {
        let row = p903_return("R001", 500.0);
        let (kind, unknown) = classify_kind(&row);
        assert_eq!(kind, LineKind::Return);
        assert!(!unknown);
    }

    #[test]
    fn classify_storage_no_srid() {
        let row = p903_storage();
        let (kind, unknown) = classify_kind(&row);
        assert_eq!(kind, LineKind::Storage);
        assert!(!unknown);
        assert_eq!(classify_detail(&row), LineDetail::General);
    }

    #[test]
    fn classify_penalty_no_srid() {
        let row = p903_penalty();
        let (kind, _) = classify_kind(&row);
        assert_eq!(kind, LineKind::Penalty);
    }

    #[test]
    fn classify_logistics() {
        let row = P903Row {
            srid: "L001".to_string(),
            supplier_oper_name: Some(OP_LOGISTICS.to_string()),
            rebill_logistic_cost: 200.0,
            ..P903Row::default()
        };
        let (kind, _) = classify_kind(&row);
        assert_eq!(kind, LineKind::Logistics);
    }

    #[test]
    fn classify_commission_adjustment() {
        let row = P903Row {
            srid: "CA001".to_string(),
            supplier_oper_name: Some("Корректировка".to_string()),
            ppvz_vw: 50.0,
            ppvz_vw_nds: 5.0,
            ..P903Row::default()
        };
        let (kind, _) = classify_kind(&row);
        assert_eq!(kind, LineKind::CommissionAdjustment);
    }

    #[test]
    fn classify_ppvz_reward() {
        let row = P903Row {
            srid: String::new(),
            supplier_oper_name: Some(OP_PPVZ_REWARD.to_string()),
            ppvz_for_pay: 300.0,
            ..P903Row::default()
        };
        let (kind, _) = classify_kind(&row);
        assert_eq!(kind, LineKind::PpvzReward);
    }

    #[test]
    fn classify_acceptance() {
        let row = P903Row {
            srid: String::new(),
            delivery_amount: 75.0,
            ..P903Row::default()
        };
        let (kind, _) = classify_kind(&row);
        assert_eq!(kind, LineKind::Acceptance);
    }

    // ── LineDetail ────────────────────────────────────────────────────────────

    #[test]
    fn detail_order_and_nomenclature() {
        let row = p903_sale("S001", 1000.0);
        assert_eq!(classify_detail(&row), LineDetail::OrderAndNomenclature);
    }

    #[test]
    fn detail_order_only() {
        let row = P903Row {
            srid: "S002".to_string(),
            supplier_oper_name: Some(OP_SALE.to_string()),
            retail_amount: 500.0,
            qty_sold: 1,
            ..P903Row::default()
        };
        assert_eq!(classify_detail(&row), LineDetail::OrderOnly);
    }

    #[test]
    fn detail_general_no_srid_no_nm() {
        let row = p903_storage();
        assert_eq!(classify_detail(&row), LineDetail::General);
    }

    // ── Базовые тесты знаков и инварианта ─────────────────────────────────────

    #[test]
    fn pure_sale_no_ads_correct_signs() {
        let row = P903Row {
            srid: "S001".to_string(),
            supplier_oper_name: Some(OP_SALE.to_string()),
            retail_amount: 1000.0,
            acquiring_fee: 20.0,
            ppvz_vw: 100.0,
            ppvz_vw_nds: 10.0,
            ppvz_sales_commission: 5.0,
            delivery_rub: 50.0,
            rebill_logistic_cost: 10.0,
            storage_fee: 5.0,
            qty_sold: 1,
            ..P903Row::default()
        };
        let input = LineComputeInput {
            all_a012_for_srid: vec![A012Row {
                id: "a012-1".to_string(),
                document_no: "S001".to_string(),
                sale_id: None,
                sale_date: "2026-05-15".to_string(),
                line_amount: 600.0,
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }],
            matched_a012: Some(A012Row {
                id: "a012-1".to_string(),
                document_no: "S001".to_string(),
                sale_id: None,
                sale_date: "2026-05-15".to_string(),
                line_amount: 600.0,
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }),
            dealer_total: 600.0,
            order: Some(A015Row {
                id: "a015-1".to_string(),
                order_date: "2026-05-15".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row)
        };
        let (line, problems) = compute_line_and_problems(input);

        assert_eq!(line.kind, LineKind::Sale);
        assert_eq!(line.revenue, 1000.0);
        assert_eq!(line.acquiring, -20.0);
        // ppvz_sales_commission исключается для Sale — только ppvz_vw + ppvz_vw_nds
        assert_eq!(line.commission, -110.0);
        assert_eq!(line.logistics, -65.0);
        assert_eq!(line.advertising, 0.0);
        assert!(line.check_invariant(), "invariant must hold");
        assert!(!problems
            .iter()
            .any(|p| p.code == "column_invariant_mismatch"));
        assert_eq!(line.order_date.as_deref(), Some("2026-05-15"));
        assert_eq!(line.sales_event_type.as_deref(), Some("sale"));
    }

    #[test]
    fn return_event_type() {
        // Стандартный формат: return_amount > 0, retail_amount = 0.
        let row = p903_return("R001", 500.0);
        let input = LineComputeInput {
            order: Some(A015Row {
                id: "a015-r".to_string(),
                order_date: "2026-05-14".to_string(),
                is_cancel: false,
            }),
            matched_a012: Some(A012Row {
                id: "a012-r".to_string(),
                document_no: "R001".to_string(),
                sale_id: None,
                sale_date: "2026-05-15".to_string(),
                line_amount: -400.0,
                dealer_total: 400.0,
                is_posted: true,
                event_type: "return".to_string(),
            }),
            all_a012_for_srid: vec![A012Row {
                id: "a012-r".to_string(),
                document_no: "R001".to_string(),
                sale_id: None,
                sale_date: "2026-05-15".to_string(),
                line_amount: -400.0,
                dealer_total: 400.0,
                is_posted: true,
                event_type: "return".to_string(),
            }],
            dealer_total: 400.0,
            ..default_input(&row)
        };
        let (line, _) = compute_line_and_problems(input);
        assert_eq!(line.kind, LineKind::Return);
        assert_eq!(line.event, SaleEvent::Return);
        // revenue должна быть отрицательной (возврат денег покупателю)
        assert_eq!(line.revenue, -500.0);
        assert!(line.check_invariant());
    }

    /// WB-стиль возврата: retail_amount > 0, return_amount = 0, supplier_oper_name = 'Возврат'.
    /// Пример — строки ebF и eF из финансового отчёта 26.05.2026.
    ///
    /// Ожидаемые знаки:
    ///   revenue   < 0  (возврат выручки покупателю)
    ///   acquiring > 0  (WB возвращает продавцу комиссию эквайрера)
    ///   commission > 0 если ppvz > 0  (WB возвращает комиссию)
    ///   commission < 0 если ppvz < 0  (WB берёт обратно соинвест)
    #[test]
    fn wb_style_return_retail_amount_positive_ppvz() {
        // Аналог строки ebF.rebb9e... (ppvz > 0 — WB возвращает комиссию).
        let row = P903Row {
            srid: "ebF-test".to_string(),
            supplier_oper_name: Some(OP_RETURN.to_string()),
            retail_amount: 4658.0,
            return_amount: 0.0,
            acquiring_fee: 186.32,
            ppvz_vw: 128.99,
            ppvz_vw_nds: 28.38,
            ppvz_sales_commission: 137.43,
            qty_returned: 1,
            ..P903Row::default()
        };
        let input = LineComputeInput {
            order: Some(A015Row {
                id: "a015-ebF".to_string(),
                order_date: "2026-05-20".to_string(),
                is_cancel: false,
            }),
            matched_a012: Some(A012Row {
                id: "a012-ebF".to_string(),
                document_no: "ebF-test".to_string(),
                sale_id: None,
                sale_date: "2026-05-26".to_string(),
                line_amount: -4658.0,
                dealer_total: 800.0,
                is_posted: true,
                event_type: "return".to_string(),
            }),
            all_a012_for_srid: vec![A012Row {
                id: "a012-ebF".to_string(),
                document_no: "ebF-test".to_string(),
                sale_id: None,
                sale_date: "2026-05-26".to_string(),
                line_amount: -4658.0,
                dealer_total: 800.0,
                is_posted: true,
                event_type: "return".to_string(),
            }],
            dealer_total: 800.0,
            ..default_input(&row)
        };
        let (line, problems) = compute_line_and_problems(input);

        assert_eq!(line.kind, LineKind::Return);
        // revenue отрицательная (WB хранит сумму в retail_amount, знак инвертируем)
        assert_eq!(line.revenue, -4658.0);
        // acquiring положительный (WB возвращает комиссию эквайрера)
        assert!(
            line.acquiring > 0.0,
            "acquiring должна быть > 0 для возврата"
        );
        assert!((line.acquiring - 186.32).abs() < 0.01);
        // commission положительная (ppvz > 0 → WB возвращает комиссию продавцу)
        // ppvz_sales_commission исключается для Return — только ppvz_vw + ppvz_vw_nds
        assert!(
            line.commission > 0.0,
            "commission должна быть > 0 когда ppvz > 0 в возврате"
        );
        assert!((line.commission - (128.99 + 28.38)).abs() < 0.01);
        // dealer_price положительная (товар возвращается продавцу → себестоимость обратно)
        assert!(
            line.dealer_price > 0.0,
            "dealer_price должна быть > 0 для возврата"
        );
        assert!((line.dealer_price - 800.0).abs() < 0.01);
        // инвариант сохраняется
        assert!(line.check_invariant(), "инвариант должен сохраняться");
        assert!(!problems
            .iter()
            .any(|p| p.code == "column_invariant_mismatch"));
    }

    /// WB-стиль возврата: retail_amount > 0, return_amount = 0, ppvz < 0 (сторно соинвеста).
    /// Пример — строка eF.r8f3f83... из финансового отчёта 26.05.2026.
    ///
    /// Отрицательный ppvz означает, что WB берёт обратно соинвест (premium-commission),
    /// поэтому commission остаётся отрицательной — дополнительный расход продавца.
    #[test]
    fn wb_style_return_retail_amount_negative_ppvz() {
        // Аналог строки eF.r8f3f83... (ppvz < 0 — WB берёт обратно соинвест).
        let row = P903Row {
            srid: "eF-test".to_string(),
            supplier_oper_name: Some(OP_RETURN.to_string()),
            retail_amount: 17623.0,
            return_amount: 0.0,
            acquiring_fee: 860.0,
            ppvz_vw: -807.85,
            ppvz_vw_nds: -177.73,
            ppvz_sales_commission: -807.85,
            qty_returned: 1,
            ..P903Row::default()
        };
        let input = LineComputeInput {
            order: Some(A015Row {
                id: "a015-eF".to_string(),
                order_date: "2026-05-10".to_string(),
                is_cancel: false,
            }),
            matched_a012: Some(A012Row {
                id: "a012-eF".to_string(),
                document_no: "eF-test".to_string(),
                sale_id: None,
                sale_date: "2026-05-26".to_string(),
                line_amount: -17623.0,
                dealer_total: 3000.0,
                is_posted: true,
                event_type: "return".to_string(),
            }),
            all_a012_for_srid: vec![A012Row {
                id: "a012-eF".to_string(),
                document_no: "eF-test".to_string(),
                sale_id: None,
                sale_date: "2026-05-26".to_string(),
                line_amount: -17623.0,
                dealer_total: 3000.0,
                is_posted: true,
                event_type: "return".to_string(),
            }],
            dealer_total: 3000.0,
            ..default_input(&row)
        };
        let (line, _) = compute_line_and_problems(input);

        assert_eq!(line.kind, LineKind::Return);
        // revenue отрицательная
        assert_eq!(line.revenue, -17623.0);
        // acquiring положительный (WB всегда возвращает комиссию эквайрера при возврате)
        assert!(
            line.acquiring > 0.0,
            "acquiring должна быть > 0 для возврата"
        );
        assert!((line.acquiring - 860.0).abs() < 0.01);
        // commission отрицательная (ppvz < 0 → WB берёт обратно соинвест → расход)
        // ppvz_sales_commission исключается для Return — только ppvz_vw + ppvz_vw_nds
        let expected_commission = -807.85 + (-177.73);
        assert!(
            line.commission < 0.0,
            "commission должна быть < 0 когда ppvz < 0 в возврате (сторно соинвеста)"
        );
        assert!((line.commission - expected_commission).abs() < 0.01);
        // dealer_price положительная (товар возвращается продавцу)
        assert!(
            line.dealer_price > 0.0,
            "dealer_price должна быть > 0 для возврата"
        );
        assert!((line.dealer_price - 3000.0).abs() < 0.01);
        // инвариант сохраняется
        assert!(line.check_invariant(), "инвариант должен сохраняться");
    }

    /// Суммарная проверка за день 26.05.2026:
    /// acquiring двух возвратных строк должен быть положительным (сторно расхода),
    /// что приведёт суммарный эквайринг дня к значению ≈ −45 963 вместо −48 055.
    #[test]
    fn return_acquiring_reduces_day_total() {
        // Возврат 1: acquiring_fee = 186.32, ppvz > 0
        let row1 = P903Row {
            srid: "R1".to_string(),
            supplier_oper_name: Some(OP_RETURN.to_string()),
            retail_amount: 4658.0,
            acquiring_fee: 186.32,
            ppvz_vw: 128.99,
            ppvz_vw_nds: 28.38,
            ppvz_sales_commission: 137.43,
            qty_returned: 1,
            ..P903Row::default()
        };
        // Возврат 2: acquiring_fee = 860.0, ppvz < 0
        let row2 = P903Row {
            srid: "R2".to_string(),
            supplier_oper_name: Some(OP_RETURN.to_string()),
            retail_amount: 17623.0,
            acquiring_fee: 860.0,
            ppvz_vw: -807.85,
            ppvz_vw_nds: -177.73,
            ppvz_sales_commission: -807.85,
            qty_returned: 1,
            ..P903Row::default()
        };
        let (line1, _) = compute_line_and_problems(LineComputeInput {
            order: Some(A015Row {
                id: "o1".to_string(),
                order_date: "2026-05-20".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row1)
        });
        let (line2, _) = compute_line_and_problems(LineComputeInput {
            order: Some(A015Row {
                id: "o2".to_string(),
                order_date: "2026-05-10".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row2)
        });

        // Оба возврата дают положительный acquiring (сторно расхода)
        assert!(
            line1.acquiring > 0.0,
            "acquiring возврата 1 должен быть > 0"
        );
        assert!(
            line2.acquiring > 0.0,
            "acquiring возврата 2 должен быть > 0"
        );
        // Суммарный acquiring по двум строкам = +(186.32 + 860.0) = +1046.32
        let total_return_acquiring = line1.acquiring + line2.acquiring;
        assert!((total_return_acquiring - 1046.32).abs() < 0.01);
    }

    #[test]
    fn storage_row_no_a015_no_a012_required() {
        let row = p903_storage();
        let (line, problems) = compute_line_and_problems(default_input(&row));
        assert_eq!(line.kind, LineKind::Storage);
        // Для хранения a015/a012 не требуются — этих проблем быть не должно
        assert!(!problems.iter().any(|p| p.code == "a015_order_missing"));
        assert!(!problems.iter().any(|p| p.code == "a012_sale_missing"));
        assert!(!problems.iter().any(|p| p.code == "dealer_price_missing"));
        assert!(line.check_invariant());
    }

    // ── Новые проблемы ────────────────────────────────────────────────────────

    #[test]
    fn a015_missing_for_sale_generates_warn() {
        let row = p903_sale("S-NO-A015", 800.0);
        let (_, problems) = compute_line_and_problems(default_input(&row));
        assert!(
            problems.iter().any(|p| p.code == "a015_order_missing"),
            "должна быть проблема a015_order_missing"
        );
    }

    #[test]
    fn a012_sale_missing_generates_warn() {
        let row = p903_sale("S-NO-A012", 800.0);
        let input = LineComputeInput {
            order: Some(A015Row {
                id: "a015-x".to_string(),
                order_date: "2026-05-15".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row)
        };
        let (_, problems) = compute_line_and_problems(input);
        assert!(
            problems.iter().any(|p| p.code == "a012_sale_missing"),
            "должна быть проблема a012_sale_missing"
        );
    }

    #[test]
    fn a012_return_missing_generates_warn() {
        let row = p903_return("R-NO-A012", 500.0);
        let input = LineComputeInput {
            order: Some(A015Row {
                id: "a015-r".to_string(),
                order_date: "2026-05-15".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row)
        };
        let (_, problems) = compute_line_and_problems(input);
        assert!(
            problems.iter().any(|p| p.code == "a012_return_missing"),
            "должна быть проблема a012_return_missing"
        );
    }

    #[test]
    fn a012_sale_date_mismatch_fin_report_when_sale_date_differs() {
        let row = p903_sale("S-DATE-MISMATCH", 1000.0);
        let input = LineComputeInput {
            matched_a012: Some(A012Row {
                id: "a012-dm".to_string(),
                document_no: "S-DATE-MISMATCH".to_string(),
                sale_id: None,
                sale_date: "2026-05-16".to_string(),
                line_amount: 600.0,
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }),
            dealer_total: 600.0,
            ..default_input(&row)
        };
        let (_, problems) = compute_line_and_problems(input);
        let prob = problems
            .iter()
            .find(|p| p.code == "a012_sale_date_mismatch_fin_report")
            .expect("должна быть проблема a012_sale_date_mismatch_fin_report");
        assert_eq!(prob.severity, ProblemSeverity::Warn);
        assert_eq!(prob.a012_ids, vec!["a012-dm".to_string()]);
        assert!(prob.message.contains("2026-05-16"));
        assert!(prob.message.contains("2026-05-17"));
    }

    #[test]
    fn dedupe_fin_date_mismatch_by_a012_id() {
        let problems = vec![
            WbDayCloseProblem {
                code: "a012_sale_date_mismatch_fin_report".to_string(),
                severity: ProblemSeverity::Warn,
                srid: Some("S1".to_string()),
                nomenclature_ref: None,
                a012_ids: vec!["a012-1".to_string()],
                a012_advert_expense: Some(100.0),
                message: "first".to_string(),
            },
            WbDayCloseProblem {
                code: "a012_sale_date_mismatch_fin_report".to_string(),
                severity: ProblemSeverity::Warn,
                srid: Some("S1".to_string()),
                nomenclature_ref: None,
                a012_ids: vec!["a012-1".to_string()],
                a012_advert_expense: Some(100.0),
                message: "duplicate".to_string(),
            },
            WbDayCloseProblem {
                code: "a015_order_missing".to_string(),
                severity: ProblemSeverity::Warn,
                srid: Some("S2".to_string()),
                nomenclature_ref: None,
                a012_ids: vec![],
                a012_advert_expense: None,
                message: "other".to_string(),
            },
        ];
        let deduped = dedupe_fin_date_mismatch_problems(problems);
        assert_eq!(deduped.len(), 2);
        assert_eq!(
            deduped
                .iter()
                .filter(|p| p.code == "a012_sale_date_mismatch_fin_report")
                .count(),
            1
        );
    }

    #[test]
    fn multiple_a012_generates_block() {
        let row = p903_sale("S-MULTI", 1000.0);
        let a012_1 = A012Row {
            id: "a012-1".to_string(),
            document_no: "S-MULTI".to_string(),
            sale_id: None,
            sale_date: "2026-05-15".to_string(),
            line_amount: 600.0,
            dealer_total: 600.0,
            is_posted: true,
            event_type: "sale".to_string(),
        };
        let a012_2 = A012Row {
            id: "a012-2".to_string(),
            document_no: "S-MULTI".to_string(),
            sale_id: None,
            sale_date: "2026-05-15".to_string(),
            line_amount: 600.0,
            dealer_total: 600.0,
            is_posted: true,
            event_type: "sale".to_string(),
        };
        let input = LineComputeInput {
            matched_a012: Some(a012_1.clone()),
            extra_a012_ids: vec![a012_2.id.clone()],
            all_a012_for_srid: vec![a012_1, a012_2],
            dealer_total: 1200.0,
            order: Some(A015Row {
                id: "a015-m".to_string(),
                order_date: "2026-05-15".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row)
        };
        let (line, problems) = compute_line_and_problems(input);
        let prob = problems
            .iter()
            .find(|p| p.code == "multiple_a012_for_srid")
            .expect("должна быть проблема multiple_a012_for_srid");
        assert_eq!(prob.severity, ProblemSeverity::Block);
        assert_eq!(line.sales_extra_ids.len(), 1);
    }

    #[test]
    fn mixed_sale_return_generates_block() {
        let row = P903Row {
            srid: "MIX-001".to_string(),
            supplier_oper_name: Some(OP_SALE.to_string()),
            retail_amount: 1000.0,
            return_amount: 300.0,
            qty_sold: 2,
            qty_returned: 1,
            ..P903Row::default()
        };
        let input = LineComputeInput {
            order: Some(A015Row {
                id: "a015-mix".to_string(),
                order_date: "2026-05-15".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row)
        };
        let (_, problems) = compute_line_and_problems(input);
        assert!(
            problems
                .iter()
                .any(|p| p.code == "mixed_sale_and_return_for_srid"),
            "должна быть проблема mixed_sale_and_return_for_srid"
        );
    }

    #[test]
    fn advert_expense_from_matched_a012_uses_registrator() {
        let mut expense_map = HashMap::new();
        expense_map.insert("a012-1".to_string(), 1722.82);
        let a012 = A012Row {
            id: "a012-1".to_string(),
            document_no: "SRID-1".to_string(),
            sale_id: None,
            sale_date: "2026-05-16".to_string(),
            line_amount: 1000.0,
            dealer_total: 500.0,
            is_posted: true,
            event_type: "sale".to_string(),
        };
        assert_eq!(
            advert_expense_from_matched_a012(&Some(a012), &expense_map),
            1722.82
        );
        assert_eq!(advert_expense_from_matched_a012(&None, &expense_map), 0.0);
    }

    #[test]
    fn advert_expense_zero_without_matched_a012() {
        let expense_map = HashMap::new();
        assert_eq!(advert_expense_from_matched_a012(&None, &expense_map), 0.0);
    }

    // ── Реклама ───────────────────────────────────────────────────────────────

    #[test]
    fn sale_with_reserve_and_expense_no_problem() {
        let row = p903_sale("SA001", 1000.0);
        let input = LineComputeInput {
            advert_expense: 50.0,
            advert_order_expense: 50.0,
            advert_reserve: 50.0,
            matched_a012: Some(A012Row {
                id: "a012-sa".to_string(),
                document_no: "SA001".to_string(),
                sale_id: None,
                sale_date: "2026-05-15".to_string(),
                line_amount: 600.0,
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }),
            all_a012_for_srid: vec![A012Row {
                id: "a012-sa".to_string(),
                document_no: "SA001".to_string(),
                sale_id: None,
                sale_date: "2026-05-15".to_string(),
                line_amount: 600.0,
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }],
            dealer_total: 600.0,
            order: Some(A015Row {
                id: "a015-sa".to_string(),
                order_date: "2026-05-15".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row)
        };
        let (line, problems) = compute_line_and_problems(input);
        assert_eq!(line.advertising, -50.0);
        assert!(!problems
            .iter()
            .any(|p| p.code == "advert_clicks_order_accrual_without_expense"));
        assert!(line.check_invariant());
    }

    #[test]
    fn sale_with_reserve_without_expense_generates_block_problem() {
        let row = p903_sale("SB001", 1000.0);
        let input = LineComputeInput {
            advert_expense: 0.0,
            advert_order_expense: 0.0,
            advert_reserve: 75.0,
            matched_a012: Some(A012Row {
                id: "a012-uuid-1".to_string(),
                document_no: "SB001".to_string(),
                sale_id: None,
                sale_date: "2026-05-15".to_string(),
                line_amount: 600.0,
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }),
            all_a012_for_srid: vec![A012Row {
                id: "a012-uuid-1".to_string(),
                document_no: "SB001".to_string(),
                sale_id: None,
                sale_date: "2026-05-15".to_string(),
                line_amount: 600.0,
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }],
            dealer_total: 600.0,
            order: Some(A015Row {
                id: "a015-sb".to_string(),
                order_date: "2026-05-15".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row)
        };
        let (line, problems) = compute_line_and_problems(input);
        assert_eq!(line.advertising, 0.0);
        let prob = problems
            .iter()
            .find(|p| p.code == "advert_clicks_order_accrual_without_expense")
            .expect("must have advert_clicks_order_accrual_without_expense problem");
        assert_eq!(prob.severity, ProblemSeverity::Block);
        assert!(line
            .problem_codes
            .contains(&"advert_clicks_order_accrual_without_expense".to_string()));
    }

    #[test]
    fn cancelled_order_with_reserve_generates_problem() {
        let row = p903_return("CANCEL-001", 200.0);
        let input = LineComputeInput {
            advert_reserve: 30.0,
            order: Some(A015Row {
                id: "a015-cancel".to_string(),
                order_date: "2026-05-14".to_string(),
                is_cancel: true,
            }),
            ..default_input(&row)
        };
        let (_, problems) = compute_line_and_problems(input);
        assert!(problems
            .iter()
            .any(|p| p.code == "advert_attributed_to_cancelled_order"),);
    }

    // ── Дилерская цена ────────────────────────────────────────────────────────

    #[test]
    fn dealer_price_present_fills_column() {
        let row = p903_sale("D001", 1000.0);
        let input = LineComputeInput {
            dealer_total: 600.0,
            matched_a012: Some(A012Row {
                id: "a012-d".to_string(),
                document_no: "D001".to_string(),
                sale_id: None,
                sale_date: "2026-05-15".to_string(),
                line_amount: 600.0,
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }),
            all_a012_for_srid: vec![A012Row {
                id: "a012-d".to_string(),
                document_no: "D001".to_string(),
                sale_id: None,
                sale_date: "2026-05-15".to_string(),
                line_amount: 600.0,
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }],
            order: Some(A015Row {
                id: "a015-d".to_string(),
                order_date: "2026-05-15".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row)
        };
        let (line, problems) = compute_line_and_problems(input);
        assert_eq!(line.dealer_price, -600.0);
        assert!(!problems.iter().any(|p| p.code == "dealer_price_missing"));
    }

    #[test]
    fn dealer_price_missing_generates_warn_for_sale() {
        let row = p903_sale("D002", 1000.0);
        let input = LineComputeInput {
            order: Some(A015Row {
                id: "a015-d2".to_string(),
                order_date: "2026-05-15".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row)
        };
        let (_, problems) = compute_line_and_problems(input);
        assert!(problems.iter().any(|p| p.code == "dealer_price_missing"),);
    }

    #[test]
    fn dealer_price_missing_when_a012_present_but_no_dealer() {
        let row = p903_sale("D003", 1000.0);
        let a012 = A012Row {
            id: "a012-d3".to_string(),
            document_no: "D003".to_string(),
            sale_id: None,
            sale_date: "2026-05-15".to_string(),
            line_amount: 600.0,
            dealer_total: 0.0,
            is_posted: true,
            event_type: "sale".to_string(),
        };
        let input = LineComputeInput {
            matched_a012: Some(a012.clone()),
            all_a012_for_srid: vec![a012],
            order: Some(A015Row {
                id: "a015-d3".to_string(),
                order_date: "2026-05-15".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row)
        };
        let (_, problems) = compute_line_and_problems(input);
        assert!(
            problems.iter().any(|p| p.code == "dealer_price_missing"),
            "должна быть проблема dealer_price_missing даже при наличии a012"
        );
    }

    #[test]
    fn info_row_generates_no_problems() {
        let row = P903Row {
            srid: "INFO001".to_string(),
            supplier_oper_name: Some("Неизвестная операция".to_string()),
            ..P903Row::default()
        };
        let (line, problems) = compute_line_and_problems(default_input(&row));
        assert_eq!(line.kind, LineKind::Info);
        assert!(
            problems.is_empty(),
            "информационные строки не должны генерировать проблемы"
        );
        assert!(line.problem_codes.is_empty());
    }

    #[test]
    fn dealer_price_missing_not_generated_for_storage() {
        let row = p903_storage();
        let (_, problems) = compute_line_and_problems(default_input(&row));
        assert!(
            !problems.iter().any(|p| p.code == "dealer_price_missing"),
            "для хранения dealer_price_missing не должен генерироваться"
        );
    }

    // ── Непроведённый a012 ───────────────────────────────────────────────────

    #[test]
    fn unposted_a012_generates_warn() {
        let row = p903_sale("U001", 800.0);
        let a012 = A012Row {
            id: "a012-unposted-1".to_string(),
            document_no: "U001".to_string(),
            sale_id: None,
            sale_date: "2026-05-15".to_string(),
            line_amount: 400.0,
            dealer_total: 400.0,
            is_posted: false,
            event_type: "sale".to_string(),
        };
        let input = LineComputeInput {
            dealer_total: 400.0,
            matched_a012: Some(a012.clone()),
            all_a012_for_srid: vec![a012],
            order: Some(A015Row {
                id: "a015-u".to_string(),
                order_date: "2026-05-15".to_string(),
                is_cancel: false,
            }),
            ..default_input(&row)
        };
        let (_, problems) = compute_line_and_problems(input);
        let prob = problems
            .iter()
            .find(|p| p.code == "a012_unposted_for_p903_row")
            .expect("must warn for unposted a012");
        assert_eq!(prob.severity, ProblemSeverity::Warn);
        assert!(prob.a012_ids.contains(&"a012-unposted-1".to_string()));
    }

    // ── Инвариант ─────────────────────────────────────────────────────────────

    #[test]
    fn invariant_holds_for_various_inputs() {
        let cases: Vec<P903Row> = vec![
            P903Row {
                srid: "S-inv-1".to_string(),
                supplier_oper_name: Some(OP_SALE.to_string()),
                retail_amount: 500.0,
                return_amount: 0.0,
                acquiring_fee: 10.0,
                ppvz_vw: 50.0,
                ppvz_vw_nds: 5.0,
                ppvz_sales_commission: 2.0,
                delivery_rub: 30.0,
                rebill_logistic_cost: 5.0,
                storage_fee: 3.0,
                penalty: 0.0,
                additional_payment: 20.0,
                cashback_amount: 5.0,
                qty_sold: 1,
                ..P903Row::default()
            },
            P903Row {
                srid: "S-inv-2".to_string(),
                supplier_oper_name: Some(OP_RETURN.to_string()),
                return_amount: 300.0,
                ppvz_vw: -50.0,
                qty_returned: 1,
                ..P903Row::default()
            },
        ];
        for row in &cases {
            let input = LineComputeInput {
                advert_expense: 15.0,
                advert_order_expense: 15.0,
                dealer_total: 200.0,
                ..default_input(row)
            };
            let (line, _) = compute_line_and_problems(input);
            assert!(
                line.check_invariant(),
                "invariant must hold for srid={}",
                row.srid
            );
        }
    }

    // ── problem_lines в totals ────────────────────────────────────────────────

    #[test]
    fn totals_counts_problem_lines_correctly() {
        use contracts::domain::a033_wb_day_close::{WbDayCloseLine, WbDayCloseTotals};

        let make_line = |srid: &str, problem_codes: Vec<String>| -> WbDayCloseLine {
            WbDayCloseLine {
                srid: srid.to_string(),
                result: 100.0,
                problem_codes,
                ..Default::default()
            }
        };

        let lines = vec![
            make_line("S001", vec!["a015_order_missing".to_string()]),
            make_line("S002", vec![]),
            make_line(
                "S003",
                vec![
                    "a012_sale_missing".to_string(),
                    "dealer_price_missing".to_string(),
                ],
            ),
        ];
        let totals = WbDayCloseTotals::from_lines(&lines, &[]);
        assert_eq!(totals.lines_count, 3);
        assert_eq!(totals.problem_lines, 2, "должно быть 2 строки с проблемами");
    }
}
