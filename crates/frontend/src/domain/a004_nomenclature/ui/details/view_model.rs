use super::model;
use contracts::domain::a004_nomenclature::aggregate::NomenclatureDto;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use std::rc::Rc;

/// ViewModel for Nomenclature details form
#[derive(Clone)]
pub struct NomenclatureDetailsViewModel {
    pub form: RwSignal<NomenclatureDto>,
    pub error: RwSignal<Option<String>>,
}

impl NomenclatureDetailsViewModel {
    pub fn new() -> Self {
        Self {
            form: RwSignal::new(NomenclatureDto::default()),
            error: RwSignal::new(None),
        }
    }

    pub fn is_edit_mode(&self) -> impl Fn() -> bool + '_ {
        move || self.form.get().id.is_some()
    }

    pub fn is_form_valid(&self) -> impl Fn() -> bool + '_ {
        move || Self::validate_form(&self.form.get()).is_ok()
    }

    fn validate_form(dto: &NomenclatureDto) -> Result<(), &'static str> {
        if dto.description.trim().is_empty() {
            return Err("Наименование обязательно для заполнения");
        }
        Ok(())
    }

    /// Load form data from server if ID is provided
    pub fn load_if_needed(&self, id: Option<String>) {
        let Some(existing_id) = id else {
            return;
        };

        let this = self.clone();
        leptos::task::spawn_local(async move {
            match super::model::fetch_by_id(existing_id).await {
                Ok(item) => {
                    this.form.update(|f| {
                        f.id = Some(item.base.id.as_string());
                        f.code = Some(item.base.code);
                        f.description = item.base.description;
                        f.full_description = Some(item.full_description);
                        f.comment = item.base.comment;
                        f.is_folder = item.is_folder;
                        f.parent_id = item.parent_id;
                        f.article = Some(item.article);
                        f.updated_at = Some(item.base.metadata.updated_at);
                        // Load dimension fields
                        f.dim1_category = Some(item.dim1_category);
                        f.dim2_line = Some(item.dim2_line);
                        f.dim3_model = Some(item.dim3_model);
                        f.dim4_format = Some(item.dim4_format);
                        f.dim5_sink = Some(item.dim5_sink);
                        f.dim6_size = Some(item.dim6_size);
                    });
                }
                Err(e) => this.error.set(Some(e)),
            }
        });
    }

    pub fn save_command(&self, on_saved: Rc<dyn Fn(())>) -> impl Fn() + '_ {
        move || {
            let this = self.clone();
            let dto = this.form.get();
            let on_saved_cb = on_saved.clone();
            leptos::task::spawn_local(async move {
                match model::save_form(dto).await {
                    Ok(_) => on_saved_cb(()),
                    Err(e) => this.error.set(Some(e)),
                }
            });
        }
    }
}
