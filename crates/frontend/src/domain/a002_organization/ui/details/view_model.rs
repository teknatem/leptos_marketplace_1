use super::model;
use contracts::domain::a002_organization::aggregate::OrganizationDto;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use std::rc::Rc;

/// ViewModel for Organization details form
#[derive(Clone)]
pub struct OrganizationDetailsViewModel {
    pub form: RwSignal<OrganizationDto>,
    pub error: RwSignal<Option<String>>,
}

impl OrganizationDetailsViewModel {
    pub fn new() -> Self {
        Self {
            form: RwSignal::new(OrganizationDto::default()),
            error: RwSignal::new(None),
        }
    }

    pub fn is_edit_mode(&self) -> impl Fn() -> bool + '_ {
        move || self.form.get().id.is_some()
    }

    pub fn is_form_valid(&self) -> impl Fn() -> bool + '_ {
        move || {
            let f = self.form.get();
            !f.description.trim().is_empty()
                && !f.full_name.trim().is_empty()
                && !f.inn.trim().is_empty()
        }
    }

    /// Load form data from server if ID is provided
    pub fn load_if_needed(&self, id: Option<String>) {
        if let Some(existing_id) = id {
            let form = self.form;
            let error = self.error;
            wasm_bindgen_futures::spawn_local(async move {
                match model::fetch_by_id(existing_id).await {
                    Ok(aggregate) => {
                        // Convert aggregate to dto
                        let dto = OrganizationDto {
                            id: Some(aggregate.base.id.as_string()),
                            code: Some(aggregate.base.code),
                            description: aggregate.base.description,
                            full_name: aggregate.full_name,
                            inn: aggregate.inn,
                            kpp: aggregate.kpp,
                            comment: aggregate.base.comment,
                        };
                        form.set(dto);
                    }
                    Err(e) => error.set(Some(format!("Ошибка загрузки: {}", e))),
                }
            });
        }
    }

    /// Save form data to server
    pub fn save_command(&self, on_saved: Rc<dyn Fn(())>) {
        let current = self.form.get();

        // Validate
        if current.description.trim().is_empty() {
            self.error
                .set(Some("Наименование обязательно для заполнения".to_string()));
            return;
        }
        if current.full_name.trim().is_empty() {
            self.error
                .set(Some("Полное наименование обязательно для заполнения".to_string()));
            return;
        }
        if current.inn.trim().is_empty() {
            self.error
                .set(Some("ИНН обязателен для заполнения".to_string()));
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
