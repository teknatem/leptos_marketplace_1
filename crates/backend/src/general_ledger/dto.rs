use contracts::general_ledger::GeneralLedgerEntryDto;
use contracts::shared::analytics::TurnoverLayer;

use crate::general_ledger::repository::Model;
use crate::general_ledger::turnover_registry::get_turnover_class;

pub fn entry_to_dto(row: Model) -> GeneralLedgerEntryDto {
    let turnover = get_turnover_class(&row.turnover_code);
    let turnover_name = turnover
        .map(|class| class.name.to_string())
        .unwrap_or_else(|| row.turnover_code.clone());
    let comment = turnover
        .map(|class| class.journal_comment.to_string())
        .unwrap_or_default();

    GeneralLedgerEntryDto {
        id: row.id,
        entry_date: row.entry_date,
        layer: TurnoverLayer::from_str(&row.layer).unwrap_or(TurnoverLayer::Oper),
        connection_mp_ref: row.connection_mp_ref,
        registrator_type: row.registrator_type,
        registrator_ref: row.registrator_ref,
        order_id: row.order_id,
        debit_account: row.debit_account,
        credit_account: row.credit_account,
        amount: row.amount,
        qty: row.qty,
        turnover_code: row.turnover_code,
        turnover_name,
        resource_table: row.resource_table,
        resource_field: row.resource_field,
        resource_sign: row.resource_sign,
        created_at: row.created_at,
        comment,
    }
}
