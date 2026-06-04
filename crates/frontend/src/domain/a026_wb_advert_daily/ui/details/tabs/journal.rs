//! Journal tab - General Ledger entries created by the document.

use super::super::view_model::WbAdvertDailyDetailsVm;
use crate::general_ledger::ui::{
    document_general_ledger_entries_nav_id, DocumentGeneralLedgerEntries,
};
use leptos::prelude::*;

#[component]
pub fn JournalTab(vm: WbAdvertDailyDetailsVm) -> impl IntoView {
    let entries = Signal::derive({
        let s = vm.general_ledger_entries;
        move || s.get()
    });
    let loading = Signal::derive({
        let s = vm.general_ledger_entries_loading;
        move || s.get()
    });
    let error = Signal::derive({
        let s = vm.general_ledger_entries_error;
        move || s.get()
    });

    view! {
        <DocumentGeneralLedgerEntries
            entries=entries
            loading=loading
            error=error
            nav_id=document_general_ledger_entries_nav_id("a026_wb_advert_daily")
            title="Журнал операций"
            empty_message="Записи General Ledger не найдены. Проведите документ для формирования проводок."
        />
    }
}
