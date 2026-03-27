use anyhow::Result;
use chrono::Utc;
use contracts::shared::analytics::TurnoverLayer;
use uuid::Uuid;

use super::repository::Model;
use crate::projections::general_ledger::repository::Model as GeneralLedgerModel;
use crate::shared::analytics::normalization::{normalize_expense, opt_nonzero};
use crate::shared::analytics::turnover_registry::get_turnover_class;

const REGISTRATOR_TYPE: &str = "p903_wb_finance_report";

pub struct PostingResult {
    pub turnovers: Vec<Model>,
    pub general_ledger_entries: Vec<GeneralLedgerModel>,
}

fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn normalize_business_date_str(value: &str) -> String {
    if value.len() >= 10 {
        value[..10].to_string()
    } else {
        value.to_string()
    }
}

fn classifier_kinds(turnover_code: &str) -> (String, String) {
    let class = get_turnover_class(turnover_code).unwrap_or_else(|| {
        panic!("Missing turnover class for code '{}'", turnover_code);
    });

    (
        class.value_kind.as_str().to_string(),
        class.agg_kind.as_str().to_string(),
    )
}

pub fn source_ref_from_model(
    entry: &crate::projections::p903_wb_finance_report::repository::Model,
) -> String {
    format!("p903:{}:{}", entry.rr_dt, entry.rrd_id)
}

fn build_row(
    entry: &crate::projections::p903_wb_finance_report::repository::Model,
    turnover_code: &str,
    amount: Option<f64>,
    comment: Option<String>,
) -> Option<Model> {
    let amount = opt_nonzero(amount)?;
    let registrator_ref = source_ref_from_model(entry);
    let now = now_str();
    let (value_kind, agg_kind) = classifier_kinds(turnover_code);

    Some(Model {
        id: format!(
            "{}:{}:{}:{}",
            entry.connection_mp_ref,
            registrator_ref,
            turnover_code,
            TurnoverLayer::Fact.as_str()
        ),
        connection_mp_ref: entry.connection_mp_ref.clone(),
        entry_date: normalize_business_date_str(&entry.rr_dt),
        layer: TurnoverLayer::Fact.as_str().to_string(),
        turnover_code: turnover_code.to_string(),
        value_kind,
        agg_kind,
        amount,
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref,
        general_ledger_ref: None,
        nomenclature_ref: None,
        comment,
        created_at: now.clone(),
        updated_at: now,
    })
}

fn make_general_ledger_entry(row: &Model, posting_id: &str) -> Option<GeneralLedgerModel> {
    let class = get_turnover_class(&row.turnover_code)?;
    if !class.generates_journal_entry || row.amount.abs() <= f64::EPSILON {
        return None;
    }

    Some(GeneralLedgerModel {
        id: Uuid::new_v4().to_string(),
        posting_id: posting_id.to_string(),
        entry_date: row.entry_date.clone(),
        layer: row.layer.clone(),
        registrator_type: row.registrator_type.clone(),
        registrator_ref: row.registrator_ref.clone(),
        debit_account: class.debit_account.to_string(),
        credit_account: class.credit_account.to_string(),
        amount: row.amount,
        qty: None,
        turnover_code: row.turnover_code.clone(),
        detail_kind: "p910_mp_unlinked_turnovers".to_string(),
        detail_id: row.id.clone(),
        resource_name: row.turnover_code.clone(),
        resource_sign: 1,
        created_at: now_str(),
    })
}

fn push_row(
    turnovers: &mut Vec<Model>,
    general_ledger_entries: &mut Vec<GeneralLedgerModel>,
    row: Option<Model>,
    posting_id: &str,
) {
    let Some(mut row) = row else {
        return;
    };

    if let Some(je) = make_general_ledger_entry(&row, posting_id) {
        row.general_ledger_ref = Some(je.id.clone());
        general_ledger_entries.push(je);
    }

    turnovers.push(row);
}

pub fn from_wb_finance_row(
    entry: &crate::projections::p903_wb_finance_report::repository::Model,
    posting_id: &str,
) -> Result<PostingResult> {
    if entry
        .srid
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Ok(PostingResult {
            turnovers: vec![],
            general_ledger_entries: vec![],
        });
    }

    let mut turnovers = Vec::new();
    let mut general_ledger_entries = Vec::new();

    push_row(
        &mut turnovers,
        &mut general_ledger_entries,
        build_row(
            entry,
            "mp_storage",
            normalize_expense(entry.storage_fee),
            Some("WB finance row without srid: storage_fee".to_string()),
        ),
        posting_id,
    );
    push_row(
        &mut turnovers,
        &mut general_ledger_entries,
        build_row(
            entry,
            "mp_logistics",
            normalize_expense(entry.rebill_logistic_cost.or(entry.delivery_rub)),
            Some("WB finance row without srid: logistics".to_string()),
        ),
        posting_id,
    );
    push_row(
        &mut turnovers,
        &mut general_ledger_entries,
        build_row(
            entry,
            "acceptance",
            normalize_expense(entry.delivery_amount),
            Some("WB finance row without srid: delivery_amount".to_string()),
        ),
        posting_id,
    );
    push_row(
        &mut turnovers,
        &mut general_ledger_entries,
        build_row(
            entry,
            "mp_penalty",
            normalize_expense(entry.penalty),
            Some("WB finance row without srid: penalty".to_string()),
        ),
        posting_id,
    );
    push_row(
        &mut turnovers,
        &mut general_ledger_entries,
        build_row(
            entry,
            "mp_acquiring",
            normalize_expense(entry.acquiring_fee),
            Some("WB finance row without srid: acquiring_fee".to_string()),
        ),
        posting_id,
    );
    push_row(
        &mut turnovers,
        &mut general_ledger_entries,
        build_row(
            entry,
            "mp_commission",
            normalize_expense(Some(
                entry.ppvz_vw.unwrap_or(0.0)
                    + entry.ppvz_vw_nds.unwrap_or(0.0)
                    + entry.ppvz_sales_commission.unwrap_or(0.0),
            )),
            Some("WB finance row without srid: commission".to_string()),
        ),
        posting_id,
    );

    if let Some(value) = entry
        .additional_payment
        .or(entry.cashback_amount)
        .filter(|value| value.abs() > f64::EPSILON)
    {
        let turnover_code = if value >= 0.0 {
            "adjustment_income"
        } else {
            "adjustment_expense"
        };
        push_row(
            &mut turnovers,
            &mut general_ledger_entries,
            build_row(
                entry,
                turnover_code,
                Some(value),
                Some("WB finance row without srid: adjustment".to_string()),
            ),
            posting_id,
        );
    }

    Ok(PostingResult {
        turnovers,
        general_ledger_entries,
    })
}
