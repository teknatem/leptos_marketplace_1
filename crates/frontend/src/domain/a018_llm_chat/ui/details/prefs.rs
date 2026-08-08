//! Настройки отображения ленты чата (a018), общие для всех чатов.
//!
//! Технические поля (токены, пилюли инструментов, мета модель/время/уверенность)
//! по умолчанию скрыты: обычному пользователю нужен разговор, а не телеметрия.
//! Включаются в «Настройках чата» и живут в `localStorage` — глобально, без
//! привязки к конкретному чату (человек один раз решает, как ему смотреть).

use serde::{Deserialize, Serialize};
use web_sys::window;

const STORAGE_KEY: &str = "a018.chat.ui_prefs";
const CURRENT_VERSION: u32 = 1;

/// Что показывать в ленте сообщений.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ChatUiPrefs {
    pub version: u32,
    /// Строка меты под ответом: 🤖 модель • ⏱ время • 📊 уверенность • intent.
    #[serde(default)]
    pub show_meta_line: bool,
    /// 🎫 tokens внутри строки меты (суб-гейт: без меты не показывается).
    #[serde(default)]
    pub show_tokens: bool,
    /// Пилюли вызовов инструментов (`ToolCallsTrace`).
    #[serde(default)]
    pub show_tool_calls: bool,
    /// Баннер «использован расширенный навык».
    #[serde(default)]
    pub show_skill_warnings: bool,
    /// События прикрепления контекста в ленте (`ContextRow`).
    #[serde(default = "default_true")]
    pub show_context_events: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ChatUiPrefs {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            show_meta_line: false,
            show_tokens: false,
            show_tool_calls: false,
            show_skill_warnings: false,
            // Прикрепление документа — действие пользователя, а не техника: видно всегда.
            show_context_events: true,
        }
    }
}

impl ChatUiPrefs {
    /// Прочитать настройки. Нет storage / нет записи / битый JSON — молча дефолт.
    pub fn load() -> Self {
        let Some(storage) = window().and_then(|w| w.local_storage().ok().flatten()) else {
            return Self::default();
        };
        match storage.get_item(STORAGE_KEY) {
            Ok(Some(raw)) => serde_json::from_str(&raw).unwrap_or_default(),
            _ => Self::default(),
        }
    }

    /// Сохранить настройки. Ошибки глотаем — UX не должен ломаться из-за storage.
    pub fn save(&self) {
        let Some(storage) = window().and_then(|w| w.local_storage().ok().flatten()) else {
            return;
        };
        if let Ok(raw) = serde_json::to_string(self) {
            let _ = storage.set_item(STORAGE_KEY, &raw);
        }
    }
}
