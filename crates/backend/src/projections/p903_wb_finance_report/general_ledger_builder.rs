use anyhow::Result;
use contracts::shared::analytics::TurnoverLayer;
use serde_json::Value;
use uuid::Uuid;

use crate::general_ledger::repository::Model as GeneralLedgerModel;
use crate::general_ledger::turnover_registry::get_turnover_class;
use crate::shared::analytics::normalization::opt_nonzero;

const DETAIL_KIND: &str = "p903_wb_finance_report";
const REGISTRATOR_TYPE: &str = "p903_wb_finance_report";
const OP_PPVZ_REWARD_RU: &str = "Возмещение за выдачу и возврат товаров на ПВЗ";

const OP_SALE_RU: &str = "Продажа";
const OP_RETURN_RU: &str = "Возврат";
const OP_STORAGE_RU: &str = "Хранение";
const OP_PENALTY_RU: &str = "Штраф";
const OP_VOLUNTARY_RETURN_COMPENSATION_RU: &str = "Добровольная компенсация при возврате";
#[cfg(test)]
const OP_LOGISTICS_RU: &str = "Логистика";
#[cfg(test)]
const OP_TRANSPORT_STORAGE_REIMBURSEMENT_RU: &str =
    "Возмещение издержек по перевозке/по складским операциям с товаром";

const PAYMENT_PROCESSING_EXCLUDED_FROM_GL_ACQUIRING: &str = "Комиссия за организацию платежа с НДС";

#[derive(Debug, Clone)]
struct ResourceAmount {
    amount: f64,
    resource_field: &'static str,
    resource_sign: i32,
}

fn now_str() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn amount_sign(amount: f64) -> i32 {
    if amount < 0.0 {
        -1
    } else {
        1
    }
}

fn resource_amount(amount: f64, resource_field: &'static str) -> Option<ResourceAmount> {
    if amount.abs() <= f64::EPSILON {
        return None;
    }

    Some(ResourceAmount {
        amount,
        resource_field,
        resource_sign: amount_sign(amount),
    })
}

fn passthrough_amount(value: Option<f64>, resource_field: &'static str) -> Option<ResourceAmount> {
    resource_amount(opt_nonzero(value)?, resource_field)
}

fn scaled_passthrough_amount(
    value: Option<f64>,
    resource_field: &'static str,
    multiplier: f64,
) -> Option<ResourceAmount> {
    resource_amount(opt_nonzero(value)? * multiplier, resource_field)
}

fn normalized_expense_amount(
    value: Option<f64>,
    resource_field: &'static str,
) -> Option<ResourceAmount> {
    let raw = opt_nonzero(value)?;
    let amount = if raw > 0.0 { -raw } else { raw };
    resource_amount(amount, resource_field)
}

fn build_entry(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
    _posting_id: &str,
    turnover_code: &str,
    resource: ResourceAmount,
) -> Option<GeneralLedgerModel> {
    let class = get_turnover_class(turnover_code)?;
    if !class.generates_journal_entry || resource.amount.abs() <= f64::EPSILON {
        return None;
    }

    Some(GeneralLedgerModel {
        id: Uuid::new_v4().to_string(),
        entry_date: row.rr_dt.clone(),
        layer: TurnoverLayer::Fact.as_str().to_string(),
        connection_mp_ref: Some(row.connection_mp_ref.clone()),
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref: row.id.clone(),
        order_id: row.srid.clone().filter(|value| !value.trim().is_empty()),
        debit_account: class.debit_account.to_string(),
        credit_account: class.credit_account.to_string(),
        amount: resource.amount,
        qty: None,
        turnover_code: turnover_code.to_string(),
        resource_table: DETAIL_KIND.to_string(),
        resource_field: resource.resource_field.to_string(),
        resource_sign: resource.resource_sign,
        created_at: now_str(),
    })
}

fn operation_name(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> Option<&str> {
    row.supplier_oper_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn has_operation(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
    expected: &[&str],
) -> bool {
    let Some(actual) = operation_name(row) else {
        return false;
    };

    expected.iter().any(|item| actual == *item)
}

fn is_linked(row: &crate::projections::p903_wb_finance_report::repository::Model) -> bool {
    row.srid
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
}

fn has_nm_id(row: &crate::projections::p903_wb_finance_report::repository::Model) -> bool {
    row.nm_id.is_some_and(|value| value > 0)
}

fn nm_turnover_code(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
    base_code: &'static str,
    nm_code: &'static str,
) -> &'static str {
    if has_nm_id(row) {
        nm_code
    } else {
        base_code
    }
}

fn is_return_row(row: &crate::projections::p903_wb_finance_report::repository::Model) -> bool {
    row.return_amount.unwrap_or(0.0).abs() > f64::EPSILON
        || row
            .supplier_oper_name
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("return") || value == OP_RETURN_RU)
}

fn is_sale_row(row: &crate::projections::p903_wb_finance_report::repository::Model) -> bool {
    row.retail_amount.unwrap_or(0.0).abs() > f64::EPSILON
        || row
            .supplier_oper_name
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("sale") || value == OP_SALE_RU)
}

fn is_sale_or_return_operation(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> bool {
    has_operation(row, &[OP_SALE_RU, OP_RETURN_RU]) || is_sale_row(row) || is_return_row(row)
}

fn is_storage_operation(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> bool {
    has_operation(row, &[OP_STORAGE_RU])
}

fn is_penalty_operation(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> bool {
    has_operation(row, &[OP_PENALTY_RU])
}

fn is_voluntary_return_compensation_operation(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> bool {
    has_operation(row, &[OP_VOLUNTARY_RETURN_COMPENSATION_RU])
}

fn extra_string_field(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
    field: &str,
) -> Option<String> {
    row.extra
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|json| {
            json.get(field)
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
        })
        .filter(|value| !value.is_empty())
}

fn extra_f64_field(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
    field: &str,
) -> Option<f64> {
    row.extra
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|json| {
            json.get(field).and_then(|value| {
                value.as_f64().or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw| raw.trim().parse::<f64>().ok())
                })
            })
        })
        .and_then(|value| opt_nonzero(Some(value)))
}

fn is_excluded_acquiring_payment_processing(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> bool {
    extra_string_field(row, "payment_processing")
        .is_some_and(|value| value == PAYMENT_PROCESSING_EXCLUDED_FROM_GL_ACQUIRING)
}

fn ppvz_reward_amount(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> Option<ResourceAmount> {
    let raw = extra_f64_field(row, "ppvz_reward")?;
    let amount = if is_return_row(row) {
        -raw.abs()
    } else if is_sale_row(row) || has_operation(row, &[OP_SALE_RU, OP_PPVZ_REWARD_RU]) {
        raw.abs()
    } else {
        return None;
    };

    resource_amount(amount, "extra.ppvz_reward")
}

fn commission_sale_return_amount(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> Option<f64> {
    opt_nonzero(Some(
        row.ppvz_vw.unwrap_or(0.0) + row.ppvz_vw_nds.unwrap_or(0.0),
    ))
}

fn commission_adjustment_amount(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> Option<f64> {
    opt_nonzero(Some(
        row.ppvz_vw.unwrap_or(0.0)
            + row.ppvz_vw_nds.unwrap_or(0.0)
            + row.ppvz_sales_commission.unwrap_or(0.0),
    ))
}

fn expense_amount_for_branch(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
    value: Option<f64>,
    resource_field: &'static str,
) -> Option<ResourceAmount> {
    if is_linked(row) {
        passthrough_amount(value, resource_field)
    } else {
        normalized_expense_amount(value, resource_field)
    }
}

fn push_optional_entry(
    entries: &mut Vec<GeneralLedgerModel>,
    row: &crate::projections::p903_wb_finance_report::repository::Model,
    posting_id: &str,
    turnover_code: &str,
    resource: Option<ResourceAmount>,
) {
    if let Some(entry) = resource.and_then(|item| build_entry(row, posting_id, turnover_code, item))
    {
        entries.push(entry);
    }
}

fn push_penalty_entry(
    entries: &mut Vec<GeneralLedgerModel>,
    row: &crate::projections::p903_wb_finance_report::repository::Model,
    posting_id: &str,
) {
    if !is_penalty_operation(row) {
        return;
    }

    let Some(raw) = opt_nonzero(row.penalty) else {
        return;
    };

    let (turnover_code, resource) = if raw > 0.0 {
        ("mp_penalty", resource_amount(raw, "penalty"))
    } else {
        ("mp_penalty_storno", resource_amount(raw, "penalty"))
    };

    push_optional_entry(entries, row, posting_id, turnover_code, resource);
}

pub fn build_general_ledger_entries(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
    posting_id: &str,
) -> Result<Vec<GeneralLedgerModel>> {
    let mut entries = Vec::new();
    let linked = is_linked(row);

    if linked {
        if is_return_row(row) {
            let resource = if row.return_amount.unwrap_or(0.0).abs() > f64::EPSILON {
                scaled_passthrough_amount(row.return_amount, "return_amount", -1.0)
            } else {
                scaled_passthrough_amount(row.retail_amount, "retail_amount", -1.0)
            };
            push_optional_entry(&mut entries, row, posting_id, "customer_return", resource);
        } else if is_sale_row(row) {
            push_optional_entry(
                &mut entries,
                row,
                posting_id,
                "customer_revenue",
                passthrough_amount(row.retail_amount, "retail_amount"),
            );
        }
    }

    if is_sale_or_return_operation(row) {
        let resource = if linked {
            passthrough_amount(
                commission_sale_return_amount(row),
                "commission_ppvz_vw_plus_ppvz_vw_nds",
            )
        } else {
            normalized_expense_amount(
                commission_sale_return_amount(row),
                "commission_ppvz_vw_plus_ppvz_vw_nds",
            )
        };
        push_optional_entry(&mut entries, row, posting_id, "mp_commission", resource);
    } else {
        let resource = if linked {
            passthrough_amount(
                commission_adjustment_amount(row),
                "commission_adjustment_full",
            )
        } else {
            normalized_expense_amount(
                commission_adjustment_amount(row),
                "commission_adjustment_full",
            )
        };
        push_optional_entry(
            &mut entries,
            row,
            posting_id,
            nm_turnover_code(
                row,
                "mp_commission_adjustment",
                "mp_commission_adjustment_nm",
            ),
            resource,
        );
    }

    if !is_excluded_acquiring_payment_processing(row) {
        let acquiring_resource = if linked && is_return_row(row) {
            scaled_passthrough_amount(row.acquiring_fee, "acquiring_fee", -1.0)
        } else {
            expense_amount_for_branch(row, row.acquiring_fee, "acquiring_fee")
        };
        push_optional_entry(
            &mut entries,
            row,
            posting_id,
            "mp_acquiring",
            acquiring_resource,
        );
    }

    if opt_nonzero(row.rebill_logistic_cost).is_some() {
        push_optional_entry(
            &mut entries,
            row,
            posting_id,
            nm_turnover_code(row, "mp_rebill_logistic_cost", "mp_rebill_logistic_cost_nm"),
            passthrough_amount(row.rebill_logistic_cost, "rebill_logistic_cost"),
        );
    }

    push_optional_entry(
        &mut entries,
        row,
        posting_id,
        nm_turnover_code(row, "mp_ppvz_reward", "mp_ppvz_reward_nm"),
        ppvz_reward_amount(row),
    );

    if is_storage_operation(row) {
        push_optional_entry(
            &mut entries,
            row,
            posting_id,
            "mp_storage",
            expense_amount_for_branch(row, row.storage_fee, "storage_fee"),
        );
    }

    push_penalty_entry(&mut entries, row, posting_id);

    if is_voluntary_return_compensation_operation(row) {
        push_optional_entry(
            &mut entries,
            row,
            posting_id,
            "voluntary_return_compensation",
            passthrough_amount(row.ppvz_for_pay, "ppvz_for_pay"),
        );
    }

    if !linked {
        push_optional_entry(
            &mut entries,
            row,
            posting_id,
            "acceptance",
            normalized_expense_amount(row.delivery_amount, "delivery_amount"),
        );
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_row() -> crate::projections::p903_wb_finance_report::repository::Model {
        crate::projections::p903_wb_finance_report::repository::Model {
            rr_dt: "2026-03-01".to_string(),
            rrd_id: 100,
            id: "p903-row-100".to_string(),
            source_row_ref: "p903:100".to_string(),
            connection_mp_ref: "conn-1".to_string(),
            organization_ref: "org-1".to_string(),
            acquiring_fee: None,
            acquiring_percent: None,
            additional_payment: None,
            bonus_type_name: None,
            commission_percent: None,
            delivery_amount: None,
            delivery_rub: None,
            nm_id: None,
            a004_nomenclature_ref: None,
            penalty: None,
            ppvz_vw: None,
            ppvz_vw_nds: None,
            ppvz_sales_commission: None,
            quantity: None,
            rebill_logistic_cost: None,
            retail_amount: None,
            retail_price: None,
            retail_price_withdisc_rub: None,
            return_amount: None,
            sa_name: None,
            storage_fee: None,
            subject_name: None,
            supplier_oper_name: None,
            cashback_amount: None,
            ppvz_for_pay: None,
            ppvz_kvw_prc: None,
            ppvz_kvw_prc_base: None,
            srv_dbs: None,
            srid: None,
            loaded_at_utc: "2026-03-01T00:00:00Z".to_string(),
            payload_version: 1,
            extra: None,
        }
    }

    #[test]
    fn linked_sale_row_generates_fact_general_ledger_entries() {
        let mut row = base_row();
        row.srid = Some("srid-1".to_string());
        row.supplier_oper_name = Some(OP_SALE_RU.to_string());
        row.retail_amount = Some(1000.0);
        row.acquiring_fee = Some(50.0);
        row.ppvz_vw = Some(120.0);
        row.ppvz_vw_nds = Some(24.0);

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().all(|item| item.layer == "fact"));
        assert!(entries.iter().all(|item| item.registrator_ref == row.id));
        assert!(entries
            .iter()
            .all(|item| item.resource_table == "p903_wb_finance_report"));
        assert!(entries
            .iter()
            .all(|item| item.connection_mp_ref.as_deref() == Some("conn-1")));

        let revenue = entries
            .iter()
            .find(|item| item.turnover_code == "customer_revenue")
            .unwrap();
        assert_eq!(revenue.debit_account, "7609");
        assert_eq!(revenue.credit_account, "9001");
        assert_eq!(revenue.amount, 1000.0);
        assert_eq!(revenue.resource_field, "retail_amount");

        let acquiring = entries
            .iter()
            .find(|item| item.turnover_code == "mp_acquiring")
            .unwrap();
        assert_eq!(acquiring.debit_account, "4403");
        assert_eq!(acquiring.amount, 50.0);
    }

    #[test]
    fn linked_return_row_books_acquiring_with_minus() {
        let mut row = base_row();
        row.srid = Some("srid-1".to_string());
        row.supplier_oper_name = Some(OP_RETURN_RU.to_string());
        row.return_amount = Some(1000.0);
        row.acquiring_fee = Some(50.0);

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        let acquiring = entries
            .iter()
            .find(|item| item.turnover_code == "mp_acquiring")
            .unwrap();

        assert_eq!(acquiring.debit_account, "4403");
        assert_eq!(acquiring.amount, -50.0);
        assert_eq!(acquiring.resource_sign, -1);
    }

    #[test]
    fn linked_return_row_books_customer_return_as_negative_revenue() {
        let mut row = base_row();
        row.srid = Some("srid-1".to_string());
        row.supplier_oper_name = Some(OP_RETURN_RU.to_string());
        row.return_amount = Some(1000.0);

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        let customer_return = entries
            .iter()
            .find(|item| item.turnover_code == "customer_return")
            .unwrap();

        assert_eq!(customer_return.debit_account, "7609");
        assert_eq!(customer_return.credit_account, "9001");
        assert_eq!(customer_return.amount, -1000.0);
        assert_eq!(customer_return.resource_field, "return_amount");
        assert_eq!(customer_return.resource_sign, -1);
    }

    #[test]
    fn ppvz_reward_uses_expected_sign_rules() {
        let mut sale_row = base_row();
        sale_row.srid = Some("srid-sale".to_string());
        sale_row.supplier_oper_name = Some(OP_SALE_RU.to_string());
        sale_row.extra = Some(r#"{"ppvz_reward":27901.01}"#.to_string());

        let sale_entries = build_general_ledger_entries(&sale_row, "posting-1").unwrap();
        let sale_reward = sale_entries
            .iter()
            .find(|item| item.turnover_code == "mp_ppvz_reward")
            .unwrap();
        assert_eq!(sale_reward.amount, 27901.01);
        assert_eq!(sale_reward.resource_field, "extra.ppvz_reward");
        assert_eq!(sale_reward.resource_sign, 1);
        assert_eq!(sale_reward.debit_account, "4404");
        assert_eq!(sale_reward.credit_account, "7609");

        let mut return_row = base_row();
        return_row.srid = Some("srid-return".to_string());
        return_row.supplier_oper_name = Some(OP_RETURN_RU.to_string());
        return_row.return_amount = Some(1.0);
        return_row.extra = Some(r#"{"ppvz_reward":639.21}"#.to_string());

        let return_entries = build_general_ledger_entries(&return_row, "posting-1").unwrap();
        let return_reward = return_entries
            .iter()
            .find(|item| item.turnover_code == "mp_ppvz_reward")
            .unwrap();
        assert_eq!(return_reward.amount, -639.21);
        assert_eq!(return_reward.resource_sign, -1);

        let mut pvz_row = base_row();
        pvz_row.supplier_oper_name = Some(OP_PPVZ_REWARD_RU.to_string());
        pvz_row.extra = Some(r#"{"ppvz_reward":526.70}"#.to_string());

        let pvz_entries = build_general_ledger_entries(&pvz_row, "posting-1").unwrap();
        let pvz_reward = pvz_entries
            .iter()
            .find(|item| item.turnover_code == "mp_ppvz_reward")
            .unwrap();
        assert_eq!(pvz_reward.amount, 526.70);
        assert_eq!(pvz_reward.resource_sign, 1);
    }

    #[test]
    fn excluded_payment_processing_skips_mp_acquiring_entry() {
        let mut row = base_row();
        row.srid = Some("srid-1".to_string());
        row.supplier_oper_name = Some(OP_SALE_RU.to_string());
        row.retail_amount = Some(1000.0);
        row.acquiring_fee = Some(50.0);
        row.extra =
            Some(r#"{"payment_processing":"Комиссия за организацию платежа с НДС"}"#.to_string());

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();

        assert!(!entries
            .iter()
            .any(|item| item.turnover_code == "mp_acquiring"));
    }

    #[test]
    fn non_sale_operation_uses_commission_adjustment_and_posts_only_rebill_logistics() {
        let mut row = base_row();
        row.supplier_oper_name = Some(OP_LOGISTICS_RU.to_string());
        row.delivery_rub = Some(70.0);
        row.rebill_logistic_cost = Some(999.0);
        row.ppvz_vw = Some(30.0);
        row.ppvz_vw_nds = Some(6.0);
        row.ppvz_sales_commission = Some(4.0);

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();

        let commission_adjustment = entries
            .iter()
            .find(|item| item.turnover_code == "mp_commission_adjustment")
            .unwrap();
        assert_eq!(commission_adjustment.debit_account, "4402");
        assert_eq!(commission_adjustment.amount, -40.0);

        assert!(!entries
            .iter()
            .any(|item| item.turnover_code == "mp_logistics"));

        let reimbursement = entries
            .iter()
            .find(|item| item.turnover_code == "mp_rebill_logistic_cost")
            .unwrap();
        assert_eq!(reimbursement.debit_account, "4404");
        assert_eq!(reimbursement.resource_field, "rebill_logistic_cost");
        assert_eq!(reimbursement.resource_sign, 1);
        assert_eq!(reimbursement.amount, 999.0);
    }

    #[test]
    fn non_sale_operation_with_nm_id_uses_nm_turnover_clones() {
        let mut row = base_row();
        row.nm_id = Some(123456);
        row.srid = Some("srid-1".to_string());
        row.supplier_oper_name = Some(OP_LOGISTICS_RU.to_string());
        row.rebill_logistic_cost = Some(999.0);
        row.ppvz_vw = Some(30.0);
        row.ppvz_vw_nds = Some(6.0);
        row.ppvz_sales_commission = Some(4.0);

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();

        assert!(entries
            .iter()
            .any(|item| item.turnover_code == "mp_commission_adjustment_nm"));
        assert!(!entries
            .iter()
            .any(|item| item.turnover_code == "mp_commission_adjustment"));
        assert!(entries
            .iter()
            .any(|item| item.turnover_code == "mp_rebill_logistic_cost_nm"));
        assert!(!entries
            .iter()
            .any(|item| item.turnover_code == "mp_rebill_logistic_cost"));
    }

    #[test]
    fn ppvz_reward_with_nm_id_uses_nm_turnover_clone() {
        let mut row = base_row();
        row.nm_id = Some(123456);
        row.srid = Some("srid-sale".to_string());
        row.supplier_oper_name = Some(OP_SALE_RU.to_string());
        row.extra = Some(r#"{"ppvz_reward":27901.01}"#.to_string());

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();

        assert!(entries
            .iter()
            .any(|item| item.turnover_code == "mp_ppvz_reward_nm"));
        assert!(!entries
            .iter()
            .any(|item| item.turnover_code == "mp_ppvz_reward"));
    }

    #[test]
    fn storage_operation_uses_account_4404() {
        let mut row = base_row();
        row.supplier_oper_name = Some(OP_STORAGE_RU.to_string());
        row.storage_fee = Some(100.0);

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        let storage = entries
            .iter()
            .find(|item| item.turnover_code == "mp_storage")
            .unwrap();

        assert_eq!(storage.debit_account, "4404");
        assert_eq!(storage.resource_field, "storage_fee");
        assert_eq!(storage.resource_sign, -1);
        assert_eq!(storage.amount, -100.0);
    }

    #[test]
    fn rebill_logistic_cost_uses_dedicated_turnover() {
        let mut row = base_row();
        row.supplier_oper_name = Some(OP_TRANSPORT_STORAGE_REIMBURSEMENT_RU.to_string());
        row.rebill_logistic_cost = Some(120.0);
        row.srid = Some("srid-1".to_string());

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();

        assert!(entries
            .iter()
            .any(|item| item.turnover_code == "mp_rebill_logistic_cost"));
        assert!(!entries
            .iter()
            .any(|item| item.turnover_code == "mp_logistics"));
        let reimbursement = entries
            .iter()
            .find(|item| {
                item.turnover_code == "mp_rebill_logistic_cost"
                    && item.resource_field == "rebill_logistic_cost"
            })
            .unwrap();
        assert_eq!(reimbursement.debit_account, "4404");
        assert_eq!(reimbursement.amount, 120.0);
    }

    #[test]
    fn voluntary_return_compensation_is_posted_as_other_income() {
        let mut row = base_row();
        row.supplier_oper_name = Some(OP_VOLUNTARY_RETURN_COMPENSATION_RU.to_string());
        row.ppvz_for_pay = Some(345.67);

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        let compensation = entries
            .iter()
            .find(|item| item.turnover_code == "voluntary_return_compensation")
            .unwrap();

        assert_eq!(compensation.debit_account, "7609");
        assert_eq!(compensation.credit_account, "91");
        assert_eq!(compensation.amount, 345.67);
    }

    #[test]
    fn penalty_is_split_into_charge_and_storno_turnovers() {
        let mut penalty_row = base_row();
        penalty_row.supplier_oper_name = Some(OP_PENALTY_RU.to_string());
        penalty_row.penalty = Some(150.0);

        let penalty_entries = build_general_ledger_entries(&penalty_row, "posting-1").unwrap();
        let penalty = penalty_entries
            .iter()
            .find(|item| item.turnover_code == "mp_penalty")
            .unwrap();

        assert_eq!(penalty.debit_account, "9102");
        assert_eq!(penalty.credit_account, "7609");
        assert_eq!(penalty.amount, 150.0);

        let mut storno_row = base_row();
        storno_row.supplier_oper_name = Some(OP_PENALTY_RU.to_string());
        storno_row.penalty = Some(-150.0);

        let storno_entries = build_general_ledger_entries(&storno_row, "posting-1").unwrap();
        let storno = storno_entries
            .iter()
            .find(|item| item.turnover_code == "mp_penalty_storno")
            .unwrap();

        assert_eq!(storno.debit_account, "9102");
        assert_eq!(storno.credit_account, "7609");
        assert_eq!(storno.amount, -150.0);
        assert_eq!(storno.resource_sign, -1);
    }
}
