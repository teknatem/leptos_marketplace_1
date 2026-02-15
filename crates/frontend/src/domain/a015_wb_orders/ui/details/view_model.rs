//! ViewModel for WB Orders details

use super::model::*;
use contracts::projections::p903_wb_finance_report::dto::WbFinanceReportDto;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
pub struct WbOrdersDetailsVm {
    pub id: RwSignal<Option<String>>,
    pub order: RwSignal<Option<WbOrderDetailDto>>,

    pub raw_json: RwSignal<Option<String>>,
    pub raw_json_loaded: RwSignal<bool>,
    pub raw_json_loading: RwSignal<bool>,

    pub finance_reports: RwSignal<Vec<WbFinanceReportDto>>,
    pub finance_reports_loaded: RwSignal<bool>,
    pub finance_reports_loading: RwSignal<bool>,
    pub finance_reports_error: RwSignal<Option<String>>,

    pub wb_sales: RwSignal<Vec<WbSalesListItemDto>>,
    pub wb_sales_loaded: RwSignal<bool>,
    pub wb_sales_loading: RwSignal<bool>,
    pub wb_sales_error: RwSignal<Option<String>>,

    pub marketplace_product_info: RwSignal<Option<MarketplaceProductInfo>>,
    pub nomenclature_info: RwSignal<Option<NomenclatureInfo>>,
    pub base_nomenclature_info: RwSignal<Option<NomenclatureInfo>>,
    pub connection_info: RwSignal<Option<ConnectionInfo>>,
    pub organization_info: RwSignal<Option<OrganizationInfo>>,
    pub marketplace_info: RwSignal<Option<MarketplaceInfo>>,

    pub active_tab: RwSignal<&'static str>,
    pub loading: RwSignal<bool>,
    pub posting: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
}

impl WbOrdersDetailsVm {
    pub fn new() -> Self {
        Self {
            id: RwSignal::new(None),
            order: RwSignal::new(None),

            raw_json: RwSignal::new(None),
            raw_json_loaded: RwSignal::new(false),
            raw_json_loading: RwSignal::new(false),

            finance_reports: RwSignal::new(Vec::new()),
            finance_reports_loaded: RwSignal::new(false),
            finance_reports_loading: RwSignal::new(false),
            finance_reports_error: RwSignal::new(None),

            wb_sales: RwSignal::new(Vec::new()),
            wb_sales_loaded: RwSignal::new(false),
            wb_sales_loading: RwSignal::new(false),
            wb_sales_error: RwSignal::new(None),

            marketplace_product_info: RwSignal::new(None),
            nomenclature_info: RwSignal::new(None),
            base_nomenclature_info: RwSignal::new(None),
            connection_info: RwSignal::new(None),
            organization_info: RwSignal::new(None),
            marketplace_info: RwSignal::new(None),

            active_tab: RwSignal::new("general"),
            loading: RwSignal::new(false),
            posting: RwSignal::new(false),
            error: RwSignal::new(None),
        }
    }

    pub fn is_posted(&self) -> Signal<bool> {
        let order = self.order;
        Signal::derive(move || order.get().map(|s| s.metadata.is_posted).unwrap_or(false))
    }

    pub fn document_no(&self) -> Signal<String> {
        let order = self.order;
        Signal::derive(move || {
            order
                .get()
                .map(|s| s.header.document_no.clone())
                .unwrap_or_default()
        })
    }

    pub fn finance_reports_count(&self) -> Signal<usize> {
        let reports = self.finance_reports;
        Signal::derive(move || reports.get().len())
    }

    pub fn wb_sales_count(&self) -> Signal<usize> {
        let wb_sales = self.wb_sales;
        Signal::derive(move || wb_sales.get().len())
    }

    pub fn set_tab(&self, tab: &'static str) {
        self.active_tab.set(tab);
    }

    pub fn load(&self, id: String) {
        let vm = self.clone();
        vm.id.set(Some(id.clone()));
        vm.loading.set(true);
        vm.error.set(None);

        spawn_local(async move {
            match fetch_by_id(&id).await {
                Ok(data) => {
                    vm.order.set(Some(data.clone()));
                    vm.load_related_data(&data);
                    vm.load_finance_reports();
                    vm.load_wb_sales();
                    vm.loading.set(false);
                }
                Err(e) => {
                    vm.error.set(Some(e));
                    vm.loading.set(false);
                }
            }
        });
    }

    fn load_related_data(&self, data: &WbOrderDetailDto) {
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

        if let Some(ref base_nom_ref) = data.base_nomenclature_ref {
            let base_nom_ref = base_nom_ref.clone();
            let base_nom_info = self.base_nomenclature_info;
            spawn_local(async move {
                if let Ok(info) = fetch_nomenclature(&base_nom_ref).await {
                    base_nom_info.set(Some(info));
                }
            });
        } else {
            self.base_nomenclature_info.set(None);
        }

        let conn_id = data.header.connection_id.clone();
        let conn_info = self.connection_info;
        spawn_local(async move {
            if let Ok(info) = fetch_connection(&conn_id).await {
                conn_info.set(Some(info));
            }
        });

        let org_id = data.header.organization_id.clone();
        let org_info = self.organization_info;
        spawn_local(async move {
            if let Ok(info) = fetch_organization(&org_id).await {
                org_info.set(Some(info));
            }
        });

        let mp_id = data.header.marketplace_id.clone();
        let mp_info = self.marketplace_info;
        spawn_local(async move {
            if let Ok(info) = fetch_marketplace(&mp_id).await {
                mp_info.set(Some(info));
            }
        });
    }

    pub fn load_raw_json(&self) {
        if self.raw_json_loaded.get() || self.raw_json_loading.get() {
            return;
        }
        let Some(order) = self.order.get() else {
            return;
        };

        let raw_payload_ref = order.source_meta.raw_payload_ref.clone();
        let vm = self.clone();
        vm.raw_json_loading.set(true);

        spawn_local(async move {
            match fetch_raw_json(&raw_payload_ref).await {
                Ok(json) => {
                    vm.raw_json.set(Some(json));
                    vm.raw_json_loaded.set(true);
                }
                Err(e) => leptos::logging::log!("Failed to load raw JSON: {}", e),
            }
            vm.raw_json_loading.set(false);
        });
    }

    pub fn load_finance_reports(&self) {
        if self.finance_reports_loaded.get_untracked()
            || self.finance_reports_loading.get_untracked()
        {
            return;
        }
        let Some(order) = self.order.get_untracked() else {
            return;
        };

        let srid = order.header.document_no.clone();
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
                Err(e) => vm.finance_reports_error.set(Some(e)),
            }
            vm.finance_reports_loading.set(false);
        });
    }

    pub fn load_wb_sales(&self) {
        if self.wb_sales_loaded.get_untracked() || self.wb_sales_loading.get_untracked() {
            return;
        }
        let Some(order) = self.order.get_untracked() else {
            return;
        };

        let document_no = order.header.document_no.clone();
        if document_no.is_empty() {
            return;
        }

        let vm = self.clone();
        vm.wb_sales_loading.set(true);
        vm.wb_sales_error.set(None);

        spawn_local(async move {
            match fetch_wb_sales(&document_no).await {
                Ok(items) => {
                    vm.wb_sales.set(items);
                    vm.wb_sales_loaded.set(true);
                }
                Err(e) => vm.wb_sales_error.set(Some(e)),
            }
            vm.wb_sales_loading.set(false);
        });
    }

    pub fn post(&self) {
        let Some(id) = self.id.get() else {
            return;
        };
        let vm = self.clone();
        vm.posting.set(true);

        spawn_local(async move {
            if let Err(e) = post_document(&id).await {
                leptos::logging::log!("Error posting: {}", e);
            } else {
                vm.reload().await;
            }
            vm.posting.set(false);
        });
    }

    pub fn unpost(&self) {
        let Some(id) = self.id.get() else {
            return;
        };
        let vm = self.clone();
        vm.posting.set(true);

        spawn_local(async move {
            if let Err(e) = unpost_document(&id).await {
                leptos::logging::log!("Error unposting: {}", e);
            } else {
                vm.reload().await;
            }
            vm.posting.set(false);
        });
    }

    async fn reload(&self) {
        let Some(id) = self.id.get() else {
            return;
        };
        if let Ok(data) = fetch_by_id(&id).await {
            self.order.set(Some(data.clone()));
            self.load_related_data(&data);
        }
    }
}

impl Default for WbOrdersDetailsVm {
    fn default() -> Self {
        Self::new()
    }
}
