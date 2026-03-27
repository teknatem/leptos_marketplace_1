use anyhow::Result;
use contracts::shared::analytics::TurnoverLayer;
use uuid::Uuid;

use crate::projections::general_ledger::repository::Model as GeneralLedgerModel;
use crate::shared::analytics::normalization::opt_nonzero;
use crate::shared::analytics::turnover_registry::get_turnover_class;

const DETAIL_KIND: &str = "p903_wb_finance_report";
const REGISTRATOR_TYPE: &str = "p903_wb_finance_report";

const OP_SALE_RU: &str = "Продажа";
const OP_RETURN_RU: &str = "Возврат";
const OP_LOGISTICS_RU: &str = "Логистика";
const OP_LOGISTICS_CORRECTION_RU: &str = "Коррекция логистики";
const OP_LOGISTICS_CORRECTION_ALT_RU: &str = "Корректировка логистики";
const OP_STORAGE_RU: &str = "Хранение";
const OP_PENALTY_RU: &str = "Штраф";
const OP_VOLUNTARY_RETURN_COMPENSATION_RU: &str = "Добровольная компенсация при возврате";
const OP_TRANSPORT_STORAGE_REIMBURSEMENT_RU: &str =
    "Возмещение издержек по перевозке/по складским операциям с товаром";

#[derive(Debug, Clone)]
struct ResourceAmount {
    amount: f64,
    resource_name: &'static str,
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

fn resource_amount(amount: f64, resource_name: &'static str) -> Option<ResourceAmount> {
    if amount.abs() <= f64::EPSILON {
        return None;
    }

    Some(ResourceAmount {
        amount,
        resource_name,
        resource_sign: amount_sign(amount),
    })
}

fn passthrough_amount(value: Option<f64>, resource_name: &'static str) -> Option<ResourceAmount> {
    resource_amount(opt_nonzero(value)?, resource_name)
}

fn scaled_passthrough_amount(
    value: Option<f64>,
    resource_name: &'static str,
    multiplier: f64,
) -> Option<ResourceAmount> {
    resource_amount(opt_nonzero(value)? * multiplier, resource_name)
}

fn normalized_expense_amount(
    value: Option<f64>,
    resource_name: &'static str,
) -> Option<ResourceAmount> {
    let raw = opt_nonzero(value)?;
    let amount = if raw > 0.0 { -raw } else { raw };
    resource_amount(amount, resource_name)
}

fn build_entry(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
    posting_id: &str,
    turnover_code: &str,
    resource: ResourceAmount,
) -> Option<GeneralLedgerModel> {
    let class = get_turnover_class(turnover_code)?;
    if !class.generates_journal_entry || resource.amount.abs() <= f64::EPSILON {
        return None;
    }

    Some(GeneralLedgerModel {
        id: Uuid::new_v4().to_string(),
        posting_id: posting_id.to_string(),
        entry_date: row.rr_dt.clone(),
        layer: TurnoverLayer::Fact.as_str().to_string(),
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref: row.source_row_ref.clone(),
        debit_account: class.debit_account.to_string(),
        credit_account: class.credit_account.to_string(),
        amount: resource.amount,
        qty: None,
        turnover_code: turnover_code.to_string(),
        detail_kind: DETAIL_KIND.to_string(),
        detail_id: row.source_row_ref.clone(),
        resource_name: resource.resource_name.to_string(),
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

fn is_logistics_operation(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> bool {
    has_operation(
        row,
        &[
            OP_LOGISTICS_RU,
            OP_LOGISTICS_CORRECTION_RU,
            OP_LOGISTICS_CORRECTION_ALT_RU,
        ],
    )
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

fn is_transport_storage_reimbursement_operation(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> bool {
    !is_linked(row) && has_operation(row, &[OP_TRANSPORT_STORAGE_REIMBURSEMENT_RU])
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
    resource_name: &'static str,
) -> Option<ResourceAmount> {
    if is_linked(row) {
        passthrough_amount(value, resource_name)
    } else {
        normalized_expense_amount(value, resource_name)
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
        (
            "mp_penalty_reversal",
            Some(ResourceAmount {
                amount: raw.abs(),
                resource_name: "penalty",
                resource_sign: -1,
            }),
        )
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
                passthrough_amount(row.return_amount, "return_amount")
            } else {
                passthrough_amount(row.retail_amount, "retail_amount")
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
            "mp_commission_adjustment",
            resource,
        );
    }

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

    if is_transport_storage_reimbursement_operation(row) {
        push_optional_entry(
            &mut entries,
            row,
            posting_id,
            "mp_transport_storage_reimbursement",
            expense_amount_for_branch(row, row.delivery_rub, "delivery_rub"),
        );
        push_optional_entry(
            &mut entries,
            row,
            posting_id,
            "mp_transport_storage_reimbursement",
            expense_amount_for_branch(row, row.storage_fee, "storage_fee"),
        );
    } else if is_logistics_operation(row) {
        push_optional_entry(
            &mut entries,
            row,
            posting_id,
            "mp_logistics",
            expense_amount_for_branch(row, row.delivery_rub, "delivery_rub"),
        );
    }

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
            source_row_ref: "p903:2026-03-01:100".to_string(),
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
        assert!(entries
            .iter()
            .all(|item| item.detail_kind == "p903_wb_finance_report"));
        assert!(entries
            .iter()
            .all(|item| item.detail_id == row.source_row_ref));
        assert!(entries
            .iter()
            .all(|item| item.registrator_ref == row.source_row_ref));

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
    fn non_sale_operation_uses_commission_adjustment_and_ignores_rebill_logistics() {
        let mut row = base_row();
        row.supplier_oper_name = Some(OP_LOGISTICS_RU.to_string());
        row.delivery_rub = Some(70.0);
        row.rebill_logistic_cost = Some(999.0);
        row.ppvz_vw = Some(30.0);
        row.ppvz_vw_nds = Some(6.0);
        row.ppvz_sales_commission = Some(4.0);

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        assert!(!entries
            .iter()
            .any(|item| item.resource_name == "rebill_logistic_cost"));

        let commission_adjustment = entries
            .iter()
            .find(|item| item.turnover_code == "mp_commission_adjustment")
            .unwrap();
        assert_eq!(commission_adjustment.debit_account, "4402");
        assert_eq!(commission_adjustment.amount, -40.0);

        let logistics = entries
            .iter()
            .find(|item| item.turnover_code == "mp_logistics")
            .unwrap();
        assert_eq!(logistics.debit_account, "4404");
        assert_eq!(logistics.resource_name, "delivery_rub");
        assert_eq!(logistics.amount, -70.0);
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
        assert_eq!(storage.resource_name, "storage_fee");
        assert_eq!(storage.resource_sign, -1);
        assert_eq!(storage.amount, -100.0);
    }

    #[test]
    fn transport_storage_reimbursement_without_srid_uses_separate_turnover() {
        let mut row = base_row();
        row.supplier_oper_name = Some(OP_TRANSPORT_STORAGE_REIMBURSEMENT_RU.to_string());
        row.delivery_rub = Some(120.0);
        row.storage_fee = Some(30.0);

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();

        assert!(entries
            .iter()
            .any(|item| item.turnover_code == "mp_transport_storage_reimbursement"));
        assert!(!entries
            .iter()
            .any(|item| item.turnover_code == "mp_logistics"));
        assert!(!entries
            .iter()
            .any(|item| item.turnover_code == "mp_storage"));

        let delivery = entries
            .iter()
            .find(|item| {
                item.turnover_code == "mp_transport_storage_reimbursement"
                    && item.resource_name == "delivery_rub"
            })
            .unwrap();
        assert_eq!(delivery.debit_account, "4404");
        assert_eq!(delivery.amount, -120.0);

        let storage = entries
            .iter()
            .find(|item| {
                item.turnover_code == "mp_transport_storage_reimbursement"
                    && item.resource_name == "storage_fee"
            })
            .unwrap();
        assert_eq!(storage.debit_account, "4404");
        assert_eq!(storage.amount, -30.0);
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
    fn penalty_is_split_into_charge_and_reversal_turnovers() {
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

        let mut reversal_row = base_row();
        reversal_row.supplier_oper_name = Some(OP_PENALTY_RU.to_string());
        reversal_row.penalty = Some(-150.0);

        let reversal_entries = build_general_ledger_entries(&reversal_row, "posting-1").unwrap();
        let reversal = reversal_entries
            .iter()
            .find(|item| item.turnover_code == "mp_penalty_reversal")
            .unwrap();

        assert_eq!(reversal.debit_account, "7609");
        assert_eq!(reversal.credit_account, "9102");
        assert_eq!(reversal.amount, 150.0);
        assert_eq!(reversal.resource_sign, -1);
    }
}
