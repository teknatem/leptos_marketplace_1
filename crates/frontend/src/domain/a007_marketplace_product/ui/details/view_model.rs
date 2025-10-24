use super::model;
use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProductDto;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use std::rc::Rc;

/// ViewModel for MarketplaceProduct details form
#[derive(Clone)]
pub struct MarketplaceProductDetailsViewModel {
    pub form: RwSignal<MarketplaceProductDto>,
    pub error: RwSignal<Option<String>>,
}

impl MarketplaceProductDetailsViewModel {
    pub fn new() -> Self {
        Self {
            form: RwSignal::new(MarketplaceProductDto::default()),
            error: RwSignal::new(None),
        }
    }

    pub fn is_edit_mode(&self) -> impl Fn() -> bool + '_ {
        move || self.form.get().id.is_some()
    }

    pub fn is_form_valid(&self) -> impl Fn() -> bool + '_ {
        move || Self::validate_form(&self.form.get()).is_ok()
    }

    fn validate_form(dto: &MarketplaceProductDto) -> Result<(), &'static str> {
        if dto.description.trim().is_empty() {
            return Err("Описание обязательно для заполнения");
        }
        if dto.marketplace_id.trim().is_empty() {
            return Err("Маркетплейс обязателен для заполнения");
        }
        if dto.marketplace_sku.trim().is_empty() {
            return Err("SKU обязателен для заполнения");
        }
        if dto.art.trim().is_empty() {
            return Err("Артикул обязателен для заполнения");
        }
        if dto.product_name.trim().is_empty() {
            return Err("Наименование товара обязательно для заполнения");
        }
        Ok(())
    }

    /// Load form data from server if ID is provided
    pub fn load_if_needed(&self, id: Option<String>) {
        let Some(existing_id) = id else {
            return;
        };
        let form = self.form;
        let error = self.error;
        wasm_bindgen_futures::spawn_local(async move {
            let result = model::fetch_by_id(existing_id).await;
            if let Err(e) = result {
                error.set(Some(format!("Ошибка загрузки: {}", e)));
                return;
            }

            let aggregate = result.unwrap();
            let dto = MarketplaceProductDto {
                id: Some(aggregate.base.id.as_string()),
                code: Some(aggregate.base.code),
                description: aggregate.base.description,
                marketplace_id: aggregate.marketplace_id,
                connection_mp_id: aggregate.connection_mp_id,
                marketplace_sku: aggregate.marketplace_sku,
                barcode: aggregate.barcode,
                art: aggregate.art,
                product_name: aggregate.product_name,
                brand: aggregate.brand,
                category_id: aggregate.category_id,
                category_name: aggregate.category_name,
                price: aggregate.price,
                stock: aggregate.stock,
                last_update: aggregate.last_update,
                marketplace_url: aggregate.marketplace_url,
                nomenclature_id: aggregate.nomenclature_id,
                comment: aggregate.base.comment,
            };
            form.set(dto);
        });
    }

    /// Save form data to server
    pub fn save_command(&self, on_saved: Rc<dyn Fn(())>) {
        let current = self.form.get();

        if let Err(msg) = Self::validate_form(&current) {
            self.error.set(Some(msg.to_string()));
            return;
        }

        let on_saved_cb = on_saved.clone();
        let error = self.error;
        wasm_bindgen_futures::spawn_local(async move {
            match model::save_form(&current).await {
                Ok(()) => (on_saved_cb)(()),
                Err(e) => error.set(Some(e)),
            }
        });
    }
}
