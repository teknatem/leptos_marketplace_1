use crate::shared::picker_aggregate::{
    AggregatePickerResult, GenericAggregatePicker, TableDisplayable,
};
use contracts::domain::a002_organization::aggregate::Organization;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;

/// Элемент для пикера организаций
#[derive(Clone, Debug)]
pub struct OrganizationPickerItem {
    pub id: String,
    pub code: String,
    pub description: String,
    pub full_name: String,
    pub inn: String,
}

impl From<Organization> for OrganizationPickerItem {
    fn from(org: Organization) -> Self {
        Self {
            id: org.base.id.as_string(),
            code: org.base.code,
            description: org.base.description,
            full_name: org.full_name,
            inn: org.inn,
        }
    }
}

impl AggregatePickerResult for OrganizationPickerItem {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn display_name(&self) -> String {
        self.description.clone()
    }
}

impl TableDisplayable for OrganizationPickerItem {
    fn code(&self) -> String {
        self.code.clone()
    }

    fn description(&self) -> String {
        format!("{} (ИНН: {})", self.description, self.inn)
    }
}

/// Компонент для выбора организации через универсальный пикер
#[component]
pub fn OrganizationPicker<F, G>(
    /// ID организации, которая должна быть выбрана при открытии
    initial_selected_id: Option<String>,
    /// Callback при подтверждении выбора
    on_confirm: F,
    /// Callback при отмене
    on_cancel: G,
) -> impl IntoView
where
    F: Fn(Option<OrganizationPickerItem>) + 'static + Clone + Send,
    G: Fn(()) + 'static + Clone + Send,
{
    let (items, set_items) = signal::<Vec<OrganizationPickerItem>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);

    // Загрузка организаций при монтировании
    wasm_bindgen_futures::spawn_local(async move {
        match fetch_organizations().await {
            Ok(orgs) => {
                let picker_items: Vec<OrganizationPickerItem> =
                    orgs.into_iter().map(Into::into).collect();
                set_items.set(picker_items);
                set_error.set(None);
            }
            Err(e) => set_error.set(Some(e)),
        }
        set_loading.set(false);
    });

    view! {
        <GenericAggregatePicker
            items=items
            error=error
            loading=loading
            initial_selected_id=initial_selected_id
            on_confirm=on_confirm
            on_cancel=on_cancel
            title="Выбор организации".to_string()
        />
    }
}

async fn fetch_organizations() -> Result<Vec<Organization>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let api_base = || {
        let window = web_sys::window().expect("no window");
        let location = window.location();
        let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
        let hostname = location
            .hostname()
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        format!("{}//{}:3000", protocol, hostname)
    };

    let url = format!("{}/api/organization", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Vec<Organization> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
