//! ViewModel for WB Sales details
//!
//! Contains reactive state, commands, and lazy loading logic.

use super::model::*;
use contracts::projections::p903_wb_finance_report::dto::WbFinanceReportDto;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// ViewModel for WB Sales details form
#[derive(Clone)]
pub struct WbSalesDetailsVm {
    // === Entity ID ===
    pub id: RwSignal<Option<String>>,

    // === Main data (loaded from API) ===
    pub sale: RwSignal<Option<WbSalesDetailDto>>,

    // === Related data (lazy loaded) ===
    pub raw_json: RwSignal<Option<String>>,
    pub raw_json_loaded: RwSignal<bool>,
    pub raw_json_loading: RwSignal<bool>,

    pub projections: RwSignal<Option<serde_json::Value>>,
    pub projections_loaded: RwSignal<bool>,
    pub projections_loading: RwSignal<bool>,

    pub finance_reports: RwSignal<Vec<WbFinanceReportDto>>,
    pub finance_reports_loaded: RwSignal<bool>,
    pub finance_reports_loading: RwSignal<bool>,
    pub finance_reports_error: RwSignal<Option<String>>,

    pub marketplace_product_info: RwSignal<Option<MarketplaceProductInfo>>,
    pub nomenclature_info: RwSignal<Option<NomenclatureInfo>>,

    // === UI State ===
    pub active_tab: RwSignal<&'static str>,
    pub loading: RwSignal<bool>,
    pub posting: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
}

impl WbSalesDetailsVm {
    /// Create a new ViewModel instance
    pub fn new() -> Self {
        Self {
            id: RwSignal::new(None),
            sale: RwSignal::new(None),

            raw_json: RwSignal::new(None),
            raw_json_loaded: RwSignal::new(false),
            raw_json_loading: RwSignal::new(false),

            projections: RwSignal::new(None),
            projections_loaded: RwSignal::new(false),
            projections_loading: RwSignal::new(false),

            finance_reports: RwSignal::new(Vec::new()),
            finance_reports_loaded: RwSignal::new(false),
            finance_reports_loading: RwSignal::new(false),
            finance_reports_error: RwSignal::new(None),

            marketplace_product_info: RwSignal::new(None),
            nomenclature_info: RwSignal::new(None),

            active_tab: RwSignal::new("general"),
            loading: RwSignal::new(false),
            posting: RwSignal::new(false),
            error: RwSignal::new(None),
        }
    }

    /// Check if document is posted
    pub fn is_posted(&self) -> Signal<bool> {
        let sale = self.sale;
        Signal::derive(move || sale.get().map(|s| s.metadata.is_posted).unwrap_or(false))
    }

    /// Get document number for display
    pub fn document_no(&self) -> Signal<String> {
        let sale = self.sale;
        Signal::derive(move || {
            sale.get()
                .map(|s| s.header.document_no.clone())
                .unwrap_or_default()
        })
    }

    /// Get projections count for badge
    pub fn projections_count(&self) -> Signal<usize> {
        let projections = self.projections;
        Signal::derive(move || {
            projections
                .get()
                .as_ref()
                .map(|p| {
                    let p900_len = p["p900_sales_register"]
                        .as_array()
                        .map(|a| a.len())
                        .unwrap_or(0);
                    let p904_len = p["p904_sales_data"]
                        .as_array()
                        .map(|a| a.len())
                        .unwrap_or(0);
                    p900_len + p904_len
                })
                .unwrap_or(0)
        })
    }

    /// Get finance reports count for badge
    pub fn finance_reports_count(&self) -> Signal<usize> {
        let reports = self.finance_reports;
        Signal::derive(move || reports.get().len())
    }

    /// Set active tab
    pub fn set_tab(&self, tab: &'static str) {
        self.active_tab.set(tab);
    }

    /// Load main document data
    pub fn load(&self, id: String) {
        let vm = self.clone();
        vm.id.set(Some(id.clone()));
        vm.loading.set(true);
        vm.error.set(None);

        spawn_local(async move {
            match fetch_by_id(&id).await {
                Ok(data) => {
                    // Load related data (marketplace product, nomenclature)
                    vm.load_related_data(&data);
                    vm.sale.set(Some(data));
                    vm.loading.set(false);
                }
                Err(e) => {
                    vm.error.set(Some(e));
                    vm.loading.set(false);
                }
            }
        });
    }

    /// Load related data (marketplace product, nomenclature info)
    fn load_related_data(&self, data: &WbSalesDetailDto) {
        // Load marketplace product info
        if let Some(ref mp_ref) = data.marketplace_product_ref {
            let mp_ref = mp_ref.clone();
            let mp_info = self.marketplace_product_info;
            spawn_local(async move {
                if let Ok(info) = fetch_marketplace_product(&mp_ref).await {
                    mp_info.set(Some(info));
                }
            });
        } else {
            self.marketplace_product_info.set(None);
        }

        // Load nomenclature info
        if let Some(ref nom_ref) = data.nomenclature_ref {
            let nom_ref = nom_ref.clone();
            let nom_info = self.nomenclature_info;
            spawn_local(async move {
                if let Ok(info) = fetch_nomenclature(&nom_ref).await {
                    nom_info.set(Some(info));
                }
            });
        } else {
            self.nomenclature_info.set(None);
        }
    }

    /// Load raw JSON (lazy, for "json" tab)
    pub fn load_raw_json(&self) {
        if self.raw_json_loaded.get() || self.raw_json_loading.get() {
            return;
        }

        let Some(sale) = self.sale.get() else {
            return;
        };

        let raw_payload_ref = sale.source_meta.raw_payload_ref.clone();
        let vm = self.clone();
        vm.raw_json_loading.set(true);

        spawn_local(async move {
            match fetch_raw_json(&raw_payload_ref).await {
                Ok(json) => {
                    vm.raw_json.set(Some(json));
                    vm.raw_json_loaded.set(true);
                }
                Err(e) => {
                    leptos::logging::log!("Failed to load raw JSON: {}", e);
                }
            }
            vm.raw_json_loading.set(false);
        });
    }

    /// Load projections (lazy, for "projections" tab)
    pub fn load_projections(&self) {
        if self.projections_loaded.get() || self.projections_loading.get() {
            return;
        }

        let Some(id) = self.id.get() else {
            return;
        };

        let vm = self.clone();
        vm.projections_loading.set(true);

        spawn_local(async move {
            match fetch_projections(&id).await {
                Ok(proj) => {
                    vm.projections.set(Some(proj));
                    vm.projections_loaded.set(true);
                }
                Err(e) => {
                    leptos::logging::log!("Failed to load projections: {}", e);
                }
            }
            vm.projections_loading.set(false);
        });
    }

    /// Load finance reports (lazy, for "links" or "line" tabs)
    pub fn load_finance_reports(&self) {
        if self.finance_reports_loaded.get() || self.finance_reports_loading.get() {
            return;
        }

        let Some(sale) = self.sale.get() else {
            return;
        };

        let srid = sale.header.document_no.clone();
        if srid.is_empty() {
            return;
        }

        let vm = self.clone();
        vm.finance_reports_loading.set(true);
        vm.finance_reports_error.set(None);

        spawn_local(async move {
            match fetch_finance_reports(&srid).await {
                Ok(reports) => {
                    vm.finance_reports.set(reports);
                    vm.finance_reports_loaded.set(true);
                }
                Err(e) => {
                    vm.finance_reports_error.set(Some(e));
                }
            }
            vm.finance_reports_loading.set(false);
        });
    }

    /// Post document (проведение)
    pub fn post(&self) {
        let Some(id) = self.id.get() else {
            return;
        };

        let vm = self.clone();
        vm.posting.set(true);

        spawn_local(async move {
            match post_document(&id).await {
                Ok(()) => {
                    // Reload document data
                    vm.reload().await;
                }
                Err(e) => {
                    leptos::logging::log!("Error posting: {}", e);
                }
            }
            vm.posting.set(false);
        });
    }

    /// Unpost document (отмена проведения)
    pub fn unpost(&self) {
        let Some(id) = self.id.get() else {
            return;
        };

        let vm = self.clone();
        vm.posting.set(true);

        spawn_local(async move {
            match unpost_document(&id).await {
                Ok(()) => {
                    // Reload document data
                    vm.reload().await;
                }
                Err(e) => {
                    leptos::logging::log!("Error unposting: {}", e);
                }
            }
            vm.posting.set(false);
        });
    }

    /// Reload document and projections after post/unpost
    async fn reload(&self) {
        let Some(id) = self.id.get() else {
            return;
        };

        // Reload main data
        if let Ok(data) = fetch_by_id(&id).await {
            self.load_related_data(&data);
            self.sale.set(Some(data));
        }

        // Reload projections if already loaded
        if self.projections_loaded.get() {
            if let Ok(proj) = fetch_projections(&id).await {
                self.projections.set(Some(proj));
            }
        }
    }
}

impl Default for WbSalesDetailsVm {
    fn default() -> Self {
        Self::new()
    }
}
