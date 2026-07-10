//! ViewModel for WB Sales Funnel Daily details
//!
//! Contains reactive state and commands.

use super::model::*;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::list_utils::sort_list;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// ViewModel for WB Sales Funnel Daily details form
#[derive(Clone)]
pub struct WbSalesFunnelDailyDetailsVm {
    /// Tabs / navigation context (open related documents, update tab title).
    pub tabs: AppGlobalContext,

    // === Entity ID ===
    pub id: RwSignal<Option<String>>,

    // === Main data (loaded from API) ===
    pub doc: RwSignal<Option<DetailsDto>>,

    // === UI State ===
    pub active_tab: RwSignal<&'static str>,
    pub loading: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,

    // === Tab-local UI state ===
    pub lines_sort_field: RwSignal<String>,
    pub lines_sort_ascending: RwSignal<bool>,
}

impl WbSalesFunnelDailyDetailsVm {
    /// Create a new ViewModel instance
    pub fn new(tabs: AppGlobalContext) -> Self {
        Self {
            tabs,
            id: RwSignal::new(None),
            doc: RwSignal::new(None),

            active_tab: RwSignal::new("general"),
            loading: RwSignal::new(true),
            error: RwSignal::new(None),

            lines_sort_field: RwSignal::new("title".to_string()),
            lines_sort_ascending: RwSignal::new(true),
        }
    }

    // ── Derived signals ─────────────────────────────────────────────────────

    /// Header title (`h1`): "Воронка WB {document_no} от {date}".
    pub fn header_title(&self) -> Signal<String> {
        let doc = self.doc;
        Signal::derive(move || {
            doc.get()
                .map(|d| {
                    format!(
                        "Воронка WB {} от {}",
                        d.document_no,
                        fmt_date(&d.document_date)
                    )
                })
                .unwrap_or_else(|| "Воронка WB".to_string())
        })
    }

    /// Tab / favorite label: "Воронка WB {document_date}".
    pub fn tab_label(&self) -> Signal<String> {
        let doc = self.doc;
        Signal::derive(move || {
            doc.get()
                .map(|d| format!("Воронка WB {}", d.document_date))
                .unwrap_or_else(|| "Воронка WB".to_string())
        })
    }

    /// Lines sorted by the current sort field/direction.
    pub fn sorted_lines(&self) -> Signal<Vec<LineDto>> {
        let doc = self.doc;
        let field = self.lines_sort_field;
        let ascending = self.lines_sort_ascending;
        Signal::derive(move || {
            let mut lines = doc.get().map(|d| d.lines).unwrap_or_default();
            sort_list(&mut lines, &field.get(), ascending.get());
            lines
        })
    }

    pub fn set_tab(&self, tab: &'static str) {
        self.active_tab.set(tab);
    }

    pub fn toggle_lines_sort(&self, field: &'static str) {
        if self.lines_sort_field.get_untracked() == field {
            self.lines_sort_ascending.update(|value| *value = !*value);
        } else {
            self.lines_sort_field.set(field.to_string());
            self.lines_sort_ascending.set(true);
        }
    }

    // ── Loaders ──────────────────────────────────────────────────────────────

    /// Load main document data
    pub fn load(&self, id: String) {
        let vm = self.clone();
        vm.id.set(Some(id.clone()));
        vm.loading.set(true);
        vm.error.set(None);

        spawn_local(async move {
            match fetch_by_id(&id).await {
                Ok(data) => {
                    let tab_id = id.clone();
                    let title = format!("Воронка WB {}", data.document_date);
                    vm.tabs.update_tab_title(
                        &format!("a036_wb_sales_funnel_daily_details_{tab_id}"),
                        &title,
                    );
                    vm.doc.set(Some(data));
                    vm.loading.set(false);
                }
                Err(e) => {
                    vm.error.set(Some(e));
                    vm.loading.set(false);
                }
            }
        });
    }

    // ── Navigation ──────────────────────────────────────────────────────────────

    pub fn open_nomenclature(&self, nom_ref: String) {
        if nom_ref.is_empty() {
            return;
        }
        self.tabs.open_tab(
            &format!("a004_nomenclature_details_{}", nom_ref),
            "Номенклатура",
        );
    }

    pub fn open_product(&self, product_ref: String) {
        if product_ref.is_empty() {
            return;
        }
        self.tabs.open_tab(
            &format!("a007_marketplace_product_details_{}", product_ref),
            "Товар маркетплейса",
        );
    }
}
