//! Постинг перечислений YM — завершает денежный контур субъекта «ym».
//!
//! Банковский ордер (`bank_order_id`) агрегирует множество строк p907 одной
//! выплаты; `bank_sum` — итог ордера, повторённый в строках. Поэтому строим
//! ОДНУ GL-проводку на distinct `bank_order_id`:
//!   Дт 51 «Расчётный счёт» / Кт 7609 «Расчёты с маркетплейсом», сумма = bank_sum,
//!   дата = bank_order_date, entity = ym, layer = fina.
//!
//! После этого дебетовое сальдо 7609 (entity=ym) = «доступно к перечислению».
//! Идемпотентно: перед записью удаляем прежнюю проводку каждого ордера.

use anyhow::Result;
use sea_orm::{ConnectionTrait, Statement, TransactionTrait, Value};
use uuid::Uuid;

use contracts::general_ledger::GlEntity;
use contracts::shared::analytics::TurnoverLayer;

use crate::general_ledger::repository::Model as GeneralLedgerModel;
use crate::general_ledger::turnover_registry::get_turnover_class;
use crate::shared::data::db::get_connection;

const REGISTRATOR_TYPE: &str = "p907_ym_settlement";
const TURNOVER_CODE: &str = "ym_settlement";

fn now_str() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn string_value(value: impl Into<String>) -> Value {
    Value::String(Some(Box::new(value.into())))
}

struct BankOrder {
    connection_mp_ref: String,
    bank_order_id: i64,
    bank_sum: f64,
    bank_order_date: String,
}

/// Перестраивает проводки перечислений за период (по `bank_order_date`).
/// Идемпотентно по каждому банковскому ордеру. Возвращает число созданных проводок.
pub async fn rebuild_settlements_for_range(date_from: &str, date_to: &str) -> Result<usize> {
    let orders = fetch_bank_orders(date_from, date_to).await?;

    let db = get_connection();
    let txn = db.begin().await?;
    let mut count = 0usize;
    for order in &orders {
        let registrator_ref = order.bank_order_id.to_string();
        crate::projections::general_ledger::repository::delete_by_registrator_with_conn(
            &txn,
            REGISTRATOR_TYPE,
            &registrator_ref,
        )
        .await?;
        if let Some(entry) = build_entry(order) {
            crate::projections::general_ledger::repository::save_entry_with_conn(&txn, &entry)
                .await?;
            count += 1;
        }
    }
    txn.commit().await?;
    Ok(count)
}

/// Построить GL-проводку перечисления из банковского ордера.
fn build_entry(order: &BankOrder) -> Option<GeneralLedgerModel> {
    if order.bank_sum.abs() < 0.005 {
        return None;
    }
    let class = get_turnover_class(TURNOVER_CODE)?;
    Some(GeneralLedgerModel {
        id: Uuid::new_v4().to_string(),
        entry_date: order.bank_order_date.clone(),
        layer: TurnoverLayer::Fina.as_str().to_string(),
        entity: Some(GlEntity::Ym.as_str().to_string()),
        connection_mp_ref: Some(order.connection_mp_ref.clone()),
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref: order.bank_order_id.to_string(),
        order_id: None,
        debit_account: class.debit_account.to_string(),
        credit_account: class.credit_account.to_string(),
        amount: order.bank_sum,
        qty: None,
        turnover_code: TURNOVER_CODE.to_string(),
        // У перечисления нет проекции-зеркала — drilldown только по common-измерениям.
        resource_table: String::new(),
        resource_field: "amount".to_string(),
        resource_sign: 1,
        created_at: now_str(),
    })
}

/// Distinct банковские ордера за период по `bank_order_date` (10 символов даты).
async fn fetch_bank_orders(date_from: &str, date_to: &str) -> Result<Vec<BankOrder>> {
    let db = get_connection();
    let sql = r#"
        SELECT connection_mp_ref AS connection_mp_ref,
               bank_order_id AS bank_order_id,
               MAX(bank_sum) AS bank_sum,
               substr(MIN(bank_order_date), 1, 10) AS bank_order_date
        FROM p907_ym_payment_report
        WHERE bank_order_id IS NOT NULL
          AND bank_order_date IS NOT NULL AND trim(bank_order_date) <> ''
          AND substr(bank_order_date, 1, 10) >= ?
          AND substr(bank_order_date, 1, 10) <= ?
        GROUP BY connection_mp_ref, bank_order_id
    "#;
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        sql,
        vec![string_value(date_from), string_value(date_to)],
    );
    let rows = db.query_all(stmt).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let connection_mp_ref: String = row.try_get("", "connection_mp_ref").unwrap_or_default();
        let bank_order_id: i64 = row.try_get("", "bank_order_id").unwrap_or_default();
        let bank_sum: f64 = row.try_get("", "bank_sum").unwrap_or(0.0);
        let bank_order_date: String = row.try_get("", "bank_order_date").unwrap_or_default();
        if connection_mp_ref.is_empty() || bank_order_date.len() < 10 {
            continue;
        }
        out.push(BankOrder {
            connection_mp_ref,
            bank_order_id,
            bank_sum,
            bank_order_date,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_entry_uses_bank_sum_on_dt51_kt7609() {
        let order = BankOrder {
            connection_mp_ref: "conn-1".to_string(),
            bank_order_id: 777,
            bank_sum: 123456.78,
            bank_order_date: "2026-05-29".to_string(),
        };
        let entry = build_entry(&order).expect("settlement entry");
        assert_eq!(entry.turnover_code, "ym_settlement");
        assert_eq!(entry.debit_account, "51");
        assert_eq!(entry.credit_account, "7609");
        assert_eq!(entry.amount, 123456.78);
        assert_eq!(entry.entry_date, "2026-05-29");
        assert_eq!(entry.entity.as_deref(), Some("ym"));
        assert_eq!(entry.layer, "fina");
        assert_eq!(entry.registrator_type, "p907_ym_settlement");
        assert_eq!(entry.registrator_ref, "777");
    }

    #[test]
    fn zero_bank_sum_produces_no_entry() {
        let order = BankOrder {
            connection_mp_ref: "conn-1".to_string(),
            bank_order_id: 1,
            bank_sum: 0.0,
            bank_order_date: "2026-05-29".to_string(),
        };
        assert!(build_entry(&order).is_none());
    }
}
