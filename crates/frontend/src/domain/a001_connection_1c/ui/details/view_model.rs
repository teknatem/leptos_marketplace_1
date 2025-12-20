use super::model;
use contracts::domain::a001_connection_1c::aggregate::{
    Connection1CDatabaseDto, ConnectionTestResult,
};
use contracts::domain::common::AggregateId;
use leptos::prelude::*;

/// ViewModel for Connection1CDatabase details form
///
/// Uses simplified MVVM pattern:
/// - Form data stored directly as Connection1CDatabaseDto (no intermediate FormState)
/// - No update_* methods - use form.update() directly in view
/// - Commands for complex operations (save, load, test)
#[derive(Clone)]
pub struct Connection1CDetailsViewModel {
    pub form: RwSignal<Connection1CDatabaseDto>,
    pub error: RwSignal<Option<String>>,
    pub test_result: RwSignal<Option<ConnectionTestResult>>,
    pub is_testing: RwSignal<bool>,
}

impl Connection1CDetailsViewModel {
    pub fn new() -> Self {
        Self {
            form: RwSignal::new(Connection1CDatabaseDto::default()),
            error: RwSignal::new(None),
            test_result: RwSignal::new(None),
            is_testing: RwSignal::new(false),
        }
    }

    /// Reset form to default state
    pub fn reset_form(&self) {
        self.form.set(Connection1CDatabaseDto::default());
        self.error.set(None);
        self.test_result.set(None);
        self.is_testing.set(false);
    }

    pub fn is_edit_mode(&self) -> impl Fn() -> bool + '_ {
        move || self.form.get().id.is_some()
    }

    pub fn is_form_valid(&self) -> impl Fn() -> bool + '_ {
        move || {
            let f = self.form.get();
            !f.description.trim().is_empty()
                && !f.url.trim().is_empty()
                && !f.login.trim().is_empty()
        }
    }

    /// Load form data from server if ID is provided, otherwise reset to default
    pub fn load_or_reset(&self, id: Option<String>) {
        if let Some(existing_id) = id {
            let form = self.form;
            let error = self.error;
            wasm_bindgen_futures::spawn_local(async move {
                match model::fetch_by_id(existing_id).await {
                    Ok(aggregate) => {
                        // Convert aggregate to dto
                        let dto = Connection1CDatabaseDto {
                            id: Some(aggregate.base.id.as_string()),
                            code: Some(aggregate.base.code),
                            description: aggregate.base.description,
                            url: aggregate.url,
                            comment: aggregate.base.comment,
                            login: aggregate.login,
                            password: aggregate.password,
                            is_primary: aggregate.is_primary,
                        };
                        form.set(dto);
                    }
                    Err(e) => error.set(Some(format!("Failed to load: {}", e))),
                }
            });
        } else {
            // Создание нового - сбрасываем форму
            self.reset_form();
        }
    }

    /// Save form data to server
    pub fn save_command(&self, on_saved: Callback<()>) {
        let current = self.form.get();

        // Validate
        if current.description.trim().is_empty() {
            self.error.set(Some("Description is required".to_string()));
            return;
        }
        if current.url.trim().is_empty() {
            self.error.set(Some("URL is required".to_string()));
            return;
        }
        if current.login.trim().is_empty() {
            self.error.set(Some("Login is required".to_string()));
            return;
        }

        let error = self.error;
        wasm_bindgen_futures::spawn_local(async move {
            match model::save_form(&current).await {
                Ok(()) => on_saved.run(()),
                Err(e) => error.set(Some(e)),
            }
        });
    }

    /// Test connection with current form data
    pub fn test_command(&self) {
        self.is_testing.set(true);
        self.test_result.set(None);
        self.error.set(None);

        let current = self.form.get();
        let is_testing = self.is_testing;
        let test_result = self.test_result;
        let error = self.error;

        wasm_bindgen_futures::spawn_local(async move {
            match model::test_connection(&current).await {
                Ok(result) => {
                    test_result.set(Some(result));
                    is_testing.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Test failed: {}", e)));
                    is_testing.set(false);
                }
            }
        });
    }
}
