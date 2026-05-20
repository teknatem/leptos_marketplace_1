//! Journal tab - General Ledger entries created by the document.

use super::super::view_model::WbSalesDetailsVm;
use crate::general_ledger::ui::{
    document_general_ledger_entries_nav_id, DocumentGeneralLedgerEntries,
};
use leptos::prelude::*;

#[component]
pub fn JournalTab(vm: WbSalesDetailsVm) -> impl IntoView {
    let vm_entries = vm.clone();
    let vm_loading = vm.clone();
    let vm_error = vm;
    let entries = Signal::derive(move || vm_entries.general_ledger_entries.get());
    let loading = Signal::derive(move || vm_loading.general_ledger_entries_loading.get());
    let error = Signal::derive(move || vm_error.general_ledger_entries_error.get());

    view! {
        <DocumentGeneralLedgerEntries
            entries=entries
            loading=loading
            error=error
            nav_id=document_general_ledger_entries_nav_id("a012_wb_sales")
            title="Журнал операций"
            empty_message="Записи General Ledger не найдены. Проведите документ для формирования проводок."
        />
    }
}
