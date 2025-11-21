use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FormSettings {
    pub form_key: String,
    pub settings_json: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveSettingsRequest {
    pub form_key: String,
    pub settings: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveSettingsResponse {
    pub success: bool,
    pub message: String,
}

