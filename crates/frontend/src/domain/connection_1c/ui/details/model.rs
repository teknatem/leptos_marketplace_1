use contracts::domain::connection_1c::aggregate::{
    Connection1CDatabase, Connection1CDatabaseForm, ConnectionTestResult,
};

#[derive(Clone, Debug)]
pub struct FormState {
    pub id: Option<String>,
    pub description: String,
    pub url: String,
    pub comment: String,
    pub login: String,
    pub password: String,
    pub is_primary: bool,
}

impl Default for FormState {
    fn default() -> Self {
        Self {
            id: None,
            description: String::new(),
            url: String::new(),
            comment: String::new(),
            login: String::new(),
            password: String::new(),
            is_primary: false,
        }
    }
}

impl From<Connection1CDatabaseForm> for FormState {
    fn from(form: Connection1CDatabaseForm) -> Self {
        Self {
            id: form.id,
            description: form.description,
            url: form.url,
            comment: form.comment.unwrap_or_default(),
            login: form.login,
            password: form.password,
            is_primary: form.is_primary,
        }
    }
}

impl From<FormState> for Connection1CDatabaseForm {
    fn from(state: FormState) -> Self {
        Self {
            id: state.id,
            description: state.description,
            url: state.url,
            comment: if state.comment.is_empty() {
                None
            } else {
                Some(state.comment)
            },
            login: state.login,
            password: state.password,
            is_primary: state.is_primary,
        }
    }
}

impl From<Connection1CDatabase> for FormState {
    fn from(c: Connection1CDatabase) -> Self {
        Self {
            id: Some(c.base.id.0.to_string()),
            description: c.description,
            url: c.url,
            comment: c.comment.unwrap_or_default(),
            login: c.login,
            password: c.password,
            is_primary: c.is_primary,
        }
    }
}

pub fn validate_form(state: &FormState) -> Result<(), String> {
    if state.description.trim().is_empty() {
        return Err("Description is required".to_string());
    }
    if state.url.trim().is_empty() {
        return Err("URL is required".to_string());
    }
    if state.login.trim().is_empty() {
        return Err("Login is required".to_string());
    }
    Ok(())
}

fn api_base() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return String::new(),
    };
    let location = window.location();
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    format!("{}//{}:3000", protocol, hostname)
}

pub async fn fetch_by_id(id: i32) -> Result<Connection1CDatabase, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/connection_1c/{}", api_base(), id);
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

    if resp.status() == 404 {
        return Err("Not found".to_string());
    }
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Connection1CDatabase = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

pub async fn save_form(state: &FormState) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let form = Connection1CDatabaseForm::from(state.clone());
    let json_data = serde_json::to_string(&form).map_err(|e| format!("{e}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    let body = wasm_bindgen::JsValue::from_str(&json_data);
    opts.set_body(&body);

    let url = format!("{}/api/connection_1c", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
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
    Ok(())
}

pub async fn test_connection(state: &FormState) -> Result<ConnectionTestResult, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let form = Connection1CDatabaseForm::from(state.clone());
    let json_data = serde_json::to_string(&form).map_err(|e| format!("{e}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    let body = wasm_bindgen::JsValue::from_str(&json_data);
    opts.set_body(&body);

    let url = format!("{}/api/connection_1c/test", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
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
    let result: ConnectionTestResult =
        serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    Ok(result)
}
