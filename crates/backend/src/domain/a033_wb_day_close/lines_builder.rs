/// Строитель строк для a033_wb_day_close.
///
/// Шаги:
///   1. Агрегация p903_wb_finance_report по (srid, nm_id, nomenclature_ref, sa_name, supplier_oper_name)
///      Включает строки БЕЗ srid (хранение, штрафы, возмещение ПВЗ, приёмка).
///   2. p913_wb_advert_order_attr — фактически списанная реклама + детектор «резерв без expense»
///   3. a012_wb_sales — dealer_price_ut × qty per (srid, event_type); fallback на p912_nomenclature_costs
///   4. a015_wb_orders — дата и статус отмены заказа
///   5. In-memory join, классификация LineKind/LineDetail, вычисление 10 колонок
use anyhow::Result;
use contracts::domain::a033_wb_day_close::aggregate::{
    LineDetail, LineKind, ProblemSeverity, SaleEvent, WbDayCloseLine, WbDayCloseProblem,
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
    pub dealer_total: f64,
    pub is_posted: bool,
    pub event_type: String,
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

async fn fetch_p903_rows(connection_id: &str, business_date: &str) -> Result<Vec<P903Row>> {
    // MIN(supplier_oper_name) для классификатора; COUNT(DISTINCT ...) для kind_ambiguous.
    // Строки с пустым/NULL srid включаются.
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

// ─────────────────────────────────────────────────────────────────────────────
// Шаг 3: a012 per srid (по всем event_type)
// ─────────────────────────────────────────────────────────────────────────────

async fn fetch_a012_for_srids(
    connection_id: &str,
    business_date: &str,
    srids: &[String],
) -> Result<HashMap<String, Vec<A012Row>>> {
    if srids.is_empty() {
        return Ok(HashMap::new());
    }

    let date_prefix = format!("{}%", business_date);
    let placeholders = srids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

    let sql = format!(
        r#"
        SELECT
            id,
            document_no,
            sale_id,
            COALESCE(dealer_price_ut, 0.0) * COALESCE(ABS(qty), 1.0) AS dealer_total,
            is_posted,
            COALESCE(event_type, 'sale') AS event_type
        FROM a012_wb_sales
        WHERE connection_id = ?
          AND sale_date LIKE ?
          AND document_no IN ({})
          AND is_deleted = 0
        "#,
        placeholders
    );

    let mut params: Vec<Value> = vec![sv(connection_id), sv(date_prefix)];
    params.extend(srids.iter().map(|s| sv(s)));

    let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
    let rows = conn().query_all(stmt).await?;

    let mut map: HashMap<String, Vec<A012Row>> = HashMap::new();
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
            dealer_total: row.try_get("", "dealer_total").unwrap_or(0.0),
            is_posted: row.try_get("", "is_posted").unwrap_or(false),
            event_type,
        });
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
// Шаг 5: p912 fallback dealer price
// ─────────────────────────────────────────────────────────────────────────────

async fn fetch_p912_dealer_price(
    nomenclature_ref: &str,
    business_date: &str,
) -> Result<Option<f64>> {
    let sql = r#"
        SELECT cost
        FROM p912_nomenclature_costs
        WHERE nomenclature_ref = ?
          AND effective_from <= ?
        ORDER BY effective_from DESC
        LIMIT 1
    "#;

    let stmt = Statement::from_sql_and_values(
        conn().get_database_backend(),
        sql,
        vec![sv(nomenclature_ref), sv(business_date)],
    );

    let row = conn().query_one(stmt).await?;
    Ok(row.and_then(|r| r.try_get::<f64>("", "cost").ok()))
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
    /// Фактически списанная реклама из p913 (turnover_code='advert_clicks_order_expense').
    pub advert_expense: f64,
    /// Зарезервированная реклама из p913 (turnover_code='advert_clicks_order_accrual').
    pub advert_reserve: f64,
    /// Суммарная дилерская стоимость (dealer_price_ut × qty) из a012 или p912.
    pub dealer_total: f64,
    /// Документ a012, соответствующий типу строки (sale или return).
    pub matched_a012: Option<A012Row>,
    /// Лишние a012 того же типа для этого srid (должен быть 0, иначе проблема).
    pub extra_a012_ids: Vec<String>,
    /// Все a012 для srid независимо от event_type (для детектора unposted).
    pub all_a012_for_srid: Vec<A012Row>,
    /// Данные заказа из a015.
    pub order: Option<A015Row>,
}

/// Вычисляет строку документа и список проблем для одного srid.
/// Чистая функция: не обращается к DB.
pub(crate) fn compute_line_and_problems(
    input: LineComputeInput<'_>,
) -> (WbDayCloseLine, Vec<WbDayCloseProblem>) {
    let p903 = input.p903;
    let srid = &p903.srid;

    // ── 10 колонок (доход +, расход −) ───────────────────────────────────────
    let revenue = p903.retail_amount - p903.return_amount;
    let advertising = -input.advert_expense;
    let logistics = -(p903.delivery_rub + p903.rebill_logistic_cost + p903.storage_fee);
    let acquiring = -p903.acquiring_fee;
    let commission = -(p903.ppvz_vw + p903.ppvz_vw_nds + p903.ppvz_sales_commission);
    let penalty = -p903.penalty;
    let other = p903.additional_payment + p903.cashback_amount;
    let result = revenue + advertising + logistics + acquiring + commission + penalty + other;
    let dealer_price = if input.dealer_total > 0.0 {
        -input.dealer_total
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

    let (sales_doc_id, sales_doc_no, sales_event_type, sales_sale_id) = match &input.matched_a012 {
        Some(a) => (
            Some(a.id.clone()),
            Some(a.document_no.clone()),
            Some(a.event_type.clone()),
            a.sale_id.clone(),
        ),
        None => (None, None, None, None),
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

    // ── Проблемы ─────────────────────────────────────────────────────────────
    let mut line_problem_codes: Vec<String> = Vec::new();
    let mut problems: Vec<WbDayCloseProblem> = Vec::new();

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
            a012_ids,
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

    // 3. Резерв без expense
    if input.advert_reserve > 0.0 && input.advert_expense == 0.0 {
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
            format!("srid={}: не найден a012 с event_type=sale", srid),
            vec![],
        ));
    }
    if kind == LineKind::Return && input.matched_a012.is_none() && !srid.is_empty() {
        line_problem_codes.push("a012_return_missing".to_string());
        problems.push(make_problem(
            "a012_return_missing",
            format!("srid={}: не найден a012 с event_type=return", srid),
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

    // 9. Отсутствует дилерская цена (только для Sale/Return)
    if kind.requires_sales_doc() && input.dealer_total == 0.0 && input.all_a012_for_srid.is_empty()
    {
        line_problem_codes.push("dealer_price_missing".to_string());
        problems.push(make_problem(
            "dealer_price_missing",
            format!("srid={}: dealer_price не найден", srid),
            vec![],
        ));
    }

    // 10. Инвариант колонок
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

/// Собирает строки и проблемы для документа a033.
pub async fn build(
    connection_id: &str,
    business_date: &str,
) -> Result<(Vec<WbDayCloseLine>, Vec<WbDayCloseProblem>)> {
    // Шаг 1: p903 агрегация (включает строки без srid)
    let p903_rows = fetch_p903_rows(connection_id, business_date).await?;
    if p903_rows.is_empty() {
        return Ok((vec![], vec![]));
    }

    // Собираем непустые srids для последующих запросов
    let srids: Vec<String> = p903_rows
        .iter()
        .map(|r| r.srid.clone())
        .filter(|s| !s.is_empty())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Шаги 2-4 параллельно
    let p913_map = fetch_p913_advert(&srids).await?;
    let a012_map = fetch_a012_for_srids(connection_id, business_date, &srids).await?;
    let a015_map = fetch_a015_for_srids(&srids).await?;

    let mut lines: Vec<WbDayCloseLine> = Vec::with_capacity(p903_rows.len());
    let mut problems: Vec<WbDayCloseProblem> = Vec::new();

    for p903 in &p903_rows {
        let srid = &p903.srid;

        // ── Классификация ─────────────────────────────────────────────────────
        let (kind, unknown_flag) = classify_kind(p903);
        let detail = classify_detail(p903);
        let kind_ambiguous = p903.kind_ambiguous || unknown_flag;

        // ── Реклама ───────────────────────────────────────────────────────────
        let advert_info = p913_map.get(srid.as_str());
        let advert_expense = advert_info.map(|a| a.advert_expense).unwrap_or(0.0);
        let advert_reserve = advert_info.map(|a| a.advert_reserve).unwrap_or(0.0);

        // ── a012 по типу строки ───────────────────────────────────────────────
        let all_a012 = a012_map.get(srid.as_str()).cloned().unwrap_or_default();

        let expected_event_type = match kind {
            LineKind::Sale => "sale",
            LineKind::Return => "return",
            _ => "",
        };

        let mut matched_a012_docs: Vec<A012Row> = if expected_event_type.is_empty() {
            vec![]
        } else {
            all_a012
                .iter()
                .filter(|r| r.event_type.eq_ignore_ascii_case(expected_event_type))
                .cloned()
                .collect()
        };

        let matched_a012 = if !matched_a012_docs.is_empty() {
            Some(matched_a012_docs.remove(0))
        } else {
            None
        };
        let extra_a012_ids: Vec<String> = matched_a012_docs.iter().map(|r| r.id.clone()).collect();

        // ── Dealer price ──────────────────────────────────────────────────────
        let dealer_total = if !all_a012.is_empty() {
            all_a012.iter().map(|r| r.dealer_total).sum()
        } else {
            // Fallback: p912
            if let Some(nr) = &p903.nomenclature_ref {
                fetch_p912_dealer_price(nr, business_date)
                    .await
                    .unwrap_or(None)
                    .unwrap_or(0.0)
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
            advert_reserve,
            dealer_total,
            matched_a012,
            extra_a012_ids,
            all_a012_for_srid: all_a012,
            order,
        };

        let (line, line_problems) = compute_line_and_problems(input);
        problems.extend(line_problems);
        lines.push(line);
    }

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
        let (kind, unknown) = classify_kind(row);
        let detail = classify_detail(row);
        LineComputeInput {
            p903: row,
            kind,
            detail,
            kind_ambiguous: unknown,
            advert_expense: 0.0,
            advert_reserve: 0.0,
            dealer_total: 0.0,
            matched_a012: None,
            extra_a012_ids: vec![],
            all_a012_for_srid: vec![],
            order: None,
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
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }],
            matched_a012: Some(A012Row {
                id: "a012-1".to_string(),
                document_no: "S001".to_string(),
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
        assert_eq!(line.commission, -115.0);
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
                dealer_total: 400.0,
                is_posted: true,
                event_type: "return".to_string(),
            }),
            all_a012_for_srid: vec![A012Row {
                id: "a012-r".to_string(),
                document_no: "R001".to_string(),
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
        assert_eq!(line.revenue, -500.0);
        assert!(line.check_invariant());
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
    fn multiple_a012_generates_block() {
        let row = p903_sale("S-MULTI", 1000.0);
        let a012_1 = A012Row {
            id: "a012-1".to_string(),
            document_no: "S-MULTI".to_string(),
            dealer_total: 600.0,
            is_posted: true,
            event_type: "sale".to_string(),
        };
        let a012_2 = A012Row {
            id: "a012-2".to_string(),
            document_no: "S-MULTI".to_string(),
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

    // ── Реклама ───────────────────────────────────────────────────────────────

    #[test]
    fn sale_with_reserve_and_expense_no_problem() {
        let row = p903_sale("SA001", 1000.0);
        let input = LineComputeInput {
            advert_expense: 50.0,
            advert_reserve: 50.0,
            matched_a012: Some(A012Row {
                id: "a012-sa".to_string(),
                document_no: "SA001".to_string(),
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }),
            all_a012_for_srid: vec![A012Row {
                id: "a012-sa".to_string(),
                document_no: "SA001".to_string(),
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
            advert_reserve: 75.0,
            matched_a012: Some(A012Row {
                id: "a012-uuid-1".to_string(),
                document_no: "SB001".to_string(),
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }),
            all_a012_for_srid: vec![A012Row {
                id: "a012-uuid-1".to_string(),
                document_no: "SB001".to_string(),
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
                dealer_total: 600.0,
                is_posted: true,
                event_type: "sale".to_string(),
            }),
            all_a012_for_srid: vec![A012Row {
                id: "a012-d".to_string(),
                document_no: "D001".to_string(),
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
