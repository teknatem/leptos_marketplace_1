use super::model::*;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
pub struct WbReturnsClaimsDetailsVm {
    pub id: RwSignal<Option<String>>,
    pub item: RwSignal<Option<WbReturnsClaimsDetailDto>>,
    pub active_tab: RwSignal<&'static str>,
    pub loading: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
    pub raw_json: RwSignal<Option<String>>,
}

impl WbReturnsClaimsDetailsVm {
    pub fn new() -> Self {
        Self {
            id: RwSignal::new(None),
            item: RwSignal::new(None),
            active_tab: RwSignal::new("general"),
            loading: RwSignal::new(false),
            error: RwSignal::new(None),
            raw_json: RwSignal::new(None),
        }
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
                    // Build a pretty-printed JSON for the JSON tab
                    let json_str = serde_json::to_string_pretty(&data).ok();
                    vm.raw_json.set(json_str);
                    vm.item.set(Some(data));
                    vm.loading.set(false);
                }
                Err(e) => {
                    vm.error.set(Some(e));
                    vm.loading.set(false);
                }
            }
        });
    }

    pub fn claim_id(&self) -> Signal<String> {
        let item = self.item;
        Signal::derive(move || {
            item.get()
                .map(|d| d.claim_id.clone())
                .unwrap_or_default()
        })
    }
}

impl Default for WbReturnsClaimsDetailsVm {
    fn default() -> Self {
        Self::new()
    }
}
